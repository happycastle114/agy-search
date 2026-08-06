//! Semantic response-policy contracts for the public CLI.

use std::path::{Path, PathBuf};

use assert_cmd::Command;
use serde_json::{Value, json};
use tempfile::TempDir;

fn fixture_agy() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake_agy.py")
}

fn command() -> Command {
    let curl = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake_curl.py");
    let mut command = Command::new(env!("CARGO_BIN_EXE_agy-search"));
    command
        .arg("--agy-path")
        .arg(fixture_agy())
        .env("AGY_SEARCH_CURL_PATH", curl);
    command
}

fn json_stdout(arguments: &[&str]) -> Value {
    let mut command = command();
    let assertion = command.args(arguments).assert().success();
    serde_json::from_slice(&assertion.get_output().stdout).unwrap_or(Value::Null)
}

#[test]
fn defaults_to_fast_effort_and_primary_sources() -> Result<(), Box<dyn std::error::Error>> {
    // Given: an isolated invocation trace for an ordinary standard search.
    let temporary = TempDir::new()?;
    let trace = temporary.path().join("invocations.jsonl");

    // When: the public search command completes.
    let assertion = command()
        .env("AGY_SEARCH_FIXTURE_TRACE", &trace)
        .args(["search", "default-fast-primary"])
        .assert()
        .success();
    let search: Value = serde_json::from_slice(&assertion.get_output().stdout)?;
    let invocations = std::fs::read_to_string(trace)?
        .lines()
        .map(serde_json::from_str::<Value>)
        .collect::<Result<Vec<_>, _>>()?;

    // Then: the standard path returns primary evidence after exactly one content invocation.
    assert_eq!(
        search.pointer("/results/0/url"),
        Some(&json!("https://example.com/primary-source"))
    );
    assert_eq!(invocations.len(), 1);
    assert_eq!(
        invocations.first().and_then(|record| record.get("scope")),
        Some(&Value::Null),
    );
    Ok(())
}

#[test]
fn temporal_queries_send_verified_primary_and_explicit_date_policies() {
    // Given: a temporal query whose downstream consumer requires explicit scope and date policies.
    // When: the public search command serializes and sends the request.
    let search = json_stdout(&["search", "temporal-policy"]);

    // Then: the consumer accepts the typed contract and returns live-search-shaped evidence.
    assert_eq!(
        search.pointer("/results/0/url"),
        Some(&json!("https://example.com/source"))
    );
}

#[test]
fn research_queries_send_verified_primary_and_explicit_date_policies() {
    // Given: a synthesis query whose downstream consumer requires the same accuracy policies.
    // When: the public research command serializes and sends the request.
    let research = json_stdout(&["research", "temporal-policy"]);

    // Then: the consumer accepts the typed contract and returns cited research evidence.
    assert_eq!(
        research.pointer("/sources/0/url"),
        Some(&json!("https://example.com/source"))
    );
}

#[test]
fn internal_evidence_audit_is_not_exposed_in_public_search_json() {
    // Given: a successful search whose downstream schema includes an internal audit.
    // When: the public CLI renders the validated response.
    let search = json_stdout(&["search", "fixture"]);

    // Then: the stable public response omits the internal reasoning scaffold.
    assert!(search.get("evidence_audit").is_none());
}

#[test]
fn preserves_explicit_date_metadata_without_inventing_updates() {
    let search = json_stdout(&[
        "--model",
        "fixture-model",
        "--effort",
        "high",
        "search",
        "explicit-date",
    ]);
    assert_eq!(
        search.pointer("/results/0/date"),
        Some(&json!("2026-08-03"))
    );
    assert_eq!(
        search.pointer("/results/0/last_updated"),
        Some(&Value::Null)
    );

    let with_update = json_stdout(&["search", "explicit-update"]);
    assert_eq!(
        with_update.pointer("/results/0/date"),
        Some(&json!("2026-08-03"))
    );
    assert_eq!(
        with_update.pointer("/results/0/last_updated"),
        Some(&json!("2026-08-04"))
    );

    let without_metadata = json_stdout(&["search", "undated-source"]);
    assert_eq!(
        without_metadata.pointer("/results/0/date"),
        Some(&Value::Null)
    );
    assert_eq!(
        without_metadata.pointer("/results/0/last_updated"),
        Some(&Value::Null)
    );

    let research = json_stdout(&["research", "explicit-date"]);
    assert_eq!(
        research.pointer("/sources/0/date"),
        Some(&json!("2026-08-03"))
    );
    assert_eq!(
        research.pointer("/sources/0/last_updated"),
        Some(&Value::Null)
    );
}
