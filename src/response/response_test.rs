use serde_json::Value;
use std::str::FromStr as _;

use super::{Document as ResponseDocument, temporal_scope_schema};
use crate::{
    source_restriction::{SourceDomain, SourceRestriction},
    types::{Operation, VerificationMode},
};

#[test]
fn search_schema_requires_internal_evidence_audit() {
    let schema = ResponseDocument::schema(
        Operation::Search,
        VerificationMode::Standard,
        None,
        &SourceRestriction::Unrestricted,
    )
    .expect("standard Search schema must render");
    let document: Value = serde_json::from_str(&schema).expect("schema must be JSON");
    let required = document
        .get("required")
        .and_then(Value::as_array)
        .expect("required fields must exist");

    assert!(required.iter().any(|field| field == "evidence_audit"));
}

#[test]
fn restricted_search_schema_accepts_a_canonical_domain_policy() {
    let restriction = SourceRestriction::parse(
        vec![SourceDomain::from_str("rust-lang.org").expect("valid test domain")],
        Vec::new(),
    )
    .expect("valid test restriction");

    let schema = ResponseDocument::schema(
        Operation::Search,
        VerificationMode::Standard,
        None,
        &restriction,
    );

    assert!(schema.is_ok(), "restricted schema failed: {schema:?}");
}

#[test]
fn temporal_scope_schema_requires_exactly_one_candidate_and_result() {
    let schema = temporal_scope_schema(&SourceRestriction::Unrestricted)
        .expect("temporal scope schema must render");
    let document: Value = serde_json::from_str(&schema).expect("schema must be JSON");
    let bounds = [
        "/$defs/EvidenceAudit/properties/candidates/minItems",
        "/$defs/EvidenceAudit/properties/candidates/maxItems",
        "/properties/results/minItems",
        "/properties/results/maxItems",
    ]
    .map(|pointer| document.pointer(pointer).and_then(Value::as_u64));

    assert_eq!(bounds, [Some(1), Some(1), Some(1), Some(1)]);
}
