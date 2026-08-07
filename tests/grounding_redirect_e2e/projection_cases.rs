//! Standard Search projection cases kept separate from transport hardening cases.

use std::fs;

use predicates::prelude::*;
use serde_json::{Value, json};

use super::{command, fake_curl, trace_records};

#[test]
fn resolves_every_grounding_transport_to_its_direct_publisher_url()
-> Result<(), Box<dyn std::error::Error>> {
    let (_temporary, curl) = fake_curl()?;
    let trace = curl.with_extension("trace");

    let assertion = command(&curl, &trace, "two-origins")
        .args(["search", "grounding-two-results"])
        .assert()
        .success();
    let stdout = &assertion.get_output().stdout;
    let search: Value = serde_json::from_slice(stdout)?;

    assert_eq!(
        search.pointer("/results/0/url"),
        Some(&json!("https://example.com/primary"))
    );
    assert_eq!(
        search.pointer("/results/1/url"),
        Some(&json!("https://iana.org/secondary"))
    );
    assert!(!String::from_utf8_lossy(stdout).contains("vertexaisearch.cloud.google.com"));
    assert_eq!(trace_records(&trace)?.len(), 4);
    Ok(())
}

#[test]
fn normalizes_google_wrappers_and_trailing_dot_grounding_hosts()
-> Result<(), Box<dyn std::error::Error>> {
    for (query, mode) in [
        ("grounding-google-wrapper", "google-wrapper"),
        ("grounding-trailing-dot", "trailing-dot"),
    ] {
        let (_temporary, curl) = fake_curl()?;
        let trace = curl.with_extension("trace");
        let assertion = command(&curl, &trace, mode)
            .args(["search", query])
            .assert()
            .success();
        let stdout = &assertion.get_output().stdout;
        let search: Value = serde_json::from_slice(stdout)?;
        assert_eq!(
            search.pointer("/results/0/url"),
            Some(&json!("https://example.com/canonical"))
        );
        assert!(!String::from_utf8_lossy(stdout).contains("google.com"));
        assert_eq!(trace_records(&trace)?.len(), 2);
    }
    Ok(())
}

#[test]
fn rejects_terminal_url_on_grounding_transport_origin() -> Result<(), Box<dyn std::error::Error>> {
    let (_temporary, curl) = fake_curl()?;
    let trace = curl.with_extension("trace");

    command(&curl, &trace, "same-transport-origin")
        .args(["search", "grounding-redirect"])
        .assert()
        .code(6)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::eq("error: agy output invalid\n"));

    assert_eq!(trace_records(&trace)?.len(), 6);
    Ok(())
}

#[test]
fn projects_mixed_standard_search_and_prunes_failed_sources()
-> Result<(), Box<dyn std::error::Error>> {
    let (_temporary, curl) = fake_curl()?;
    let trace = curl.with_extension("trace");
    let invocation_trace = curl.with_extension("agy-trace");

    let assertion = command(&curl, &trace, "two-origins-one-dead")
        .env("AGY_SEARCH_FIXTURE_TRACE", &invocation_trace)
        .args(["search", "grounding-two-results"])
        .assert()
        .success();
    let stdout = &assertion.get_output().stdout;
    let search: Value = serde_json::from_slice(stdout)?;

    assert_eq!(
        search.pointer("/results/0/url"),
        Some(&json!("https://example.com/primary"))
    );
    assert_eq!(
        search
            .pointer("/results")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(1)
    );
    assert!(!String::from_utf8_lossy(stdout).contains("vertexaisearch.cloud.google.com"));
    assert_eq!(fs::read_to_string(invocation_trace)?.lines().count(), 1);
    assert_eq!(trace_records(&trace)?.len(), 5);
    Ok(())
}

#[test]
fn retries_all_dead_once_and_projects_mixed_second_attempt()
-> Result<(), Box<dyn std::error::Error>> {
    let (_temporary, curl) = fake_curl()?;
    let trace = curl.with_extension("trace");
    let invocation_trace = curl.with_extension("agy-trace");

    let assertion = command(&curl, &trace, "retry-projection")
        .env("AGY_SEARCH_FIXTURE_TRACE", &invocation_trace)
        .args(["search", "grounding-all-dead-then-mixed"])
        .assert()
        .success();
    let search: Value = serde_json::from_slice(&assertion.get_output().stdout)?;

    assert_eq!(fs::read_to_string(invocation_trace)?.lines().count(), 2);
    assert_eq!(
        search
            .pointer("/results")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(1)
    );
    assert_eq!(
        search.pointer("/results/0/url"),
        Some(&json!("https://example.com/primary"))
    );
    Ok(())
}
