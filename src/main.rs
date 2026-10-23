pub mod config;
pub mod engine;
pub mod ibus;
pub mod layout_viewer;
pub mod setup;

use std::env;
use std::io::{self, BufRead, Write};

use engine::{BornonaLayout, TypingConfig, TypingProcessor};
use ibus::run_ibus_engine;

const HELP: &str = "\
ibus-bornona: Bornona Bengali Fixed Layout Input Method for Linux
USAGE:
    ibus-bornona <command>

COMMANDS:
    setup            Configure Bornona for your desktop (Alt+Space to switch)
    remove-source    Remove Bornona from desktop and IBus settings
    doctor           Check that everything is installed and running
    config           Show or change typing feature settings
    reset            Restore all typing features to their defaults
    layout           Show native graphical layout popup
    compose <keys>   Show what a key sequence types, e.g. `ibus-bornona compose hakhi`
    version          Print the version

CONFIG:
    ibus-bornona config <feature> <on|off>
    Features: auto-vowel, auto-chandra, old-reph, traditional-kar
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
    let mut config = config::load()?;
    match args {
        [] => {
            for feature in [
                "auto-vowel",
                "auto-chandra",
                "old-reph",
                "traditional-kar",
            ] {
                let enabled = match feature {
                    "auto-vowel" => config.auto_vowel,
                    "auto-chandra" => config.auto_chandra,
                    "old-reph" => config.old_reph,
                    "traditional-kar" => config.traditional_kar,
                    _ => unreachable!(),
                };
                println!(
                    "{}: {}",
                    config::feature_name(feature).unwrap(),
                    if enabled { "on" } else { "off" }
                );
            }
            Ok(())
        }
        [feature, state] => {
            let enabled = match state.as_str() {
                "on" => true,
                "off" => false,
                _ => return Err("setting must be `on` or `off`".into()),
            };
            if !config::set_feature(&mut config, feature, enabled) {
                return Err(format!("unknown feature `{feature}`"));
            }
            config::save(&config)?;
            println!(
                "{}: {}",
                config::feature_name(feature).unwrap(),
                if enabled { "on" } else { "off" }
            );
            Ok(())
        }
        _ => Err("usage: ibus-bornona config [<feature> <on|off>]".into()),
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
