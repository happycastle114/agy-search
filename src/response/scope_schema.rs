//! Exact-one schema for an internal temporal recovery scope.

use schemars::schema_for;
use serde_json::Value;

use crate::{
    error::AgyError, response_models::SearchResponse, verification::require_temporal_fields,
};

pub(crate) fn temporal_scope_schema(
    restriction: &crate::source_restriction::SourceRestriction,
) -> Result<String, AgyError> {
    let mut schema =
        serde_json::to_value(schema_for!(SearchResponse)).map_err(|_| AgyError::InvalidCommand)?;
    require_temporal_fields(&mut schema)?;
    super::source_schema::narrow_source_urls(&mut schema, restriction)?;
    for pointer in [
        "/$defs/EvidenceAudit/properties/candidates/minItems",
        "/$defs/EvidenceAudit/properties/candidates/maxItems",
        "/properties/results/minItems",
        "/properties/results/maxItems",
    ] {
        let bound = schema
            .pointer_mut(pointer)
            .ok_or(AgyError::InvalidCommand)?;
        *bound = Value::from(1);
    }
    serde_json::to_string(&schema).map_err(|_| AgyError::InvalidCommand)
}
