//! Operation-specific JSON Schema rendering.

use schemars::schema_for;

use crate::{
    error::AgyError,
    response_models::{
        CrawlResponse, ExtractResponse, MapResponse, ResearchResponse, SearchResponse,
    },
    source_restriction::SourceRestriction,
    temporal_contract::TemporalContract,
    types::{Operation, VerificationMode},
    verification::require_temporal_schema_for_operation,
};

pub(super) fn render(
    operation: Operation,
    verification: VerificationMode,
    temporal_contract: Option<&TemporalContract>,
    source_restriction: &SourceRestriction,
    search_result_limit: Option<u16>,
) -> Result<String, AgyError> {
    let mut schema = operation_schema(operation)?;
    super::source_schema::narrow_source_urls(&mut schema, source_restriction)?;
    if operation == Operation::Search
        && verification == VerificationMode::Standard
        && matches!(source_restriction, SourceRestriction::Unrestricted)
    {
        super::source_schema::require_grounding_transport_urls(&mut schema)?;
        super::source_schema::require_diverse_search_results(
            &mut schema,
            search_result_limit.ok_or(AgyError::InvalidCommand)?,
        )?;
    }
    require_verification_schema(&mut schema, operation, verification, temporal_contract)?;
    serde_json::to_string(&schema).map_err(|_| AgyError::InvalidCommand)
}

fn operation_schema(operation: Operation) -> Result<serde_json::Value, AgyError> {
    match operation {
        Operation::Search => serde_json::to_value(schema_for!(SearchResponse)),
        Operation::Extract => serde_json::to_value(schema_for!(ExtractResponse)),
        Operation::Map => serde_json::to_value(schema_for!(MapResponse)),
        Operation::Crawl => serde_json::to_value(schema_for!(CrawlResponse)),
        Operation::Research => serde_json::to_value(schema_for!(ResearchResponse)),
    }
    .map_err(|_| AgyError::InvalidCommand)
}

fn require_verification_schema(
    schema: &mut serde_json::Value,
    operation: Operation,
    verification: VerificationMode,
    temporal_contract: Option<&TemporalContract>,
) -> Result<(), AgyError> {
    match (verification, temporal_contract) {
        (VerificationMode::Standard, None) => Ok(()),
        (VerificationMode::TemporalComparison, Some(contract)) => {
            require_temporal_schema_for_operation(schema, operation, contract)
        }
        (VerificationMode::Standard, Some(_)) | (VerificationMode::TemporalComparison, None) => {
            Err(AgyError::InvalidCommand)
        }
    }
}
