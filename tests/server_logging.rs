#![cfg(feature = "server")]

//! SDK payloads must not enter diagnostics, even under verbose user filters.

use std::{process::Stdio, time::Duration};

use serde_json::{Value, json};
use tempfile::NamedTempFile;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::Command,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const QUERY_CANARY: &str = "PRIVATE_QUERY_LOG_CANARY";
const CLIENT_CANARY: &str = "PRIVATE_CLIENT_LOG_CANARY";

#[tokio::test]
async fn verbose_filters_preserve_sanitized_logs_without_mcp_payloads() -> TestResult {
    tokio::time::timeout(Duration::from_secs(15), exercise_verbose_logging()).await?
}

async fn exercise_verbose_logging() -> TestResult {
    // Given: user logging directives explicitly enable nested SDK trace events.
    let log = NamedTempFile::new()?;
    let temporary = tempfile::tempdir()?;
    let absent = temporary.path().join("missing-agy");
    let mut child = Command::new(env!("CARGO_BIN_EXE_agy-search-server"))
        .arg("--agy-path")
        .arg(absent)
        .arg("stdio")
        .env("RUST_LOG", "trace,rmcp::service=trace")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(log.reopen()?)
        .kill_on_drop(true)
        .spawn()?;
    let mut stdin = child.stdin.take().ok_or("server stdin missing")?;
    let mut stdout = BufReader::new(child.stdout.take().ok_or("server stdout missing")?).lines();
    let initialize = json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18", "capabilities": {},
            "clientInfo": {"name": CLIENT_CANARY, "version": "1"}
        }
    });
    stdin
        .write_all(format!("{initialize}\n").as_bytes())
        .await?;
    let initialized: Value =
        serde_json::from_str(&stdout.next_line().await?.ok_or("initialization missing")?)?;
    assert_eq!(initialized.get("id"), Some(&json!(1)));
    assert!(initialized.get("result").is_some());

    // When: a request with private text produces a sanitized backend failure.
    let notification = json!({"jsonrpc": "2.0", "method": "notifications/initialized"});
    let call = json!({
        "jsonrpc": "2.0", "id": 2, "method": "tools/call",
        "params": {"name": "agy_search", "arguments": {"query": QUERY_CANARY}}
    });
    stdin
        .write_all(format!("{notification}\n{call}\n").as_bytes())
        .await?;
    let response: Value =
        serde_json::from_str(&stdout.next_line().await?.ok_or("tool response missing")?)?;
    assert_eq!(response.get("id"), Some(&json!(2)));
    assert_eq!(response.pointer("/result/isError"), Some(&json!(true)));
    drop(stdin);
    assert!(child.wait().await?.success());

    // Then: application diagnostics remain, but SDK request/client payloads do not.
    let diagnostics = std::fs::read_to_string(log.path())?;
    assert!(diagnostics.contains("MCP runtime request failed"));
    assert!(!diagnostics.contains(QUERY_CANARY));
    assert!(!diagnostics.contains(CLIENT_CANARY));
    Ok(())
}
