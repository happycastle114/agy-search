//! Public discovery contract for the standalone Rust binary.

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn exposes_research_commands_when_help_is_requested() {
    // Given: the compiled standalone Rust binary
    let mut command = Command::new(env!("CARGO_BIN_EXE_agy-search"));

    // When: root help is requested
    let assertion = command.arg("--help").assert();

    // Then: every public research operation is directly discoverable
    assertion.success().stdout(
        predicate::str::contains("status")
            .and(predicate::str::contains("models"))
            .and(predicate::str::contains("search"))
            .and(predicate::str::contains("extract"))
            .and(predicate::str::contains("map"))
            .and(predicate::str::contains("crawl"))
            .and(predicate::str::contains("research")),
    );
}

#[test]
fn reports_package_version_without_downstream_agy() {
    // Given: the compiled standalone Rust binary
    let mut command = Command::new(env!("CARGO_BIN_EXE_agy-search"));

    // When: the eager version flag is requested
    let assertion = command.arg("--version").assert();

    // Then: version discovery succeeds without launching Antigravity
    assertion
        .success()
        .stdout(predicate::eq("agy-search 0.2.6\n"));
}
