//! Public direct-source reachability at the Standard Search boundary.

use std::{
    fs,
    path::{Path, PathBuf},
};

use assert_cmd::Command;
use serde_json::{Value, json};

const FAKE_CURL: &str = include_str!("fixtures/fake_grounding_redirect_curl.py");

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn command(
    temporary: &tempfile::TempDir,
) -> Result<(Command, PathBuf, PathBuf), Box<dyn std::error::Error>> {
    let curl = temporary.path().join("curl");
    fs::write(&curl, FAKE_CURL)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        fs::set_permissions(&curl, fs::Permissions::from_mode(0o700))?;
    }
    let redirect_trace = temporary.path().join("redirects.jsonl");
    let invocation_trace = temporary.path().join("invocations.jsonl");
    let mut command = Command::new(env!("CARGO_BIN_EXE_agy-search"));
    command
        .arg("--agy-path")
        .arg(fixture("fake_agy.py"))
        .env("AGY_SEARCH_CURL_PATH", &curl)
        .env("AGY_REDIRECT_MODE", "direct-validation")
        .env("AGY_REDIRECT_TRACE", &redirect_trace)
        .env("AGY_SEARCH_FIXTURE_TRACE", &invocation_trace);
    Ok((command, redirect_trace, invocation_trace))
}

fn line_count(path: &Path) -> Result<usize, Box<dyn std::error::Error>> {
    Ok(fs::read_to_string(path)?.lines().count())
}

#[test]
fn projects_reachable_direct_results_and_prunes_dead_rows() -> Result<(), Box<dyn std::error::Error>>
{
    let temporary = tempfile::tempdir()?;
    let (mut command, redirect_trace, invocation_trace) = command(&temporary)?;

    let assertion = command
        .args(["search", "standard-direct-mixed"])
        .assert()
        .success();
    let response: Value = serde_json::from_slice(&assertion.get_output().stdout)?;

    assert_eq!(
        response.pointer("/results/0/url"),
        Some(&json!("https://example.com/reachable"))
    );
    assert_eq!(
        response
            .get("results")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(1)
    );
    assert_eq!(line_count(&redirect_trace)?, 2);
    assert_eq!(line_count(&invocation_trace)?, 1);
    Ok(())
}

#[test]
fn replaces_a_direct_redirect_with_its_terminal_public_url()
-> Result<(), Box<dyn std::error::Error>> {
    let temporary = tempfile::tempdir()?;
    let (mut command, redirect_trace, invocation_trace) = command(&temporary)?;

    let assertion = command
        .args(["search", "standard-direct-redirect"])
        .assert()
        .success();
    let response: Value = serde_json::from_slice(&assertion.get_output().stdout)?;

    assert_eq!(
        response.pointer("/results/0/url"),
        Some(&json!("https://iana.org/terminal"))
    );
    assert_eq!(line_count(&redirect_trace)?, 2);
    assert_eq!(line_count(&invocation_trace)?, 1);
    Ok(())
}

#[test]
fn retries_once_after_unsafe_dead_or_regional_google_direct_output()
-> Result<(), Box<dyn std::error::Error>> {
    for query in [
        "standard-direct-http-first",
        "standard-direct-private-first",
        "standard-direct-localhost-dot-first",
        "standard-direct-dead-first",
        "standard-regional-google-first",
    ] {
        let temporary = tempfile::tempdir()?;
        let (mut command, _redirect_trace, invocation_trace) = command(&temporary)?;

        let assertion = command.args(["search", query]).assert().success();
        let response: Value = serde_json::from_slice(&assertion.get_output().stdout)?;

        assert_eq!(
            response.pointer("/results/0/url"),
            Some(&json!("https://example.com/direct-safe")),
            "query: {query}"
        );
        assert_eq!(line_count(&invocation_trace)?, 2, "query: {query}");
    }
    Ok(())
}
