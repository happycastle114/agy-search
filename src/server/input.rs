//! Shared HTTP and MCP input boundaries.

mod content;
mod search;

pub(crate) use content::{CrawlInput, ExtractInput, MapInput, ResearchInput};
pub(crate) use search::SearchInput;

use super::ServerError;
use crate::{cli::OutputArgs, types::NonEmptyText};

const MAX_QUERY_BYTES: usize = 100 * 1024;

const fn output_args() -> OutputArgs {
    OutputArgs {
        output: None,
        _json: true,
    }
}

fn bounded_text(value: &NonEmptyText) -> Result<(), ServerError> {
    if value.as_str().len() > MAX_QUERY_BYTES {
        return Err(ServerError::InvalidInput);
    }
    Ok(())
}

#[cfg(test)]
#[path = "input_test.rs"]
mod tests;
