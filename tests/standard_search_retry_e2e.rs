//! Bounded standard Search recovery at the public CLI boundary.

mod common;

use std::fs;

use common::traced_command;
use predicates::prelude::*;
use serde_json::Value;

fn invocation_count(path: &std::path::Path) -> Result<usize, Box<dyn std::error::Error>> {
    Ok(fs::read_to_string(path)?.lines().count())
}

#[test]
fn does_not_retry_when_standard_search_audit_coverage_is_missing()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: the first complete tool run omits the second public URL from its audit.
    let temporary = tempfile::tempdir()?;
    let (mut command, trace) = traced_command(&temporary);

    // When: the standard Search crosses the CLI boundary.
    command
        .args(["search", "standard-audit-retry"])
        .assert()
        .code(6)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::eq("error: agy output invalid\n"));

    // Then: audit/model correction is not attempted by the latency-sensitive CLI path.
    assert_eq!(invocation_count(&trace)?, 1);
    Ok(())
}

#[test]
fn does_not_retry_when_the_first_structured_output_cannot_be_parsed()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: the first complete tool run returns an empty URL rejected by the typed boundary.
    let temporary = tempfile::tempdir()?;
    let (mut command, trace) = traced_command(&temporary);

    // When: a valid second run is available within the shared deadline.
    command
        .args(["search", "standard-output-retry"])
        .assert()
        .code(6)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::eq("error: agy output invalid\n"));

    // Then: malformed structured output fails once instead of paying for another model call.
    assert_eq!(invocation_count(&trace)?, 1);
    Ok(())
}

#[test]
fn does_not_retry_when_first_standard_search_is_valid() -> Result<(), Box<dyn std::error::Error>> {
    // Given: the first complete tool run audits both public URLs.
    let temporary = tempfile::tempdir()?;
    let (mut command, trace) = traced_command(&temporary);

    // When: the valid Search crosses the CLI boundary.
    command
        .args(["search", "standard-audit-first-valid"])
        .assert()
        .success();

    // Then: the fast path uses exactly one Antigravity invocation.
    assert_eq!(invocation_count(&trace)?, 1);
    Ok(())
}

#[test]
fn does_not_retry_when_audit_coverage_stays_missing() -> Result<(), Box<dyn std::error::Error>> {
    // Given: both the primary run and its one retry omit a public audit URL.
    let temporary = tempfile::tempdir()?;
    let (mut command, trace) = traced_command(&temporary);

    // When/Then: the CLI remains fail-closed without retrying a provenance defect.
    command
        .args(["search", "standard-audit-missing-twice"])
        .assert()
        .code(6)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::eq("error: agy output invalid\n"));
    assert_eq!(invocation_count(&trace)?, 1);
    Ok(())
}

#[test]
fn retries_once_after_an_unlisted_tool_invalidates_the_primary_run()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: the primary model run executes a tool outside the custom-agent contract.
    let temporary = tempfile::tempdir()?;
    let (mut command, trace) = traced_command(&temporary);

    // When: the bounded retry returns evidence using only search_web.
    command
        .args(["search", "standard-unlisted-tool-retry"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());

    // Then: the unsafe primary output is discarded and exactly one retry is used.
    assert_eq!(invocation_count(&trace)?, 2);
    Ok(())
}

#[test]
fn does_not_mask_an_invalid_audit_with_an_unlisted_tool() -> Result<(), Box<dyn std::error::Error>>
{
    // Given: one run contains both a foreign tool and incomplete same-URL audit coverage.
    let temporary = tempfile::tempdir()?;
    let (mut command, trace) = traced_command(&temporary);

    // When/Then: the provenance defect wins and no model retry is attempted.
    command
        .args(["search", "standard-unlisted-tool-invalid-audit"])
        .assert()
        .code(6)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::eq("error: agy output invalid\n"));
    assert_eq!(invocation_count(&trace)?, 1);
    Ok(())
}

#[test]
fn does_not_retry_a_non_recoverable_document_validation_failure()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: a parsed single-source response has malformed public update-date syntax.
    let temporary = tempfile::tempdir()?;
    let (mut command, trace) = traced_command(&temporary);

    // When/Then: the unrelated failure is rejected without another model call.
    command
        .args(["search", "standard-malformed-update"])
        .assert()
        .code(6)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::eq("error: agy output invalid\n"));
    assert_eq!(invocation_count(&trace)?, 1);
    Ok(())
}

#[test]
fn downgrades_an_unbound_standard_date_without_retrying() -> Result<(), Box<dyn std::error::Error>>
{
    // Given: one otherwise valid Search result asserts a day absent from its source text.
    let temporary = tempfile::tempdir()?;
    let (mut command, trace) = traced_command(&temporary);

    // When: the standard Search crosses the CLI boundary.
    let assertion = command
        .args(["search", "standard-date-month-only"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
    let response: Value = serde_json::from_slice(&assertion.get_output().stdout)?;

    // Then: only the unverified date is removed and no second model call is made.
    assert_eq!(response.pointer("/results/0/date"), Some(&Value::Null));
    assert_eq!(invocation_count(&trace)?, 1);
    Ok(())
}

#[test]
fn discards_a_google_search_page_and_its_unbound_date_before_retrying()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: the first run returns only a Google search page with unsupported date metadata.
    let temporary = tempfile::tempdir()?;
    let (mut command, trace) = traced_command(&temporary);

    // When: the bounded no-reachable-result retry returns a direct source.
    let assertion = command
        .args(["search", "standard-non-source-first"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
    let response: Value = serde_json::from_slice(&assertion.get_output().stdout)?;

    // Then: no rejected search-page URL or unsupported date reaches public JSON.
    assert_eq!(
        response.pointer("/results/0/url").and_then(Value::as_str),
        Some("https://example.com/direct-market-source")
    );
    assert_eq!(response.pointer("/results/0/date"), Some(&Value::Null));
    assert_eq!(invocation_count(&trace)?, 2);
    Ok(())
}
