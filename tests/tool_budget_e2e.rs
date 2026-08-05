//! Public tool-budget contract against a deterministic Antigravity process.

use std::path::Path;

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn standard_search_rejects_more_than_two_completed_research_calls() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake_agy.py");
    let mut command = Command::new(env!("CARGO_BIN_EXE_agy-search"));

    command
        .arg("--agy-path")
        .arg(fixture)
        .args(["search", "too-many-tools"])
        .assert()
        .code(6)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::eq("error: agy output invalid\n"));
}

#[test]
fn standard_search_rejects_failed_extra_research_attempts() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake_agy.py");
    let mut command = Command::new(env!("CARGO_BIN_EXE_agy-search"));

    command
        .arg("--agy-path")
        .arg(fixture)
        .args(["search", "failed-extra-tools"])
        .assert()
        .code(6)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::eq("error: agy output invalid\n"));
}

#[test]
fn standard_search_rejects_unfinished_extra_research_attempts() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake_agy.py");
    let mut command = Command::new(env!("CARGO_BIN_EXE_agy-search"));

    command
        .arg("--agy-path")
        .arg(fixture)
        .args(["search", "unfinished-extra-tools"])
        .assert()
        .code(6)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::eq("error: agy output invalid\n"));
}

#[test]
fn standard_search_rejects_research_attempts_from_another_conversation() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake_agy.py");
    let mut command = Command::new(env!("CARGO_BIN_EXE_agy-search"));

    command
        .arg("--agy-path")
        .arg(fixture)
        .args(["search", "foreign-conversation-tools"])
        .assert()
        .code(6)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::eq("error: agy output invalid\n"));
}

#[test]
fn temporal_search_allows_discovery_and_exact_date_followups_per_scope() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = root.join("tests/fixtures/fake_agy.py");
    let curl = root.join("tests/fixtures/fake_curl.py");
    let mut command = Command::new(env!("CARGO_BIN_EXE_agy-search"));

    command
        .arg("--agy-path")
        .arg(fixture)
        .env("AGY_SEARCH_CURL_PATH", curl)
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
            "temporal-eight-tools",
        ])
        .assert()
        .success();
}

#[test]
fn synthesis_research_allows_five_attempts_with_four_sources() {
    // Given two searches and three reads for a four-source synthesis request.
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake_agy.py");
    let mut command = Command::new(env!("CARGO_BIN_EXE_agy-search"));

    // When/Then all five legitimate attempts complete below the derived budget of six.
    command
        .arg("--agy-path")
        .arg(fixture)
        .args(["research", "research-five-tools", "--max-sources", "4"])
        .assert()
        .success();
}

#[test]
fn synthesis_research_rejects_attempts_above_derived_budget() {
    // Given seven completed attempts for a four-source synthesis request.
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake_agy.py");
    let mut command = Command::new(env!("CARGO_BIN_EXE_agy-search"));

    // When/Then the seventh attempt exceeds the derived budget of six and fails closed.
    command
        .arg("--agy-path")
        .arg(fixture)
        .args(["research", "research-seven-tools", "--max-sources", "4"])
        .assert()
        .code(6)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::eq("error: agy output invalid\n"));
}

#[test]
fn deep_research_accepts_exact_ten_attempt_boundary() {
    // Given ten completed attempts for an eight-source deep request.
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake_agy.py");
    let mut command = Command::new(env!("CARGO_BIN_EXE_agy-search"));

    // When/Then the exact derived budget boundary remains valid.
    command
        .arg("--agy-path")
        .arg(fixture)
        .args(["research", "research-ten-tools", "--max-sources", "8"])
        .assert()
        .success();
}

#[test]
fn deep_research_rejects_attempts_above_ten_attempt_boundary() {
    // Given eleven completed attempts for an eight-source deep request.
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake_agy.py");
    let mut command = Command::new(env!("CARGO_BIN_EXE_agy-search"));

    // When/Then the attempt beyond the derived budget fails closed.
    command
        .arg("--agy-path")
        .arg(fixture)
        .args(["research", "research-eleven-tools", "--max-sources", "8"])
        .assert()
        .code(6)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::eq("error: agy output invalid\n"));
}

#[test]
fn dynamic_research_budget_still_rejects_failed_attempts() {
    // Given a Research run containing failed web attempts.
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake_agy.py");
    let mut command = Command::new(env!("CARGO_BIN_EXE_agy-search"));

    // When/Then failed lifecycle events remain invalid regardless of available budget.
    command
        .arg("--agy-path")
        .arg(fixture)
        .args(["research", "failed-extra-tools", "--max-sources", "8"])
        .assert()
        .code(6)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::eq("error: agy output invalid\n"));
}

#[test]
fn dynamic_research_budget_still_rejects_unfinished_attempts() {
    // Given a Research run containing unfinished web attempts.
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake_agy.py");
    let mut command = Command::new(env!("CARGO_BIN_EXE_agy-search"));

    // When/Then unfinished lifecycle events remain invalid regardless of available budget.
    command
        .arg("--agy-path")
        .arg(fixture)
        .args(["research", "unfinished-extra-tools", "--max-sources", "8"])
        .assert()
        .code(6)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::eq("error: agy output invalid\n"));
}

#[test]
fn dynamic_research_budget_still_rejects_foreign_conversation_attempts() {
    // Given a Research run whose web attempt belongs to another conversation.
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake_agy.py");
    let mut command = Command::new(env!("CARGO_BIN_EXE_agy-search"));

    // When/Then conversation provenance remains fail-closed regardless of available budget.
    command
        .arg("--agy-path")
        .arg(fixture)
        .args([
            "research",
            "foreign-conversation-tools",
            "--max-sources",
            "8",
        ])
        .assert()
        .code(6)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::eq("error: agy output invalid\n"));
}
