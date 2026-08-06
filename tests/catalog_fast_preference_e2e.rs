//! End-to-end catalog-negotiated default search-model policy.

use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

const PREFERRED_MODEL: &str = "gemini-3.6-flash-low";

fn fixture_agy() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake_agy_catalog_policy.py")
}

fn command(trace: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_agy-search"));
    command
        .arg("--agy-path")
        .arg(fixture_agy())
        .env("AGY_SEARCH_CATALOG_TRACE", trace);
    command
}

fn trace_records(path: &Path) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
    std::fs::read_to_string(path)?
        .lines()
        .map(serde_json::from_str)
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

#[test]
fn standard_low_search_uses_the_discovered_preferred_model_once()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: an unpinned low-effort standard search and a catalog with the preferred model.
    let temporary = TempDir::new()?;
    let trace = temporary.path().join("invocations.jsonl");

    // When: the public CLI completes the search.
    command(&trace)
        .args(["search", "catalog policy"])
        .assert()
        .success();

    // Then: one advisory catalog probe selects the preferred typed model for one content attempt.
    assert_eq!(
        trace_records(&trace)?,
        vec![
            serde_json::json!({"kind":"version","model":null,"effort":null}),
            serde_json::json!({"kind":"models","model":null,"effort":null}),
            serde_json::json!({"kind":"content","model":PREFERRED_MODEL,"effort":"low"}),
        ]
    );
    Ok(())
}

#[test]
fn standard_low_search_falls_back_to_the_provider_default_after_advisory_catalog_failure()
-> Result<(), Box<dyn std::error::Error>> {
    for mode in ["absent", "failed"] {
        // Given: a catalog where the preferred model is absent or discovery itself fails.
        let temporary = TempDir::new()?;
        let trace = temporary.path().join("invocations.jsonl");

        // When: the unpinned low-effort standard search completes within its shared deadline.
        command(&trace)
            .env("AGY_SEARCH_CATALOG_MODE", mode)
            .args(["search", "catalog policy"])
            .assert()
            .success();

        // Then: content runs once with no --model, preserving the provider default.
        assert_eq!(
            trace_records(&trace)?,
            vec![
                serde_json::json!({"kind":"version","model":null,"effort":null}),
                serde_json::json!({"kind":"models","model":null,"effort":null}),
                serde_json::json!({"kind":"content","model":null,"effort":"low"}),
            ]
        );
    }
    Ok(())
}

#[test]
fn explicit_model_remains_a_strict_catalog_validated_override()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: a caller-owned explicit model that the catalog exposes.
    let temporary = TempDir::new()?;
    let trace = temporary.path().join("invocations.jsonl");

    // When: the public search command completes.
    command(&trace)
        .args(["--model", "fixture-model", "search", "catalog policy"])
        .assert()
        .success();

    // Then: required validation precedes exactly one explicitly pinned content invocation.
    assert_eq!(
        trace_records(&trace)?,
        vec![
            serde_json::json!({"kind":"version","model":null,"effort":null}),
            serde_json::json!({"kind":"models","model":null,"effort":null}),
            serde_json::json!({"kind":"content","model":"fixture-model","effort":"low"}),
        ]
    );
    Ok(())
}

#[test]
fn medium_and_high_searches_skip_advisory_catalog_preference()
-> Result<(), Box<dyn std::error::Error>> {
    for effort in ["medium", "high"] {
        // Given: an unpinned standard search requesting a non-low effort.
        let temporary = TempDir::new()?;
        let trace = temporary.path().join("invocations.jsonl");

        // When: the public CLI completes the search.
        command(&trace)
            .args(["--effort", effort, "search", "catalog policy"])
            .assert()
            .success();

        // Then: no catalog process runs and the provider default receives the requested effort.
        assert_eq!(
            trace_records(&trace)?,
            vec![
                serde_json::json!({"kind":"version","model":null,"effort":null}),
                serde_json::json!({"kind":"content","model":null,"effort":effort}),
            ]
        );
    }
    Ok(())
}

#[test]
fn advisory_catalog_discovery_is_capped_at_five_seconds_inside_the_shared_deadline()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: a catalog that exceeds five seconds and content that needs the remaining shared time.
    let temporary = TempDir::new()?;
    let trace = temporary.path().join("invocations.jsonl");
    let started = Instant::now();

    // When: the caller grants eight seconds to the complete unpinned standard search.
    command(&trace)
        .env("AGY_SEARCH_CATALOG_DELAY", "6")
        .env("AGY_SEARCH_CONTENT_DELAY", "2")
        .args(["--timeout", "8", "search", "catalog policy"])
        .assert()
        .success();

    // Then: catalog failure falls back before content consumes the original deadline.
    assert!(started.elapsed() < Duration::from_millis(7_750));
    assert_eq!(
        trace_records(&trace)?,
        vec![
            serde_json::json!({"kind":"version","model":null,"effort":null}),
            serde_json::json!({"kind":"models","model":null,"effort":null}),
            serde_json::json!({"kind":"content","model":null,"effort":"low"}),
        ]
    );
    Ok(())
}
