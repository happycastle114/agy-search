//! Temporal source and mode constraints at the public CLI boundary.

use predicates::prelude::*;
use tempfile::TempDir;

mod support;

use support::{TestResult, assert_semantic_invalid, command};

#[test]
fn temporal_contract_rejects_invalid_source_sets_before_agy() -> TestResult {
    for arguments in [
        &[
            "--verification",
            "temporal-comparison",
            "research",
            "--scope",
            "alpha",
            "--scope",
            "beta",
            "q",
        ][..],
        &[
            "--verification",
            "temporal-comparison",
            "research",
            "--scope",
            "alpha",
            "--scope",
            "beta",
            "--source-url",
            "http://example.com/releases",
            "q",
        ][..],
        &[
            "--verification",
            "temporal-comparison",
            "research",
            "--scope",
            "alpha",
            "--scope",
            "beta",
            "--source-url",
            "https://example.com/releases#one",
            "--source-url",
            "https://example.com/releases#two",
            "q",
        ][..],
    ] {
        assert_semantic_invalid(arguments)?;
    }

    let temporary = TempDir::new()?;
    let trace = temporary.path().join("too-many-sources.jsonl");
    let mut command = command(&trace);
    command.args([
        "--verification",
        "temporal-comparison",
        "research",
        "--scope",
        "alpha",
        "--scope",
        "beta",
    ]);
    for index in 0..9 {
        command.args([
            "--source-url",
            &format!("https://source-{index}.example/releases"),
        ]);
    }
    command
        .arg("q")
        .assert()
        .code(2)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::eq("error: invalid agy command\n"));
    assert!(!trace.exists());
    Ok(())
}

#[test]
fn standard_mode_rejects_temporal_only_flags_before_agy() -> TestResult {
    assert_semantic_invalid(&[
        "search",
        "--scope",
        "alpha",
        "--scope",
        "beta",
        "--source-url",
        "https://example.com/releases",
        "q",
    ])
}

#[test]
fn standard_mode_rejects_as_of_before_agy() -> TestResult {
    assert_semantic_invalid(&["search", "--as-of", "2026-08-05", "q"])
}
