//! Just main(). Keep as small as possible.

// The `main.rs` file is special in Rust.
// So attributes here have no affect on the main codebase. If the file remains minimal we can just
// blanket allow lint groups.
#![allow(clippy::cargo)]
#![allow(clippy::restriction)]

use flexi_logger::{Logger, Duplicate, FileSpec};
use docx_git_extension::utils::cli::run;

fn main() {
    Logger::try_with_str("info") // set log level
        .unwrap()
        .log_to_file(
            FileSpec::default()
                .directory("logs")            // logs folder
                .basename("docx_extension")   // fixed filename base
                .suffix("log"),               // ensure extension is .log
        )
        // .duplicate_to_stderr(Duplicate::Info) // also print to stderr
        .start()
        .unwrap();
    // Logger::try_with_str("info") // set log level
    //     .unwrap()
    //     .log_to_file(FileSpec::default().directory("logs").basename("app"))
    //     .duplicate_to_stderr(Duplicate::Info) // also print to stderr
    //     .rotate(
    //         flexi_logger::Criterion::Size(10_000_000), // rotate after 10 MB
    //         flexi_logger::Naming::Timestamps,
    //         flexi_logger::Cleanup::KeepLogFiles(7),   // keep last 7 logs
    //     )
    //     .start()
    //     .unwrap();
    run()
}