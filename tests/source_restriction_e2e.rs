//! End-to-end caller-owned source restriction scenarios.

use std::path::{Path, PathBuf};

use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

fn fixture_agy() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake_agy.py")
}

fn command() -> Command {
    let curl = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake_curl.py");
    let mut command = Command::new(env!("CARGO_BIN_EXE_agy-search"));
    command
        .arg("--agy-path")
        .arg(fixture_agy())
        .env("AGY_SEARCH_CURL_PATH", curl);
    command
}

#[test]
fn canonical_domain_tree_accepts_subdomains_and_rejects_suffix_lookalikes() {
    // Given: one caller-owned canonical domain tree.
    // When/Then: an in-tree source succeeds, while a suffix lookalike fails closed.
    command()
        .args([
            "search",
            "source-domain-subdomain",
            "--domain",
            "RUST-LANG.ORG.",
        ])
        .assert()
        .success();
    command()
        .args([
            "search",
            "source-domain-lookalike",
            "--domain",
            "rust-lang.org",
        ])
        .assert()
        .code(6);
}

#[test]
fn research_rejects_one_disallowed_public_citation_or_hidden_candidate() {
    // Given: restricted Research responses with one disallowed URL at each trust surface.
    // When/Then: validation is all-or-nothing for sources, citations, and hidden audit candidates.
    for query in [
        "source-research-public-mixed",
        "source-research-citation-mixed",
        "source-research-audit-mixed",
    ] {
        command()
            .args(["research", query, "--domain", "rust-lang.org"])
            .assert()
            .code(6);
    }
}

#[test]
fn restricted_search_rejects_contributor_and_mutated_site_queries() {
    for query in [
        "source-contributor-blog",
        "source-search-missing-site",
        "source-search-mutated-site",
    ] {
        command()
            .args(["search", query, "--domain", "rust-lang.org"])
            .assert()
            .code(6);
    }
}

#[test]
fn exact_source_url_is_an_output_allowlist_without_body_fetching()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: a standard Search exact URL restriction and a curl trace sentinel.
    let temporary = TempDir::new()?;
    let curl_trace = temporary.path().join("curl.jsonl");

    // When: the returned URL exactly equals the caller-owned canonical URL.
    let assertion = command()
        .env("AGY_SEARCH_SOURCE_FETCH_TRACE", &curl_trace)
        .args([
            "search",
            "source-exact-url",
            "--source-url",
            "https://doc.rust-lang.org/book/",
        ])
        .assert()
        .success();

    // Then: the public result succeeds after terminal validation without fetching a body.
    let output: Value = serde_json::from_slice(&assertion.get_output().stdout)?;
    assert_eq!(
        output.pointer("/results/0/url").and_then(Value::as_str),
        Some("https://doc.rust-lang.org/book/")
    );
    assert!(!curl_trace.exists());
    Ok(())
}

#[test]
fn standard_research_accepts_an_exact_source_url_without_fetching()
-> Result<(), Box<dyn std::error::Error>> {
    let temporary = TempDir::new()?;
    let curl_trace = temporary.path().join("curl.jsonl");

    command()
        .env("AGY_SEARCH_SOURCE_FETCH_TRACE", &curl_trace)
        .args([
            "research",
            "source-research-allowed",
            "--source-url",
            "https://doc.rust-lang.org/book/",
        ])
        .assert()
        .success();

    assert!(!curl_trace.exists());
    Ok(())
}

#[test]
fn exact_url_research_accepts_a_paired_direct_read_without_search() {
    // Given: a caller-owned exact source URL and a Research fixture with no search event.
    // When: the fixture completes a same-conversation read_url_content pair for that URL.
    // Then: the CLI accepts the exact-source result as web-evidenced.
    command()
        .args([
            "research",
            "source-research-exact-read-only",
            "--source-url",
            "https://doc.rust-lang.org/book/",
        ])
        .assert()
        .success();
}

#[test]
fn domain_only_research_rejects_a_direct_read_without_search() {
    // Given: a domain-only restriction and a Research fixture with a direct read but no search.
    // When: the fixture returns the otherwise allowed in-domain source.
    // Then: the CLI fails closed because domain-only Research still requires search_web.
    command()
        .args([
            "research",
            "source-research-domain-read-only",
            "--domain",
            "rust-lang.org",
        ])
        .assert()
        .code(6);
}

#[test]
fn unrestricted_research_rejects_a_direct_read_without_search() {
    // Given: an unrestricted Research request and a fixture with a direct read but no search.
    // When: the fixture returns a valid public Research document.
    // Then: the unrestricted policy still requires search_web evidence.
    command()
        .args(["research", "source-research-domain-read-only"])
        .assert()
        .code(6);
}

#[test]
fn unrestricted_research_rejects_a_news_portal_source() {
    command()
        .args(["research", "source-research-news-portal"])
        .assert()
        .code(6);
}

#[test]
fn restricted_research_preserves_an_explicit_news_portal_source() {
    for restriction in [
        ["--source-url", "https://v.daum.net/v/20260807120301584"],
        ["--domain", "v.daum.net"],
    ] {
        command()
            .args(["research", "source-research-news-portal"])
            .args(restriction)
            .assert()
            .success();
    }
}

#[test]
fn temporal_exact_url_rejects_another_path_on_the_same_domain() {
    command()
        .args([
            "--verification",
            "temporal-comparison",
            "search",
            "temporal-complete",
            "--scope",
            "newer fixture",
            "--scope",
            "older fixture",
            "--source-url",
            "https://example.com/releases",
        ])
        .assert()
        .code(6);
}

#[test]
fn grounding_final_url_must_satisfy_the_caller_allowlist() {
    let curl = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake_curl.py");
    for (final_mode, expected_code) in [("allowed", 0), ("disallowed", 6)] {
        command()
            .env("AGY_SEARCH_CURL_PATH", &curl)
            .env("AGY_SEARCH_REDIRECT_FINAL", final_mode)
            .args(["search", "grounding-redirect", "--domain", "example.com"])
            .assert()
            .code(expected_code);
    }
}

#[test]
fn invalid_or_duplicate_restrictions_exit_before_antigravity()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: an invocation trace and malformed, duplicate, or over-limit restrictions.
    let temporary = TempDir::new()?;
    let trace = temporary.path().join("agy.jsonl");
    let invalid = [
        vec!["search", "fixture", "--domain", "https://rust-lang.org"],
        vec!["search", "fixture", "--domain", "rust-lang.org:443"],
        vec!["search", "fixture", "--domain", "*.rust-lang.org"],
        vec!["search", "fixture", "--domain", "127.0.0.1"],
        vec![
            "search",
            "fixture",
            "--domain",
            "rust-lang.org",
            "--domain",
            "RUST-LANG.ORG.",
        ],
        vec![
            "search",
            "fixture",
            "--domain",
            "bücher.example",
            "--domain",
            "xn--bcher-kva.example",
        ],
        vec![
            "search",
            "fixture",
            "--source-url",
            "https://user@rust-lang.org/book/",
        ],
        vec![
            "search",
            "fixture",
            "--source-url",
            "https://doc.rust-lang.org/book#one",
            "--source-url",
            "https://doc.rust-lang.org/book#two",
        ],
    ];

    // When/Then: each invalid boundary value exits 2 without invoking agy.
    for arguments in invalid {
        command()
            .env("AGY_SEARCH_FIXTURE_TRACE", &trace)
            .args(arguments)
            .assert()
            .code(2);
    }
    let mut over_limit = vec!["search".to_owned(), "fixture".to_owned()];
    for index in 0..21 {
        over_limit.extend(["--domain".to_owned(), format!("d{index}.example")]);
    }
    command()
        .env("AGY_SEARCH_FIXTURE_TRACE", &trace)
        .args(over_limit)
        .assert()
        .code(2);
    assert!(!trace.exists());
    Ok(())
}

#[test]
fn unrestricted_search_omits_policy_and_uses_one_process_without_body_fetching()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: traces for an ordinary no-flag Search.
    let temporary = TempDir::new()?;
    let agy_trace = temporary.path().join("agy.jsonl");
    let curl_trace = temporary.path().join("curl.jsonl");

    // When: the no-restriction path runs.
    command()
        .env("AGY_SEARCH_FIXTURE_TRACE", &agy_trace)
        .env("AGY_SEARCH_SOURCE_FETCH_TRACE", &curl_trace)
        .args(["search", "fixture"])
        .assert()
        .success();

    // Then: one content process ran, no body fetch ran, and no restriction was serialized.
    let records = std::fs::read_to_string(agy_trace)?;
    let records = records.lines().collect::<Vec<_>>();
    assert_eq!(records.len(), 1);
    let record: Value =
        serde_json::from_str(records.first().copied().ok_or("missing invocation trace")?)?;
    assert!(record.get("source_restriction").is_none());
    assert!(!curl_trace.exists());
    Ok(())
}
