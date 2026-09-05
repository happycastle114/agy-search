//! Opt-in production quality gate against a signed-in real Antigravity CLI.

use std::{collections::HashSet, env, io, process::Command, time::Instant};

use serde_json::Value;
use url::Url;

const GEMINI_MODEL_URL: &str = "https://ai.google.dev/gemini-api/docs/models/gemini-3.8-flash";
const GEMINI_LATEST_URL: &str = "https://ai.google.dev/gemini-api/docs/latest-model";
const GEMINI_CHANGELOG_URL: &str = "https://ai.google.dev/gemini-api/docs/changelog";
const GEMINI_THINKING_URL: &str = "https://ai.google.dev/gemini-api/docs/thinking";
const GEMINI_SEARCH_URL: &str =
    "https://ai.google.dev/gemini-api/docs/generate-content/google-search";
const GEMINI_SEARCH_ALIAS_URL: &str = "https://ai.google.dev/gemini-api/docs/google-search";
const IANA_EXAMPLE_DOMAINS_URL: &str = "https://www.iana.org/help/example-domains";

#[derive(Clone, Copy)]
enum ExpectedObject {
    Extract,
    Research,
}

impl ExpectedObject {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Extract => "extract",
            Self::Research => "research",
        }
    }
}

fn run(
    agy_path: &str,
    expected: ExpectedObject,
    arguments: &[&str],
) -> Result<(Value, f64), Box<dyn std::error::Error>> {
    let started = Instant::now();
    let output = Command::new(env!("CARGO_BIN_EXE_agy-search"))
        .args(["--agy-path", agy_path])
        .args(arguments)
        .output()?;
    let elapsed = started.elapsed().as_secs_f64();
    if !output.status.success() {
        return Err(io::Error::other(format!(
            "{} failed after {elapsed:.2}s: {}",
            expected.as_str(),
            String::from_utf8_lossy(&output.stderr)
        ))
        .into());
    }
    if !output.stderr.is_empty() {
        return Err(io::Error::other("successful command emitted stderr").into());
    }
    let document: Value = serde_json::from_slice(&output.stdout)?;
    if document.get("object").and_then(Value::as_str) != Some(expected.as_str()) {
        return Err(io::Error::other("response discriminator mismatch").into());
    }
    Ok((document, elapsed))
}

fn direct_deep_url<'value>(
    value: &'value Value,
    domain: &str,
) -> Result<&'value str, Box<dyn std::error::Error>> {
    let raw = value
        .as_str()
        .ok_or_else(|| io::Error::other("source URL must be a string"))?;
    let parsed = Url::parse(raw)?;
    let host = parsed
        .host_str()
        .ok_or_else(|| io::Error::other("source URL must have a host"))?;
    let allowed_host = host == domain || host.ends_with(&format!(".{domain}"));
    if parsed.scheme() != "https"
        || !allowed_host
        || parsed.path() == "/"
        || host == "vertexaisearch.cloud.google.com"
        || parsed
            .path()
            .trim_matches('/')
            .eq_ignore_ascii_case("search")
    {
        return Err(io::Error::other(format!("non-canonical evidence URL: {raw}")).into());
    }
    Ok(raw)
}

fn require_text_markers(text: &str, markers: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    let normalized = text.to_lowercase();
    for marker in markers {
        if !normalized.contains(&marker.to_lowercase()) {
            return Err(io::Error::other(format!("missing evidence marker: {marker}")).into());
        }
    }
    Ok(())
}

fn validated_research_sources(
    research: &Value,
) -> Result<HashSet<&str>, Box<dyn std::error::Error>> {
    let sources = research
        .get("sources")
        .and_then(Value::as_array)
        .ok_or_else(|| io::Error::other("research sources are missing"))?;
    if sources.len() != 4 {
        return Err(io::Error::other("research must retain exactly four claim sources").into());
    }
    let source_urls = sources
        .iter()
        .map(|source| {
            direct_deep_url(
                source
                    .get("url")
                    .ok_or_else(|| io::Error::other("research source URL is missing"))?,
                "ai.google.dev",
            )
        })
        .collect::<Result<HashSet<_>, _>>()?;
    let citations = research
        .get("findings")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|finding| finding.get("citations").and_then(Value::as_array))
        .flatten()
        .filter_map(Value::as_str);
    if !citations
        .into_iter()
        .all(|citation| source_urls.contains(citation))
    {
        return Err(io::Error::other("research citation is not a retained exact source").into());
    }
    Ok(source_urls)
}

fn require_finding_citation(
    research: &Value,
    marker: &str,
    expected_urls: &[&str],
) -> Result<String, Box<dyn std::error::Error>> {
    let normalized_marker = marker.to_lowercase();
    research
        .get("findings")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|finding| {
            format!(
                "{} {}",
                finding
                    .get("title")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
                finding
                    .get("summary")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
            )
            .to_lowercase()
            .contains(&normalized_marker)
        })
        .filter_map(|finding| finding.get("citations").and_then(Value::as_array))
        .flatten()
        .filter_map(Value::as_str)
        .find(|citation| expected_urls.contains(citation))
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            io::Error::other(format!(
                "finding `{marker}` did not cite one of {expected_urls:?}"
            ))
            .into()
        })
}

fn require_extract_markers(
    extract: &Value,
    expected_url: &str,
    markers: &[&str],
) -> Result<(), Box<dyn std::error::Error>> {
    let page = extract
        .get("results")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|page| page.get("url").and_then(Value::as_str) == Some(expected_url))
        .ok_or_else(|| io::Error::other(format!("extract omitted {expected_url}")))?;
    require_text_markers(&page.to_string(), markers)
}

fn run_known_extract(agy_path: &str) -> Result<f64, Box<dyn std::error::Error>> {
    let (known_extract, extract_seconds) = run(
        agy_path,
        ExpectedObject::Extract,
        &[
            "--effort",
            "low",
            "--timeout",
            "120",
            "extract",
            IANA_EXAMPLE_DOMAINS_URL,
            "--query",
            "What are IANA example domains reserved for?",
        ],
    )?;
    require_text_markers(&known_extract.to_string(), &["example", "documentation"])?;
    Ok(extract_seconds)
}

fn run_research_and_source_extract(
    agy_path: &str,
) -> Result<(f64, f64, usize), Box<dyn std::error::Error>> {
    let mut research_arguments = vec![
        "--effort",
        "medium",
        "--timeout",
        "300",
        "research",
        "Using only the four exact caller-supplied official Google AI pages, return exactly four distinct findings and four retained sources: (1) the exact Gemini 3.8 Flash generally-available statement, (2) the Gemini 3.8 Flash model page's supported data types row with every listed input type, (3) the official thinking guide table row listing every supported thinking level for gemini-3.8-flash, and (4) the official Google Search grounding guide's exact real-time-web explanation. Cite one supplied page for each numbered claim and copy predicate-bearing body evidence without paraphrasing.",
        "--max-sources",
        "4",
    ];
    for source in [
        GEMINI_LATEST_URL,
        GEMINI_MODEL_URL,
        GEMINI_THINKING_URL,
        GEMINI_SEARCH_URL,
    ] {
        research_arguments.extend(["--source-url", source]);
    }
    let (research, research_seconds) =
        run(agy_path, ExpectedObject::Research, &research_arguments)?;
    let source_urls = validated_research_sources(&research)?;
    for expected in [GEMINI_MODEL_URL, GEMINI_THINKING_URL] {
        if !source_urls.contains(expected) {
            return Err(io::Error::other(format!(
                "research omitted required claim-specific source: {expected}"
            ))
            .into());
        }
    }
    let ga_url = require_finding_citation(
        &research,
        "generally available",
        &[GEMINI_LATEST_URL, GEMINI_CHANGELOG_URL],
    )?;
    if !source_urls.contains(ga_url.as_str()) {
        return Err(io::Error::other("GA finding did not retain its official source").into());
    }
    for input_type in ["input", "text", "image", "video", "audio", "pdf"] {
        require_finding_citation(&research, input_type, &[GEMINI_MODEL_URL])?;
    }
    require_finding_citation(&research, "thinking", &[GEMINI_THINKING_URL])?;
    let search_url = require_finding_citation(
        &research,
        "google search",
        &[GEMINI_SEARCH_URL, GEMINI_SEARCH_ALIAS_URL],
    )?;
    if !source_urls.contains(search_url.as_str()) {
        return Err(io::Error::other("Search finding did not retain its official source").into());
    }

    let extract_cases = [
        (
            ga_url.as_str(),
            "Extract the exact Gemini 3.8 Flash generally-available statement from this announcement page; do not discuss unrelated capabilities.",
            &["3.8", "generally available"][..],
        ),
        (
            GEMINI_MODEL_URL,
            "Extract the exact Gemini 3.8 Flash supported data types row and all five listed input types from this model page.",
            &["3.8", "input", "text", "image", "video", "audio", "pdf"][..],
        ),
        (
            GEMINI_THINKING_URL,
            "Extract the exact gemini-3.8-flash table row and every supported thinking level from this thinking guide.",
            &["gemini-3.8-flash", "thinking", "low", "medium", "high"][..],
        ),
        (
            search_url.as_str(),
            "Extract the exact sentence explaining how Grounding with Google Search uses real-time web information.",
            &["google search", "ground"][..],
        ),
    ];
    let mut research_extract_seconds = 0.0;
    for (url, query, markers) in extract_cases {
        let (research_extract, seconds) = run(
            agy_path,
            ExpectedObject::Extract,
            &[
                "--effort",
                "medium",
                "--timeout",
                "180",
                "extract",
                url,
                "--query",
                query,
            ],
        )?;
        research_extract_seconds += seconds;
        require_extract_markers(&research_extract, url, markers)?;
    }
    Ok((
        research_seconds,
        research_extract_seconds,
        source_urls.len(),
    ))
}

#[test]
#[ignore = "spends real Antigravity usage; production releases run this gate explicitly"]
fn production_extract_and_research_gate() -> Result<(), Box<dyn std::error::Error>> {
    let agy_path = env::var("AGY_SEARCH_AGY_PATH").unwrap_or_else(|_| "agy".to_owned());
    let extract_seconds = run_known_extract(&agy_path)?;
    let (research_seconds, research_extract_seconds, source_count) =
        run_research_and_source_extract(&agy_path)?;

    println!(
        "production gate PASS extract={extract_seconds:.2}s research={research_seconds:.2}s research_extract={research_extract_seconds:.2}s sources={source_count}"
    );
    Ok(())
}
