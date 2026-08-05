//! Temporal-comparison validation contracts against deterministic child output.

mod common;

use common::{default_search, trace_scopes, version_search};
use predicates::prelude::*;
use serde_json::{Value, json};
use tempfile::TempDir;

#[test]
fn temporal_comparison_mode_requires_complete_dated_candidates() {
    let assertion = version_search("temporal-complete").assert().success();
    let complete: Value =
        serde_json::from_slice(&assertion.get_output().stdout).unwrap_or(Value::Null);
    assert_eq!(
        complete.pointer("/results/0/date"),
        Some(&json!("2026-08-03"))
    );

    default_search("temporal-incomplete")
        .assert()
        .code(6)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::eq("error: agy output invalid\n"));
}

#[test]
fn temporal_comparison_rejects_unbound_public_last_updated() {
    // Given: otherwise complete temporal candidates and a public winner claiming an update date.
    // When: Search runs through the public temporal-comparison CLI contract.
    // Then: no unbound temporal metadata reaches stdout.
    version_search("temporal-unbound-last-updated")
        .assert()
        .code(6)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::eq("error: agy output invalid\n"));
}

#[test]
fn temporal_comparison_accepts_null_public_last_updated() {
    // Given: otherwise complete temporal candidates with no claimed public update date.
    // When: Search runs through the public temporal-comparison CLI contract.
    // Then: the valid response remains available.
    let assertion = version_search("temporal-complete").assert().success();
    let response: Value =
        serde_json::from_slice(&assertion.get_output().stdout).unwrap_or(Value::Null);
    assert_eq!(
        response.pointer("/results/0/last_updated"),
        Some(&Value::Null)
    );
}

#[test]
fn temporal_comparison_rejects_dates_not_bound_to_exact_source_evidence() {
    default_search("temporal-unbound")
        .assert()
        .code(6)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::eq("error: agy output invalid\n"));
}

#[test]
fn temporal_comparison_rejects_a_public_winner_older_than_the_maximum_candidate() {
    version_search("temporal-wrong-winner")
        .assert()
        .code(6)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::eq("error: agy output invalid\n"));
}

#[test]
fn temporal_comparison_rejects_non_iso_candidate_dates() {
    version_search("temporal-invalid-date")
        .assert()
        .code(6)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::eq("error: agy output invalid\n"));
}

#[test]
fn temporal_comparison_requires_exact_source_date_text() {
    version_search("temporal-source-text-missing")
        .assert()
        .code(6)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::eq("error: agy output invalid\n"));
}

#[test]
fn temporal_comparison_schema_requires_every_ordering_field() {
    version_search("temporal-schema").assert().success();
}

#[test]
fn temporal_comparison_does_not_recover_an_incomplete_inventory()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: a primary response with coverage_complete=false.
    let temporary = TempDir::new()?;
    let trace = temporary.path().join("invocations.jsonl");

    // When: temporal search runs through the public CLI.
    default_search("temporal-incomplete")
        .env("AGY_SEARCH_FIXTURE_TRACE", &trace)
        .assert()
        .code(6)
        .stdout(predicate::str::is_empty());

    // Then: it fails closed after the primary invocation only.
    assert_eq!(trace_scopes(&trace)?, vec![None]);
    Ok(())
}
