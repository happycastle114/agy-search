//! Runtime version-floor contract before every Antigravity content invocation.

use std::{
    path::{Path, PathBuf},
    process::Command as ProcessCommand,
    thread,
    time::Duration,
};

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::{Value, json};
use tempfile::TempDir;

fn fixture_agy() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake_agy_versioned.py")
}

fn command(trace: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_agy-search"));
    command
        .arg("--agy-path")
        .arg(fixture_agy())
        .env("AGY_SEARCH_VERSION_TRACE", trace);
    command
}

fn trace(temporary: &TempDir) -> PathBuf {
    temporary.path().join("invocations.log")
}

fn trace_lines(path: &Path) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    Ok(std::fs::read_to_string(path)?
        .lines()
        .map(ToOwned::to_owned)
        .collect())
}

#[test]
fn rejects_older_version_before_model_discovery_or_content()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: a real process surface reporting the last unsupported patch release.
    let temporary = TempDir::new()?;
    let invocation_trace = trace(&temporary);

    // When: a model-pinned content operation is requested.
    command(&invocation_trace)
        .env("AGY_SEARCH_VERSION", "1.1.9")
        .args(["--model", "fixture-model", "search", "fixture"])
        .assert()
        .code(6)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::eq("error: agy output invalid\n"));

    // Then: only the floor probe ran; neither discovery nor `-p` executed.
    assert_eq!(trace_lines(&invocation_trace)?, ["version"]);
    Ok(())
}

#[test]
fn accepts_minimum_and_later_official_versions_before_content()
-> Result<(), Box<dyn std::error::Error>> {
    for version in ["1.1.10", "1.1.11", "2.0.0"] {
        // Given: one official CLI version at or above the documented floor.
        let temporary = TempDir::new()?;
        let invocation_trace = trace(&temporary);

        // When: a normal content operation is requested.
        command(&invocation_trace)
            .env("AGY_SEARCH_VERSION", version)
            .args(["search", "fixture"])
            .assert()
            .success()
            .stdout(predicate::str::contains("\"object\": \"search\""));

        // Then: the floor probe precedes bounded advisory discovery and one content invocation.
        assert_eq!(
            trace_lines(&invocation_trace)?,
            ["version", "models", "content"]
        );
    }
    Ok(())
}

#[test]
fn accepted_version_preserves_model_discovery_before_content()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: the minimum supported release and an explicit model pin.
    let temporary = TempDir::new()?;
    let invocation_trace = trace(&temporary);

    // When: the content command completes.
    command(&invocation_trace)
        .env("AGY_SEARCH_VERSION", "1.1.10")
        .args(["--model", "fixture-model", "search", "fixture"])
        .assert()
        .success();

    // Then: version validation precedes the existing model preflight and `-p`.
    assert_eq!(
        trace_lines(&invocation_trace)?,
        ["version", "models", "content"]
    );
    Ok(())
}

#[test]
fn malformed_or_missing_version_fails_closed_without_partial_stdout()
-> Result<(), Box<dyn std::error::Error>> {
    for version in ["", "1.1.10 ", "agy 1.1.10", "1.1.10-rc.1", "1.1.10\nextra"] {
        // Given: a missing or non-official version payload.
        let temporary = TempDir::new()?;
        let invocation_trace = trace(&temporary);

        // When: a content operation is requested.
        command(&invocation_trace)
            .env("AGY_SEARCH_VERSION", version)
            .args(["search", "fixture"])
            .assert()
            .code(6)
            .stdout(predicate::str::is_empty())
            .stderr(predicate::eq("error: agy output invalid\n"));

        // Then: the sanitized failure occurs before downstream work.
        assert_eq!(trace_lines(&invocation_trace)?, ["version"]);
    }
    Ok(())
}

#[test]
fn status_enforces_the_floor_while_models_remains_a_diagnostic_command()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: an unsupported installation and separate status/models invocations.
    let unsupported = TempDir::new()?;
    let unsupported_trace = trace(&unsupported);

    // When: status is requested.
    command(&unsupported_trace)
        .env("AGY_SEARCH_VERSION", "1.1.9")
        .arg("status")
        .assert()
        .code(6)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::eq("error: agy output invalid\n"));

    // Then: status cannot claim availability and does not discover models.
    assert_eq!(trace_lines(&unsupported_trace)?, ["version"]);

    // Given: the same unsupported installation for explicit diagnostics.
    let diagnostic = TempDir::new()?;
    let diagnostic_trace = trace(&diagnostic);

    // When: models is requested directly.
    let assertion = command(&diagnostic_trace).arg("models").assert().success();

    // Then: it reports raw discovery without starting a version/content operation.
    let document: Value = serde_json::from_slice(&assertion.get_output().stdout)?;
    assert_eq!(
        document,
        json!({"models": ["fixture-model", "fixture-model-high"], "object": "models"})
    );
    assert_eq!(trace_lines(&diagnostic_trace)?, ["models"]);
    Ok(())
}

#[test]
fn all_preflight_and_content_work_share_the_original_deadline()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: preflight phases whose combined duration exceeds the caller's timeout.
    let temporary = TempDir::new()?;
    let invocation_trace = trace(&temporary);

    // When: model-pinned content is requested with one shared five-second budget.
    command(&invocation_trace)
        .env("AGY_SEARCH_VERSION_DELAY", "1.5")
        .env("AGY_SEARCH_MODELS_DELAY", "4")
        .args([
            "--timeout",
            "5",
            "--model",
            "fixture-model",
            "search",
            "fixture",
        ])
        .assert()
        .code(4)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::eq("error: agy timed out\n"));

    // Then: no fresh post-preflight budget exists for `-p`.
    assert_eq!(trace_lines(&invocation_trace)?, ["version", "models"]);
    Ok(())
}

#[test]
fn version_preflight_uses_the_original_deadline_and_kills_its_process_group()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: a slow version probe with a background child in its process group.
    let temporary = TempDir::new()?;
    let invocation_trace = trace(&temporary);
    let child_pid = temporary.path().join("version-child.pid");

    // When: the one-second invocation deadline elapses in `agy --version`.
    command(&invocation_trace)
        .env("AGY_SEARCH_VERSION_DELAY", "2")
        .env("AGY_SEARCH_VERSION_CHILD_PID", &child_pid)
        .args(["--timeout", "1", "search", "fixture"])
        .assert()
        .code(4)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::eq("error: agy timed out\n"));

    // Then: the floor probe consumed the original deadline and its child is gone.
    assert_eq!(trace_lines(&invocation_trace)?, ["version"]);
    let pid = std::fs::read_to_string(child_pid)?;
    let mut child_is_dead = false;
    for _ in 0..50 {
        let output = ProcessCommand::new("kill")
            .args(["-0", pid.trim()])
            .output()?;
        if !output.status.success() {
            child_is_dead = true;
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    assert!(child_is_dead, "version preflight child survived cleanup");
    Ok(())
}
