//! Standard-mode public date provenance contracts at the CLI boundary.

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::{Value, json};
use std::path::Path;

fn command() -> Command {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake_agy.py");
    let mut command = Command::new(env!("CARGO_BIN_EXE_agy-search"));
    command.arg("--agy-path").arg(fixture);
    command
}

fn assert_invalid(operation: &str, query: &str) {
    command()
        .args([operation, query])
        .assert()
        .code(6)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::eq("error: agy output invalid\n"));
}

fn public_source(operation: &str, query: &str) -> Value {
    let assertion = command().args([operation, query]).assert().success();
    let response: Value =
        serde_json::from_slice(&assertion.get_output().stdout).unwrap_or(Value::Null);
    response
        .pointer(if operation == "search" {
            "/results/0"
        } else {
            "/sources/0"
        })
        .cloned()
        .unwrap_or(Value::Null)
}

#[test]
fn standard_search_downgrades_an_unbound_day_to_null() {
    // Given: a standard Search result whose public day is absent from its audit text.
    // When: the otherwise valid response crosses the public CLI boundary.
    let source = public_source("search", "standard-date-month-only");

    // Then: the result survives without exposing an invented exact day.
    assert_eq!(source.get("date"), Some(&Value::Null));
}

#[test]
fn standard_research_rejects_an_invented_day_bound_to_month_only_source_text() {
    // Given: a standard Research source whose public day is absent from its audit text.
    // When/Then: the public CLI rejects the response with its stable output-invalid code.
    assert_invalid("research", "standard-date-month-only");
}

#[test]
fn standard_search_rejects_non_iso_public_date() {
    // Given: a standard Search result whose public date is not ISO CalendarDate syntax.
    // When/Then: the public CLI rejects the response with its stable output-invalid code.
    assert_invalid("search", "standard-malformed-date");
}

#[test]
fn standard_research_rejects_non_iso_public_date() {
    // Given: a standard Research source whose public date is not ISO CalendarDate syntax.
    // When/Then: the public CLI rejects the response with its stable output-invalid code.
    assert_invalid("research", "standard-malformed-date");
}

#[test]
fn standard_search_rejects_malformed_public_last_updated() {
    // Given: a standard Search result with a non-calendar update date.
    // When/Then: the public CLI rejects the response with its stable output-invalid code.
    assert_invalid("search", "standard-malformed-update");
}

#[test]
fn standard_research_rejects_malformed_public_last_updated() {
    // Given: a standard Research source with a non-calendar update date.
    // When/Then: the public CLI rejects the response with its stable output-invalid code.
    assert_invalid("research", "standard-malformed-update");
}

#[test]
fn standard_search_accepts_exact_iso_source_date_text() {
    // Given: an ISO public date bound to the exact same ISO text in its audit evidence.
    // When: the standard Search response crosses the public CLI boundary.
    let source = public_source("search", "standard-date-iso");

    // Then: the exact public date is preserved.
    assert_eq!(source.get("date"), Some(&json!("2013-02-20")));
}

#[test]
fn standard_research_accepts_exact_iso_source_date_text() {
    // Given: an ISO public date bound to the exact same ISO text in its audit evidence.
    // When: the standard Research response crosses the public CLI boundary.
    let source = public_source("research", "standard-date-iso");

    // Then: the exact public date is preserved.
    assert_eq!(source.get("date"), Some(&json!("2013-02-20")));
}

#[test]
fn standard_search_accepts_unambiguous_english_source_date_text() {
    // Given: an RFC-era public date bound to an exact English full date in its audit.
    // When: the standard Search response crosses the public CLI boundary.
    let source = public_source("search", "standard-date-english");

    // Then: the ISO public date is preserved.
    assert_eq!(source.get("date"), Some(&json!("1999-06-01")));
}

#[test]
fn standard_research_accepts_unambiguous_english_source_date_text() {
    // Given: an RFC-era public date bound to an exact English full date in its audit.
    // When: the standard Research response crosses the public CLI boundary.
    let source = public_source("research", "standard-date-english");

    // Then: the ISO public date is preserved.
    assert_eq!(source.get("date"), Some(&json!("1999-06-01")));
}

#[test]
fn standard_search_accepts_unambiguous_korean_source_date_text() {
    // Given: an ISO public date bound to an exact Korean full date in its audit.
    // When: the standard Search response crosses the public CLI boundary.
    let source = public_source("search", "standard-date-korean");

    // Then: the ISO public date is preserved.
    assert_eq!(source.get("date"), Some(&json!("2026-08-06")));
}

#[test]
fn standard_search_accepts_null_date_metadata() {
    // Given: a standard Search source with no asserted date metadata.
    // When: the response crosses the public CLI boundary.
    let source = public_source("search", "standard-date-null");

    // Then: both nullable metadata fields remain null.
    assert_eq!(source.get("date"), Some(&Value::Null));
    assert_eq!(source.get("last_updated"), Some(&Value::Null));
}

#[test]
fn standard_research_accepts_null_date_metadata() {
    // Given: a standard Research source with no asserted date metadata.
    // When: the response crosses the public CLI boundary.
    let source = public_source("research", "standard-date-null");

    // Then: both nullable metadata fields remain null.
    assert_eq!(source.get("date"), Some(&Value::Null));
    assert_eq!(source.get("last_updated"), Some(&Value::Null));
}
