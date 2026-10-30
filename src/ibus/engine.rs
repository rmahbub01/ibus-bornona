//! IBus engine implementation for Bornona.

use std::ffi::{c_char, c_int, c_uint, c_void, CString};
use std::sync::atomic::{AtomicU32, Ordering};

use super::ffi::*;
use super::keycodes::{classify_key, KeyAction};
use crate::config::ConfigWatcher;
use crate::engine::{
    BornonaLayout, LookupModel, SuggestionEngine, SuggestionOrientation, TypingProcessor,
};

pub static ENGINE_NAME: &str = "bornona";
pub static COMPONENT_NAME: &str = "org.freedesktop.IBus.Bornona";

static ENGINE_COUNTER: AtomicU32 = AtomicU32::new(0);

struct EngineInstance {
    layout: BornonaLayout,
    processor: TypingProcessor,
    config_watcher: ConfigWatcher,
    lookup_table: *mut IBusLookupTable,
    lookup_model: LookupModel,
    lookup_visible: bool,
    cosmic_session: bool,
    input_content_type: Option<(u32, u32)>,
}

fn is_sensitive_input(purpose: u32, hints: u32) -> bool {
    matches!(purpose, IBUS_INPUT_PURPOSE_PASSWORD | IBUS_INPUT_PURPOSE_PIN)
        || hints & IBUS_INPUT_HINT_HIDDEN_TEXT != 0
}

#[allow(non_upper_case_globals)]
fn is_lock_toggle_key(keyval: u32) -> bool {
    matches!(keyval, IBUS_KEY_Caps_Lock | IBUS_KEY_Shift_Lock)
}

fn should_bypass_input(content_type: Option<(u32, u32)>) -> bool {
    content_type.is_some_and(|(purpose, hints)| is_sensitive_input(purpose, hints))
}

fn find_installed_icon_path() -> Option<String> {
    let home = std::env::var_os("HOME").map(std::path::PathBuf::from);
    let xdg_data_home = std::env::var_os("XDG_DATA_HOME").map(std::path::PathBuf::from);

    let candidates = [
        xdg_data_home.as_ref().map(|p| p.join("ibus-bornona/ibus-bornona.png")),
        home.as_ref().map(|p| p.join(".local/share/ibus-bornona/ibus-bornona.png")),
        Some(std::path::PathBuf::from("/usr/share/ibus-bornona/ibus-bornona.png")),
        Some(std::path::PathBuf::from("/usr/local/share/ibus-bornona/ibus-bornona.png")),
        xdg_data_home.as_ref().map(|p| p.join("ibus-bornona/ibus-bornona.svg")),
        home.as_ref().map(|p| p.join(".local/share/ibus-bornona/ibus-bornona.svg")),
        Some(std::path::PathBuf::from("/usr/share/ibus-bornona/ibus-bornona.svg")),
        Some(std::path::PathBuf::from("/usr/local/share/ibus-bornona/ibus-bornona.svg")),
    ];

    for candidate in candidates.into_iter().flatten() {
        if candidate.is_file() {
            return Some(candidate.to_string_lossy().to_string());
        }
    }
    None
}

unsafe fn new_ibus_text(text: &str) -> *mut IBusText {
    let Ok(text) = CString::new(text) else {
        return std::ptr::null_mut();
    };
    ibus_text_new_from_string(text.as_ptr())
}

unsafe fn update_preedit_text(engine: *mut IBusEngine, processor: &TypingProcessor) {
    if engine.is_null() {
        return;
    }
    let preedit = processor.preedit_text();
    let text = new_ibus_text(&preedit);
    if text.is_null() {
        return;
    }
    let cursor = preedit.chars().count() as u32;
    if cursor != 0 {
        ibus_text_append_attribute(
            text,
            IBUS_ATTR_TYPE_UNDERLINE,
            IBUS_ATTR_UNDERLINE_SINGLE,
            0,
            cursor as i32,
        );
    }
    ibus_engine_update_preedit_text(engine, text, cursor, 1);
}

fn replace_word_in_preedit(preedit: &str, prefix: &str, candidate: &str) -> String {
    preedit
        .strip_suffix(prefix)
        .map(|before_word| format!("{before_word}{candidate}"))
        .unwrap_or_else(|| candidate.to_string())
}

fn candidate_commit_text(
    preedit: &str,
    prefix: &str,
    candidate: &str,
    trailing_space: bool,
) -> String {
    let mut committed = replace_word_in_preedit(preedit, prefix, candidate);
    if trailing_space {
        committed.push(' ');
    }
    committed
}

unsafe fn update_composition_preedit(engine: *mut IBusEngine, instance: &EngineInstance) {
    update_preedit_text(engine, &instance.processor);
}

fn suggestions_active(instance: &EngineInstance) -> bool {
    instance.lookup_model.len() > 1 && instance.lookup_visible
}

fn active_candidate_index(instance: &EngineInstance) -> usize {
    instance.lookup_model.cursor()
}

fn candidate_number_from_keyval(keyval: u32) -> Option<usize> {
    match keyval {
        0x0031..=0x0039 => Some((keyval - 0x0030) as usize),
        IBUS_KEY_KP_1..=IBUS_KEY_KP_9 => Some((keyval - IBUS_KEY_KP_1 + 1) as usize),
        _ => None,
    }
}

unsafe fn navigate_candidates(engine: *mut IBusEngine, instance: &mut EngineInstance, step: isize) {
    if instance.lookup_model.len() <= 1 {
        return;
    }
    if step > 0 {
        ibus_lookup_table_cursor_down(instance.lookup_table);
    } else {
        ibus_lookup_table_cursor_up(instance.lookup_table);
    }
    instance
        .lookup_model
        .set_cursor(ibus_lookup_table_get_cursor_pos(instance.lookup_table) as usize);
    ibus_engine_update_lookup_table_fast(engine, instance.lookup_table, 1);
}

unsafe fn hide_preedit(engine: *mut IBusEngine) {
    if !engine.is_null() {
        ibus_engine_hide_preedit_text(engine);
    }
}

unsafe fn update_suggestions(engine: *mut IBusEngine, instance: &mut EngineInstance) {
    if engine.is_null() {
        return;
    }

    update_composition_preedit(engine, instance);
    instance.lookup_visible = false;

    if !instance.config_watcher.config().word_suggestion
        || instance.cosmic_session
    {
        hide_suggestions(engine, instance);
        return;
    }
    if instance.lookup_table.is_null() {
        eprintln!("ibus-bornona: cannot show suggestions because the IBus lookup table is unavailable");
        return;
    }

    let preedit = instance.processor.preedit_text();
    let word_prefix = preedit.split_whitespace().last().unwrap_or(&preedit);
    if word_prefix.is_empty() {
        hide_suggestions(engine, instance);
        return;
    }

    let config = instance.config_watcher.config();
    let mut candidates = vec![preedit.clone()];
    candidates.extend(
        SuggestionEngine::suggest_with_learning(word_prefix, 8, config.learning)
            .into_iter()
            .filter(|candidate| candidate != &preedit),
    );
    if candidates.len() == 1 {
        hide_suggestions(engine, instance);
        return;
    }

    instance.lookup_model.set_candidates(candidates);
    ibus_lookup_table_clear(instance.lookup_table);
    let orientation = match config.suggestion_orientation {
        SuggestionOrientation::Horizontal => IBUS_ORIENTATION_HORIZONTAL,
        SuggestionOrientation::Vertical => IBUS_ORIENTATION_VERTICAL,
        SuggestionOrientation::System => IBUS_ORIENTATION_SYSTEM,
    };
    ibus_lookup_table_set_orientation(instance.lookup_table, orientation);
    ibus_lookup_table_set_page_size(instance.lookup_table, 9);
    ibus_lookup_table_set_cursor_visible(instance.lookup_table, 1);
    ibus_lookup_table_set_cursor_pos(instance.lookup_table, 0);

    for candidate in instance.lookup_model.candidates() {
        let candidate_text = new_ibus_text(candidate);
        if !candidate_text.is_null() {
            ibus_lookup_table_append_candidate(instance.lookup_table, candidate_text);
        }
    }

    ibus_engine_update_lookup_table_fast(engine, instance.lookup_table, 1);
    instance.lookup_visible = true;
}

unsafe fn hide_suggestions(engine: *mut IBusEngine, instance: &mut EngineInstance) {
    if instance.lookup_model.len() == 0 && !instance.lookup_visible {
        return;
    }
    instance.lookup_model.clear();
    instance.lookup_visible = false;
    if !instance.lookup_table.is_null() {
        ibus_lookup_table_clear(instance.lookup_table);
    }
    if !engine.is_null() {
        ibus_engine_hide_lookup_table(engine);
    }
}

unsafe fn select_candidate(
    engine: *mut IBusEngine,
    instance: &mut EngineInstance,
    index: usize,
) -> bool {
    if index >= instance.lookup_model.len() {
        return false;
    }
    if index == 0 {
        commit_current_text(engine, instance);
        return true;
    }

    let Some(chosen) = instance.lookup_model.candidate(index).map(str::to_string) else {
        return false;
    };
    if instance.config_watcher.config().learning {
        if let Err(error) = SuggestionEngine::learn_word(&chosen) {
            eprintln!("ibus-bornona: could not learn selected word: {error}");
        }
    }
    let preedit = instance.processor.preedit_text();
    let word_prefix = preedit.split_whitespace().last().unwrap_or(&preedit);
    let committed = candidate_commit_text(
        &preedit,
        word_prefix,
        &chosen,
        instance.config_watcher.config().suggestion_trailing_space,
    );
    let text = new_ibus_text(&committed);
    if text.is_null() || engine.is_null() {
        eprintln!("ibus-bornona: could not commit selected suggestion");
        return false;
    }
    ibus_engine_commit_text(engine, text);
    instance.processor.clear();
    hide_preedit(engine);
    hide_suggestions(engine, instance);
    true
}

unsafe fn commit_current_text(engine: *mut IBusEngine, instance: &mut EngineInstance) {
    if !instance.processor.ongoing_session() {
        hide_suggestions(engine, instance);
        return;
    }
    let text = instance.processor.commit();
    if !text.is_empty() {
        let ibus_text = new_ibus_text(&text);
        if !ibus_text.is_null() && !engine.is_null() {
            ibus_engine_commit_text(engine, ibus_text);
        }
    }
    hide_preedit(engine);
    hide_suggestions(engine, instance);
}

unsafe extern "C" fn candidate_clicked_cb(
    engine: *mut IBusEngine,
    index: c_uint,
    _button: c_uint,
    _state: c_uint,
    user_data: *mut c_void,
) {
    if user_data.is_null() {
        return;
    }
    let instance = &mut *(user_data as *mut EngineInstance);
    select_candidate(engine, instance, index as usize);
}

unsafe extern "C" fn create_engine_cb(
    _factory: *mut IBusFactory,
    engine_name: *const c_char,
    user_data: *mut c_void,
) -> *mut IBusEngine {
    if user_data.is_null() {
        eprintln!("ibus-bornona: IBus factory supplied a null D-Bus connection");
        return std::ptr::null_mut();
    }

    let id = ENGINE_COUNTER.fetch_add(1, Ordering::Relaxed) + 1;
    let path = CString::new(format!("/org/freedesktop/IBus/Engine/{id}")).unwrap();
    let engine = ibus_engine_new(
        engine_name,
        path.as_ptr(),
        user_data as *mut GDBusConnection,
    );
    if engine.is_null() {
        return std::ptr::null_mut();
    }

    let config_watcher = match ConfigWatcher::new() {
        Ok(watcher) => watcher,
        Err(error) => {
            eprintln!("ibus-bornona: {error}");
            return std::ptr::null_mut();
        }
    };
    let cosmic_session = ["XDG_CURRENT_DESKTOP", "XDG_SESSION_DESKTOP"]
        .iter()
        .filter_map(|name| std::env::var(name).ok())
        .any(|desktop| desktop.to_ascii_lowercase().contains("cosmic"));
    let lookup_table = unsafe {
        let table = ibus_lookup_table_new(9, 0, 1, 1);
        if !table.is_null() {
            g_object_ref_sink(table as *mut c_void) as *mut IBusLookupTable
        } else {
            std::ptr::null_mut()
        }
    };
    let instance = Box::into_raw(Box::new(EngineInstance {
        layout: BornonaLayout::new(),
        processor: TypingProcessor::new(config_watcher.config().clone()),
        config_watcher,
        lookup_table,
        lookup_model: LookupModel::default(),
        lookup_visible: false,
        cosmic_session,
        input_content_type: None,
    }));
    let data = instance as *mut c_void;

    for (signal, callback) in [
        (
            "process-key-event",
            std::mem::transmute::<
                unsafe extern "C" fn(*mut IBusEngine, u32, u32, u32, *mut c_void) -> c_int,
                GCallback,
            >(process_key_event_cb),
        ),
        (
            "candidate-clicked",
            std::mem::transmute::<
                unsafe extern "C" fn(*mut IBusEngine, c_uint, c_uint, c_uint, *mut c_void),
                GCallback,
            >(candidate_clicked_cb),
        ),
        (
            "focus-out",
            std::mem::transmute::<
                unsafe extern "C" fn(*mut IBusEngine, *mut c_void),
                GCallback,
            >(focus_out_cb),
        ),
        (
            "reset",
            std::mem::transmute::<
                unsafe extern "C" fn(*mut IBusEngine, *mut c_void),
                GCallback,
            >(reset_cb),
        ),
        (
            "disable",
            std::mem::transmute::<
                unsafe extern "C" fn(*mut IBusEngine, *mut c_void),
                GCallback,
            >(disable_cb),
        ),
        (
            "set-content-type",
            std::mem::transmute::<
                unsafe extern "C" fn(*mut IBusEngine, c_uint, c_uint, *mut c_void),
                GCallback,
            >(set_content_type_cb),
        ),
        (
            "destroy",
            std::mem::transmute::<
                unsafe extern "C" fn(*mut IBusEngine, *mut c_void),
                GCallback,
            >(destroy_cb),
        ),
    ] {
        let signal = CString::new(signal).unwrap();
        g_signal_connect_data(
            engine,
            signal.as_ptr(),
            callback,
            data,
            std::ptr::null_mut(),
            0,
        );
    }

    engine
}

unsafe extern "C" fn process_key_event_cb(
    engine: *mut IBusEngine,
    keyval: u32,
    _keycode: u32,
    state: u32,
    user_data: *mut c_void,
) -> c_int {
    if state & IBUS_RELEASE_MASK != 0 || user_data.is_null() || is_lock_toggle_key(keyval) {
        return 0;
    }
    let instance = &mut *(user_data as *mut EngineInstance);
    if should_bypass_input(instance.input_content_type) {
        return 0;
    }

    match instance.config_watcher.refresh() {
        Ok(Some(config)) => {
            instance.processor.set_config(config);
            if instance.config_watcher.config().word_suggestion && !instance.cosmic_session {
                if instance.processor.ongoing_session() {
                    update_suggestions(engine, instance);
                }
            } else {
                hide_suggestions(engine, instance);
                if instance.processor.ongoing_session() {
                    update_preedit_text(engine, &instance.processor);
                } else {
                    hide_preedit(engine);
                }
            }
        }
        Ok(None) => {}
        Err(error) => eprintln!("ibus-bornona: {error}"),
    }

    let has_suggestions = suggestions_active(instance);
    let orientation = instance.config_watcher.config().suggestion_orientation;
    let no_modifiers =
        state & (IBUS_CONTROL_MASK | IBUS_MOD1_MASK | IBUS_MOD4_MASK | IBUS_SHIFT_MASK) == 0;
    let no_ctrl_alt = state & (IBUS_CONTROL_MASK | IBUS_MOD1_MASK | IBUS_MOD4_MASK) == 0;
    let no_shift = state & IBUS_SHIFT_MASK == 0;
    let next_key = match orientation {
        SuggestionOrientation::Horizontal => keyval == IBUS_KEY_Right,
        SuggestionOrientation::Vertical => keyval == IBUS_KEY_Down,
        SuggestionOrientation::System => {
            keyval == IBUS_KEY_Right || keyval == IBUS_KEY_Down
        }
    };
    let previous_key = match orientation {
        SuggestionOrientation::Horizontal => keyval == IBUS_KEY_Left,
        SuggestionOrientation::Vertical => keyval == IBUS_KEY_Up,
        SuggestionOrientation::System => keyval == IBUS_KEY_Left || keyval == IBUS_KEY_Up,
    };

    if has_suggestions && no_modifiers {
        if let Some(number) = candidate_number_from_keyval(keyval) {
            if let Some(candidate_index) = instance.lookup_model.select_number(number) {
                if select_candidate(engine, instance, candidate_index) {
                    return 1;
                }
            }
        }
    }

    if has_suggestions && no_ctrl_alt {
        if keyval == IBUS_KEY_Tab {
            navigate_candidates(engine, instance, if no_shift { 1 } else { -1 });
            return 1;
        } else if next_key {
            navigate_candidates(engine, instance, 1);
            return 1;
        } else if previous_key {
            navigate_candidates(engine, instance, -1);
            return 1;
        }
    }

    match classify_key(keyval, state) {
        KeyAction::Modifier => 0,
        KeyAction::CommitAndPassThrough | KeyAction::PassThrough => {
            commit_current_text(engine, instance);
            0
        }
        KeyAction::Escape => {
            if has_suggestions {
                hide_suggestions(engine, instance);
                update_composition_preedit(engine, instance);
                return 1;
            }
            if instance.processor.ongoing_session() {
                instance.processor.clear();
                hide_preedit(engine);
                1
            } else {
                0
            }
        }
        KeyAction::Return => {
            if has_suggestions {
                let cursor = active_candidate_index(instance);
                if select_candidate(engine, instance, cursor) {
                    return 1;
                }
            }
            if instance.processor.ongoing_session() {
                commit_current_text(engine, instance);
                1
            } else {
                0
            }
        }
        KeyAction::Space => {
            if has_suggestions {
                let cursor = active_candidate_index(instance);
                if cursor > 0 && select_candidate(engine, instance, cursor) {
                    return 1;
                }
            }
            commit_current_text(engine, instance);
            0
        }
        KeyAction::Backspace => {
            if instance.processor.backspace() {
                if instance.processor.is_empty() {
                    hide_preedit(engine);
                    hide_suggestions(engine, instance);
                } else if instance.config_watcher.config().word_suggestion
                    && !instance.cosmic_session
                {
                    update_suggestions(engine, instance);
                } else {
                    update_preedit_text(engine, &instance.processor);
                }
                1
            } else {
                hide_suggestions(engine, instance);
                if instance.processor.ongoing_session() {
                    update_preedit_text(engine, &instance.processor);
                }
                0
            }
        }
        KeyAction::TypeKeypad(keyval) => {
            if let Some(value) = instance.layout.map_numpad(keyval) {
                instance.processor.process_value(value);
                if instance.config_watcher.config().word_suggestion && !instance.cosmic_session {
                    update_suggestions(engine, instance);
                } else {
                    update_preedit_text(engine, &instance.processor);
                }
                1
            } else {
                commit_current_text(engine, instance);
                0
            }
        }
        KeyAction::TypeChar(character) => {
            if let Some(value) = instance.layout.map_char(character) {
                instance.processor.process_value(value);
                if instance.config_watcher.config().word_suggestion && !instance.cosmic_session {
                    update_suggestions(engine, instance);
                } else {
                    update_preedit_text(engine, &instance.processor);
                }
                1
            } else {
                commit_current_text(engine, instance);
                0
            }
        }
    }
}

unsafe extern "C" fn focus_out_cb(engine: *mut IBusEngine, user_data: *mut c_void) {
    if !user_data.is_null() {
        let instance = &mut *(user_data as *mut EngineInstance);
        commit_current_text(engine, instance);
    }
}

unsafe extern "C" fn reset_cb(engine: *mut IBusEngine, user_data: *mut c_void) {
    if !user_data.is_null() {
        let instance = &mut *(user_data as *mut EngineInstance);
        instance.processor.clear();
        hide_suggestions(engine, instance);
    }
    hide_preedit(engine);
}

unsafe extern "C" fn disable_cb(engine: *mut IBusEngine, user_data: *mut c_void) {
    if !user_data.is_null() {
        let instance = &mut *(user_data as *mut EngineInstance);
        commit_current_text(engine, instance);
    }
}

unsafe extern "C" fn set_content_type_cb(
    engine: *mut IBusEngine,
    purpose: c_uint,
    hints: c_uint,
    user_data: *mut c_void,
) {
    if user_data.is_null() {
        return;
    }

    let instance = &mut *(user_data as *mut EngineInstance);
    instance.input_content_type = Some((purpose, hints));
    if is_sensitive_input(purpose, hints) {
        instance.processor.clear();
        hide_preedit(engine);
        hide_suggestions(engine, instance);
    }
}

unsafe extern "C" fn destroy_cb(_engine: *mut IBusEngine, user_data: *mut c_void) {
    if !user_data.is_null() {
        let instance = Box::from_raw(user_data as *mut EngineInstance);
        if !instance.lookup_table.is_null() {
            g_object_unref(instance.lookup_table as *mut c_void);
        }
    }
}

unsafe extern "C" fn bus_disconnected_cb(_bus: *mut IBusBus, _user_data: *mut c_void) {
    ibus_quit();
}

pub fn run_ibus_engine(ibus_daemon_mode: bool) -> Result<(), String> {
    unsafe {
        ibus_init();
        let bus = ibus_bus_new();
        if bus.is_null() || ibus_bus_is_connected(bus) == 0 {
            return Err("Failed to connect to IBus bus".to_string());
        }

        let disconnected = CString::new("disconnected").unwrap();
        g_signal_connect_data(
            bus,
            disconnected.as_ptr(),
            std::mem::transmute::<unsafe extern "C" fn(*mut IBusBus, *mut c_void), GCallback>(
                bus_disconnected_cb,
            ),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            0,
        );

        let conn = ibus_bus_get_connection(bus);
        if conn.is_null() {
            return Err("Failed to get D-Bus connection from IBus bus".to_string());
        }
        let factory = ibus_factory_new(conn);
        if factory.is_null() {
            return Err("Failed to create IBus factory".to_string());
        }

        let create_engine = CString::new("create-engine").unwrap();
        g_signal_connect_data(
            factory,
            create_engine.as_ptr(),
            std::mem::transmute::<
                unsafe extern "C" fn(*mut IBusFactory, *const c_char, *mut c_void) -> *mut IBusEngine,
                GCallback,
            >(create_engine_cb),
            conn as *mut c_void,
            std::ptr::null_mut(),
            0,
        );

        let component_name = CString::new(COMPONENT_NAME).unwrap();
        if ibus_daemon_mode {
            ibus_bus_request_name(bus, component_name.as_ptr(), 0);
        } else {
            let description = CString::new("Bornona Bengali Input Method").unwrap();
            let version = CString::new(env!("CARGO_PKG_VERSION")).unwrap();
            let license = CString::new("GPL-3.0-or-later").unwrap();
            let author = CString::new("Bornona Contributors").unwrap();
            let homepage =
                CString::new("https://github.com/rmahbub01/ibus-bornona").unwrap();
            let command = CString::new("ibus-bornona --ibus").unwrap();
            let textdomain = CString::new("ibus-bornona").unwrap();
            let component = ibus_component_new(
                component_name.as_ptr(),
                description.as_ptr(),
                version.as_ptr(),
                license.as_ptr(),
                author.as_ptr(),
                homepage.as_ptr(),
                command.as_ptr(),
                textdomain.as_ptr(),
            );
            if component.is_null() {
                return Err("Failed to create IBus component".to_string());
            }

            let engine_name = CString::new(ENGINE_NAME).unwrap();
            let longname = CString::new("Bornona").unwrap();
            let engine_description =
                CString::new("Bornona Bengali Fixed Layout").unwrap();
            let language = CString::new("bn").unwrap();
            let icon_str = find_installed_icon_path().unwrap_or_else(|| "ibus-bornona".to_string());
            let icon = CString::new(icon_str).unwrap();
            let layout = CString::new("us").unwrap();
            let engine = ibus_engine_desc_new(
                engine_name.as_ptr(),
                longname.as_ptr(),
                engine_description.as_ptr(),
                language.as_ptr(),
                license.as_ptr(),
                author.as_ptr(),
                icon.as_ptr(),
                layout.as_ptr(),
            );
            if engine.is_null() {
                return Err("Failed to create Bornona IBus engine descriptor".to_string());
            }
            ibus_component_add_engine(component, engine);
            if ibus_bus_register_component(bus, component) == 0 {
                return Err("Failed to register Bornona with IBus".to_string());
            }
            println!("Registered {} with IBus daemon.", COMPONENT_NAME);
        }

        ibus_main();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        candidate_commit_text, candidate_number_from_keyval, is_lock_toggle_key, is_sensitive_input,
        should_bypass_input,
    };
    use crate::ibus::ffi::{IBUS_KEY_KP_1, IBUS_KEY_KP_9};

    #[test]
    fn password_purposes_and_hidden_text_hint_disable_ime_composition() {
        assert!(is_sensitive_input(
            super::IBUS_INPUT_PURPOSE_PASSWORD,
            0
        ));
        assert!(is_sensitive_input(super::IBUS_INPUT_PURPOSE_PIN, 0));
        assert!(is_sensitive_input(
            0,
            super::IBUS_INPUT_HINT_HIDDEN_TEXT
        ));
        assert!(!is_sensitive_input(0, 0));
    }

    #[test]
    fn missing_content_type_does_not_disable_ime() {
        assert!(!should_bypass_input(None));
        assert!(!should_bypass_input(Some((0, 0))));
        assert!(should_bypass_input(Some((super::IBUS_INPUT_PURPOSE_PASSWORD, 0))));
    }

    #[test]
    fn lock_toggle_keys_always_bypass_ime_processing() {
        assert!(is_lock_toggle_key(super::IBUS_KEY_Caps_Lock));
        assert!(is_lock_toggle_key(super::IBUS_KEY_Shift_Lock));
        assert!(!is_lock_toggle_key(0x0041));
    }

    #[test]
    fn caps_lock_press_is_returned_unhandled_by_the_ibus_callback() {
        let mut user_data = 0_u8;
        let result = unsafe {
            super::process_key_event_cb(
                std::ptr::null_mut(),
                super::IBUS_KEY_Caps_Lock,
                0,
                0,
                (&mut user_data as *mut u8).cast(),
            )
        };
        assert_eq!(result, 0);
    }

    #[test]
    fn number_keys_map_to_candidate_numbers_one_through_nine() {
        assert_eq!(candidate_number_from_keyval(0x0031), Some(1));
        assert_eq!(candidate_number_from_keyval(0x0039), Some(9));
        assert_eq!(candidate_number_from_keyval(0x0030), None);
        assert_eq!(candidate_number_from_keyval(IBUS_KEY_KP_1), Some(1));
        assert_eq!(candidate_number_from_keyval(IBUS_KEY_KP_9), Some(9));
        assert_eq!(candidate_number_from_keyval(0xffb0), None);
    }

    #[test]
    fn explicitly_committed_suggestion_has_trailing_space() {
        assert_eq!(
            candidate_commit_text("আগের শব্দ", "শব্দ", "স্বাধীন", true),
            "আগের স্বাধীন "
        );
        assert_eq!(
            candidate_commit_text("আগের শব্দ", "শব্দ", "স্বাধীন", false),
            "আগের স্বাধীন"
        );
    }

    #[test]
    fn suggestion_replaces_only_the_current_word() {
        assert_eq!(
            candidate_commit_text("আগের শব্দ", "শব্দ", "শব্দটি", true),
            "আগের শব্দটি "
        );
        assert_eq!(
            candidate_commit_text("শব্দ", "শব্দ", "শব্দটি", true),
            "শব্দটি "
        );
    }
}
