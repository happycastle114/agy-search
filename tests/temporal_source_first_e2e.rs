//! Source-first temporal recovery contracts against deterministic child output.

mod common;

use common::{
    TemporalSearchFixture, assert_recovery_trace, local_recovery_search, source_trace_urls,
    temporal_search, trace_scopes, traced_command,
};
use predicates::prelude::*;
use serde_json::{Value, json};
use tempfile::TempDir;

#[path = "temporal_source_first_cases/value_mismatch.rs"]
mod value_mismatch;

#[test]
fn temporal_comparison_recovers_both_scopes_locally_without_scoped_model_calls()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: one canonical source whose exact first rows carry strong date pins.
    let temporary = TempDir::new()?;
    let (command, agy_trace) = traced_command(&temporary);
    let source_trace = temporary.path().join("local-source.jsonl");

    // When: the primary model output requires recovery.
    let assertion = local_recovery_search(command, "temporal-local", "https://example.com/local")
        .env("AGY_SEARCH_SOURCE_FETCH_TRACE", &source_trace)
        .assert()
        .success();
    let response: Value = serde_json::from_slice(&assertion.get_output().stdout)?;

    // Then: local facts select alpha and avoid every scoped model invocation.
    assert_eq!(
        response.pointer("/results/0/title"),
        Some(&json!("alpha alpha-v2"))
    );
    assert_eq!(
        response.pointer("/results/0/date"),
        Some(&json!("2026-08-05"))
    );
    assert_eq!(
        response.pointer("/results/0/url"),
        Some(&json!("https://example.com/local"))
    );
    assert_eq!(trace_scopes(&agy_trace)?, vec![None]);
    assert_eq!(
        source_trace_urls(&source_trace)?,
        vec!["https://example.com/local"]
    );
    Ok(())
}

#[test]
fn temporal_as_of_uses_exact_source_rows_without_normalizing_prefixed_model_values()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: v25-shaped primary values that are prefixed or wrong, plus exact strong source rows.
    let temporary = TempDir::new()?;
    let (command, agy_trace) = traced_command(&temporary);

    // When: the caller opts into source-first recovery with a typed cutoff.
    let assertion = local_recovery_search(
        command,
        "temporal-source-first-v25",
        "https://example.com/local-v25",
    )
    .args(["--as-of", "2026-08-05"])
    .assert()
    .success();
    let response: Value = serde_json::from_slice(&assertion.get_output().stdout)?;

    // Then: exact source facts win unchanged after exactly one Antigravity invocation.
    assert_eq!(
        response.pointer("/results/0/title"),
        Some(&json!("alpha 25.1.0"))
    );
    assert_eq!(
        response.pointer("/results/0/date"),
        Some(&json!("2026-08-05"))
    );
    assert_eq!(trace_scopes(&agy_trace)?, vec![None]);
    Ok(())
}

#[test]
fn temporal_as_of_repairs_a_complete_after_cutoff_primary_from_source_facts()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: an exact-scope primary whose alpha candidate exceeds the cutoff.
    let temporary = TempDir::new()?;
    let (command, agy_trace) = traced_command(&temporary);

    // When: exact strong source rows are at or before the cutoff.
    let assertion = local_recovery_search(
        command,
        "temporal-primary-after-cutoff-source-first",
        "https://example.com/local-v25",
    )
    .args(["--as-of", "2026-08-05"])
    .assert()
    .success();
    let response: Value = serde_json::from_slice(&assertion.get_output().stdout)?;

    // Then: source-first repair succeeds without a scoped model wave.
    assert_eq!(
        response.pointer("/results/0/title"),
        Some(&json!("alpha 25.1.0"))
    );
    assert_eq!(
        response.pointer("/results/0/date"),
        Some(&json!("2026-08-05"))
    );
    assert_eq!(trace_scopes(&agy_trace)?, vec![None]);
    Ok(())
}

#[test]
fn temporal_as_of_falls_back_as_one_wave_when_any_local_fact_is_after_cutoff()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: one strong local fact after the caller cutoff.
    let temporary = TempDir::new()?;
    let (command, agy_trace) = traced_command(&temporary);

    // When: source-first local extraction assesses the complete fact set.
    let assertion = temporal_search(
        command,
        TemporalSearchFixture {
            scopes: ["alpha", "beta"],
            sources: &[
                "https://example.com/local-after-cutoff",
                "https://example.com/alpha",
                "https://example.com/beta",
            ],
            query: "temporal-source-after-cutoff-fallback",
        },
    )
    .args(["--as-of", "2026-08-05"])
    .assert()
    .success();
    let response: Value = serde_json::from_slice(&assertion.get_output().stdout)?;

    // Then: no local fact is merged and the existing complete scoped wave wins.
    assert_eq!(
        response.pointer("/results/0/title"),
        Some(&json!("alpha-v2"))
    );
    assert_recovery_trace(&trace_scopes(&agy_trace)?);
    Ok(())
}
#[test]
fn temporal_as_of_rejects_an_after_cutoff_scoped_fallback_without_partial_output()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: local recovery falls back and alpha's scoped result exceeds the cutoff.
    let temporary = TempDir::new()?;
    let (command, agy_trace) = traced_command(&temporary);

    // When/Then: the complete wave runs, but the public boundary exits 6 with no output.
    temporal_search(
        command,
        TemporalSearchFixture {
            scopes: ["alpha", "beta"],
            sources: &[
                "https://example.com/local-after-cutoff",
                "https://example.com/alpha",
                "https://example.com/beta",
            ],
            query: "temporal-fallback-after-cutoff",
        },
    )
    .args(["--as-of", "2026-08-05"])
    .assert()
    .code(6)
    .stdout(predicate::str::is_empty())
    .stderr(predicate::eq("error: agy output invalid\n"));
    let scopes = trace_scopes(&agy_trace)?;
    assert_eq!(scopes.first(), Some(&None));
    let mut recovered = scopes.into_iter().skip(1).collect::<Vec<_>>();
    recovered.sort();
    let alpha_only = vec![Some("alpha".to_owned())];
    let both_scopes = vec![Some("alpha".to_owned()), Some("beta".to_owned())];
    assert!(recovered == alpha_only || recovered == both_scopes);
    Ok(())
}
#[test]
fn temporal_as_of_rejects_a_tied_source_first_latest_date() -> Result<(), Box<dyn std::error::Error>>
{
    // Given: two exact source facts tied at the cutoff.
    let temporary = TempDir::new()?;
    let (command, agy_trace) = traced_command(&temporary);

    // When/Then: the existing unique-latest merge rejects the tie after one model call.
    local_recovery_search(
        command,
        "temporal-source-first-tie",
        "https://example.com/local-tie",
    )
    .args(["--as-of", "2026-08-05"])
    .assert()
    .code(6)
    .stdout(predicate::str::is_empty())
    .stderr(predicate::eq("error: agy output invalid\n"));
    assert_eq!(trace_scopes(&agy_trace)?, vec![None]);
    Ok(())
}

#[test]
fn temporal_comparison_uses_the_existing_full_scoped_wave_when_local_rows_have_no_pin()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: the primary source has valid panels but no strong data-date-pin facts.
    let temporary = TempDir::new()?;
    let (command, agy_trace) = traced_command(&temporary);
    let source_trace = temporary.path().join("unextractable-source.jsonl");
    let sources = [
        "https://example.com/local-unextractable",
        "https://example.com/alpha",
        "https://example.com/beta",
    ];

    // When: temporal recovery cannot complete all scopes locally.
    let assertion = temporal_search(
        command,
        TemporalSearchFixture {
            scopes: ["alpha", "beta"],
            sources: &sources,
            query: "temporal-local-unextractable",
        },
    )
    .env("AGY_SEARCH_SOURCE_FETCH_TRACE", &source_trace)
    .assert()
    .success();
    let response: Value = serde_json::from_slice(&assertion.get_output().stdout)?;

    // Then: the existing all-scope LLM fallback runs and still selects alpha.
    assert_eq!(
        response.pointer("/results/0/title"),
        Some(&json!("alpha-v2"))
    );
    assert_recovery_trace(&trace_scopes(&agy_trace)?);
    let mut fetched = source_trace_urls(&source_trace)?;
    fetched.sort();
    let mut expected = sources.map(str::to_owned).into_iter().collect::<Vec<_>>();
    expected.sort();
    assert_eq!(fetched, expected);
    Ok(())
}
