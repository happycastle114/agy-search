#![allow(
    dead_code,
    reason = "shared helpers are compiled independently for each integration test binary"
)]

use std::path::{Path, PathBuf};

use assert_cmd::Command;
use serde::Deserialize;
use serde_json::Value;
use tempfile::TempDir;

pub(crate) fn command() -> Command {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = root.join("tests/fixtures/fake_agy.py");
    let curl = root.join("tests/fixtures/fake_curl.py");
    let mut command = Command::new(env!("CARGO_BIN_EXE_agy-search"));
    command
        .arg("--agy-path")
        .arg(fixture)
        .env("AGY_SEARCH_CURL_PATH", curl);
    command
}

#[derive(Clone, Copy)]
pub(crate) struct TemporalSearchFixture<'a> {
    pub(crate) scopes: [&'a str; 2],
    pub(crate) sources: &'a [&'a str],
    pub(crate) query: &'a str,
}

pub(crate) fn temporal_search(mut command: Command, fixture: TemporalSearchFixture<'_>) -> Command {
    command.args([
        "--verification",
        "temporal-comparison",
        "search",
        "--scope",
        fixture.scopes[0],
        "--scope",
        fixture.scopes[1],
    ]);
    for source in fixture.sources {
        command.args(["--source-url", source]);
    }
    command.arg(fixture.query);
    command
}

pub(crate) fn version_search(query: &str) -> Command {
    temporal_search(
        command(),
        TemporalSearchFixture {
            scopes: ["newer fixture", "older fixture"],
            sources: &["https://example.com/source"],
            query,
        },
    )
}

pub(crate) fn default_search(query: &str) -> Command {
    temporal_search(
        command(),
        TemporalSearchFixture {
            scopes: ["primary fixture", "corroborating fixture"],
            sources: &["https://example.com/source"],
            query,
        },
    )
}

pub(crate) fn recovery_search(command: Command, query: &str) -> Command {
    temporal_search(
        command,
        TemporalSearchFixture {
            scopes: ["alpha", "beta"],
            sources: &[
                "https://example.com/primary",
                "https://example.com/alpha",
                "https://example.com/beta",
            ],
            query,
        },
    )
}

pub(crate) fn local_recovery_search(command: Command, query: &str, source: &str) -> Command {
    temporal_search(
        command,
        TemporalSearchFixture {
            scopes: ["alpha", "beta"],
            sources: &[source],
            query,
        },
    )
}

pub(crate) fn traced_command(temporary: &TempDir) -> (Command, PathBuf) {
    let trace = temporary.path().join("invocations.jsonl");
    let mut command = command();
    command.env("AGY_SEARCH_FIXTURE_TRACE", &trace);
    (command, trace)
}

#[derive(Deserialize)]
pub(crate) struct TraceRecord {
    pub(crate) scope: Option<String>,
    pub(crate) query: Option<String>,
    pub(crate) required_search_query: Option<String>,
}

pub(crate) fn trace_scopes(
    trace: &Path,
) -> Result<Vec<Option<String>>, Box<dyn std::error::Error>> {
    std::fs::read_to_string(trace)?
        .lines()
        .map(|line| {
            serde_json::from_str::<TraceRecord>(line)
                .map(|record| record.scope)
                .map_err(Into::into)
        })
        .collect()
}

pub(crate) fn assert_recovery_trace(scopes: &[Option<String>]) {
    assert_eq!(scopes.first(), Some(&None));
    let mut recovered = scopes.iter().skip(1).cloned().collect::<Vec<_>>();
    recovered.sort();
    assert_eq!(
        recovered,
        [Some("alpha".to_owned()), Some("beta".to_owned())]
    );
}

pub(crate) fn source_trace_urls(trace: &Path) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    std::fs::read_to_string(trace)?
        .lines()
        .map(|line| {
            let record: Value = serde_json::from_str(line)?;
            record
                .pointer("/url")
                .and_then(Value::as_str)
                .map(str::to_owned)
                .ok_or_else(|| "source trace URL missing".into())
        })
        .collect()
}
