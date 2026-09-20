//! Long-lived HTTP and MCP entry point.

use agy_search::server::{self, ServerCli};
use clap::Parser;
use std::{
    io::{self, Write},
    process::ExitCode,
    time::Duration,
};
use tracing_subscriber::EnvFilter;

const RUNTIME_SHUTDOWN_GRACE: Duration = Duration::from_secs(1);

fn main() -> ExitCode {
    let cli = ServerCli::parse();
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    if tracing_subscriber::fmt()
        .json()
        .with_env_filter(filter)
        .with_writer(io::stderr)
        .try_init()
        .is_err()
    {
        let _ = writeln!(io::stderr(), "failed to initialize logging");
        return ExitCode::FAILURE;
    }
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            tracing::error!(error = %error, "failed to initialize runtime");
            return ExitCode::FAILURE;
        }
    };
    let result = runtime.block_on(server::run(cli));
    // Tokio's stdin reader cannot be cancelled while a pipe remains open.
    runtime.shutdown_timeout(RUNTIME_SHUTDOWN_GRACE);
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            tracing::error!(error = %error, "server stopped");
            ExitCode::FAILURE
        }
    }
}
