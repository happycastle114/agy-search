//! Temporal JSON Schema constraints derived from the caller-owned contract.

use serde_json::Value;

use super::ScopeLabel;
use crate::{
    error::AgyError,
    temporal_contract::TemporalContract,
    types::{HttpUrl, Operation},
};

const CANDIDATE_DEFINITION: &str = "/$defs/ScopeEvidence";
const CANDIDATE_MINIMUM: &str = "/$defs/EvidenceAudit/properties/candidates/minItems";
const PUBLIC_SOURCE_DEFINITION: &str = "/$defs/WebSource";
const PUBLIC_SOURCE_LAST_UPDATED: &str = "/$defs/WebSource/properties/last_updated";
const TEMPORAL_CANDIDATE_FIELDS: [&str; 4] =
    ["date", "value", "source_date_text", "evidence_excerpt"];
const TEMPORAL_PUBLIC_FIELDS: [&str; 1] = ["date"];

pub(crate) fn require_temporal_fields(document: &mut Value) -> Result<(), AgyError> {
    require_non_null_strings(document, CANDIDATE_DEFINITION, &TEMPORAL_CANDIDATE_FIELDS)?;
    require_non_null_strings(document, PUBLIC_SOURCE_DEFINITION, &TEMPORAL_PUBLIC_FIELDS)?;
    require_null(document, PUBLIC_SOURCE_LAST_UPDATED)?;
    set_bound(document, CANDIDATE_MINIMUM, 1)
}

pub(crate) fn require_temporal_schema_for_operation(
    document: &mut Value,
    operation: Operation,
    contract: &TemporalContract,
) -> Result<(), AgyError> {
    require_temporal_fields(document)?;
    let expected_count =
        u64::try_from(contract.expected_scopes().len()).map_err(|_| AgyError::InvalidCommand)?;
    set_bound(document, CANDIDATE_MINIMUM, expected_count)?;
    set_bound(
        document,
        "/$defs/EvidenceAudit/properties/candidates/maxItems",
        expected_count,
    )?;
    constrain_string_enum(
        document,
        "/$defs/ScopeEvidence/properties/scope",
        contract.expected_scopes().iter().map(ScopeLabel::as_str),
    )?;
    constrain_string_enum(
        document,
        "/$defs/ScopeEvidence/properties/url",
        contract.source_urls().iter().map(HttpUrl::as_str),
    )?;
    constrain_string_enum(
        document,
        "/$defs/WebSource/properties/url",
        contract.source_urls().iter().map(HttpUrl::as_str),
    )?;
    match operation {
        Operation::Search => set_bound(document, "/properties/results/maxItems", 1),
        Operation::Research => Ok(()),
        Operation::Extract | Operation::Map | Operation::Crawl => Err(AgyError::InvalidCommand),
    }
}

fn set_bound(document: &mut Value, pointer: &str, value: u64) -> Result<(), AgyError> {
    let bound = document
        .pointer_mut(pointer)
        .ok_or(AgyError::InvalidCommand)?;
    *bound = Value::from(value);
    Ok(())
}

fn require_non_null_strings(
    document: &mut Value,
    pointer: &str,
    fields: &[&str],
) -> Result<(), AgyError> {
    let definition = document
        .pointer_mut(pointer)
        .and_then(Value::as_object_mut)
        .ok_or(AgyError::InvalidCommand)?;
    let required = definition
        .get_mut("required")
        .and_then(Value::as_array_mut)
        .ok_or(AgyError::InvalidCommand)?;
    for field in fields {
        if !required.iter().any(|value| value.as_str() == Some(field)) {
            required.push(Value::from(*field));
        }
    }
    let properties = definition
        .get_mut("properties")
        .and_then(Value::as_object_mut)
        .ok_or(AgyError::InvalidCommand)?;
    for field in fields {
        let property = properties
            .get_mut(*field)
            .and_then(Value::as_object_mut)
            .ok_or(AgyError::InvalidCommand)?;
        property.insert("type".to_owned(), Value::from("string"));
        property.remove("default");
    }
    Ok(())
}

fn require_null(document: &mut Value, pointer: &str) -> Result<(), AgyError> {
    let property = document
        .pointer_mut(pointer)
        .ok_or(AgyError::InvalidCommand)?;
    *property = serde_json::json!({"type": "null"});
    Ok(())
}

fn constrain_string_enum<'a>(
    document: &mut Value,
    pointer: &str,
    values: impl Iterator<Item = &'a str>,
) -> Result<(), AgyError> {
    let property = document
        .pointer_mut(pointer)
        .ok_or(AgyError::InvalidCommand)?;
    *property = serde_json::json!({
        "type": "string",
        "enum": values.collect::<Vec<_>>(),
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use serde_json::{Value, json};

    use crate::{
        response::Document as ResponseDocument,
        temporal_contract::{ScopeLabel, TemporalContract},
        types::{HttpUrl, Operation, VerificationMode},
    };

    fn contract(scopes: &[&str], sources: &[&str]) -> TemporalContract {
        let scopes = scopes
            .iter()
            .map(|scope| ScopeLabel::from_str(scope).expect("valid caller scope"))
            .collect();
        let sources = sources
            .iter()
            .map(|source| HttpUrl::parse(source).expect("valid caller source"))
            .collect();
        TemporalContract::parse(VerificationMode::TemporalComparison, scopes, sources, None)
            .expect("valid temporal contract")
            .expect("temporal mode creates a contract")
    }

    #[test]
    fn temporal_schema_constrains_caller_count_labels_and_search_winner() {
        let contract = contract(
            &["alpha", "beta", "gamma"],
            &["https://example.com/releases"],
        );
        let schema = ResponseDocument::schema(
            Operation::Search,
            VerificationMode::TemporalComparison,
            Some(&contract),
            &crate::source_restriction::SourceRestriction::Unrestricted,
            Some(5),
        )
        .expect("temporal schema must render");
        let document: Value = serde_json::from_str(&schema).expect("schema must be JSON");
        let bounds = [
            "/$defs/EvidenceAudit/properties/candidates/minItems",
            "/$defs/EvidenceAudit/properties/candidates/maxItems",
            "/properties/results/maxItems",
        ]
        .map(|pointer| document.pointer(pointer).and_then(Value::as_u64));
        let labels = document
            .pointer("/$defs/ScopeEvidence/properties/scope/enum")
            .and_then(Value::as_array)
            .expect("scope enum must exist");

        assert_eq!(bounds, [Some(3), Some(3), Some(1)]);
        assert_eq!(labels, &[json!("alpha"), json!("beta"), json!("gamma")]);
    }

    #[test]
    fn temporal_schema_allows_only_null_public_last_updated_for_search_and_research() {
        // Given: temporal comparison requests for each public WebSource operation.
        let contract = contract(
            &["alpha", "beta"],
            &["https://example.com/alpha", "https://example.com/beta"],
        );

        // When: their generated response schemas are rendered.
        let schemas = [Operation::Search, Operation::Research].map(|operation| {
            ResponseDocument::schema(
                operation,
                VerificationMode::TemporalComparison,
                Some(&contract),
                &crate::source_restriction::SourceRestriction::Unrestricted,
                (operation == Operation::Search).then_some(5),
            )
            .expect("temporal schema must render")
        });

        // Then: a source update can only be the JSON null sentinel.
        for schema in schemas {
            let document: Value = serde_json::from_str(&schema).expect("schema must be JSON");
            assert_eq!(
                document.pointer("/$defs/WebSource/properties/last_updated"),
                Some(&json!({"type": "null"}))
            );
        }
    }

    #[test]
    fn standard_schema_requires_redundant_candidate_minimum() {
        let schema = ResponseDocument::schema(
            Operation::Search,
            VerificationMode::Standard,
            None,
            &crate::source_restriction::SourceRestriction::Unrestricted,
            Some(5),
        )
        .expect("standard schema must render");
        let document: Value = serde_json::from_str(&schema).expect("schema must be JSON");
        assert_eq!(
            document
                .pointer("/$defs/EvidenceAudit/properties/candidates/minItems")
                .and_then(Value::as_u64),
            Some(2),
        );
        assert!(
            document.pointer("/$defs/WebSource/properties/last_updated/type")
                == Some(&json!(["string", "null"])),
            "standard schema must retain string-capable last_updated metadata",
        );
    }
}
