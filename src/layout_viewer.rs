//! Native GUI layout popup viewer using GTK3 and WebKit2GTK.

use std::env;
use std::fs;
use std::path::PathBuf;

#[cfg(target_os = "linux")]
use gtk::prelude::*;
#[cfg(target_os = "linux")]
use webkit2gtk::WebViewExt;

const EMBEDDED_LAYOUT_HTML: &str = include_str!("../data/bornona-keyboard.html");

/// Open a native GTK window displaying the rendered Bornona HTML layout.
pub fn show_layout() -> Result<(), String> {
    let html = match locate_page() {
        Ok(path) => fs::read_to_string(&path)
            .unwrap_or_else(|_| EMBEDDED_LAYOUT_HTML.to_string()),
        Err(_) => EMBEDDED_LAYOUT_HTML.to_string(),
    };
    open_window("Bornona Keyboard Layout", &html)
}

fn open_window(title: &str, html: &str) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        gtk::init().map_err(|e| format!("cannot initialize GUI window: {e}"))?;
        gtk::Window::set_default_icon_name("ibus-bornona");

        let window = gtk::Window::new(gtk::WindowType::Toplevel);
        window.set_title(title);
        window.set_default_size(1020, 500);
        window.set_position(gtk::WindowPosition::Center);
        window.set_resizable(true);
        window.set_icon_name(Some("ibus-bornona"));

        if let Ok(icon_path) = locate_icon() {
            let _ = window.set_icon_from_file(&icon_path);
        }

        window.connect_delete_event(|_, _| {
            gtk::main_quit();
            gtk::glib::Propagation::Proceed
        });

        let webview = webkit2gtk::WebView::new();
        webview.load_html(html, Some("file:///"));
        window.add(&webview);
        window.show_all();
        gtk::main();
        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = (title, html);
        Err("layout popup is only supported on Linux".to_string())
    }
}

fn locate_icon() -> Result<PathBuf, String> {
    let mut candidates = Vec::new();

    // 1. Current working directory
    if let Ok(cwd) = env::current_dir() {
        candidates.push(cwd.join("data/logo/ibus-bornona.svg"));
    }

    // 2. Relative to executable path
    if let Ok(exe) = env::current_exe() {
        if let Some(parent) = exe.parent() {
            candidates.push(parent.join("../share/icons/hicolor/scalable/apps/ibus-bornona.svg"));
            candidates.push(parent.join("../share/ibus-bornona/ibus-bornona.svg"));
            candidates.push(parent.join("data/logo/ibus-bornona.svg"));
        }
    }

    // 3. User local icon dirs
    if let Ok(home) = env::var("HOME") {
        candidates.push(PathBuf::from(home.clone()).join(".local/share/icons/hicolor/scalable/apps/ibus-bornona.svg"));
        candidates.push(PathBuf::from(home).join(".local/share/ibus-bornona/ibus-bornona.svg"));
    }

    // 4. System icon dirs
    candidates.push(PathBuf::from("/usr/share/icons/hicolor/scalable/apps/ibus-bornona.svg"));
    candidates.push(PathBuf::from("/usr/local/share/icons/hicolor/scalable/apps/ibus-bornona.svg"));
    candidates.push(PathBuf::from("/usr/share/ibus-bornona/ibus-bornona.svg"));
    candidates.push(PathBuf::from("/usr/local/share/ibus-bornona/ibus-bornona.svg"));

    candidates
        .into_iter()
        .find(|p| p.is_file())
        .ok_or_else(|| "icon file not found on disk".to_string())
}

fn locate_page() -> Result<PathBuf, String> {
    let mut candidates = Vec::new();

    // 1. Current working directory
    if let Ok(cwd) = env::current_dir() {
        candidates.push(cwd.join("data/bornona-keyboard.html"));
        candidates.push(cwd.join("bornona-keyboard.html"));
    }

    // 2. Relative to executable path
    if let Ok(exe) = env::current_exe() {
        if let Some(parent) = exe.parent() {
            candidates.push(parent.join("../share/ibus-bornona/bornona-keyboard.html"));
            candidates.push(parent.join("data/bornona-keyboard.html"));
        }
    }

    // 3. User local data dir
    if let Ok(home) = env::var("HOME") {
        candidates.push(PathBuf::from(home).join(".local/share/ibus-bornona/bornona-keyboard.html"));
    }

    // 4. System data dirs
    candidates.push(PathBuf::from("/usr/local/share/ibus-bornona/bornona-keyboard.html"));
    candidates.push(PathBuf::from("/usr/share/ibus-bornona/bornona-keyboard.html"));

    candidates
        .into_iter()
        .find(|p| p.is_file())
        .ok_or_else(|| "layout file not found on disk, using embedded fallback".to_string())
}
