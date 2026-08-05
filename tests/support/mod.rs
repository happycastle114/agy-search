use std::path::{Path, PathBuf};

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

pub(crate) type TestResult = Result<(), Box<dyn std::error::Error>>;

fn fixture_agy() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake_agy.py")
}

pub(crate) fn command(trace: &Path) -> Command {
    let curl = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake_curl.py");
    let mut command = Command::new(env!("CARGO_BIN_EXE_agy-search"));
    command
        .arg("--agy-path")
        .arg(fixture_agy())
        .env("AGY_SEARCH_FIXTURE_TRACE", trace)
        .env("AGY_SEARCH_CURL_PATH", curl);
    command
}

pub(crate) fn assert_semantic_invalid(arguments: &[&str]) -> TestResult {
    let temporary = TempDir::new()?;
    let trace = temporary.path().join("invalid.jsonl");
    command(&trace)
        .args(arguments)
        .assert()
        .code(2)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::eq("error: invalid agy command\n"));
    assert!(!trace.exists());
    Ok(())
}
