//! Typed response-document deserialization.

use serde_json::Value;

use crate::{error::AgyError, response_models::ResponseDocument, types::Operation};

pub(super) fn parse(operation: Operation, value: Value) -> Result<ResponseDocument, AgyError> {
    match operation {
        Operation::Search => serde_json::from_value(value).map(ResponseDocument::Search),
        Operation::Extract => serde_json::from_value(value).map(ResponseDocument::Extract),
        Operation::Map => serde_json::from_value(value).map(ResponseDocument::Map),
        Operation::Crawl => serde_json::from_value(value).map(ResponseDocument::Crawl),
        Operation::Research => serde_json::from_value(value).map(ResponseDocument::Research),
    }
    .map_err(|_| AgyError::OutputInvalid)
}
