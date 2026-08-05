//! Caller-owned scope and source constraints at the public CLI boundary.

use predicates::prelude::*;
use serde_json::Value;
use tempfile::TempDir;

mod support;

use support::{TestResult, assert_semantic_invalid, command};

#[test]
fn temporal_search_and_research_accept_explicit_caller_contracts() -> TestResult {
    // Given: isolated traces for exact caller-owned temporal inventories.
    let temporary = TempDir::new()?;
    let search_trace = temporary.path().join("search.jsonl");
    let research_trace = temporary.path().join("research.jsonl");

    // When: Search and Research receive their exact scope and source sets.
    command(&search_trace)
        .args([
            "--verification",
            "temporal-comparison",
            "search",
            "--scope",
            "newer fixture",
            "--scope",
            "older fixture",
            "--source-url",
            "https://example.com/source",
            "temporal-complete",
        ])
        .assert()
        .success();
    command(&research_trace)
        .args([
            "--verification",
            "temporal-comparison",
            "research",
            "--scope",
            "alpha",
            "--scope",
            "beta",
            "--source-url",
            "https://example.com/alpha",
            "--source-url",
            "https://example.com/beta",
            "temporal-research-multi-source",
        ])
        .assert()
        .success();

    // Then: each valid contract reaches Antigravity exactly once.
    assert_eq!(std::fs::read_to_string(search_trace)?.lines().count(), 1);
    assert_eq!(std::fs::read_to_string(research_trace)?.lines().count(), 1);
    Ok(())
}

#[test]
fn temporal_contract_rejects_invalid_scope_sets_before_agy() -> TestResult {
    assert_semantic_invalid(&[
        "--verification",
        "temporal-comparison",
        "search",
        "--source-url",
        "https://example.com/releases",
        "q",
    ])?;
    assert_semantic_invalid(&[
        "--verification",
        "temporal-comparison",
        "search",
        "--scope",
        "alpha",
        "--scope",
        "alpha",
        "--source-url",
        "https://example.com/releases",
        "q",
    ])?;
    assert_semantic_invalid(&[
        "--verification",
        "temporal-comparison",
        "search",
        "--scope",
        "one",
        "--scope",
        "two",
        "--scope",
        "three",
        "--scope",
        "four",
        "--scope",
        "five",
        "--scope",
        "six",
        "--scope",
        "seven",
        "--scope",
        "eight",
        "--scope",
        "nine",
        "--source-url",
        "https://example.com/releases",
        "q",
    ])?;

    let temporary = TempDir::new()?;
    let empty_trace = temporary.path().join("empty-scope.jsonl");
    command(&empty_trace)
        .args([
            "--verification",
            "temporal-comparison",
            "search",
            "--scope",
            "",
            "--scope",
            "beta",
            "--source-url",
            "https://example.com/releases",
            "q",
        ])
        .assert()
        .code(2)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(
            "scope must be non-empty without surrounding whitespace",
        ));
    assert!(!empty_trace.exists());
    Ok(())
}

#[test]
fn temporal_cutoff_is_serialized_inside_the_typed_contract() -> TestResult {
    // Given: an isolated trace for a temporal Search cutoff.
    let temporary = TempDir::new()?;
    let trace = temporary.path().join("cutoff.jsonl");

    // When: the subcommand receives a valid calendar date.
    command(&trace)
        .args([
            "--verification",
            "temporal-comparison",
            "search",
            "--scope",
            "newer fixture",
            "--scope",
            "older fixture",
            "--source-url",
            "https://example.com/source",
            "--as-of",
            "2026-08-05",
            "temporal-complete",
        ])
        .assert()
        .success();

    // Then: the Antigravity request JSON carries the exact typed cutoff.
    let record: Value = serde_json::from_str(std::fs::read_to_string(trace)?.trim())?;
    assert_eq!(
        record.pointer("/cutoff").and_then(Value::as_str),
        Some("2026-08-05")
    );
    Ok(())
}
