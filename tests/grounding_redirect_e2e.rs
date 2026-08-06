//! Direct-source normalization contract against deterministic child processes.

use std::{
    fs,
    path::{Path, PathBuf},
};

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::{Value, json};

const FAKE_CURL: &str = include_str!("fixtures/fake_grounding_redirect_curl.py");

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn fake_curl() -> Result<(tempfile::TempDir, PathBuf), Box<dyn std::error::Error>> {
    let temporary = tempfile::tempdir()?;
    let executable = temporary.path().join("curl");
    fs::write(&executable, FAKE_CURL)?;
    fs::write(
        temporary.path().join(".curlrc"),
        "--location\n--proxy http://127.0.0.1:9\n--cookie stolen\n",
    )?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o700))?;
    }
    Ok((temporary, executable))
}

fn command(curl: &Path, trace: &Path, mode: &str) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_agy-search"));
    command
        .arg("--agy-path")
        .arg(fixture("fake_agy.py"))
        .env("AGY_SEARCH_CURL_PATH", curl)
        .env("AGY_REDIRECT_MODE", mode)
        .env("AGY_REDIRECT_TRACE", trace)
        .env("HTTPS_PROXY", "http://127.0.0.1:9")
        .env("ALL_PROXY", "http://127.0.0.1:9")
        .env("CURL_HOME", curl.parent().unwrap_or_else(|| Path::new("/")));
    command
}

fn trace_records(path: &Path) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
    Ok(fs::read_to_string(path)?
        .lines()
        .map(serde_json::from_str)
        .collect::<Result<Vec<_>, _>>()?)
}

#[test]
fn resolves_real_style_relative_and_query_redirect_chain_with_hardened_pinned_hops()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: a Google-style transport chain with a relative hop and query strings.
    let (_temporary, curl) = fake_curl()?;
    let trace = curl.with_extension("trace");

    // When: the CLI normalizes the grounding transport URL.
    let assertion = command(&curl, &trace, "success")
        .args(["search", "grounding-redirect"])
        .assert()
        .success();
    let search: Value = serde_json::from_slice(&assertion.get_output().stdout)?;

    // Then: the unique final direct URL is returned and every hop was explicit.
    assert_eq!(
        search.pointer("/results/0/url"),
        Some(&json!("https://example.com/canonical?source=grounding"))
    );
    let records = trace_records(&trace)?;
    assert_eq!(records.len(), 3);
    assert!(records.iter().all(|record| {
        record["argv"].as_array().is_some_and(|argv| {
            argv.first() == Some(&json!("--disable"))
                && !argv.contains(&json!("--location"))
                && argv.contains(&json!("--head"))
                && !argv.contains(&json!("--max-filesize"))
                && argv
                    .windows(2)
                    .any(|pair| pair == [json!("--noproxy"), json!("*")])
                && argv
                    .windows(2)
                    .any(|pair| pair == [json!("--proxy"), json!("")])
                && argv.iter().any(|value| value == "--resolve")
        })
    }));
    Ok(())
}

#[test]
fn resolves_large_final_content_length_with_header_only_requests()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: a redirect chain whose direct destination advertises a 1.86 MB body.
    let (_temporary, curl) = fake_curl()?;
    let trace = curl.with_extension("trace");

    // When: the CLI normalizes the grounding transport URL.
    let assertion = command(&curl, &trace, "large-final-body")
        .args(["search", "grounding-redirect"])
        .assert()
        .success();
    let search: Value = serde_json::from_slice(&assertion.get_output().stdout)?;

    // Then: each hop is a header-only request, so the final Content-Length cannot trip a body cap.
    assert_eq!(
        search.pointer("/results/0/url"),
        Some(&json!("https://example.com/canonical?source=grounding"))
    );
    let records = trace_records(&trace)?;
    assert_eq!(records.len(), 3);
    assert!(records.iter().all(|record| {
        record["argv"].as_array().is_some_and(|argv| {
            argv.contains(&json!("--head")) && !argv.contains(&json!("--max-filesize"))
        })
    }));
    Ok(())
}

#[test]
fn rejects_nonzero_redirect_transport_as_invalid_output() -> Result<(), Box<dyn std::error::Error>>
{
    // Given: a resolver transport that exits with a curl-style transport failure status.
    let (_temporary, curl) = fake_curl()?;
    let trace = curl.with_extension("trace");

    // When/Then: transport failure cannot be reported as an Antigravity process failure.
    command(&curl, &trace, "transport-failure")
        .args(["search", "grounding-redirect"])
        .assert()
        .code(6)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::eq("error: agy output invalid\n"));
    Ok(())
}

#[test]
fn rejects_unsafe_redirect_targets_before_a_second_request()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: redirect targets covering private IPs and unsafe URL authorities.
    for mode in [
        "private-v4",
        "private-v6",
        "link-local-v4",
        "link-local-v6",
        "multicast-v4",
        "multicast-v6",
        "documentation-v4",
        "documentation-v6",
        "reserved-v4",
        "special-v4",
        "six-to-four-v6",
        "future-documentation-v6",
        "userinfo",
        "port",
        "fragment",
        "http",
    ] {
        let (_temporary, curl) = fake_curl()?;
        let trace = curl.with_extension("trace");

        // When: the CLI receives the unsafe Location value.
        command(&curl, &trace, mode)
            .args(["search", "grounding-redirect"])
            .assert()
            .code(6)
            .stdout(predicate::str::is_empty())
            .stderr(predicate::eq("error: agy output invalid\n"));

        // Then: only the already-validated Google hop was requested.
        assert_eq!(trace_records(&trace)?.len(), 1, "unsafe mode: {mode}");
    }
    Ok(())
}

#[test]
fn rejects_loops_hop_overflow_and_malformed_multiple_or_oversize_headers()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: ambiguous redirect-chain response modes.
    for mode in [
        "loop",
        "too-many-hops",
        "malformed-location",
        "malformed-status",
        "multiple-location",
        "redirect-without-location",
        "success-with-location",
        "oversize-header",
    ] {
        let (_temporary, curl) = fake_curl()?;
        let trace = curl.with_extension("trace");

        // When/Then: normalization fails closed with the stable CLI error.
        command(&curl, &trace, mode)
            .args(["search", "grounding-redirect"])
            .assert()
            .code(6)
            .stdout(predicate::str::is_empty())
            .stderr(predicate::eq("error: agy output invalid\n"));
    }
    Ok(())
}
