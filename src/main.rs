pub mod engine;
pub mod ibus;
pub mod layout_viewer;
pub mod setup;

use std::env;
use std::io::{self, BufRead, Write};

use engine::{BornonaLayout, TypingConfig, TypingProcessor};
use ibus::{init_engine_state, run_ibus_engine};

const HELP: &str = "\
ibus-bornona: Bornona Bengali Fixed Layout Input Method for IBus
USAGE:
    ibus-bornona <command>

COMMANDS:
    setup            Add Bornona to your DE input sources (Alt+Space to switch)
    doctor           Check that everything is installed and running
    layout           Show native graphical layout popup
    compose <keys>   Show what a key sequence types, e.g. `ibus-bornona compose hakhi`
    version          Print the version
";

fn run_interactive() {
    println!("=== Bornona Bengali Interactive Mode ===");
    println!("Type keys on your keyboard and press Enter to see Bengali output.");
    println!("Type 'exit' or 'quit' to exit.\n");

    let layout = BornonaLayout::new();
    let config = TypingConfig::default();
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
        init_engine_state(TypingConfig::default());
        if let Err(e) = run_ibus_engine(true) {
            eprintln!("Error running IBus engine: {}", e);
            std::process::exit(1);
        }
        return;
    }

    if args.iter().any(|a| a == "--standalone") {
        init_engine_state(TypingConfig::default());
        if let Err(e) = run_ibus_engine(false) {
            eprintln!("Error in standalone mode: {}", e);
            std::process::exit(1);
        }
        return;
    }

    if args.iter().any(|a| a == "--interactive" || a == "-i") {
        run_interactive();
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
            println!("{}", setup::compose(&keys));
            0
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
                init_engine_state(TypingConfig::default());
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
