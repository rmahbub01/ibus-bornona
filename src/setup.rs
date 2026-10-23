//! Desktop integration, setup, and diagnostics for ibus-bornona.

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Duration;

pub const SCHEMA: &str = "org.gnome.desktop.input-sources";
pub const SOURCE_ID: &str = "bornona";
const WM_KEYBINDINGS_SCHEMA: &str = "org.gnome.desktop.wm.keybindings";
const IBUS_HOTKEY_SCHEMA: &str = "org.freedesktop.ibus.general.hotkey";
const IBUS_GENERAL_SCHEMA: &str = "org.freedesktop.ibus.general";
const GNOME_IBUS_UNIT: &str = "org.freedesktop.IBus.session.GNOME.service";
const GNOME_IBUS_EXEC_START: &str = "sh -c 'exec /usr/bin/ibus-daemon --panel disable --cache=none $([ \"$XDG_SESSION_TYPE\" = \"x11\" ] && echo \"--xim\")'";

fn is_cosmic() -> bool {
    ["XDG_CURRENT_DESKTOP", "XDG_SESSION_DESKTOP"]
        .iter()
        .filter_map(|name| env::var(name).ok())
        .any(|desktop| desktop.to_ascii_lowercase().contains("cosmic"))
}

fn is_gnome() -> bool {
    ["XDG_CURRENT_DESKTOP", "XDG_SESSION_DESKTOP"]
        .iter()
        .filter_map(|name| env::var(name).ok())
        .any(|desktop| desktop.to_ascii_lowercase().contains("gnome"))
}

fn has_gnome_input_sources() -> bool {
    run("gsettings", &["list-keys", SCHEMA]).is_ok()
}

fn has_ibus_general_settings() -> bool {
    run("gsettings", &["list-keys", IBUS_GENERAL_SCHEMA]).is_ok()
}

fn is_wayland() -> bool {
    env::var("XDG_SESSION_TYPE")
        .map(|v| v.to_ascii_lowercase() == "wayland")
        .unwrap_or(false)
        || env::var_os("WAYLAND_DISPLAY").is_some()
}

fn component_path_env() -> String {
    let local = env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .map(|d| d.join("ibus/component"))
        .filter(|p| p.exists())
        .map(|p| p.to_string_lossy().to_string());

    let mut paths = Vec::new();
    if let Some(l) = local {
        paths.push(l);
    }
    paths.push("/usr/share/ibus/component".to_string());
    paths.push("/usr/local/share/ibus/component".to_string());
    paths.join(":")
}

fn is_component_installed() -> bool {
    let xdg_data_home = env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")));
    Path::new("/usr/share/ibus/component/ibus-bornona.xml").exists()
        || Path::new("/usr/local/share/ibus/component/ibus-bornona.xml").exists()
        || xdg_data_home
            .map(|data_home| data_home.join("ibus/component/ibus-bornona.xml"))
            .map(|path| path.exists())
            .unwrap_or(false)
        || env::var_os("IBUS_COMPONENT_PATH")
            .into_iter()
            .flat_map(|paths| env::split_paths(&paths).collect::<Vec<_>>())
            .any(|path| path.join("ibus-bornona.xml").exists())
}

fn autostart_dir() -> Result<PathBuf, String> {
    let config_home = match env::var_os("XDG_CONFIG_HOME") {
        Some(path) if !path.is_empty() => {
            let path = PathBuf::from(path);
            if !path.is_absolute() {
                return Err("XDG_CONFIG_HOME must be an absolute path".into());
            }
            path
        }
        _ => env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or_else(|| "HOME is not set".to_string())?
            .join(".config"),
    };
    Ok(config_home.join("autostart"))
}

const EMBEDDED_ICON_SVG: &str = include_str!("../data/logo/ibus-bornona.svg");

pub fn is_icon_installed() -> bool {
    let xdg_data_home = env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")));

    Path::new("/usr/share/icons/hicolor/scalable/apps/ibus-bornona.svg").exists()
        || Path::new("/usr/local/share/icons/hicolor/scalable/apps/ibus-bornona.svg").exists()
        || xdg_data_home
            .map(|data_home| data_home.join("icons/hicolor/scalable/apps/ibus-bornona.svg"))
            .map(|path| path.exists())
            .unwrap_or(false)
        || env::var_os("XDG_DATA_DIRS")
            .into_iter()
            .flat_map(|paths| env::split_paths(&paths).collect::<Vec<_>>())
            .any(|path| path.join("icons/hicolor/scalable/apps/ibus-bornona.svg").exists())
}

pub fn install_user_icon() -> Result<(), String> {
    let Some(home) = env::var_os("HOME").map(PathBuf::from) else {
        return Ok(());
    };
    let icon_dir = home.join(".local/share/icons/hicolor/scalable/apps");
    let icon_path = icon_dir.join("ibus-bornona.svg");
    if !icon_path.is_file() {
        std::fs::create_dir_all(&icon_dir)
            .map_err(|e| format!("cannot create {}: {e}", icon_dir.display()))?;
        std::fs::write(&icon_path, EMBEDDED_ICON_SVG)
            .map_err(|e| format!("cannot write {}: {e}", icon_path.display()))?;
        let _ = Command::new("gtk-update-icon-cache")
            .args(["-f", "-t", home.join(".local/share/icons/hicolor").to_str().unwrap()])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }
    Ok(())
}

fn remove_legacy_desktop_files() {
    if let Ok(dir) = autostart_dir() {
        let _ = std::fs::remove_file(dir.join("ibus-daemon.desktop"));
        let _ = std::fs::remove_file(dir.join("ibus-bornona-cosmic.desktop"));
        let _ = std::fs::remove_file(dir.join("ibus-bornona.desktop"));
    }
}

fn install_environment_conf() -> Result<(), String> {
    let Some(home) = env::var_os("HOME").map(PathBuf::from) else {
        return Ok(());
    };
    let env_dir = home.join(".config/environment.d");
    std::fs::create_dir_all(&env_dir)
        .map_err(|e| format!("cannot create {}: {e}", env_dir.display()))?;
    let env_file = env_dir.join("50-ibus-bornona.conf");
    let contents = "IBUS_COMPONENT_PATH=\"${HOME}/.local/share/ibus/component:/usr/share/ibus/component:/usr/local/share/ibus/component\"\n";
    std::fs::write(&env_file, contents)
        .map_err(|e| format!("cannot write {}: {e}", env_file.display()))?;
    Ok(())
}

fn find_wayland_ibus_panel() -> Option<String> {
    for path in [
        "/usr/libexec/ibus-ui-gtk3",
        "/usr/lib/ibus/ibus-ui-gtk3",
        "/usr/lib64/ibus/ibus-ui-gtk3",
    ] {
        if Path::new(path).is_file() {
            return Some(path.to_string());
        }
    }
    run("which", &["ibus-ui-gtk3"]).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

/// Run a command and return its stdout, or an error string if failed.
fn run(cmd: &str, args: &[&str]) -> Result<String, String> {
    let comp_path = component_path_env();
    run_with_env(cmd, args, &[("IBUS_COMPONENT_PATH", comp_path.as_str())])
}

fn run_with_env(cmd: &str, args: &[&str], extra_env: &[(&str, &str)]) -> Result<String, String> {
    let mut command = Command::new(cmd);
    command.args(args);
    command.stdin(std::process::Stdio::null());
    for (k, v) in extra_env {
        command.env(k, v);
    }
    let out = command.output().map_err(|e| format!("cannot run `{cmd}`: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "`{cmd} {}` failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

fn run_command_status(cmd: &str, args: &[&str], extra_env: &[(&str, &str)]) -> Result<bool, String> {
    let mut command = Command::new(cmd);
    command.args(args);
    command.stdin(std::process::Stdio::null());
    command.stdout(std::process::Stdio::null());
    command.stderr(std::process::Stdio::null());
    for (k, v) in extra_env {
        command.env(k, v);
    }
    let status = command.status().map_err(|e| format!("cannot run `{cmd}`: {e}"))?;
    Ok(status.success())
}

pub fn parse_gsettings_string_list(value: &str) -> Result<Vec<String>, String> {
    let value = value.trim();
    let value = value.strip_prefix("@as").map(str::trim).unwrap_or(value);
    let inner = value
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .ok_or_else(|| format!("unexpected string-list value: {value}"))?
        .trim();
    if inner.is_empty() {
        return Ok(Vec::new());
    }

    let mut values = Vec::new();
    let mut start = 0;
    let mut quote = None;
    let mut escaped = false;
    for (index, character) in inner.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match (quote, character) {
            (Some(_), '\\') => escaped = true,
            (Some(delimiter), character) if delimiter == character => quote = None,
            (None, '\'' | '"') => quote = Some(character),
            (None, ',') => {
                values.push(parse_gsettings_string(&inner[start..index])?);
                start = index + character.len_utf8();
            }
            _ => {}
        }
    }
    if quote.is_some() || escaped {
        return Err(format!("unexpected string-list value: {value}"));
    }
    values.push(parse_gsettings_string(&inner[start..])?);
    Ok(values)
}

fn parse_gsettings_string(value: &str) -> Result<String, String> {
    let value = value.trim();
    let Some(delimiter) = value.chars().next().filter(|c| *c == '\'' || *c == '"') else {
        return Err(format!("unexpected string-list entry: {value}"));
    };
    if !value.ends_with(delimiter) || value.len() < 2 {
        return Err(format!("unexpected string-list entry: {value}"));
    }
    Ok(value[delimiter.len_utf8()..value.len() - delimiter.len_utf8()].to_string())
}

pub fn format_gsettings_string_list(values: &[String]) -> String {
    let entries = values
        .iter()
        .map(|value| {
            format!(
                "'{}'",
                value.replace('\\', "\\\\").replace('\'', "\\'")
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{entries}]")
}

pub fn registered_engine_names() -> Result<Vec<String>, String> {
    let engines = run("ibus", &["list-engine"])?;
    Ok(engines
        .lines()
        .filter_map(|line| line.trim().split_once(" - ").map(|(name, _)| name.trim()))
        .filter(|name| !name.is_empty() && !name.contains(char::is_whitespace))
        .map(str::to_string)
        .collect())
}

pub fn retain_registered_engines(engines: &mut Vec<String>, registered: &[String]) -> Vec<String> {
    let mut removed = Vec::new();
    engines.retain(|engine| {
        if registered.iter().any(|available| available == engine) {
            true
        } else {
            if !removed.contains(engine) {
                removed.push(engine.clone());
            }
            false
        }
    });

    // Ensure at least one Latin / XKB fallback engine is available
    if !engines.iter().any(|e| e.starts_with("xkb:")) {
        if let Some(xkb_engine) = registered.iter().find(|e| e.starts_with("xkb:us:") || e.starts_with("xkb:")) {
            engines.insert(0, xkb_engine.clone());
        } else {
            engines.insert(0, "xkb:us::eng".to_string());
        }
    }

    if registered.iter().any(|engine| engine == SOURCE_ID)
        && !engines.iter().any(|engine| engine == SOURCE_ID)
    {
        engines.push(SOURCE_ID.to_string());
    }
    removed
}

pub fn configure_ibus_engines() -> Result<Vec<String>, String> {
    let registered = registered_engine_names()?;
    let mut removed = Vec::new();
    for key in ["preload-engines", "engines-order"] {
        let current = run("gsettings", &["get", IBUS_GENERAL_SCHEMA, key])?;
        let mut engines = parse_gsettings_string_list(&current)?;
        for engine in retain_registered_engines(&mut engines, &registered) {
            if !removed.contains(&engine) {
                removed.push(engine);
            }
        }
        let formatted = format_gsettings_string_list(&engines);
        if parse_gsettings_string_list(&current)? != engines {
            run(
                "gsettings",
                &["set", IBUS_GENERAL_SCHEMA, key, &formatted],
            )?;
        }
    }
    Ok(removed)
}

pub fn configure_ibus_hotkeys() -> Result<(), String> {
    let _ = run(
        "gsettings",
        &["set", IBUS_HOTKEY_SCHEMA, "triggers", "['<Alt>space']"],
    );
    let _ = run(
        "gsettings",
        &["set", IBUS_HOTKEY_SCHEMA, "next-engine", "['<Alt>space', 'Alt+Shift_L']"],
    );
    Ok(())
}

pub fn remove_ibus_engine_references(remove_bornona: bool) -> Result<Vec<String>, String> {
    if !has_ibus_general_settings() {
        return Ok(Vec::new());
    }
    let registered = match registered_engine_names() {
        Ok(engines) => Some(engines),
        Err(error) => {
            eprintln!("ibus-bornona: cannot verify registered IBus engines: {error}");
            None
        }
    };
    let mut removed = Vec::new();

    // Determine a safe fallback Latin / XKB engine
    let mut fallback = None;
    if let Ok(current) = run("gsettings", &["get", IBUS_GENERAL_SCHEMA, "preload-engines"]) {
        if let Ok(configured) = parse_gsettings_string_list(&current) {
            fallback = configured
                .into_iter()
                .find(|e| e != SOURCE_ID && e.starts_with("xkb:"));
        }
    }
    if fallback.is_none() {
        if let Some(ref reg) = registered {
            fallback = reg
                .iter()
                .find(|e| *e != SOURCE_ID && e.starts_with("xkb:"))
                .cloned();
        }
    }
    let fallback_engine = fallback.unwrap_or_else(|| "xkb:us::eng".to_string());

    // If Bornona is currently the active engine, switch to fallback BEFORE modifying preload-engines
    // so IBus daemon always maintains a valid global engine throughout the removal!
    if remove_bornona {
        let current_engine = run("ibus", &["engine"])
            .ok()
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        if current_engine == SOURCE_ID || current_engine.is_empty() {
            let _ = run("ibus", &["engine", &fallback_engine]);
        }
    }

    for key in ["preload-engines", "engines-order"] {
        let current = run("gsettings", &["get", IBUS_GENERAL_SCHEMA, key])?;
        let configured = parse_gsettings_string_list(&current)?;
        let mut engines = configured.clone();
        for engine in
            retain_configured_engines(&mut engines, registered.as_deref(), remove_bornona)
        {
            if !removed.contains(&engine) {
                removed.push(engine);
            }
        }
        // Ensure at least one Latin / XKB fallback engine remains in the list
        if !engines.iter().any(|e| e.starts_with("xkb:")) {
            engines.insert(0, fallback_engine.clone());
        }
        if engines != configured {
            let formatted = format_gsettings_string_list(&engines);
            run(
                "gsettings",
                &["set", IBUS_GENERAL_SCHEMA, key, &formatted],
            )?;
        }
    }

    // Ensure the current engine is valid after modification
    let active = run("ibus", &["engine"])
        .ok()
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    if active == SOURCE_ID || active.is_empty() {
        let _ = run("ibus", &["engine", &fallback_engine]);
    }

    Ok(removed)
}

pub fn retain_configured_engines(
    configured: &mut Vec<String>,
    registered: Option<&[String]>,
    remove_bornona: bool,
) -> Vec<String> {
    let mut removed = Vec::new();
    configured.retain(|engine| {
        let available = (!remove_bornona || engine != SOURCE_ID)
            && registered.is_none_or(|engines| engines.iter().any(|available| available == engine));
        if !available {
            removed.push(engine.clone());
        }
        available
    });
    removed
}

fn configure_gnome_component_path() -> Result<bool, String> {
    let Some(home) = env::var_os("HOME").map(PathBuf::from) else {
        return Ok(false);
    };
    let component = home.join(".local/share/ibus/component/ibus-bornona.xml");
    if !component.is_file() || run("systemctl", &["--user", "cat", GNOME_IBUS_UNIT]).is_err() {
        return Ok(false);
    }

    let drop_in_dir = home.join(
        ".config/systemd/user/org.freedesktop.IBus.session.GNOME.service.d",
    );
    std::fs::create_dir_all(&drop_in_dir)
        .map_err(|e| format!("cannot create {}: {e}", drop_in_dir.display()))?;
    let drop_in = drop_in_dir.join("ibus-bornona.conf");
    let contents = format!(
        "[Service]\nEnvironment=\"IBUS_COMPONENT_PATH=%h/.local/share/ibus/component:/usr/share/ibus/component:/usr/local/share/ibus/component\"\nExecStart=\nExecStart={GNOME_IBUS_EXEC_START}\n"
    );
    std::fs::write(&drop_in, contents)
        .map_err(|e| format!("cannot write {}: {e}", drop_in.display()))?;

    run("systemctl", &["--user", "daemon-reload"])?;
    run("systemctl", &["--user", "restart", GNOME_IBUS_UNIT])?;
    Ok(true)
}

/// Check if the IBus daemon lists the Bornona engine.
pub fn engine_registered() -> bool {
    let comp_path = component_path_env();
    run_with_env("ibus", &["list-engine"], &[("IBUS_COMPONENT_PATH", comp_path.as_str())])
        .map(|s| s.lines().any(|line| {
            let line = line.trim();
            line == SOURCE_ID || line.starts_with(&format!("{SOURCE_ID} -"))
        }))
        .unwrap_or(false)
}

/// Ensure ibus-daemon is running and knows the Bornona engine.
pub fn ensure_engine_registered() -> Result<(), String> {
    let comp_path = component_path_env();
    let envs = [("IBUS_COMPONENT_PATH", comp_path.as_str())];

    let _ = run_command_status("ibus", &["write-cache"], &envs);

    if engine_registered() {
        return Ok(());
    }

    let daemon_running = Command::new("pgrep")
        .args(["-x", "ibus-daemon"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    let panel_arg = find_wayland_ibus_panel();
    let mut daemon_cmd = Command::new("ibus-daemon");
    daemon_cmd.env("IBUS_COMPONENT_PATH", &comp_path);
    daemon_cmd.stdin(std::process::Stdio::null());
    daemon_cmd.stdout(std::process::Stdio::null());
    daemon_cmd.stderr(std::process::Stdio::null());
    daemon_cmd.args(["-drx", "-t", "refresh"]);
    if is_wayland() {
        if let Some(panel) = panel_arg {
            daemon_cmd.arg(format!("--panel={panel} --enable-wayland-im"));
        }
    }

    if daemon_running {
        let _ = run_command_status("ibus", &["restart"], &envs);
        thread::sleep(Duration::from_millis(500));
        if !engine_registered() {
            let _ = daemon_cmd.spawn();
        }
    } else {
        daemon_cmd
            .spawn()
            .map_err(|e| format!("cannot start ibus-daemon: {e}"))?;
    }

    for _ in 0..20 {
        thread::sleep(Duration::from_millis(300));
        if engine_registered() {
            return Ok(());
        }
    }
    Err("ibus-daemon does not list the bornona engine; try restarting ibus or logging out and back in".into())
}

/// Return the new `sources` value with `('ibus', 'bornona')` appended, or
/// `None` if it is already present or unparseable.
pub fn append_source(current: &str) -> Option<String> {
    let cur = current.trim();
    if cur.contains(&format!("'{SOURCE_ID}'")) {
        return None;
    }
    let entry = format!("('ibus', '{SOURCE_ID}')");
    let cur = cur.strip_prefix("@a(ss)").map(str::trim).unwrap_or(cur);
    let inner = cur.strip_prefix('[')?.strip_suffix(']')?.trim();
    Some(if inner.is_empty() {
        format!("[{entry}]")
    } else {
        format!("[{inner}, {entry}]")
    })
}

fn source_entries(current: &str) -> Result<Vec<&str>, String> {
    let current = current.trim();
    let current = current.strip_prefix("@a(ss)").map(str::trim).unwrap_or(current);
    let inner = current
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .ok_or_else(|| format!("unexpected input-sources value: {current}"))?
        .trim();
    if inner.is_empty() {
        return Ok(Vec::new());
    }

    let mut entries = Vec::new();
    let mut start = 0;
    let mut quoted = false;
    let mut escaped = false;
    let mut tuple_depth = 0;
    for (index, character) in inner.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match character {
            '\\' => escaped = true,
            '\'' | '"' => quoted = !quoted,
            '(' if !quoted => tuple_depth += 1,
            ')' if !quoted => {
                if tuple_depth > 0 {
                    tuple_depth -= 1;
                }
            }
            ',' if !quoted && tuple_depth == 0 => {
                entries.push(inner[start..index].trim());
                start = index + character.len_utf8();
            }
            _ => {}
        }
    }
    if quoted || tuple_depth != 0 {
        return Err(format!("unexpected input-sources value: {current}"));
    }
    entries.push(inner[start..].trim());
    Ok(entries)
}

fn is_bornona_source(entry: &str) -> bool {
    let mut inner = entry.trim();
    if !inner.starts_with('(') || !inner.ends_with(')') {
        return false;
    }
    inner = &inner[1..inner.len() - 1];
    let Some((kind, id)) = inner.split_once(',') else {
        return false;
    };
    let kind = kind.trim().trim_matches('\'').trim_matches('"');
    let id = id.trim().trim_matches('\'').trim_matches('"');
    kind == "ibus" && id == SOURCE_ID
}

/// Remove only Bornona from a GNOME input-source value.
pub fn remove_source(current: &str) -> Result<Option<String>, String> {
    let entries = source_entries(current)?;
    let entry_count = entries.len();
    let filtered: Vec<_> = entries
        .into_iter()
        .filter(|entry| !is_bornona_source(entry))
        .collect();
    if filtered.len() == entry_count {
        return Ok(None);
    }
    Ok(Some(format!("[{}]", filtered.join(", "))))
}

/// Add Bornona to GNOME's input sources. Returns `Ok(true)` if changed.
pub fn add_gnome_input_source() -> Result<bool, String> {
    let current = run("gsettings", &["get", SCHEMA, "sources"])?;
    match append_source(&current) {
        None if current.contains(&format!("'{SOURCE_ID}'")) => Ok(false),
        None => Err(format!("unexpected input-sources value: {}", current.trim())),
        Some(new) => {
            run("gsettings", &["set", SCHEMA, "sources", &new])?;
            Ok(true)
        }
    }
}

/// Remove Bornona from GNOME's input sources, preserving all other sources.
pub fn remove_gnome_input_source() -> Result<bool, String> {
    let current = run("gsettings", &["get", SCHEMA, "sources"])?;
    match remove_source(&current)? {
        None => Ok(false),
        Some(new) => {
            run("gsettings", &["set", SCHEMA, "sources", &new])?;
            Ok(true)
        }
    }
}

/// Remove Bornona from desktop and IBus engine settings before uninstalling it.
pub fn remove_input_method_config() -> Result<bool, String> {
    let mut changed = false;
    let mut errors = Vec::new();
    if has_gnome_input_sources() {
        match remove_gnome_input_source() {
            Ok(removed) => changed |= removed,
            Err(error) => errors.push(error),
        }
    }
    match remove_ibus_engine_references(true) {
        Ok(removed) => changed |= !removed.is_empty(),
        Err(error) => errors.push(error),
    }
    remove_legacy_desktop_files();
    if !errors.is_empty() {
        return Err(errors.join("; "));
    }
    Ok(changed)
}

/// Configure Alt+Space shortcut for input-source switching.
pub fn set_alt_space_switcher() -> Result<(), String> {
    let _ = run(
        "gsettings",
        &["set", WM_KEYBINDINGS_SCHEMA, "switch-input-source", "['<Alt>space']"],
    );
    let _ = run(
        "gsettings",
        &["set", WM_KEYBINDINGS_SCHEMA, "switch-input-source-backward", "[]"],
    );
    let _ = run(
        "gsettings",
        &["set", IBUS_HOTKEY_SCHEMA, "triggers", "['<Alt>space']"],
    );
    Ok(())
}

/// Run `ibus-bornona setup`.
pub fn setup() -> Result<String, String> {
    if !cfg!(target_os = "linux") {
        return Err("setup is only available on Linux".into());
    }

    let gnome = is_gnome() && has_gnome_input_sources();

    if gnome {
        configure_gnome_component_path()?;
    } else {
        let _ = install_environment_conf();
    }
    remove_legacy_desktop_files();
    let _ = install_user_icon();

    ensure_engine_registered()?;

    let mut removed_engines = Vec::new();
    if has_ibus_general_settings() {
        removed_engines = configure_ibus_engines()?;
        let _ = configure_ibus_hotkeys();
    }

    let changed = if gnome {
        let changed = add_gnome_input_source()?;
        set_alt_space_switcher()?;
        changed
    } else {
        true
    };

    if !removed_engines.is_empty() {
        return Ok(format!(
            "Registered Bornona with IBus and removed unavailable input methods: {}. Switch with Alt+Space.",
            removed_engines.join(", ")
        ));
    }

    Ok(if changed {
        if gnome {
            "Added Bornona to your GNOME input sources. Switch with Alt+Space.".to_string()
        } else if is_cosmic() {
            "Configured Bornona in IBus for COSMIC desktop. Switch with Alt+Space.".to_string()
        } else {
            "Added Bornona to your input sources. Switch with Alt+Space.".to_string()
        }
    } else {
        if gnome {
            "Bornona is already in your GNOME input sources. Switch with Alt+Space.".to_string()
        } else {
            "Bornona is already in your input sources. Switch with Alt+Space.".to_string()
        }
    })
}

/// Run `ibus-bornona doctor`: human-readable diagnostics.
pub fn doctor() -> String {
    let mut lines = Vec::new();
    let mut check = |ok: bool, what: &str, hint: &str| {
        lines.push(format!(
            "[{}] {what}{}",
            if ok { " ok " } else { "FAIL" },
            if ok {
                String::new()
            } else {
                format!("\n       -> {hint}")
            }
        ));
    };

    let xml_installed = is_component_installed();
    check(
        xml_installed,
        "IBus component file installed",
        "run `./install.sh` to install component",
    );

    let icon_installed = is_icon_installed();
    check(
        icon_installed,
        "IBus engine logo icon installed",
        "run `ibus-bornona setup` or install with `make install`",
    );

    let daemon = Command::new("pgrep")
        .args(["-x", "ibus-daemon"])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);
    check(
        daemon,
        "ibus-daemon is running",
        "run `ibus-bornona setup` or log out and back in",
    );

    check(
        engine_registered(),
        "ibus lists the bornona engine",
        "run `ibus-bornona setup` or log out and back in",
    );

    if is_gnome() && has_gnome_input_sources() {
        let sources = run("gsettings", &["get", SCHEMA, "sources"]).unwrap_or_default();
        check(
            sources.contains(SOURCE_ID),
            "Bornona is in GNOME input sources",
            "run `ibus-bornona setup`",
        );

        let shortcut = run(
            "gsettings",
            &["get", WM_KEYBINDINGS_SCHEMA, "switch-input-source"],
        )
        .unwrap_or_default();
        check(
            shortcut.contains("<Alt>space"),
            "Alt+Space switches input sources",
            "run `ibus-bornona setup`",
        );
    } else {
        let preload = run("gsettings", &["get", IBUS_GENERAL_SCHEMA, "preload-engines"]).unwrap_or_default();
        check(
            preload.contains(SOURCE_ID),
            "Bornona is in IBus preload engines",
            "run `ibus-bornona setup`",
        );
    }

    let ibus_triggers = run("gsettings", &["get", IBUS_HOTKEY_SCHEMA, "triggers"]).unwrap_or_default();
    check(
        ibus_triggers.contains("<Alt>space"),
        "IBus Alt+Space trigger is configured",
        "run `ibus-bornona setup`",
    );

    lines.join("\n")
}

/// Compose a string of keys using Bornona layout rules.
pub fn compose(keys: &str) -> String {
    crate::engine::compose(keys)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_append_source() {
        assert_eq!(
            append_source("[('xkb', 'us')]").as_deref(),
            Some("[('xkb', 'us'), ('ibus', 'bornona')]")
        );
        assert_eq!(
            append_source("@a(ss) []").as_deref(),
            Some("[('ibus', 'bornona')]")
        );
        assert_eq!(
            append_source("[('xkb', 'us'), ('ibus', 'bornona')]"),
            None
        );
    }

    #[test]
    fn test_remove_source_preserves_other_input_sources() {
        assert_eq!(
            remove_source("[('xkb', 'us'), ('ibus', 'bornona'), ('xkb', 'bd')]").unwrap(),
            Some("[('xkb', 'us'), ('xkb', 'bd')]".into())
        );
        assert_eq!(
            remove_source(" @a(ss) [('ibus', 'bornona')] ").unwrap(),
            Some("[]".into())
        );
        assert_eq!(remove_source("[('xkb', 'us')]").unwrap(), None);
        assert_eq!(
            remove_source("[('xkb', 'us, intl'), ('ibus', 'bornona')]").unwrap(),
            Some("[('xkb', 'us, intl')]".into())
        );
    }

    #[test]
    fn test_gsettings_string_list_round_trip() {
        let values = parse_gsettings_string_list("['xkb:us::eng', 'torongo']").unwrap();
        assert_eq!(values, ["xkb:us::eng", "torongo"]);
        assert_eq!(format_gsettings_string_list(&values), "['xkb:us::eng', 'torongo']");
        assert_eq!(parse_gsettings_string_list("@as []").unwrap(), Vec::<String>::new());
    }

    #[test]
    fn test_retain_registered_engines_replaces_missing_engine_with_bornona() {
        let mut configured = vec!["xkb:us::eng".into(), "torongo".into()];
        let registered = vec!["xkb:us::eng".into(), SOURCE_ID.into()];
        let removed = retain_registered_engines(&mut configured, &registered);
        assert_eq!(configured, ["xkb:us::eng", SOURCE_ID]);
        assert_eq!(removed, ["torongo"]);
    }

    #[test]
    fn test_remove_bornona_and_stale_engines_but_preserve_available_methods() {
        let mut configured = vec![
            "xkb:us::eng".into(),
            SOURCE_ID.into(),
            "torongo".into(),
            "hangul".into(),
        ];
        let registered = vec!["xkb:us::eng".into(), "hangul".into()];
        let removed = retain_configured_engines(&mut configured, Some(&registered), true);
        assert_eq!(configured, ["xkb:us::eng", "hangul"]);
        assert_eq!(removed, [SOURCE_ID, "torongo"]);
    }

    #[test]
    fn test_embedded_icon_svg_is_valid() {
        assert!(EMBEDDED_ICON_SVG.contains("<svg"));
        assert!(EMBEDDED_ICON_SVG.contains("ibus-bornona") || EMBEDDED_ICON_SVG.contains("ব"));
        assert!(EMBEDDED_ICON_SVG.ends_with("</svg>\n") || EMBEDDED_ICON_SVG.ends_with("</svg>"));
    }

    #[test]
    fn test_compose() {
        assert_eq!(compose("k"), "ক");
        assert_eq!(compose("ka"), "কা");
    }
}
