//! Single-scope temporal verification against an exact caller-owned source.

mod common;

use common::{command, source_trace_urls};
use predicates::prelude::*;
use serde_json::{Value, json};
use tempfile::TempDir;

const SOURCE_URL: &str = "https://example.com/antigravity-releases";

#[test]
fn single_scope_recovers_the_exact_source_tuple_without_a_scoped_rerun()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: a primary answer that borrows a sibling product's version.
    let temporary = TempDir::new()?;
    let agy_trace = temporary.path().join("agy.jsonl");
    let source_trace = temporary.path().join("source.jsonl");
    let mut process = command();
    process.env("AGY_SEARCH_FIXTURE_TRACE", &agy_trace);

    // When: one exact scope, canonical source, and typed cutoff are requested.
    let assertion = process
        .env("AGY_SEARCH_SOURCE_FETCH_TRACE", &source_trace)
        .args([
            "--verification",
            "temporal-comparison",
            "search",
            "--scope",
            "Antigravity CLI",
            "--source-url",
            SOURCE_URL,
            "--as-of",
            "2026-08-03",
            "temporal-single-scope",
        ])
        .assert()
        .success();
    let response: Value = serde_json::from_slice(&assertion.get_output().stdout)?;

    // Then: the source-backed CLI tuple wins and the sibling value never escapes.
    assert_eq!(
        response.pointer("/results/0/title"),
        Some(&json!("Antigravity CLI 1.1.10"))
    );
    assert_eq!(
        response.pointer("/results/0/date"),
        Some(&json!("2026-08-03"))
    );
    assert_eq!(response.pointer("/results/0/url"), Some(&json!(SOURCE_URL)));
    assert!(!String::from_utf8_lossy(&assertion.get_output().stdout).contains("2.5.0"));
    assert_eq!(std::fs::read_to_string(agy_trace)?.lines().count(), 1);
    assert_eq!(
        source_trace_urls(&source_trace)?,
        vec![SOURCE_URL.to_owned()]
    );
    Ok(())
}

#[test]
fn single_scope_without_cutoff_is_rejected_before_agy() -> Result<(), Box<dyn std::error::Error>> {
    // Given: a one-scope request without the cutoff needed for source-first recovery.
    let temporary = TempDir::new()?;
    let trace = temporary.path().join("agy.jsonl");

    // When: the request crosses the public CLI boundary.
    command()
        .env("AGY_SEARCH_FIXTURE_TRACE", &trace)
        .args([
            "--verification",
            "temporal-comparison",
            "search",
            "--scope",
            "Antigravity CLI",
            "--source-url",
            SOURCE_URL,
            "temporal-single-scope",
        ])
        .assert()
        .code(2)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::eq("error: invalid agy command\n"));

    // Then: Antigravity was never started.
    assert!(!trace.exists());
    Ok(())
}

#[test]
fn single_scope_research_accepts_one_exact_source_backed_tuple()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: one exact caller-owned research scope and canonical source.
    let temporary = TempDir::new()?;
    let trace = temporary.path().join("agy.jsonl");

    // When: the temporal Research operation receives its required cutoff.
    command()
        .env("AGY_SEARCH_FIXTURE_TRACE", &trace)
        .args([
            "--verification",
            "temporal-comparison",
            "research",
            "--scope",
            "Antigravity CLI",
            "--source-url",
            SOURCE_URL,
            "--as-of",
            "2026-08-03",
            "temporal-research-single-scope",
        ])
        .assert()
        .success();

    // Then: the exact-one schema and source verification both accepted the request.
    assert_eq!(std::fs::read_to_string(trace)?.lines().count(), 1);
    Ok(())
}

#[test]
fn ambiguous_single_scope_source_fails_closed() -> Result<(), Box<dyn std::error::Error>> {
    // Given/When/Then: two exact-scope rows cannot produce a partial public tuple.
    assert_single_scope_fails_closed(
        "temporal-single-scope-ambiguous",
        "https://example.com/antigravity-ambiguous",
    )
}

#[test]
fn missing_single_scope_source_fails_closed() -> Result<(), Box<dyn std::error::Error>> {
    // Given/When/Then: a sibling-only source cannot produce a partial public tuple.
    assert_single_scope_fails_closed(
        "temporal-single-scope-missing",
        "https://example.com/antigravity-missing",
    )
}

#[test]
fn after_cutoff_single_scope_source_fails_closed() -> Result<(), Box<dyn std::error::Error>> {
    // Given/When/Then: an exact tuple after the cutoff cannot reach public output.
    assert_single_scope_fails_closed(
        "temporal-single-scope-after-cutoff",
        "https://example.com/antigravity-after-cutoff",
    )
}

fn assert_single_scope_fails_closed(
    query: &str,
    source_url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let temporary = TempDir::new()?;
    let agy_trace = temporary.path().join("agy.jsonl");
    let source_trace = temporary.path().join("source.jsonl");
    command()
        .env("AGY_SEARCH_FIXTURE_TRACE", &agy_trace)
        .env("AGY_SEARCH_SOURCE_FETCH_TRACE", &source_trace)
        .args([
            "--verification",
            "temporal-comparison",
            "search",
            "--scope",
            "Antigravity CLI",
            "--source-url",
            source_url,
            "--as-of",
            "2026-08-03",
            query,
        ])
        .assert()
        .code(6)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::eq("error: agy output invalid\n"));
    assert_eq!(std::fs::read_to_string(&agy_trace)?.lines().count(), 2);
    assert_eq!(
        source_trace_urls(&source_trace)?,
        vec![source_url.to_owned()]
    );
    Ok(())
}
