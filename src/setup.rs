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
const COSMIC_IBUS_SERVICE: &str = "ibus-bornona.service";
const GENERIC_IBUS_UNIT: &str = "org.freedesktop.IBus.session.generic.service";
const COSMIC_IBUS_MARKER: &str = "# Managed by ibus-bornona.";
const COSMIC_IBUS_LINK_MARKER: &str = "# ibus-bornona owns the COSMIC session link";
const GNOME_IBUS_EXEC_START: &str = "sh -c 'exec /usr/bin/ibus-daemon --panel disable --cache=none $([ \"$XDG_SESSION_TYPE\" = \"x11\" ] && echo \"--xim\") 2>/dev/null'";

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

fn has_gnome_schema() -> bool {
    run("gsettings", &["list-keys", SCHEMA]).is_ok()
}

fn has_gnome_input_sources() -> bool {
    is_gnome() && has_gnome_schema()
}

fn has_ibus_general_settings() -> bool {
    run("gsettings", &["list-keys", IBUS_GENERAL_SCHEMA]).is_ok()
}

#[allow(dead_code)]
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

fn config_home() -> Result<PathBuf, String> {
    match env::var_os("XDG_CONFIG_HOME") {
        Some(path) if !path.is_empty() => {
            let path = PathBuf::from(path);
            if !path.is_absolute() {
                return Err("XDG_CONFIG_HOME must be an absolute path".into());
            }
            Ok(path)
        }
        _ => env::var_os("HOME")
            .map(PathBuf::from)
            .map(|home| home.join(".config"))
            .ok_or_else(|| "HOME is not set".to_string()),
    }
}

fn autostart_dir() -> Result<PathBuf, String> {
    Ok(config_home()?.join("autostart"))
}

const EMBEDDED_ICON_SVG: &str = include_str!("../data/logo/ibus-bornona.svg");
const EMBEDDED_ICON_PNG: &[u8] = include_bytes!("../data/logo/ibus-bornona.png");

pub fn is_icon_installed() -> bool {
    let xdg_data_home = env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")));

    let icon_names = [
        "icons/hicolor/scalable/apps/ibus-bornona.svg",
        "icons/hicolor/128x128/apps/ibus-bornona.png",
        "ibus-bornona/ibus-bornona.png",
        "ibus-bornona/ibus-bornona.svg",
    ];

    for prefix in ["/usr/share", "/usr/local/share"] {
        for icon in &icon_names {
            if Path::new(prefix).join(icon).exists() {
                return true;
            }
        }
    }

    if let Some(data_home) = xdg_data_home {
        for icon in &icon_names {
            if data_home.join(icon).exists() {
                return true;
            }
        }
    }

    env::var_os("XDG_DATA_DIRS")
        .into_iter()
        .flat_map(|paths| env::split_paths(&paths).collect::<Vec<_>>())
        .any(|path| icon_names.iter().any(|icon| path.join(icon).exists()))
}

pub fn install_user_icon() -> Result<(), String> {
    let Some(home) = env::var_os("HOME").map(PathBuf::from) else {
        return Ok(());
    };
    let svg_dir = home.join(".local/share/icons/hicolor/scalable/apps");
    let png_dir = home.join(".local/share/icons/hicolor/128x128/apps");
    let pkgdata_dir = home.join(".local/share/ibus-bornona");

    let _ = std::fs::create_dir_all(&svg_dir);
    let _ = std::fs::create_dir_all(&png_dir);
    let _ = std::fs::create_dir_all(&pkgdata_dir);

    let _ = std::fs::write(svg_dir.join("ibus-bornona.svg"), EMBEDDED_ICON_SVG);
    let _ = std::fs::write(png_dir.join("ibus-bornona.png"), EMBEDDED_ICON_PNG);
    let _ = std::fs::write(pkgdata_dir.join("ibus-bornona.png"), EMBEDDED_ICON_PNG);
    let _ = std::fs::write(pkgdata_dir.join("ibus-bornona.svg"), EMBEDDED_ICON_SVG);

    let _ = Command::new("gtk-update-icon-cache")
        .args(["-f", "-t", home.join(".local/share/icons/hicolor").to_str().unwrap()])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    Ok(())
}

fn remove_legacy_desktop_files() {
    if let Ok(dir) = autostart_dir() {
        let _ = std::fs::remove_file(dir.join("ibus-bornona-cosmic.desktop"));
        let _ = std::fs::remove_file(dir.join("ibus-bornona.desktop"));
    }
}

fn install_autostart_desktop() -> Result<(), String> {
    let dir = autostart_dir()?;
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    let desktop_file = dir.join("ibus-daemon.desktop");
    let panel = find_wayland_ibus_panel();
    let home = env::var("HOME").unwrap_or_default();
    let comp_path = if !home.is_empty() {
        format!("{home}/.local/share/ibus/component:/usr/share/ibus/component:/usr/local/share/ibus/component")
    } else {
        "/usr/share/ibus/component:/usr/local/share/ibus/component".to_string()
    };
    let contents = autostart_desktop_contents(&comp_path, panel.as_deref());
    std::fs::write(&desktop_file, contents)
        .map_err(|e| format!("cannot write {}: {e}", desktop_file.display()))?;
    reload_user_systemd_manager()?;
    Ok(())
}

fn cosmic_ibus_service_contents(
    component_path: &str,
    daemon_path: &str,
    panel: &str,
    owns_link: bool,
) -> String {
    format!(
        "{}\n{}[Unit]\nPartOf=cosmic-session.target\nAfter=graphical-session-pre.target\n\n[Service]\nType=exec\nEnvironment=\"IBUS_COMPONENT_PATH={}\"\nEnvironment=\"XMODIFIERS=@im=ibus\"\nExecStart=\"{}\" --replace --xim --restart --cache=none \"{}\"\nRestart=on-failure\nRestartSec=2s\n\n[Install]\nWantedBy=cosmic-session.target\n",
        COSMIC_IBUS_MARKER,
        if owns_link {
            format!("{COSMIC_IBUS_LINK_MARKER}\n")
        } else {
            String::new()
        },
        systemd_escape(component_path),
        systemd_escape(daemon_path),
        systemd_escape(&format!("--panel={}", wayland_panel_command(panel)))
    )
}

fn systemd_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('%', "%%")
}

fn configure_cosmic_ibus_service() -> Result<bool, String> {
    if !is_cosmic()
        || !env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .is_some_and(|runtime_dir| runtime_dir.join("systemd/private").exists())
        || run("systemctl", &["--user", "cat", "cosmic-session.target"]).is_err()
    {
        return Ok(false);
    }

    let panel = find_wayland_ibus_panel().ok_or_else(|| {
        "COSMIC Wayland input requires `ibus-ui-gtk3`; install the IBus GTK panel and run `ibus-bornona setup` again".to_string()
    })?;
    let daemon_path = run("which", &["ibus-daemon"])?;
    let daemon_path = daemon_path.trim();
    if !Path::new(daemon_path).is_absolute() {
        return Err("cannot locate an absolute path for ibus-daemon".into());
    }
    let unit_dir = config_home()?.join("systemd/user");
    std::fs::create_dir_all(&unit_dir)
        .map_err(|error| format!("cannot create {}: {error}", unit_dir.display()))?;
    let unit_file = unit_dir.join(COSMIC_IBUS_SERVICE);
    let previous_contents = match std::fs::read_to_string(&unit_file) {
        Ok(contents) if contents.contains(COSMIC_IBUS_MARKER) => contents,
        Ok(_) => {
            return Err(format!(
                "refusing to replace unrecognized unit {}",
                unit_file.display()
            ));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(format!("cannot read {}: {error}", unit_file.display())),
    };
    let wants_dir = unit_dir.join("cosmic-session.target.wants");
    std::fs::create_dir_all(&wants_dir)
        .map_err(|error| format!("cannot create {}: {error}", wants_dir.display()))?;
    let link = wants_dir.join(COSMIC_IBUS_SERVICE);
    let previous_link_owned = previous_contents.contains(COSMIC_IBUS_LINK_MARKER);
    let mut owns_link = previous_link_owned;
    match std::fs::read_link(&link) {
        Ok(existing) if existing == unit_file && previous_link_owned => {}
        Ok(existing) if existing == unit_file => {
            return Err(format!(
                "refusing to adopt unexpected systemd link {}",
                link.display()
            ));
        }
        Ok(_) => {
            return Err(format!(
                "refusing to replace unexpected systemd link {}",
                link.display()
            ));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            owns_link = true;
        }
        Err(error) => return Err(format!("cannot read {}: {error}", link.display())),
    }
    let contents = cosmic_ibus_service_contents(
        &component_path_env(),
        daemon_path,
        &panel,
        owns_link,
    );
    std::fs::write(&unit_file, contents)
        .map_err(|error| format!("cannot write {}: {error}", unit_file.display()))?;
    if owns_link {
        match std::fs::symlink_metadata(&link) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                std::os::unix::fs::symlink(&unit_file, &link)
                    .map_err(|error| format!("cannot create {}: {error}", link.display()))?;
            }
            Ok(_) => {}
            Err(error) => return Err(format!("cannot inspect {}: {error}", link.display())),
        }
    }

    remove_owned_autostart_desktop()?;
    let _ = remove_legacy_generic_cosmic_service(&unit_dir)?;
    reload_user_systemd_manager()?;
    run("systemctl", &["--user", "restart", COSMIC_IBUS_SERVICE])?;
    Ok(true)
}

fn remove_owned_autostart_desktop() -> Result<(), String> {
    let dir = autostart_dir()?;
    let desktop_file = dir.join("ibus-daemon.desktop");
    match std::fs::read_to_string(&desktop_file) {
        Ok(contents) if contents.contains("Comment=Start IBus daemon with Bornona support") => {
            std::fs::remove_file(&desktop_file)
                .map_err(|error| format!("cannot remove {}: {error}", desktop_file.display()))
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("cannot read {}: {error}", desktop_file.display())),
    }
}

fn remove_legacy_generic_cosmic_service(unit_dir: &Path) -> Result<bool, String> {
    let dropin_dir = unit_dir.join(format!("{GENERIC_IBUS_UNIT}.d"));
    let dropin = dropin_dir.join("ibus-bornona.conf");
    let link = unit_dir
        .join("cosmic-session.target.wants")
        .join(GENERIC_IBUS_UNIT);
    let owns_link = match std::fs::read_to_string(&dropin) {
        Ok(contents) => contents.contains(COSMIC_IBUS_LINK_MARKER),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(error) => return Err(format!("cannot read {}: {error}", dropin.display())),
    };
    if owns_link {
        for path in [&link, &dropin] {
            match std::fs::remove_file(path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(format!("cannot remove {}: {error}", path.display())),
            }
        }
        let _ = std::fs::remove_dir(&dropin_dir);
        Ok(true)
    } else {
        Ok(false)
    }
}

fn cosmic_ibus_service_configured() -> bool {
    let Ok(config_home) = config_home() else {
        return false;
    };
    let unit_dir = config_home.join("systemd/user");
    let service = unit_dir.join(COSMIC_IBUS_SERVICE);
    std::fs::read_to_string(service)
        .is_ok_and(|contents| contents.contains(COSMIC_IBUS_MARKER))
        && unit_dir
            .join("cosmic-session.target.wants")
            .join(COSMIC_IBUS_SERVICE)
            .is_symlink()
}

fn reload_user_systemd_manager() -> Result<(), String> {
    let Some(runtime_dir) = env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from) else {
        return Ok(());
    };
    if !runtime_dir.join("systemd/private").exists() {
        return Ok(());
    }

    let output = Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .stdin(std::process::Stdio::null())
        .output()
        .map_err(|error| format!("cannot reload user systemd manager: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "cannot reload user systemd manager: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(())
}

fn desktop_exec_arg(value: &str) -> String {
    let escaped = value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('`', "\\`")
        .replace('$', "\\$")
        .replace('%', "%%");
    format!("\"{escaped}\"")
}

fn autostart_desktop_contents(comp_path: &str, panel: Option<&str>) -> String {
    let mut args = vec![
        "env".to_string(),
        desktop_exec_arg(&format!("IBUS_COMPONENT_PATH={comp_path}")),
        "ibus-daemon".to_string(),
        "--daemonize".to_string(),
        "--replace".to_string(),
        "--xim".to_string(),
        "--restart".to_string(),
        "--cache=none".to_string(),
    ];
    if let Some(panel) = panel {
        args.push(desktop_exec_arg(&format!(
            "--panel={}",
            wayland_panel_command(panel)
        )));
    }
    format!(
        "[Desktop Entry]\n\
Type=Application\n\
Name=IBus Daemon\n\
Comment=Start IBus daemon with Bornona support\n\
Exec={}\n\
Terminal=false\n\
NotShowIn=GNOME;\n",
args.join(" ")
    )
}

fn environment_conf_contents() -> String {
    String::from(
        "IBUS_COMPONENT_PATH=\"${HOME}/.local/share/ibus/component:/usr/share/ibus/component:/usr/local/share/ibus/component\"\nXMODIFIERS=@im=ibus\n",
    )
}

fn install_environment_conf() -> Result<(), String> {
    let Some(home) = env::var_os("HOME").map(PathBuf::from) else {
        return Ok(());
    };
    let env_dir = home.join(".config/environment.d");
    std::fs::create_dir_all(&env_dir)
        .map_err(|e| format!("cannot create {}: {e}", env_dir.display()))?;
    let env_file = env_dir.join("50-ibus-bornona.conf");
    let contents = environment_conf_contents();
    std::fs::write(&env_file, contents)
        .map_err(|e| format!("cannot write {}: {e}", env_file.display()))?;
    Ok(())
}

fn import_session_environment() {
    let component_path = component_path_env();
    let variables = vec![
        format!("IBUS_COMPONENT_PATH={component_path}"),
        "XMODIFIERS=@im=ibus".to_string(),
    ];

    if is_cosmic() {
        let mut dbus_args = vec!["--systemd".to_string()];
        dbus_args.extend(variables.iter().cloned());
        dbus_args.extend([
            "GTK_IM_MODULE=".to_string(),
            "QT_IM_MODULE=".to_string(),
        ]);
        let dbus_args = dbus_args.iter().map(String::as_str).collect::<Vec<_>>();
        match run_command_status(
            "dbus-update-activation-environment",
            &dbus_args,
            &[],
        ) {
            Ok(true) => {}
            Ok(false) => eprintln!("ibus-bornona: could not update D-Bus activation environment"),
            Err(error) => eprintln!("ibus-bornona: {error}"),
        }
    }

    let mut args = vec!["--user", "set-environment"];
    args.extend(variables.iter().map(String::as_str));
    let _ = Command::new("systemctl")
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    let _ = Command::new("systemctl")
        .args([
            "--user",
            "unset-environment",
            "GTK_IM_MODULE",
            "QT_IM_MODULE",
        ])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
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
    run("which", &["ibus-ui-gtk3"])
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn wayland_panel_command(panel: &str) -> String {
    format!("{panel} -i")
}

fn ibus_daemon_uses_wayland_panel(panel: &str) -> bool {
    Command::new("pgrep")
        .args(["-a", "-x", "ibus-daemon"])
        .output()
        .map(|output| {
            output.status.success()
                && daemon_command_uses_wayland_panel(
                    &String::from_utf8_lossy(&output.stdout),
                    panel,
                )
        })
        .unwrap_or(false)
}

fn daemon_command_uses_wayland_panel(commands: &str, panel: &str) -> bool {
    commands
    .lines()
    .any(|line| line.contains(panel) && line.contains("-i"))
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

pub fn normalize_keyboard_switch_shortcut(value: &str) -> Result<(String, String), String> {
    let parts: Vec<_> = value.split('+').map(str::trim).collect();
    if parts.len() < 2 || parts.iter().any(|part| part.is_empty()) {
        return Err("shortcut must include a modifier and key, for example `Alt+Space`".into());
    }

    let mut modifiers = Vec::new();
    for modifier in &parts[..parts.len() - 1] {
        let normalized = match modifier.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => "Ctrl",
            "alt" => "Alt",
            "shift" => "Shift",
            "super" | "meta" => "Super",
            _ => return Err(format!("unsupported shortcut modifier `{modifier}`")),
        };
        if modifiers.contains(&normalized) {
            return Err(format!("duplicate shortcut modifier `{modifier}`"));
        }
        modifiers.push(normalized);
    }

    let key = parts[parts.len() - 1];
    let (display_key, accelerator_key) = match key.to_ascii_lowercase().as_str() {
        "space" => ("Space".to_string(), "space".to_string()),
        "tab" => ("Tab".to_string(), "Tab".to_string()),
        "enter" | "return" => ("Enter".to_string(), "Return".to_string()),
        "esc" | "escape" => ("Escape".to_string(), "Escape".to_string()),
        "backspace" => ("Backspace".to_string(), "BackSpace".to_string()),
        "left" => ("Left".to_string(), "Left".to_string()),
        "right" => ("Right".to_string(), "Right".to_string()),
        "up" => ("Up".to_string(), "Up".to_string()),
        "down" => ("Down".to_string(), "Down".to_string()),
        key if key.len() == 1 && key.as_bytes()[0].is_ascii_alphabetic() => {
            (key.to_ascii_uppercase(), key.to_string())
        }
        key if key.len() == 1 && key.as_bytes()[0].is_ascii_digit() => {
            (key.to_string(), key.to_string())
        }
        key if key.starts_with('f')
            && key[1..]
                .parse::<u8>()
                .is_ok_and(|number| (1..=35).contains(&number)) =>
        {
            let key = key.to_ascii_uppercase();
            (key.clone(), key)
        }
        _ => {
            return Err(format!(
                "unsupported shortcut key `{key}`; use a letter, digit, F1-F35, or a named key"
            ));
        }
    };

    let modifier_order = ["Ctrl", "Alt", "Shift", "Super"];
    let mut display_parts = Vec::new();
    let mut accelerator = String::new();
    for modifier in modifier_order {
        if modifiers.contains(&modifier) {
            display_parts.push(modifier);
            accelerator.push_str(match modifier {
                "Ctrl" => "<Control>",
                "Alt" => "<Alt>",
                "Shift" => "<Shift>",
                "Super" => "<Super>",
                _ => unreachable!(),
            });
        }
    }
    display_parts.push(&display_key);
    accelerator.push_str(&accelerator_key);
    Ok((display_parts.join("+"), accelerator))
}

pub fn apply_keyboard_switch_shortcut(shortcut: &str) -> Result<(), String> {
    let (_, accelerator) = normalize_keyboard_switch_shortcut(shortcut)?;
    let mut configured = false;
    if is_gnome() && has_gnome_schema() {
        configure_gnome_switch_shortcut(&accelerator)?;
        configured = true;
    }
    if has_ibus_general_settings() {
        configure_ibus_hotkeys(&accelerator)?;
        configured = true;
    }
    if !configured {
        return Err("no supported GNOME or IBus shortcut settings are available".into());
    }
    Ok(())
}

fn configure_ibus_hotkeys(accelerator: &str) -> Result<(), String> {
    let triggers = format!("['{accelerator}']");
    let next_engine = format!("['{accelerator}']");
    run(
        "gsettings",
        &["set", IBUS_HOTKEY_SCHEMA, "triggers", &triggers],
    )?;
    run(
        "gsettings",
        &["set", IBUS_HOTKEY_SCHEMA, "next-engine", &next_engine],
    )?;
    Ok(())
}

fn remove_legacy_cosmic_toggle_shortcut() -> Result<bool, String> {
    let Some(home) = env::var_os("HOME").map(PathBuf::from) else {
        return Ok(false);
    };
    let custom_file = home.join(".config/cosmic/com.system76.CosmicSettings.Shortcuts/v1/custom");
    if !custom_file.is_file() {
        return Ok(false);
    }

    let content = std::fs::read_to_string(&custom_file)
        .map_err(|error| format!("cannot read {}: {error}", custom_file.display()))?;
    let filtered: Vec<_> = content
        .lines()
        .filter(|line| !line.contains("ibus-bornona toggle"))
        .collect();
    if filtered.len() == content.lines().count() {
        return Ok(false);
    }
    std::fs::write(&custom_file, filtered.join("\n") + "\n")
        .map_err(|error| format!("cannot update {}: {error}", custom_file.display()))?;
    Ok(true)
}

pub fn configure_ibus_panel() -> Result<(), String> {
    let _ = run(
        "gsettings",
        &["set", "org.freedesktop.ibus.panel", "show-icon-on-systray", "true"],
    );
    let _ = run(
        "gsettings",
        &["set", "org.freedesktop.ibus.panel", "show", "0"],
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
    if !is_gnome() {
        return Ok(false);
    }
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
    let envs = [
        ("IBUS_COMPONENT_PATH", comp_path.as_str()),
        ("XMODIFIERS", "@im=ibus"),
    ];

    let _ = run_command_status("ibus", &["write-cache"], &envs);

    let panel = if is_gnome() {
        None
    } else {
        find_wayland_ibus_panel()
    };
    if is_cosmic() && panel.is_none() {
        return Err(
            "COSMIC Wayland input requires `ibus-ui-gtk3`; install the IBus GTK panel and run `ibus-bornona setup` again"
                .into(),
        );
    }
    let cosmic_panel_ready = !is_cosmic()
        || panel
            .as_deref()
            .is_some_and(ibus_daemon_uses_wayland_panel);
    if engine_registered() && cosmic_panel_ready {
        return Ok(());
    }

    let daemon_running = Command::new("pgrep")
        .args(["-x", "ibus-daemon"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    let mut daemon_cmd = Command::new("ibus-daemon");
    daemon_cmd.env("IBUS_COMPONENT_PATH", &comp_path);
    daemon_cmd.env("XMODIFIERS", "@im=ibus");
    let mut args = vec!["-drxR", "--cache=none"];
    let panel_arg;
    if daemon_running && is_cosmic() && panel.is_some() {
        args.push("--replace");
    }
    if is_gnome() {
        args.push("--panel=disable");
    } else if let Some(panel) = panel {
        panel_arg = wayland_panel_command(&panel);
        args.push("--panel");
        args.push(&panel_arg);
    }
    daemon_cmd.args(&args);
    daemon_cmd.stdin(std::process::Stdio::null());
    daemon_cmd.stdout(std::process::Stdio::null());
    daemon_cmd.stderr(std::process::Stdio::null());

    if daemon_running && !(is_cosmic() && find_wayland_ibus_panel().is_some()) {
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
    if has_gnome_schema() {
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
    remove_legacy_cosmic_toggle_shortcut()?;
    match remove_cosmic_ibus_service() {
        Ok(removed) => changed |= removed,
        Err(error) => errors.push(error),
    }
    if let Ok(dir) = autostart_dir() {
        let file = dir.join("ibus-daemon.desktop");
        if file.is_file() {
            let _ = std::fs::remove_file(file);
            changed = true;
        }
    }
    if let Some(home) = env::var_os("HOME").map(PathBuf::from) {
        let conf = env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".config"))
            .join("environment.d/50-ibus-bornona.conf");
        if conf.is_file() {
            let _ = std::fs::remove_file(conf);
            changed = true;
        }
        let config_home = env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".config"));
        let gnome_dropin = config_home.join(format!(
            "systemd/user/{GNOME_IBUS_UNIT}.d/ibus-bornona.conf"
        ));
        if gnome_dropin.is_file() {
            let _ = std::fs::remove_file(&gnome_dropin);
            let _ = std::fs::remove_dir(
                config_home.join(format!("systemd/user/{GNOME_IBUS_UNIT}.d")),
            );
            changed = true;
        }
    }
    if !errors.is_empty() {
        return Err(errors.join("; "));
    }
    Ok(changed)
}

fn remove_cosmic_ibus_service() -> Result<bool, String> {
    let Ok(config_home) = config_home() else {
        return Ok(false);
    };
    let unit_dir = config_home.join("systemd/user");
    let service = unit_dir.join(COSMIC_IBUS_SERVICE);
    let service_contents = match std::fs::read_to_string(&service) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(format!("cannot read {}: {error}", service.display())),
    };
    let owns_service = service_contents.contains(COSMIC_IBUS_MARKER);
    let owns_link = service_contents.contains(COSMIC_IBUS_LINK_MARKER);
    let link = unit_dir
        .join("cosmic-session.target.wants")
        .join(COSMIC_IBUS_SERVICE);
    let runtime_active = env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .is_some_and(|runtime_dir| runtime_dir.join("systemd/private").exists());
    if owns_service
        && owns_link
        && runtime_active
        && run("systemctl", &["--user", "is-active", COSMIC_IBUS_SERVICE]).is_ok()
    {
        run("systemctl", &["--user", "stop", COSMIC_IBUS_SERVICE])?;
    }
    let mut removed = false;
    for path in [
        owns_service.then_some(&service),
        (owns_service && owns_link).then_some(&link),
    ]
        .into_iter()
        .flatten()
    {
        match std::fs::remove_file(path) {
            Ok(()) => removed = true,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(format!("cannot remove {}: {error}", path.display())),
        }
    }
    if owns_service && owns_link {
        let _ = std::fs::remove_dir(link.parent().unwrap());
    }
    removed |= remove_legacy_generic_cosmic_service(&unit_dir)?;
    if removed {
        reload_user_systemd_manager()?;
    }
    Ok(removed)
}

fn configure_gnome_switch_shortcut(accelerator: &str) -> Result<(), String> {
    let switch_source = format!("['{accelerator}']");
    run(
        "gsettings",
        &["set", WM_KEYBINDINGS_SCHEMA, "switch-input-source", &switch_source],
    )?;
    run(
        "gsettings",
        &["set", WM_KEYBINDINGS_SCHEMA, "switch-input-source-backward", "[]"],
    )?;
    Ok(())
}

/// Run `ibus-bornona setup`.
pub fn setup() -> Result<String, String> {
    if !cfg!(target_os = "linux") {
        return Err("setup is only available on Linux".into());
    }

    let config = crate::config::load()?;
    let (_, accelerator) =
        normalize_keyboard_switch_shortcut(&config.keyboard_switch_shortcut)?;

    if is_gnome() && has_gnome_schema() {
        configure_gnome_component_path()?;
        add_gnome_input_source()?;
        configure_gnome_switch_shortcut(&accelerator)?;
    }
    install_environment_conf()?;
    if !configure_cosmic_ibus_service()? {
        install_autostart_desktop()?;
    }
    remove_legacy_desktop_files();
    let _ = remove_legacy_cosmic_toggle_shortcut()?;
    import_session_environment();
    let _ = install_user_icon();
    ensure_engine_registered()?;

    let mut removed_engines = Vec::new();
    if has_ibus_general_settings() {
        removed_engines = configure_ibus_engines()?;
        configure_ibus_hotkeys(&accelerator)?;
        let _ = configure_ibus_panel();
    }

    if !removed_engines.is_empty() {
        return Ok(format!(
            "Bornona is ready in IBus. Removed unavailable input methods: {}. Switch layouts with {}.",
            removed_engines.join(", "),
            config.keyboard_switch_shortcut
        ));
    }

    Ok(if is_gnome() {
        format!(
            "Bornona is ready in your GNOME input sources. Switch layouts with {}.",
            config.keyboard_switch_shortcut
        )
    } else if is_cosmic() {
        format!(
            "Bornona is ready in IBus. Press {} to switch.",
            config.keyboard_switch_shortcut
        )
    } else {
        format!(
            "Bornona is ready in IBus. Switch layouts with {}.",
            config.keyboard_switch_shortcut
        )
    })
}

/// Run `ibus-bornona doctor`: human-readable diagnostics.
pub fn doctor() -> String {
    let config = crate::config::load();
    let switch_accelerator = match config.as_ref() {
        Ok(config) => normalize_keyboard_switch_shortcut(&config.keyboard_switch_shortcut)
            .map(|(_, accelerator)| accelerator),
        Err(error) => Err(error.clone()),
    };
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

    if has_gnome_input_sources() {
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
        let shortcut_name = config
            .as_ref()
            .map(|config| config.keyboard_switch_shortcut.as_str())
            .unwrap_or("configured shortcut");
        check(
            switch_accelerator
                .as_ref()
                .is_ok_and(|accelerator| shortcut.contains(accelerator)),
            &format!("GNOME {shortcut_name} switches input sources"),
            "run `ibus-bornona setup`",
        );
    }

    if is_cosmic() {
        let panel = find_wayland_ibus_panel();
        let panel_ready = panel
            .as_deref()
            .is_some_and(ibus_daemon_uses_wayland_panel);
        check(
            panel_ready,
            "COSMIC IBus panel uses the Wayland input-method protocol",
            "run `ibus-bornona setup` and ensure `ibus-ui-gtk3` is installed",
        );
    }

    if has_ibus_general_settings() {
        let preload = run("gsettings", &["get", IBUS_GENERAL_SCHEMA, "preload-engines"]).unwrap_or_default();
        check(
            preload.contains(SOURCE_ID),
            "Bornona is in IBus preload engines",
            "run `ibus-bornona setup`",
        );
    }

    let autostart_ok = if is_cosmic() && cosmic_ibus_service_configured() {
        true
    } else {
        autostart_dir()
            .map(|dir| dir.join("ibus-daemon.desktop").is_file())
            .unwrap_or(false)
    };
    check(
        autostart_ok,
        if is_cosmic() {
            "IBus daemon starts with the COSMIC session"
        } else {
            "IBus daemon autostart desktop entry configured"
        },
        "run `ibus-bornona setup`",
    );

    let env_ok = env::var_os("HOME")
        .map(PathBuf::from)
        .map(|h| h.join(".config/environment.d/50-ibus-bornona.conf").is_file())
        .unwrap_or(false);
    check(
        env_ok,
        "IBus environment configuration installed (~/.config/environment.d/50-ibus-bornona.conf)",
        "run `ibus-bornona setup`",
    );

    let ibus_triggers =
        run("gsettings", &["get", IBUS_HOTKEY_SCHEMA, "triggers"]).unwrap_or_default();
    let shortcut_name = config
        .as_ref()
        .map(|config| config.keyboard_switch_shortcut.as_str())
        .unwrap_or("configured shortcut");
    check(
        switch_accelerator
            .as_ref()
            .is_ok_and(|accelerator| ibus_triggers.contains(accelerator)),
        &format!("IBus {shortcut_name} trigger is configured"),
        "run `ibus-bornona setup`",
    );

    match (config, switch_accelerator) {
        (Ok(config), Ok(_)) => lines.push(format!(
            "[ ok ] Keyboard switching shortcut: {}",
            config.keyboard_switch_shortcut
        )),
        (Ok(_), Err(error)) => {
            lines.push(format!("[FAIL] Invalid keyboard switching shortcut: {error}"))
        }
        (Err(error), _) => lines.push(format!("[FAIL] Could not read Bornona settings: {error}")),
    }

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
    fn test_embedded_icon_png_is_valid() {
        // PNG header is 89 50 4E 47 0D 0A 1A 0A
        assert!(EMBEDDED_ICON_PNG.starts_with(b"\x89PNG\r\n\x1a\n"));
    }

    #[test]
    fn test_compose() {
        assert_eq!(compose("k"), "ক");
        assert_eq!(compose("ka"), "কা");
    }

    #[test]
    fn test_autostart_desktop_creation() {
        let read = autostart_desktop_contents(
            "/home/user/.local/share/ibus/component:/usr/share/ibus/component:/usr/local/share/ibus/component",
            Some("/usr/libexec/ibus-ui-gtk3"),
        );
        assert!(read.contains("IBUS_COMPONENT_PATH"));
        assert!(read.contains("Exec=env "));
        assert!(read.contains("ibus-daemon --daemonize --replace --xim --restart --cache=none"));
        assert!(read.contains("\"--panel=/usr/libexec/ibus-ui-gtk3 -i\""));
        assert!(!read.contains("sh -c"));
        assert!(read.contains("NotShowIn=GNOME;"));
        assert!(!read.contains("X-GNOME-Autostart-enabled=false"));
        assert!(GNOME_IBUS_EXEC_START.contains("2>/dev/null"));
    }

    #[test]
    fn cosmic_ibus_service_starts_with_the_session_and_keeps_bornona_components_visible() {
        let contents = cosmic_ibus_service_contents(
            "/home/user/.local/share/ibus/component:/usr/share/ibus/component",
            "/usr/bin/ibus-daemon",
            "/usr/libexec/ibus-ui-gtk3",
            true,
        );
        assert!(contents.contains(COSMIC_IBUS_LINK_MARKER));
        assert!(contents.contains("[Service]\nType=exec"));
        assert!(contents.contains("PartOf=cosmic-session.target"));
        assert!(contents.contains("After=graphical-session-pre.target"));
        assert!(contents.contains("Environment=\"IBUS_COMPONENT_PATH=/home/user/.local/share/ibus/component:/usr/share/ibus/component\""));
        assert!(
            contents.contains("ExecStart=\"/usr/bin/ibus-daemon\" --replace --xim --restart --cache=none \"--panel=/usr/libexec/ibus-ui-gtk3 -i\""),
            "{contents:?}"
        );
        assert!(contents.contains("Environment=\"XMODIFIERS=@im=ibus\""));
        assert!(contents.contains("Restart=on-failure"));
        assert!(contents.contains("WantedBy=cosmic-session.target"));
    }

    #[test]
    fn desktop_exec_args_escape_desktop_entry_characters() {
        assert_eq!(
            desktop_exec_arg(r#"IBUS_COMPONENT_PATH=/home/user's work/$tmp%dir"#),
            r#""IBUS_COMPONENT_PATH=/home/user's work/\$tmp%%dir""#
        );
    }

    #[test]
    fn cosmic_panel_is_only_ready_when_wayland_im_is_enabled() {
        let panel = "/usr/libexec/ibus-ui-gtk3";
        assert!(daemon_command_uses_wayland_panel(
            "123 ibus-daemon --panel=/usr/libexec/ibus-ui-gtk3 -i",
            panel
        ));
        assert!(!daemon_command_uses_wayland_panel(
            "123 ibus-daemon --panel=/usr/libexec/ibus-ui-gtk3",
            panel
        ));
        assert!(!daemon_command_uses_wayland_panel(
            "123 ibus-daemon --panel=disable",
            panel
        ));
    }

    #[test]
    fn environment_does_not_force_ibus_modules_on_sensitive_prompts() {
        let environment = environment_conf_contents();
        assert!(environment.contains("IBUS_COMPONENT_PATH="));
        assert!(environment.contains("XMODIFIERS=@im=ibus"));
        assert!(!environment.contains("GTK_IM_MODULE"));
        assert!(!environment.contains("QT_IM_MODULE"));
    }

    #[test]
    fn keyboard_switch_shortcut_is_normalized_for_gsettings() {
        assert_eq!(
            normalize_keyboard_switch_shortcut("Alt+Space").unwrap(),
            ("Alt+Space".into(), "<Alt>space".into())
        );
        assert_eq!(
            normalize_keyboard_switch_shortcut("shift+ctrl+F4").unwrap(),
            ("Ctrl+Shift+F4".into(), "<Control><Shift>F4".into())
        );
        assert_eq!(
            normalize_keyboard_switch_shortcut("Control+Enter").unwrap(),
            ("Ctrl+Enter".into(), "<Control>Return".into())
        );
    }

    #[test]
    fn invalid_keyboard_switch_shortcuts_are_rejected() {
        for shortcut in ["Space", "Alt+Alt+Space", "Hyper+Space", "Ctrl+F36"] {
            assert!(
                normalize_keyboard_switch_shortcut(shortcut).is_err(),
                "accepted invalid shortcut {shortcut}"
            );
        }
    }
}
