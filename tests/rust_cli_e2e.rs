//! End-to-end public CLI contract against a deterministic Antigravity process.

use std::path::{Path, PathBuf};

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::{Value, json};
use tempfile::TempDir;

fn fixture_agy() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake_agy.py")
}

fn command() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_agy-search"));
    command.arg("--agy-path").arg(fixture_agy());
    command
}

fn json_stdout(arguments: &[&str]) -> Value {
    let mut command = command();
    let assertion = command.args(arguments).assert().success();
    serde_json::from_slice(&assertion.get_output().stdout).unwrap_or(Value::Null)
}

#[test]
fn discovers_status_and_models_as_json() {
    assert_eq!(
        json_stdout(&["status", "--json"]),
        json!({
            "available": true,
            "model_count": 2,
            "object": "status",
            "version": "9.9.9-fixture"
        })
    );
    assert_eq!(
        json_stdout(&["models"]),
        json!({
            "models": ["fixture-model", "fixture-model-high"],
            "object": "models"
        })
    );
}

#[test]
fn executes_all_five_content_operations() {
    let scenarios: [(&[&str], &str); 5] = [
        (&["search", "fixture", "-n", "1"], "search"),
        (&["extract", "https://example.com/page"], "extract"),
        (&["map", "https://example.com", "--limit", "1"], "map"),
        (&["crawl", "https://example.com", "--limit", "1"], "crawl"),
        (&["research", "fixture", "--max-sources", "1"], "research"),
    ];

    for (arguments, expected) in scenarios {
        assert_eq!(json_stdout(arguments).get("object"), Some(&json!(expected)));
    }
}

#[test]
fn reads_search_query_from_stdin() {
    command()
        .args(["--model", "fixture-model", "search", "-", "--json"])
        .write_stdin("stdin query\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"snippet\": \"stdin query\""));
}

#[test]
fn writes_json_to_nested_output_without_polluting_stdout() -> Result<(), Box<dyn std::error::Error>>
{
    let temporary = TempDir::new()?;
    let output = temporary.path().join("context/search.json");

    command()
        .args(["search", "fixture", "-o"])
        .arg(&output)
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(output.to_string_lossy()));

    let document: Value = serde_json::from_slice(&std::fs::read(output)?)?;
    assert_eq!(document.get("object"), Some(&json!("search")));
    Ok(())
}

#[test]
fn fails_closed_with_stable_exit_codes() {
    command()
        .args(["search", "invalid-source"])
        .assert()
        .code(6)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::eq("error: agy output invalid\n"));

    command()
        .args(["--model", "missing-model", "search", "fixture"])
        .assert()
        .code(7)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::eq("error: unknown agy model\n"));
}

#[test]
fn rejects_non_finite_timeout_as_usage_error() {
    command()
        .args(["--timeout", "nan", "status"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("finite number between 1 and 1800"));
}

#[test]
fn rejects_http_urls_without_an_explicit_authority() {
    for url in ["http:example.com", "http:/example.com"] {
        command()
            .args(["extract", url])
            .assert()
            .code(2)
            .stdout(predicate::str::is_empty())
            .stderr(predicate::str::contains("URL must use HTTP(S)"));
    }
}

#[test]
fn rejects_unproven_or_structurally_invalid_search_results() {
    for query in ["mcp-only", "duplicate-source", "extra-field"] {
        command()
            .args(["search", query])
            .assert()
            .code(6)
            .stdout(predicate::str::is_empty())
            .stderr(predicate::eq("error: agy output invalid\n"));
    }
}

#[test]
fn site_operations_are_same_origin_unless_explicitly_relaxed() {
    command()
        .args(["map", "https://example.com", "--instructions", "external"])
        .assert()
        .code(6);

    let relaxed = json_stdout(&[
        "map",
        "https://example.com",
        "--instructions",
        "external",
        "--allow-external",
    ]);
    assert_eq!(
        relaxed.pointer("/results/0/url"),
        Some(&json!("https://outside.example/docs"))
    );
}

#[test]
fn rejects_empty_collections_even_when_the_json_shape_is_valid() {
    command()
        .args(["map", "https://example.com", "--instructions", "empty"])
        .assert()
        .code(6);

    command()
        .args(["research", "empty-findings"])
        .assert()
        .code(6);
}
