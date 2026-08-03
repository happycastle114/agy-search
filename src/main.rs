//! Command-line entry point for `agy-search`.

use std::{
    io::{self, Write},
    process::ExitCode,
};

use agy_search::cli::Cli;
use clap::Parser;

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    match agy_search::run(Cli::parse()).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let _ignored = writeln!(io::stderr().lock(), "error: {error}");
            error.exit_code()
        }
    }
}
