use super::*;

fn assert_trace(
    trace: &Path,
    expected_content: &[(&str, &str)],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut expected = vec![
        serde_json::json!({"kind":"version","model":null,"effort":null}),
        serde_json::json!({"kind":"models","model":null,"effort":null}),
    ];
    expected.extend(expected_content.iter().map(
        |(model, effort)| serde_json::json!({"kind":"content","model":model,"effort":effort}),
    ));
    assert_eq!(trace_records(trace)?, expected);
    Ok(())
}

#[test]
fn missing_medium_uses_high_once() -> Result<(), Box<dyn std::error::Error>> {
    let temporary = TempDir::new()?;
    let trace = temporary.path().join("invocations.jsonl");

    command(&trace)
        .env("AGY_SEARCH_CATALOG_MODE", "without-medium")
        .env("AGY_SEARCH_CATALOG_CONTENT_MODE", "retry-twice")
        .args(["search", "partial catalog"])
        .assert()
        .code(6);

    assert_trace(
        &trace,
        &[(PREFERRED_MODEL, "low"), (FINAL_RETRY_MODEL, "high")],
    )
}

#[test]
fn missing_high_uses_medium_once() -> Result<(), Box<dyn std::error::Error>> {
    let temporary = TempDir::new()?;
    let trace = temporary.path().join("invocations.jsonl");

    command(&trace)
        .env("AGY_SEARCH_CATALOG_MODE", "without-high")
        .env("AGY_SEARCH_CATALOG_CONTENT_MODE", "retry-twice")
        .args(["search", "partial catalog"])
        .assert()
        .code(6);

    assert_trace(
        &trace,
        &[(PREFERRED_MODEL, "low"), (FIRST_RETRY_MODEL, "medium")],
    )
}

#[test]
fn low_only_has_no_recovery() -> Result<(), Box<dyn std::error::Error>> {
    let temporary = TempDir::new()?;
    let trace = temporary.path().join("invocations.jsonl");

    command(&trace)
        .env("AGY_SEARCH_CATALOG_MODE", "low-only")
        .env("AGY_SEARCH_CATALOG_CONTENT_MODE", "retry")
        .args(["search", "partial catalog"])
        .assert()
        .code(6);

    assert_trace(&trace, &[(PREFERRED_MODEL, "low")])
}

#[test]
fn explicit_model_keeps_two_bounded_recoveries() -> Result<(), Box<dyn std::error::Error>> {
    let temporary = TempDir::new()?;
    let trace = temporary.path().join("invocations.jsonl");

    command(&trace)
        .env("AGY_SEARCH_CATALOG_CONTENT_MODE", "retry-twice")
        .args(["--model", "fixture-model", "search", "explicit retry"])
        .assert()
        .success();

    assert_trace(
        &trace,
        &[
            ("fixture-model", "low"),
            ("fixture-model", "low"),
            ("fixture-model", "low"),
        ],
    )
}

#[test]
fn provider_default_keeps_two_bounded_recoveries() -> Result<(), Box<dyn std::error::Error>> {
    let temporary = TempDir::new()?;
    let trace = temporary.path().join("invocations.jsonl");

    command(&trace)
        .env("AGY_SEARCH_CATALOG_CONTENT_MODE", "retry-twice")
        .args(["--effort", "medium", "search", "default retry"])
        .assert()
        .success();

    let expected = vec![
        serde_json::json!({"kind":"version","model":null,"effort":null}),
        serde_json::json!({"kind":"content","model":null,"effort":"medium"}),
        serde_json::json!({"kind":"content","model":null,"effort":"medium"}),
        serde_json::json!({"kind":"content","model":null,"effort":"medium"}),
    ];
    assert_eq!(trace_records(&trace)?, expected);
    Ok(())
}
