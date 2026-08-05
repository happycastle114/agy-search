use std::{str::FromStr, time::Duration};

use clap::ValueEnum;
use proptest::prelude::*;
use serde_json::json;

use super::*;
use crate::source_restriction::SourceRestriction;

proptest! {
    #[test]
    fn model_slug_rejects_embedded_whitespace(
        prefix in "[a-z0-9-]{1,12}",
        suffix in "[a-z0-9-]{1,12}",
        whitespace in prop_oneof![Just(" "), Just("\t"), Just("\n")],
    ) {
        let candidate = format!("{prefix}{whitespace}{suffix}");
        prop_assert!(ModelSlug::from_str(&candidate).is_err());
    }
}

#[test]
fn required_search_query_classifies_only_one_safe_followup_token() {
    let query = NonEmptyText::parse("release status").expect("test query must be valid");
    let required = RequiredSearchQuery::for_exact_scope(&query, "alpha", &[], None);
    let initial = serde_json::to_value(&required)
        .expect("required query must serialize")
        .as_str()
        .expect("required query must serialize as a string")
        .to_owned();

    assert_eq!(required.classify(&initial), ScopedQueryKind::Initial);
    assert_eq!(
        required.classify(&format!("{initial} 0.1.9")),
        ScopedQueryKind::ExactValueFollowup
    );
    for unsafe_suffix in [
        "0.1.9 July 29, 2026 https://example.com/release",
        "2026-07-29",
        "https://example.com/release",
        "two tokens",
        "",
    ] {
        assert_eq!(
            required.classify(&format!("{initial} {unsafe_suffix}")),
            ScopedQueryKind::Invalid
        );
    }
}

#[test]
fn model_slug_parses_only_a_closed_effort_suffix() {
    assert_eq!(
        ModelSlug::from_str("gemini-pro-high")
            .expect("model slug must parse")
            .effort_suffix(),
        Some(Effort::High)
    );
    assert_eq!(
        ModelSlug::from_str("gemini-pro-medium")
            .expect("model slug must parse")
            .effort_suffix(),
        Some(Effort::Medium)
    );
    assert_eq!(
        ModelSlug::from_str("gemini-pro-low")
            .expect("model slug must parse")
            .effort_suffix(),
        Some(Effort::Low)
    );
    assert_eq!(
        ModelSlug::from_str("gemini-pro-standard")
            .expect("model slug must parse")
            .effort_suffix(),
        None
    );
}

#[test]
fn policy_spellings_and_maxima_remain_closed() {
    assert_eq!(
        serde_json::to_value(VerificationMode::TemporalComparison)
            .expect("verification mode must serialize"),
        json!("temporal_comparison")
    );
    assert_eq!(
        VerificationMode::TemporalComparison.to_string(),
        "temporal-comparison"
    );
    assert_eq!(OutputFormat::StreamJson.to_string(), "stream-json");
    assert_eq!(
        <Effort as ValueEnum>::from_str("high", false),
        Ok(Effort::High)
    );

    assert_eq!(ResearchToolBudget::Single.maximum(), 1);
    assert_eq!(ResearchToolBudget::StandardSearch.maximum(), 2);
    assert_eq!(ResearchToolBudget::TemporalSearch.maximum(), 8);
    assert_eq!(
        ResearchToolBudget::Research(ResearchAttemptBudget::from_max_sources(4)).maximum(),
        6
    );

    let query = NonEmptyText::parse("release status").expect("test query must be valid");
    let required = RequiredSearchQuery::for_exact_scope(&query, "alpha", &[], None);
    assert_eq!(
        ResearchToolPolicy::ScopedTemporalSearch(required.clone()).maximum(),
        2
    );
    assert_eq!(
        ResearchToolPolicy::Restricted {
            budget: ResearchToolBudget::TemporalSearch,
            restriction: Box::new(SourceRestriction::Unrestricted),
        }
        .maximum(),
        8
    );
    assert_eq!(
        ResearchToolPolicy::RestrictedScopedTemporalSearch {
            required_query: required,
            restriction: Box::new(SourceRestriction::Unrestricted),
        }
        .maximum(),
        2
    );
}

#[test]
fn research_attempt_budget_adds_bounded_discovery_and_verification_overhead() {
    // Given source limits at the lower, synthesis, deep, default, and upper boundaries.
    let cases = [(0, 2), (1, 3), (4, 6), (8, 10), (10, 12), (20, 12)];

    // When/Then the typed budget adds two attempts and never exceeds twelve.
    for (max_sources, expected) in cases {
        assert_eq!(
            ResearchAttemptBudget::from_max_sources(max_sources).maximum(),
            expected
        );
    }
}

#[test]
fn timeout_preserves_its_cli_range_and_error_message() {
    assert_eq!(
        TimeoutSeconds::from_str("120")
            .expect("default timeout must parse")
            .duration(),
        Duration::from_secs(120)
    );
    for invalid in ["0", "1801", "nan", "infinity", "invalid"] {
        assert_eq!(
            TimeoutSeconds::from_str(invalid),
            Err("timeout must be a finite number between 1 and 1800 seconds")
        );
    }
}
