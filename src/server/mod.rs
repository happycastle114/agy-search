//! HTTP and MCP adapters sharing bounded, source-validated execution.

mod config;
mod error;
mod http;
mod input;
mod mcp;
mod runtime;
mod security;

pub use config::ServerCli;
pub use error::ServerError;
use runtime::Runtime;

/// Run the selected transport until shutdown or EOF.
pub async fn run(cli: ServerCli) -> Result<(), ServerError> {
    let runtime = Runtime::new(cli.runtime)?;
    match cli.command {
        config::ServerCommand::Http(options) => http::serve(runtime, options).await,
        config::ServerCommand::Stdio => mcp::serve_stdio(runtime).await,
    }
}
