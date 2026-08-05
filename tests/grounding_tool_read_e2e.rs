//! Restricted grounding tool-read validation through the real CLI boundary.

use std::fs;

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::{Value, json};
use tempfile::TempDir;

const FAKE_AGY: &str = r#"#!/usr/bin/env python3
import json
import sys

if sys.argv[1:] == ["--version"]:
    print("9.9.9")
    raise SystemExit(0)
if sys.argv[1:] == ["models"]:
    print("fixture-model")
    raise SystemExit(0)

prompt = sys.argv[sys.argv.index("-p") + 1]
payload = json.loads(prompt.split("INPUT_JSON=", maxsplit=1)[1])
restriction = payload.get("source_restriction")
direct = restriction["urls"][0] if restriction else "https://example.com/canonical"
transport = "https://vertexaisearch.cloud.google.com/grounding-api-redirect/tool-read"
events = [
    {"event": "init", "conversation_id": "current-conversation"},
    {"event": "step_update", "step_update": {
        "conversation_id": "current-conversation", "state": "DONE",
        "step_type": "tool", "tool_info": {
            "name": "search_web", "parameters": {"query": "exact source evidence"}
        }
    }},
    {"event": "step_update", "step_update": {
        "conversation_id": "current-conversation", "state": "ACTIVE",
        "step_type": "tool", "tool_info": {
            "name": "read_url_content", "parameters": {"Url": transport}
        }
    }},
    {"event": "step_update", "step_update": {
        "conversation_id": "current-conversation", "state": "DONE",
        "step_type": "tool", "tool_info": {
            "name": "read_url_content", "parameters": {"Url": transport}
        }
    }},
    {"event": "result", "result": {"structured_output": {
        "object": "search",
        "evidence_audit": {
            "candidates": [{"scope": "primary", "claim": "Evidence", "url": direct,
                            "date": None}],
            "coverage_complete": True,
            "conclusion": "Evidence"
        },
        "results": [{"title": "Source", "url": direct, "snippet": "Evidence"}]
    }}}
]
for event in events:
    print(json.dumps(event, separators=(",", ":")))
"#;

const FAKE_CURL: &str = r#"#!/usr/bin/env python3
import json
import os
import sys
from urllib.parse import urlparse

arguments = sys.argv[1:]
url = arguments[arguments.index("--url") + 1]
with open(os.environ["AGY_TOOL_READ_TRACE"], "a", encoding="utf-8") as stream:
    stream.write(json.dumps({"url": url}) + "\n")
host = urlparse(url).hostname
if host == "vertexaisearch.cloud.google.com":
    final = "example.com" if os.environ["AGY_SEARCH_REDIRECT_FINAL"] == "allowed" else "iana.org"
    print("HTTP/1.1 302 Test\r")
    print(f"Location: https://{final}/canonical\r")
else:
    print("HTTP/1.1 200 Test\r")
print("\r")
print("\nAGY_REDIRECT_META:302:0" if host == "vertexaisearch.cloud.google.com" else
      "\nAGY_REDIRECT_META:200:0")
"#;

struct Fixtures {
    agy: std::path::PathBuf,
    curl: std::path::PathBuf,
    trace: std::path::PathBuf,
}

fn fixtures(temporary: &TempDir) -> Result<Fixtures, Box<dyn std::error::Error>> {
    let agy = temporary.path().join("agy");
    let curl = temporary.path().join("curl");
    fs::write(&agy, FAKE_AGY)?;
    fs::write(&curl, FAKE_CURL)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        fs::set_permissions(&agy, fs::Permissions::from_mode(0o700))?;
        fs::set_permissions(&curl, fs::Permissions::from_mode(0o700))?;
    }
    Ok(Fixtures {
        agy,
        curl,
        trace: temporary.path().join("redirects.jsonl"),
    })
}

fn command(fixtures: &Fixtures, final_mode: &str) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_agy-search"));
    command
        .arg("--agy-path")
        .arg(&fixtures.agy)
        .env("AGY_SEARCH_CURL_PATH", &fixtures.curl)
        .env("AGY_TOOL_READ_TRACE", &fixtures.trace)
        .env("AGY_SEARCH_REDIRECT_FINAL", final_mode);
    command
}

#[test]
fn grounding_tool_read_resolves_to_exact_allowed_source_before_exposure()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: a restricted exact source, a grounding tool read, and direct terminal output.
    let temporary = TempDir::new()?;
    let fixtures = fixtures(&temporary)?;

    // When: the tool transport resolves to the exact caller-owned source.
    let assertion = command(&fixtures, "allowed")
        .args([
            "search",
            "grounding tool read",
            "--source-url",
            "https://example.com/canonical",
        ])
        .assert()
        .success();

    // Then: only the already-direct structured URL is exposed.
    let response: Value = serde_json::from_slice(&assertion.get_output().stdout)?;
    assert_eq!(
        response.pointer("/results/0/url"),
        Some(&json!("https://example.com/canonical"))
    );
    assert_eq!(fs::read_to_string(&fixtures.trace)?.lines().count(), 2);
    Ok(())
}

#[test]
fn grounding_tool_read_rejects_a_final_url_outside_exact_source()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: the same restricted run whose hidden tool transport resolves elsewhere.
    let temporary = TempDir::new()?;
    let fixtures = fixtures(&temporary)?;

    // When/Then: the CLI fails closed before exposing the otherwise-valid terminal document.
    command(&fixtures, "disallowed")
        .args([
            "search",
            "grounding tool read",
            "--source-url",
            "https://example.com/canonical",
        ])
        .assert()
        .code(6)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::eq("error: agy output invalid\n"));
    Ok(())
}

#[test]
fn unrestricted_tool_read_does_not_trigger_redirect_resolution()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: an unrestricted run with a grounding tool read and direct terminal output.
    let temporary = TempDir::new()?;
    let fixtures = fixtures(&temporary)?;

    // When: the ordinary unrestricted search succeeds.
    command(&fixtures, "allowed")
        .args(["search", "grounding tool read"])
        .assert()
        .success();

    // Then: no redirect transport was invoked solely for the hidden tool read.
    assert!(!fixtures.trace.exists());
    Ok(())
}
