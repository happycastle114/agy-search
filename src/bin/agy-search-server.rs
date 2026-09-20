//! Long-lived HTTP and MCP entry point.

use agy_search::server::{self, ServerCli};
use clap::Parser;
use std::{
    io::{self, Write},
    process::ExitCode,
    time::Duration,
};
use tracing_subscriber::{
    EnvFilter, Layer,
    filter::{FilterExt, LevelFilter, Targets},
    layer::SubscriberExt,
    util::SubscriberInitExt,
};

const RUNTIME_SHUTDOWN_GRACE: Duration = Duration::from_secs(1);

fn main() -> ExitCode {
    let cli = ServerCli::parse();
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    // The SDK logs request/response payloads, including caller metadata.
    let payload_safe_targets = Targets::new()
        .with_default(LevelFilter::TRACE)
        .with_target("rmcp", LevelFilter::OFF);
    let diagnostics = tracing_subscriber::fmt::layer()
        .json()
        .with_writer(io::stderr)
        .with_filter(filter.and(payload_safe_targets));
    if tracing_subscriber::registry()
        .with(diagnostics)
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
