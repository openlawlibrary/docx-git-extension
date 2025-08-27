use std::env;
use std::process::exit;
use docx_git_extension::filters::{clean_filter, smudge_filter};

fn main() {
    let mut args = env::args();
    let _program = args.next();

    match args.next().as_deref() {
        Some("clean") => {
            let docx_path = match args.next() {
                Some(p) => p,
                None => {
                    eprintln!("No file path provided for clean filter");
                    exit(1);
                }
            };
            if let Err(err) = clean_filter(&docx_path) {
                eprintln!("Clean error: {err}");
                exit(1);
            }
        }
        Some("smudge") => {
            if let Err(err) = smudge_filter() {
                eprintln!("Smudge error: {err}");
                exit(1);
            }
        }
        Some(cmd) => {
            eprintln!("Unknown command: {cmd}");
            exit(2);
        }
        None => {
            eprintln!("Usage: docx_extension <clean|smudge>");
            exit(2);
        }
    }
}