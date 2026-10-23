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
const GNOME_IBUS_UNIT: &str = "org.freedesktop.IBus.session.GNOME.service";
const GNOME_IBUS_EXEC_START: &str = "sh -c 'exec /usr/bin/ibus-daemon --panel disable --cache=none $([ \"$XDG_SESSION_TYPE\" = \"x11\" ] && echo \"--xim\")'";

fn is_cosmic() -> bool {
    env::var("XDG_CURRENT_DESKTOP")
        .or_else(|_| env::var("XDG_SESSION_DESKTOP"))
        .map(|desktop| desktop.to_ascii_lowercase().contains("cosmic"))
        .unwrap_or(false)
}

/// Run a command and return its stdout, or an error string if failed.
fn run(cmd: &str, args: &[&str]) -> Result<String, String> {
    let out = Command::new(cmd)
        .args(args)
        .output()
        .map_err(|e| format!("cannot run `{cmd}`: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "`{cmd} {}` failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
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
    run("ibus", &["list-engine"])
        .map(|s| s.contains(SOURCE_ID))
        .unwrap_or(false)
}

/// Ensure ibus-daemon is running and knows the Bornona engine.
pub fn ensure_engine_registered() -> Result<(), String> {
    if run("ibus", &["list-engine"]).is_err() {
        Command::new("ibus-daemon")
            .args(["-drx"])
            .spawn()
            .map_err(|e| format!("cannot start ibus-daemon: {e}"))?;
        thread::sleep(Duration::from_millis(700));
    }
    if engine_registered() {
        return Ok(());
    }
    let _ = run("ibus", &["restart"]);
    for _ in 0..20 {
        thread::sleep(Duration::from_millis(500));
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
            '\\' if quoted => escaped = true,
            '\'' => quoted = !quoted,
            '(' if !quoted => tuple_depth += 1,
            ')' if !quoted => tuple_depth -= 1,
            ',' if !quoted && tuple_depth == 0 => {
                entries.push(inner[start..index].trim());
                start = index + character.len_utf8();
            }
            _ => {}
        }
    }
    if quoted || escaped || tuple_depth != 0 {
        return Err(format!("unexpected input-sources value: {current}"));
    }
    entries.push(inner[start..].trim());
    Ok(entries)
}

fn is_bornona_source(entry: &str) -> bool {
    let mut normalized = String::with_capacity(entry.len());
    let mut quoted = false;
    let mut escaped = false;
    for character in entry.chars() {
        if escaped {
            normalized.push(character);
            escaped = false;
            continue;
        }
        match character {
            '\\' if quoted => {
                normalized.push(character);
                escaped = true;
            }
            '\'' => {
                normalized.push(character);
                quoted = !quoted;
            }
            character if !quoted && character.is_whitespace() => {}
            _ => normalized.push(character),
        }
    }
    normalized == "('ibus','bornona')"
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

    configure_gnome_component_path()?;
    let _ = run("ibus", &["write-cache"]);

    ensure_engine_registered()?;
    let changed = if is_cosmic() {
        let _ = run(
            "gsettings",
            &["set", IBUS_HOTKEY_SCHEMA, "triggers", "['<Alt>space']"],
        );
        false
    } else {
        if run("gsettings", &["list-keys", SCHEMA]).is_err() {
            return Err("desktop input-source settings not found; add Bornona from your desktop's input-source settings".into());
        }
        let changed = add_gnome_input_source()?;
        set_alt_space_switcher()?;
        changed
    };

    Ok(if changed {
        if is_cosmic() {
            "Registered Bornona with IBus. Add/select Bornona in COSMIC Input Sources; switch with Alt+Space.".to_string()
        } else {
            "Added Bornona to your input sources. Switch with Alt+Space.".to_string()
        }
    } else {
        if is_cosmic() {
            "Bornona is ready in IBus. Select it in COSMIC Input Sources; switch with Alt+Space.".to_string()
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

    let xml_installed = Path::new("/usr/share/ibus/component/ibus-bornona.xml").exists()
        || Path::new("/usr/local/share/ibus/component/ibus-bornona.xml").exists()
        || env::var_os("HOME")
            .map(|h| PathBuf::from(h).join(".local/share/ibus/component/ibus-bornona.xml"))
            .map(|p| p.exists())
            .unwrap_or(false);

    check(
        xml_installed,
        "IBus component file installed",
        "run `./install.sh` to install component",
    );

    let daemon = Command::new("pgrep")
        .args(["-x", "ibus-daemon"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    check(
        daemon,
        "ibus-daemon is running",
        "run `ibus-daemon -drx` or log out and back in",
    );

    check(
        engine_registered(),
        "ibus lists the bornona engine",
        "run `ibus restart` or log out and back in",
    );

    if is_cosmic() {
        check(
            true,
            "COSMIC input-source integration detected",
            "run `ibus-bornona setup`",
        );
    } else {
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
    fn test_compose() {
        assert_eq!(compose("k"), "ক");
        assert_eq!(compose("ka"), "কা");
    }
}
