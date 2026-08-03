//! Lightweight, source-backed web research through Google Antigravity.

pub mod cli;

pub(crate) mod backend;
pub(crate) mod error;
pub(crate) mod events;
pub(crate) mod invocation;
pub(crate) mod output;
pub(crate) mod process;
pub(crate) mod request;
pub(crate) mod response;
pub(crate) mod response_models;
pub(crate) mod types;

use cli::Cli;
pub use error::AgyError;

/// Execute one parsed CLI invocation and emit its validated result.
pub async fn run(cli: Cli) -> Result<(), AgyError> {
    let invocation = cli.into_invocation()?;
    let destination = invocation.output.clone();
    let response = backend::execute(invocation).await?;
    output::emit(&response, destination.as_deref())
}
