//! Scoped temporal recovery contracts against deterministic child output.

mod common;

use common::{
    TraceRecord, assert_recovery_trace, command, recovery_search, source_trace_urls, trace_scopes,
    traced_command,
};
use predicates::prelude::*;
use serde_json::{Value, json};
use tempfile::TempDir;

#[test]
fn temporal_comparison_recovers_each_complete_scope_and_selects_unique_latest()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: a complete two-scope primary audit with recoverable evidence binding failures.
    let temporary = TempDir::new()?;
    let (command, trace) = traced_command(&temporary);

    // When: temporal search runs through the public CLI.
    let assertion = recovery_search(command, "temporal-recoverable")
        .args([
            "--domain",
            "docs.example",
            "--domain",
            "support.example",
            "--country",
            "KR",
        ])
        .assert()
        .success();
    let response: Value = serde_json::from_slice(&assertion.get_output().stdout)?;

    // Then: one primary plus one scoped run per label yields alpha's exact newer source.
    assert_eq!(
        response.pointer("/results/0/url"),
        Some(&json!("https://example.com/alpha"))
    );
    assert_eq!(
        response.pointer("/results/0/date"),
        Some(&json!("2026-08-05"))
    );
    assert_eq!(
        response.pointer("/results/0/title"),
        Some(&json!("alpha-v2"))
    );
    assert_recovery_trace(&trace_scopes(&trace)?);
    Ok(())
}

#[test]
fn temporal_recovery_serializes_and_enforces_the_exact_scoped_search_query()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: a recoverable temporal comparison with two exact scope labels.
    let temporary = TempDir::new()?;
    let (command, trace) = traced_command(&temporary);

    // When: the public CLI performs scoped recovery.
    recovery_search(command, "temporal-recoverable")
        .args([
            "--domain",
            "docs.example",
            "--domain",
            "support.example",
            "--country",
            "KR",
        ])
        .assert()
        .success();
    let records = std::fs::read_to_string(trace)?
        .lines()
        .map(serde_json::from_str::<TraceRecord>)
        .collect::<Result<Vec<_>, _>>()?;

    // Then: each recovery payload binds its original query to its exact quoted label.
    let recovered = records.into_iter().skip(1).collect::<Vec<_>>();
    assert_eq!(recovered.len(), 2);
    for record in recovered {
        let scope = record.scope.ok_or("recovery scope missing")?;
        let expected = format!(
            "For exact scope \"{scope}\" only, find its latest release, exact version, and source-published date; do not use another scope's value. Original request constraints: temporal-recoverable site:docs.example site:support.example country:KR"
        );
        assert_eq!(record.query.as_deref(), Some("temporal-recoverable"));
        assert_eq!(
            record.required_search_query.as_deref(),
            Some(expected.as_str()),
        );
    }
    Ok(())
}

#[test]
fn temporal_recovery_rejects_scoped_query_and_tool_policy_violations() {
    // Given: real-shape recovery streams with invalid first queries or tool calls.
    for query in [
        "temporal-recoverable-bare-query",
        "temporal-recoverable-first-followup-query",
        "temporal-recoverable-poisoned-followup",
        "temporal-recoverable-read-url",
        "temporal-recoverable-three-searches",
    ] {
        // When: the public CLI parses the scoped stream-json evidence.
        let assertion = recovery_search(command(), query).assert();

        // Then: recovery fails closed at the public error boundary.
        assertion
            .code(6)
            .stdout(predicate::str::is_empty())
            .stderr(predicate::eq("error: agy output invalid\n"));
    }
}

#[test]
fn temporal_recovery_accepts_one_exact_value_followup() {
    // Given: each scoped run searches the exact request, then appends only its version token.
    let assertion = recovery_search(command(), "temporal-recoverable-value-followup")
        .assert()
        .success();

    // When/Then: the public result remains fully source-backed and selects the latest scope.
    let response: Value =
        serde_json::from_slice(&assertion.get_output().stdout).expect("response must be JSON");
    assert_eq!(
        response.pointer("/results/0/title"),
        Some(&json!("alpha-v2"))
    );
    assert_eq!(
        response.pointer("/results/0/date"),
        Some(&json!("2026-08-05"))
    );
}

#[test]
fn temporal_comparison_discards_every_scope_when_one_recovery_is_invalid()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: a complete two-scope plan whose beta recovery lacks exact value binding.
    let temporary = TempDir::new()?;
    let (command, trace) = traced_command(&temporary);

    // When: temporal search runs through the public CLI.
    recovery_search(command, "temporal-recoverable-one-invalid")
        .assert()
        .code(6)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::eq("error: agy output invalid\n"));

    // Then: all planned calls ran once and no partial result escaped.
    assert_recovery_trace(&trace_scopes(&trace)?);
    Ok(())
}

#[test]
fn temporal_comparison_rejects_a_recovered_tuple_borrowed_from_a_sibling_panel()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: a primary winner that is valid publicly, but a recovered beta tuple
    // that borrows alpha's value/date from the primary source panel.
    let temporary = TempDir::new()?;
    let (command, agy_trace) = traced_command(&temporary);
    let source_trace = temporary.path().join("source-fetch.jsonl");

    // When: source-backed temporal recovery runs through the public CLI.
    recovery_search(command, "temporal-recoverable-borrowed")
        .env("AGY_SEARCH_SOURCE_FETCH_TRACE", &source_trace)
        .assert()
        .code(6)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::eq("error: agy output invalid\n"));

    // Then: all recovery calls ran, but no partial result escaped, and each
    // caller-owned source was fetched exactly once for the shared contract.
    assert_recovery_trace(&trace_scopes(&agy_trace)?);
    let mut fetched = source_trace_urls(&source_trace)?;
    fetched.sort();
    assert_eq!(
        fetched,
        vec![
            "https://example.com/alpha".to_owned(),
            "https://example.com/beta".to_owned(),
            "https://example.com/primary".to_owned(),
        ]
    );
    Ok(())
}
