use std::env;
use std::process::exit;
use docx_git_extension::filters::{clean_filter, smudge_filter};

fn main() {
    // // log_message("Program started");

    let mut args = env::args();
    let _program = args.next();

    match args.next().as_deref() {
        Some("clean") => {
            let docx_path = match args.next() {
                Some(p) => p,
                None => {
                    // log_message("No file path provided for clean");
                    eprintln!("No file path provided for clean filter");
                    exit(1);
                }
            };
            if let Err(err) = clean_filter(&docx_path) {
                // log_message(&format!("Clean error: {}", err));
                eprintln!("Clean error: {err}");
                exit(1);
            }
        }
        Some("smudge") => {
            if let Err(err) = smudge_filter() {
                // log_message(&format!("Smudge error: {}", err));
                eprintln!("Smudge error: {err}");
                exit(1);
            }
        }
        Some(cmd) => {
            // log_message(&format!("Unknown command: {}", cmd));
            eprintln!("Unknown command: {cmd}");
            exit(2);
        }
        None => {
            // log_message("No command provided");
            eprintln!("Usage: docx_extension <clean|smudge>");
            exit(2);
        }
    }

    // log_message("Program finished");
}