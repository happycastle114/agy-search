//! Lightweight, source-backed web research through Google Antigravity.

pub mod cli;

pub(crate) mod antigravity_version;
pub(crate) mod backend;
pub(crate) mod calendar_date;
pub(crate) mod error;
pub(crate) mod events;
pub(crate) mod invocation;
pub(crate) mod output;
pub(crate) mod process;
pub(crate) mod prompt;
pub(crate) mod redirect;
pub(crate) mod request;
pub(crate) mod response;
pub(crate) mod response_models;
pub(crate) mod response_urls;
pub(crate) mod source_contract;
pub(crate) mod source_date;
pub(crate) mod source_document;
pub(crate) mod source_document_heading;
pub(crate) mod source_document_html;
pub(crate) mod source_fact;
pub(crate) mod source_fetch;
pub(crate) mod source_network;
pub(crate) mod source_restriction;
pub(crate) mod source_url;
pub(crate) mod source_verification;
pub(crate) mod temporal_contract;
pub(crate) mod types;
pub(crate) mod verification;

use cli::Cli;
pub use error::AgyError;

/// Execute one parsed CLI invocation and emit its validated result.
pub async fn run(cli: Cli) -> Result<(), AgyError> {
    let invocation = cli.into_invocation()?;
    let destination = invocation.output.clone();
    let response = backend::execute(invocation).await?;
    output::emit(&response, destination.as_deref())
}
