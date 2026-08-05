use super::temporal_fixtures_test::{
    alpha_beta_contract, search_response, temporal_contract, temporal_contract_with_cutoff,
};
use std::str::FromStr;

use super::*;
use crate::types::{NonEmptyText, VerificationMode};

#[test]
fn temporal_primary_assessment_recovers_complete_candidates_after_cutoff() {
    // Given: an otherwise complete primary response with alpha after the caller cutoff.
    let response = search_response(&["alpha", "beta"], true);
    let contract = temporal_contract_with_cutoff(
        &["alpha", "beta"],
        &["https://example.com/alpha", "https://example.com/beta"],
        Some("2026-08-02"),
    );

    // When: direct primary eligibility is assessed.
    let assessment = assess_search(
        &response,
        VerificationMode::TemporalComparison,
        Some(&contract),
    );

    // Then: the full caller-owned plan remains eligible for source-first or scoped repair.
    assert!(matches!(
        assessment,
        TemporalAssessment::Recoverable(plan)
            if plan.scopes().iter().map(ScopeLabel::as_str).collect::<Vec<_>>()
                == ["alpha", "beta"]
    ));
}

#[test]
fn caller_scope_set_replaces_model_coverage_complete_as_the_completeness_proof() {
    // Given: exact caller scopes with complete evidence despite a false model coverage opinion.
    let response = search_response(&["alpha", "beta"], false);
    let contract = alpha_beta_contract();

    // When: temporal eligibility is assessed.
    let assessment = assess_search(
        &response,
        VerificationMode::TemporalComparison,
        Some(&contract),
    );

    // Then: exact caller-owned set equality, not model self-attestation, proves coverage.
    assert!(matches!(assessment, TemporalAssessment::Verified));
}

#[test]
fn temporal_assessment_rejects_duplicate_scope_labels() {
    // Given: a complete audit with two identical scope labels.
    let response = search_response(&["alpha", "alpha"], true);
    let contract = alpha_beta_contract();

    // When: temporal eligibility is assessed.
    let assessment = assess_search(
        &response,
        VerificationMode::TemporalComparison,
        Some(&contract),
    );

    // Then: recovery is not eligible.
    assert!(matches!(assessment, TemporalAssessment::Invalid));
}

#[test]
fn temporal_assessment_rejects_oversized_scope_labels() {
    // Given: a complete audit containing a scope beyond the fixed byte boundary.
    let oversized = "x".repeat(129);
    let response = search_response(&["alpha", &oversized], true);
    let contract = alpha_beta_contract();

    // When: temporal eligibility is assessed.
    let assessment = assess_search(
        &response,
        VerificationMode::TemporalComparison,
        Some(&contract),
    );

    // Then: recovery is not eligible.
    assert!(matches!(assessment, TemporalAssessment::Invalid));
}

#[test]
fn temporal_assessment_recovers_only_complete_candidate_binding_failure() {
    // Given: complete typed fields whose excerpt does not bind the alpha value.
    let mut response = search_response(&["beta", "alpha"], true);
    let contract = alpha_beta_contract();
    response
        .evidence_audit
        .candidates
        .first_mut()
        .expect("fixture has alpha")
        .evidence_excerpt =
        Some(NonEmptyText::from_str("released 2026-08-03").expect("valid excerpt"));

    // When: temporal eligibility is assessed.
    let assessment = assess_search(
        &response,
        VerificationMode::TemporalComparison,
        Some(&contract),
    );

    // Then: the recovery plan uses caller order rather than the model candidate order.
    assert!(matches!(
        assessment,
        TemporalAssessment::Recoverable(plan)
            if plan.scopes().iter().map(ScopeLabel::as_str).collect::<Vec<_>>()
                == ["alpha", "beta"]
    ));
}

#[test]
fn temporal_assessment_requires_exact_order_insensitive_caller_scope_equality() {
    // Given: the same caller set in reverse model order, plus missing and extra variants.
    let contract = alpha_beta_contract();
    let reverse = search_response(&["beta", "alpha"], true);
    let missing = search_response(&["alpha"], true);
    let extra = search_response(&["alpha", "beta", "gamma"], true);

    // When: each response is checked against the caller contract.
    let reverse = assess_search(
        &reverse,
        VerificationMode::TemporalComparison,
        Some(&contract),
    );
    let missing = assess_search(
        &missing,
        VerificationMode::TemporalComparison,
        Some(&contract),
    );
    let extra = assess_search(
        &extra,
        VerificationMode::TemporalComparison,
        Some(&contract),
    );

    // Then: order does not matter, while missing or extra exact labels fail closed.
    assert!(!matches!(reverse, TemporalAssessment::Invalid));
    assert!(matches!(missing, TemporalAssessment::Invalid));
    assert!(matches!(extra, TemporalAssessment::Invalid));
}

#[test]
fn temporal_assessment_rejects_candidate_and_public_urls_outside_caller_allowlist() {
    // Given: otherwise complete evidence whose URLs are absent from the caller sources.
    let response = search_response(&["alpha", "beta"], true);
    let contract = temporal_contract(&["alpha", "beta"], &["https://example.com/releases"]);

    // When: temporal eligibility is assessed.
    let assessment = assess_search(
        &response,
        VerificationMode::TemporalComparison,
        Some(&contract),
    );

    // Then: model-selected candidate/public URLs cannot expand the caller allowlist.
    assert!(matches!(assessment, TemporalAssessment::Invalid));
}
