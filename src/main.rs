pub mod config;
pub mod engine;
pub mod ibus;
pub mod layout_viewer;
pub mod setup;

use std::env;
use std::io::{self, BufRead, Write};

use engine::{
    BornonaLayout, SuggestionEngine, SuggestionOrientation, TypingConfig, TypingProcessor,
};
use ibus::run_ibus_engine;

const HELP: &str = "\
ibus-bornona: Bornona Bengali Fixed Layout Input Method for Linux
USAGE:
    ibus-bornona <command>

COMMANDS:
    setup            Configure Bornona for your desktop
    remove-source    Remove Bornona from desktop and IBus settings
    doctor           Check that everything is installed and running
    config           Show or change typing feature settings
    reset            Restore all typing features to their defaults
    learn clear      Clear learned suggestion preferences
    layout           Show native graphical layout popup
    compose <keys>   Show what a key sequence types, e.g. `ibus-bornona compose hakhi`
    version          Print the version

CONFIG:
    ibus-bornona config [<feature> <value>]

FEATURES:
    auto-vowel              Automatic Vowel Forming
                            Possible values: on, off (default: on)

    auto-chandra            Automatic Chandrabindu Position Fix
                            Possible values: on, off (default: on)

    old-reph                Old Style Reph
                            Possible values: on, off (default: on)

    traditional-kar         Traditional Kar Joining
                            Possible values: on, off (default: off)

    suggestions             Word Suggestions
                            Possible values: on, off (default: off)

    orientation              Suggestion Orientation (alias: suggestion-orientation)
                            Possible values: horizontal, vertical, system (default: horizontal)

    keyboard-switch-shortcut Keyboard source switching shortcut
                            Possible values: modifier+key (default: Alt+Space)

    learning                 Learn selected words
                            Possible values: on, off (default: on)
";

const CONFIG_HELP: &str = "\
ibus-bornona config: Show or change typing feature settings

USAGE:
    ibus-bornona config
    ibus-bornona config <feature> <value>

FEATURES:
    auto-vowel              Automatic Vowel Forming
                            Possible values: on, off (default: on)

    auto-chandra            Automatic Chandrabindu Position Fix
                            Possible values: on, off (default: on)

    old-reph                Old Style Reph
                            Possible values: on, off (default: on)

    traditional-kar         Traditional Kar Joining
                            Possible values: on, off (default: off)

    suggestions             Word Suggestions
                            Possible values: on, off (default: off)

    orientation              Suggestion Orientation (alias: suggestion-orientation)
                            Possible values: horizontal, vertical, system (default: horizontal)

    keyboard-switch-shortcut Keyboard source switching shortcut
                            Possible values: modifier+key (default: Alt+Space)

    learning                 Learn selected words
                            Possible values: on, off (default: on)
";

fn run_interactive(config: TypingConfig) {
    println!("=== Bornona Bengali Interactive Mode ===");
    println!("Type keys on your keyboard and press Enter to see Bengali output.");
    println!("Type 'exit' or 'quit' to exit.\n");

    let layout = BornonaLayout::new();
    let stdin = io::stdin();

    loop {
        print!("bornona> ");
        io::stdout().flush().unwrap();

        let mut line = String::new();
        if stdin.lock().read_line(&mut line).is_err() || line.is_empty() {
            break;
        }

        let trimmed = line.trim_end();
        if trimmed == "exit" || trimmed == "quit" {
            break;
        }

        let mut processor = TypingProcessor::new(config.clone());
        for ch in trimmed.chars() {
            if ch == ' ' {
                processor.process_value(" ");
            } else if let Some(mapped) = layout.map_char(ch) {
                processor.process_value(mapped);
            } else {
                processor.process_value(&ch.to_string());
            }
        }

        println!("Output:   {}\n", processor.commit());
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    // Check if launched by IBus daemon (--ibus flag)
    if args.iter().any(|a| a == "--ibus") {
        if let Err(e) = config::load() {
            eprintln!("ibus-bornona: {e}");
            std::process::exit(1);
        }
        if let Err(e) = run_ibus_engine(true) {
            eprintln!("Error running IBus engine: {}", e);
            std::process::exit(1);
        }
        return;
    }

    if args.iter().any(|a| a == "--standalone") {
        if let Err(e) = config::load() {
            eprintln!("ibus-bornona: {e}");
            std::process::exit(1);
        }
        if let Err(e) = run_ibus_engine(false) {
            eprintln!("Error in standalone mode: {}", e);
            std::process::exit(1);
        }
        return;
    }

    if args.iter().any(|a| a == "--interactive" || a == "-i") {
        match config::load() {
            Ok(config) => run_interactive(config),
            Err(e) => {
                eprintln!("ibus-bornona: {e}");
                std::process::exit(1);
            }
        }
        return;
    }

    let code = match args.first().map(String::as_str) {
        Some("setup") => {
            match setup::setup() {
                Ok(msg) => {
                    if !msg.is_empty() {
                        println!("{msg}");
                    }
                    0
                }
                Err(e) => {
                    eprintln!("ibus-bornona: {e}");
                    1
                }
            }
        }
        Some("doctor") => {
            println!("{}", setup::doctor());
            0
        }
        Some("remove-source") if args.len() == 1 => match setup::remove_input_method_config() {
            Ok(true) => {
                println!("Removed Bornona from desktop and IBus input-method settings.");
                0
            }
            Ok(false) => {
                println!("Bornona was not configured in desktop or IBus input-method settings.");
                0
            }
            Err(error) => {
                eprintln!("ibus-bornona: could not remove Bornona input-method settings: {error}");
                1
            }
        },
        Some("remove-source") => {
            eprintln!("Usage: ibus-bornona remove-source");
            1
        }
        Some("config") => match update_config(&args[1..]) {
            Ok(()) => 0,
            Err(error) => {
                eprintln!("ibus-bornona: {error}");
                1
            }
        },
        Some("reset") if args.len() == 1 => match config::save(&TypingConfig::default()) {
            Ok(()) => {
                println!("Bornona typing settings restored to defaults.");
                0
            }
            Err(error) => {
                eprintln!("ibus-bornona: {error}");
                1
            }
        },
        Some("reset") => {
            eprintln!("Usage: ibus-bornona reset");
            1
        }
        Some("learn") if args.get(1).map(String::as_str) == Some("clear") && args.len() == 2 => {
            match SuggestionEngine::clear_learned_words() {
                Ok(()) => {
                    println!("Learned suggestion preferences cleared.");
                    0
                }
                Err(error) => {
                    eprintln!("ibus-bornona: {error}");
                    1
                }
            }
        }
        Some("learn") => {
            eprintln!("Usage: ibus-bornona learn clear");
            1
        }
        Some("layout") => {
            match layout_viewer::show_layout() {
                Ok(()) => 0,
                Err(e) => {
                    eprintln!("ibus-bornona layout: {e}");
                    1
                }
            }
        }
        Some("compose") if args.len() > 1 => {
            let keys = args[1..].join(" ");
            match config::load() {
                Ok(config) => {
                    println!("{}", engine::compose_with_config(&keys, config));
                    0
                }
                Err(error) => {
                    eprintln!("ibus-bornona: {error}");
                    1
                }
            }
        }
        Some("compose") => {
            eprintln!("Usage: ibus-bornona compose <keys>");
            1
        }
        Some("version") | Some("--version") | Some("-v") | Some("-V") => {
            println!("ibus-bornona {}", env!("CARGO_PKG_VERSION"));
            0
        }
        Some("--help") | Some("-h") => {
            print!("{HELP}");
            0
        }
        Some("help") => {
            if args.get(1).map(String::as_str) == Some("config") {
                print!("{CONFIG_HELP}");
            } else {
                print!("{HELP}");
            }
            0
        }
        None => {
            if atty_check() {
                print!("{HELP}");
                0
            } else {
                // Started by IBus daemon without args
                if let Err(e) = config::load() {
                    eprintln!("ibus-bornona: {e}");
                    std::process::exit(1);
                }
                if let Err(e) = run_ibus_engine(true) {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
                0
            }
        }
        Some(unknown) => {
            eprintln!("Unknown command: {}", unknown);
            print!("{HELP}");
            1
        }
    };

    std::process::exit(code);
}

fn update_config(args: &[String]) -> Result<(), String> {
    if args.iter().any(|a| a == "--help" || a == "-h") || args == ["help"] {
        print!("{CONFIG_HELP}");
        return Ok(());
    }

    match args {
        [] => {
            let config = config::load()?;
            println!(
                "{}: {}",
                config::feature_name("auto-vowel").unwrap(),
                if config.auto_vowel { "on" } else { "off" }
            );
            println!(
                "{}: {}",
                config::feature_name("auto-chandra").unwrap(),
                if config.auto_chandra { "on" } else { "off" }
            );
            println!(
                "{}: {}",
                config::feature_name("old-reph").unwrap(),
                if config.old_reph { "on" } else { "off" }
            );
            println!(
                "{}: {}",
                config::feature_name("traditional-kar").unwrap(),
                if config.traditional_kar { "on" } else { "off" }
            );
            println!(
                "{}: {}",
                config::feature_name("word-suggestion").unwrap(),
                if config.word_suggestion { "on" } else { "off" }
            );
            println!(
                "{}: {}",
                config::feature_name("suggestion-orientation").unwrap(),
                config.suggestion_orientation.as_str()
            );
            println!(
                "{}: {}",
                config::feature_name("keyboard-switch-shortcut").unwrap(),
                config.keyboard_switch_shortcut
            );
            println!("Suggestion Learning: {}", if config.learning { "on" } else { "off" });
            Ok(())
        }
        [feature, val] => {
            let mut config = config::load()?;
            let feat_lower = feature.to_ascii_lowercase();
            let val_lower = val.to_ascii_lowercase();

            if matches!(
                feat_lower.as_str(),
                "suggestion-orientation" | "orientation" | "direction"
            ) {
                if let Some(orient) = SuggestionOrientation::from_str_loose(&val_lower) {
                    config.suggestion_orientation = orient;
                    config::save(&config)?;
                    println!(
                        "{}: {}",
                        config::feature_name("suggestion-orientation").unwrap(),
                        orient.as_str()
                    );
                    return Ok(());
                } else {
                    return Err("setting must be `horizontal` or `vertical`".into());
                }
            }

            if feat_lower == "suggestion" {
                if let Some(orientation) = SuggestionOrientation::from_str_loose(&val_lower) {
                    config.suggestion_orientation = orientation;
                    config::save(&config)?;
                    println!(
                        "{}: {}",
                        config::feature_name("suggestion-orientation").unwrap(),
                        orientation.as_str()
                    );
                    return Ok(());
                }
            }

            if feat_lower == "keyboard-switch-shortcut" {
                let (display, _) = setup::normalize_keyboard_switch_shortcut(val)?;
                config.keyboard_switch_shortcut = display.clone();
                config::save(&config)?;
                setup::apply_keyboard_switch_shortcut(&config.keyboard_switch_shortcut)?;
                println!(
                    "{}: {}",
                    config::feature_name("keyboard-switch-shortcut").unwrap(),
                    display
                );
                return Ok(());
            }

            let enabled = match val_lower.as_str() {
                "on" => true,
                "off" => false,
                _ => return Err("setting must be `on` or `off` (or `horizontal` / `vertical` for orientation)".into()),
            };
            if !config::set_feature(&mut config, &feat_lower, enabled) {
                return Err(format!("unknown feature `{feature}`"));
            }
            config::save(&config)?;
            println!(
                "{}: {}",
                config::feature_name(&feat_lower).unwrap(),
                if enabled { "on" } else { "off" }
            );
            Ok(())
        }
        _ => Err("usage: ibus-bornona config [<feature> <value>]".into()),
    }
}

fn atty_check() -> bool {
    // Check if stdin is a terminal without adding external crate
    unsafe {
        libc_isatty(0) != 0
    }
}

unsafe extern "C" {
    #[link_name = "isatty"]
    fn libc_isatty(fd: std::ffi::c_int) -> std::ffi::c_int;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_help_shows_features_and_possible_values() {
        for feature in [
            "auto-vowel",
            "auto-chandra",
            "old-reph",
            "traditional-kar",
            "suggestions",
            "suggestion-orientation",
            "keyboard-switch-shortcut",
            "learning",
        ] {
            assert!(CONFIG_HELP.contains(feature), "missing feature: {feature}");
        }
        assert!(CONFIG_HELP.contains("Possible values: on, off"));
        assert!(CONFIG_HELP.contains("Possible values: horizontal, vertical, system"));
        assert!(CONFIG_HELP.contains("Automatic Vowel Forming"));
        assert!(CONFIG_HELP.contains("Automatic Chandrabindu Position Fix"));
        assert!(CONFIG_HELP.contains("Old Style Reph"));
        assert!(CONFIG_HELP.contains("Traditional Kar Joining"));
        assert!(CONFIG_HELP.contains("Word Suggestions"));
        assert!(CONFIG_HELP.contains("Suggestion Orientation"));
        assert!(CONFIG_HELP.contains("keyboard-switch-shortcut"));
        assert!(!CONFIG_HELP.contains("lookup-mode"));
        assert!(!CONFIG_HELP.contains("page-size"));
    }

    #[test]
    fn test_help_shows_features_and_possible_values() {
        for feature in [
            "auto-vowel",
            "auto-chandra",
            "old-reph",
            "traditional-kar",
            "suggestions",
            "suggestion-orientation",
            "keyboard-switch-shortcut",
            "learning",
        ] {
            assert!(HELP.contains(feature), "missing feature: {feature}");
        }
        assert!(HELP.contains("Possible values: on, off"));
        assert!(HELP.contains("Possible values: horizontal, vertical, system"));
        assert!(!HELP.contains("toggle"));
        assert!(!HELP.contains("suggest <keys>"));
    }

    #[test]
    fn test_update_config_help_flags() {
        assert!(update_config(&["--help".to_string()]).is_ok());
        assert!(update_config(&["-h".to_string()]).is_ok());
        assert!(update_config(&["help".to_string()]).is_ok());
        assert!(update_config(&["auto-vowel".to_string(), "--help".to_string()]).is_ok());
    }

    #[test]
    fn test_update_config_invalid_arguments() {
        assert!(update_config(&[
            "invalid".to_string(),
            "arguments".to_string(),
            "here".to_string()
        ])
        .is_err());
    }

    #[test]
    fn test_update_config_suggestion_and_orientation() {
        assert!(update_config(&["suggestion".to_string(), "off".to_string()]).is_ok());
        assert!(update_config(&["suggestion".to_string(), "on".to_string()]).is_ok());
        assert!(update_config(&["suggestion-orientation".to_string(), "vertical".to_string()]).is_ok());
        assert!(update_config(&["orientation".to_string(), "horizontal".to_string()]).is_ok());
        assert!(update_config(&["suggestion".to_string(), "vertical".to_string()]).is_ok());
        assert!(update_config(&["suggestion".to_string(), "horizontal".to_string()]).is_ok());
    }
}
