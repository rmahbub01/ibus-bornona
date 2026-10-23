//! IBus engine implementation for Bornona.

use std::ffi::{c_char, c_int, c_void, CString};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock};

use super::ffi::*;
use super::keycodes::{classify_key, KeyAction};
use crate::engine::{BornonaLayout, TypingConfig, TypingProcessor};

pub static ENGINE_NAME: &str = "bornona";
pub static COMPONENT_NAME: &str = "org.freedesktop.IBus.Bornona";

static ENGINE_CONFIG: OnceLock<Mutex<TypingConfig>> = OnceLock::new();
static ENGINE_COUNTER: AtomicU32 = AtomicU32::new(0);

struct EngineInstance {
    layout: BornonaLayout,
    processor: TypingProcessor,
}

pub fn init_engine_state(config: TypingConfig) {
    *ENGINE_CONFIG
        .get_or_init(|| Mutex::new(TypingConfig::default()))
        .lock()
        .unwrap() = config;
}

fn engine_config() -> TypingConfig {
    ENGINE_CONFIG
        .get_or_init(|| Mutex::new(TypingConfig::default()))
        .lock()
        .unwrap()
        .clone()
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
    let text = new_ibus_text(processor.buffer());
    if text.is_null() {
        return;
    }
    let cursor = processor.buffer().chars().count() as u32;
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

unsafe fn hide_preedit(engine: *mut IBusEngine) {
    if !engine.is_null() {
        ibus_engine_hide_preedit_text(engine);
    }
}

unsafe fn commit_current_text(engine: *mut IBusEngine, processor: &mut TypingProcessor) {
    if !processor.ongoing_session() {
        return;
    }
    let text = processor.commit();
    if !text.is_empty() {
        let ibus_text = new_ibus_text(&text);
        if !ibus_text.is_null() && !engine.is_null() {
            ibus_engine_commit_text(engine, ibus_text);
        }
    }
    hide_preedit(engine);
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

    let instance = Box::into_raw(Box::new(EngineInstance {
        layout: BornonaLayout::new(),
        processor: TypingProcessor::new(engine_config()),
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
    if state & IBUS_RELEASE_MASK != 0 || user_data.is_null() {
        return 0;
    }
    let instance = &mut *(user_data as *mut EngineInstance);
    match classify_key(keyval, state) {
        KeyAction::Modifier => 0,
        KeyAction::CommitAndPassThrough | KeyAction::PassThrough => {
            commit_current_text(engine, &mut instance.processor);
            0
        }
        KeyAction::Escape => {
            if instance.processor.ongoing_session() {
                instance.processor.clear();
                hide_preedit(engine);
                1
            } else {
                0
            }
        }
        KeyAction::Return | KeyAction::Space => {
            commit_current_text(engine, &mut instance.processor);
            0
        }
        KeyAction::Backspace => {
            if instance.processor.backspace() {
                if instance.processor.is_empty() {
                    hide_preedit(engine);
                } else {
                    update_preedit_text(engine, &instance.processor);
                }
                1
            } else {
                0
            }
        }
        KeyAction::TypeKeypad(keyval) => {
            if let Some(value) = instance.layout.map_numpad(keyval) {
                instance.processor.process_value(value);
                update_preedit_text(engine, &instance.processor);
                1
            } else {
                commit_current_text(engine, &mut instance.processor);
                0
            }
        }
        KeyAction::TypeChar(character) => {
            if let Some(value) = instance.layout.map_char(character) {
                instance.processor.process_value(value);
                update_preedit_text(engine, &instance.processor);
                1
            } else {
                commit_current_text(engine, &mut instance.processor);
                0
            }
        }
    }
}

unsafe extern "C" fn focus_out_cb(engine: *mut IBusEngine, user_data: *mut c_void) {
    if !user_data.is_null() {
        let instance = &mut *(user_data as *mut EngineInstance);
        commit_current_text(engine, &mut instance.processor);
    }
}

unsafe extern "C" fn reset_cb(engine: *mut IBusEngine, user_data: *mut c_void) {
    if !user_data.is_null() {
        let instance = &mut *(user_data as *mut EngineInstance);
        instance.processor.clear();
    }
    hide_preedit(engine);
}

unsafe extern "C" fn disable_cb(engine: *mut IBusEngine, user_data: *mut c_void) {
    if !user_data.is_null() {
        let instance = &mut *(user_data as *mut EngineInstance);
        commit_current_text(engine, &mut instance.processor);
    }
}

unsafe extern "C" fn destroy_cb(_engine: *mut IBusEngine, user_data: *mut c_void) {
    if !user_data.is_null() {
        drop(Box::from_raw(user_data as *mut EngineInstance));
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
            let icon = CString::new("ibus-bornona").unwrap();
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
