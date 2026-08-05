//! Temporal Research validation against deterministic public CLI output.

use std::path::Path;

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::{Value, json};
use tempfile::TempDir;

fn command() -> Command {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = root.join("tests/fixtures/fake_agy.py");
    let curl = root.join("tests/fixtures/fake_curl.py");
    let mut command = Command::new(env!("CARGO_BIN_EXE_agy-search"));
    command
        .arg("--agy-path")
        .arg(fixture)
        .env("AGY_SEARCH_CURL_PATH", curl);
    command
}

fn temporal_research(query: &str, source_urls: &[&str]) -> Command {
    let mut command = command();
    command.args([
        "--verification",
        "temporal-comparison",
        "research",
        "--scope",
        "alpha",
        "--scope",
        "beta",
    ]);
    for source_url in source_urls {
        command.args(["--source-url", source_url]);
    }
    command.arg(query);
    command
}

fn distinct_source_research(query: &str) -> Command {
    temporal_research(
        query,
        &["https://example.com/alpha", "https://example.com/beta"],
    )
}

fn shared_page_research(query: &str) -> Command {
    temporal_research(query, &["https://example.com/releases"])
}

fn assert_one_shot_failure(mut command: Command) -> Result<(), Box<dyn std::error::Error>> {
    let temporary = TempDir::new()?;
    let trace = temporary.path().join("invocations.jsonl");
    command
        .env("AGY_SEARCH_FIXTURE_TRACE", &trace)
        .assert()
        .code(6)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::eq("error: agy output invalid\n"));
    assert_eq!(std::fs::read_to_string(trace)?.lines().count(), 1);
    Ok(())
}

#[test]
fn temporal_research_accepts_multiple_distinct_public_sources_without_recovery()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: two complete temporal candidates grounded by distinct public sources.
    let temporary = TempDir::new()?;
    let trace = temporary.path().join("invocations.jsonl");

    // When: temporal Research runs through the public CLI.
    let assertion = distinct_source_research("temporal-research-multi-source")
        .env("AGY_SEARCH_FIXTURE_TRACE", &trace)
        .assert()
        .success();
    let response: Value = serde_json::from_slice(&assertion.get_output().stdout)?;

    // Then: both sources remain public and Research performs only its single invocation.
    assert_eq!(
        response.pointer("/sources/1/url"),
        Some(&json!("https://example.com/beta"))
    );
    assert_eq!(std::fs::read_to_string(trace)?.lines().count(), 1);
    Ok(())
}

#[test]
fn temporal_research_rejects_unbound_public_last_updated() -> Result<(), Box<dyn std::error::Error>>
{
    // Given: otherwise complete temporal Research sources with an unbound update date.
    // When: Research runs through the public temporal-comparison CLI contract.
    // Then: the invalid response exits closed without stdout.
    assert_one_shot_failure(distinct_source_research(
        "temporal-research-unbound-last-updated",
    ))
}

#[test]
fn temporal_research_accepts_null_public_last_updated() {
    // Given: otherwise complete temporal Research sources with null update metadata.
    // When: Research runs through the public temporal-comparison CLI contract.
    // Then: the valid response remains available.
    let assertion = distinct_source_research("temporal-research-multi-source")
        .assert()
        .success();
    let response: Value =
        serde_json::from_slice(&assertion.get_output().stdout).unwrap_or(Value::Null);
    assert_eq!(
        response.pointer("/sources/0/last_updated"),
        Some(&Value::Null)
    );
}

#[test]
fn temporal_research_accepts_one_shared_page_for_differently_dated_candidates_without_recovery()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: one canonical source whose text carries both values and exact date spellings.
    let temporary = TempDir::new()?;
    let trace = temporary.path().join("invocations.jsonl");

    // When: temporal Research validates the shared-page evidence set.
    let assertion = shared_page_research("temporal-research-shared-page")
        .env("AGY_SEARCH_FIXTURE_TRACE", &trace)
        .assert()
        .success();
    let response: Value = serde_json::from_slice(&assertion.get_output().stdout)?;

    // Then: the latest scalar source date is accepted and the call is not retried.
    assert_eq!(
        response.pointer("/sources/0/date"),
        Some(&json!("2026-08-03"))
    );
    assert_eq!(std::fs::read_to_string(trace)?.lines().count(), 1);
    Ok(())
}

#[test]
fn temporal_research_rejects_a_shared_page_missing_the_older_value_and_date_text()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: a shared page that omits its older candidate's required text.
    // When: temporal Research validates it.
    // Then: it exits closed without recovery.
    assert_one_shot_failure(shared_page_research(
        "temporal-research-shared-page-missing-older",
    ))
}

#[test]
fn temporal_research_rejects_a_non_iso_secondary_source_date()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: a valid latest source and a second public source with a non-ISO scalar date.
    // When: temporal Research validates it.
    // Then: it exits closed without recovery.
    assert_one_shot_failure(distinct_source_research("temporal-research-non-iso-source"))
}

#[test]
fn temporal_research_rejects_an_orphan_public_source_scalar_date()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: a public source date that matches no same-URL candidate.
    // When: temporal Research validates it.
    // Then: it exits closed without recovery.
    assert_one_shot_failure(distinct_source_research(
        "temporal-research-orphan-source-date",
    ))
}

#[test]
fn temporal_research_rejects_when_no_source_scalar_date_matches_the_latest_candidate()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: a shared page whose scalar date verifies only the older candidate.
    // When: temporal Research validates it.
    // Then: it exits closed without recovery.
    assert_one_shot_failure(shared_page_research(
        "temporal-research-no-latest-source-date",
    ))
}

#[test]
fn temporal_research_schema_keeps_candidate_and_public_date_fields_strict() {
    // Given: the temporal Research schema consumed by Antigravity.
    // When: the fixture inspects its required machine fields.
    // Then: the schema-constrained call succeeds only with strict date evidence fields.
    distinct_source_research("temporal-research-schema")
        .assert()
        .success();
}

#[test]
fn temporal_research_accepts_candidates_within_as_of_cutoff_without_recovery()
-> Result<(), Box<dyn std::error::Error>> {
    // Given/When: every Research candidate is on or before the caller cutoff.
    let temporary = TempDir::new()?;
    let trace = temporary.path().join("within-cutoff.jsonl");
    distinct_source_research("temporal-research-multi-source")
        .args(["--as-of", "2026-08-03"])
        .env("AGY_SEARCH_FIXTURE_TRACE", &trace)
        .assert()
        .success();

    // Then: Research stays one-shot.
    assert_eq!(std::fs::read_to_string(trace)?.lines().count(), 1);
    Ok(())
}

#[test]
fn temporal_research_rejects_candidate_after_as_of_cutoff_without_recovery()
-> Result<(), Box<dyn std::error::Error>> {
    // Given/When/Then: beta is after the cutoff and Research exits after one invocation.
    let mut command = distinct_source_research("temporal-research-multi-source");
    command.args(["--as-of", "2026-08-02"]);
    assert_one_shot_failure(command)
}
