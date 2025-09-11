//! Running the CLI
#![expect(
    clippy::exit,
    reason = "Allow exits because in this file we ideally handle all errors with known exit codes"
)]

use std::process;
use clap::Parser;
use log::{info, error};
use crate::filters::{clean_filter, smudge_filter};
use crate::post_commit::post_commit::post_commit;

/// Stelae is currently just a simple git server.
/// run from the library directory or pass
/// path to archive.
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Docx git extension cli subcommands
    #[command(subcommand)]
    subcommands: Subcommands,
}

/// Subcommands for the Stelae CLI
#[derive(Clone, clap::Subcommand)]
enum Subcommands {
    /// Trigger clean filter
    Clean {
        /// Port on which to serve the archive.
        docx_name: String,
    },
    /// Trigger smudge filter
    Smudge,
    /// Trigger post-commit hook logic
    PostCommit
}


#[expect(
    clippy::pattern_type_mismatch,
    reason = "Matching on a reference (&cli.subcommands) instead of by value; the match patterns borrow fields, which is intentional to avoid moving data."
)]
/// Central place to execute commands
///
/// # Errors
/// This function returns the generic `CliError`, based on which we exit with a known exit code.
fn execute_command(cli: &Cli) -> Result<(), Box<dyn std::error::Error>> {
    match &cli.subcommands {
        Subcommands::Clean { docx_name } => {
            // Use the filename here
            if let Err(e) = clean_filter(&docx_name) {
                error!("clean_filter failed: {}", e);
            }
            Ok(())
        }
        Subcommands::Smudge => {
            info!("Running smudge filter");
            if let Err(e) = smudge_filter() {
                error!("clean_filter failed: {}", e);
            }
            Ok(())
        }
        Subcommands::PostCommit => {
            info!("Running post-commit");
            if let Err(e) = post_commit() {
                error!("clean_filter failed: {}", e);
            }
            Ok(())
        }
    }
}

/// Main entrypoint to application
///
/// Exits with 1 if we encounter an error
pub fn run() {
    // Parse the command-line arguments
    let cli = Cli::parse();

    // Execute the chosen subcommand
    let result = execute_command(&cli);

    match result {
        Ok(()) => {
            // Success → exit code 0
            process::exit(0);
        }
        Err(err) => {
            // Print error and exit with code 1
            error!("Application error: {}", err);
            process::exit(1);
        }
    }
}