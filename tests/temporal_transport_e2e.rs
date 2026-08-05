//! Temporal source transport error contracts against deterministic child output.

mod common;

use common::{
    TemporalSearchFixture, source_trace_urls, temporal_search, trace_scopes, traced_command,
};
use predicates::prelude::*;
use tempfile::TempDir;

#[derive(Clone, Copy)]
enum SourceTransportFailure {
    Failed,
    Invalid,
}

impl SourceTransportFailure {
    const fn url(self) -> &'static str {
        match self {
            Self::Failed => "https://example.com/failed",
            Self::Invalid => "https://example.com/invalid",
        }
    }
}

#[test]
fn temporal_comparison_maps_source_fetch_deadline_to_public_timeout_exit()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: a valid primary audit and one caller source whose transport exceeds
    // the one-second request deadline.
    let temporary = TempDir::new()?;
    let (command, agy_trace) = traced_command(&temporary);
    let source_trace = temporary.path().join("source-timeout.jsonl");
    let sources = ["https://example.com/primary", "https://example.com/timeout"];
    let mut command = command;
    command.args(["--timeout", "1"]);
    let mut command = temporal_search(
        command,
        TemporalSearchFixture {
            scopes: ["alpha", "beta"],
            sources: &sources,
            query: "temporal-recoverable",
        },
    );

    // When/Then: the VerifiedSources boundary maps the transport deadline to exit 4.
    command
        .env("AGY_SEARCH_SOURCE_FETCH_TRACE", &source_trace)
        .assert()
        .code(4)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::eq("error: agy timed out\n"));
    assert_eq!(trace_scopes(&agy_trace)?, vec![None]);
    assert_eq!(source_trace_urls(&source_trace)?.len(), 2);
    Ok(())
}

#[test]
fn temporal_comparison_maps_failed_or_invalid_source_transport_to_exit_six()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: valid primary evidence plus a source transport that either exits
    // non-zero or returns a malformed status/body envelope.
    for source_path in [
        SourceTransportFailure::Failed,
        SourceTransportFailure::Invalid,
    ] {
        let temporary = TempDir::new()?;
        let (command, agy_trace) = traced_command(&temporary);
        let source_trace = temporary.path().join("source-error.jsonl");
        let sources = ["https://example.com/primary", source_path.url()];
        let mut command = temporal_search(
            command,
            TemporalSearchFixture {
                scopes: ["alpha", "beta"],
                sources: &sources,
                query: "temporal-recoverable",
            },
        );

        // When/Then: source process errors are sanitized as output-invalid exit 6.
        command
            .env("AGY_SEARCH_SOURCE_FETCH_TRACE", &source_trace)
            .assert()
            .code(6)
            .stdout(predicate::str::is_empty())
            .stderr(predicate::eq("error: agy output invalid\n"));
        assert_eq!(trace_scopes(&agy_trace)?, vec![None]);
        assert_eq!(source_trace_urls(&source_trace)?.len(), 2);
    }
    Ok(())
}
