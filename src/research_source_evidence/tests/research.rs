use super::*;
use crate::response_models::{ExtractResponse, ResearchResponse};

#[test]
fn accepts_body_prose_and_rejects_headings_markup_and_fabrication() {
    let body = "<strong>Gemini 3.7 Flash is generally available (GA)</strong>: \
                ready for production use across supported Gemini API clients.";

    assert!(bind_evidence(
        &canonical_evidence_text(body),
        "Gemini 3.7 Flash is generally available (GA): ready for production use across supported Gemini API clients.",
        "generally available (GA)"
    ).is_some());
    let ga_binding = bind_evidence(
        &canonical_evidence_text(body),
        "Gemini 3.7 Flash is now generally available in supported production clients.",
        "now generally available in supported production clients",
    )
    .expect("a long exact two-token status predicate must bind");
    assert_eq!(ga_binding.value, "generally available");
    assert!(bind_evidence(body, "Gemini 3.7 Flash", "3.7").is_none());
    assert!(
        bind_evidence(
            body,
            "<a href='/model'>Gemini 3.7 Flash documentation</a>",
            "3.7"
        )
        .is_none()
    );
    assert!(
        bind_evidence(
            body,
            "Gemini 3.7 Flash is unavailable and not ready for production use.",
            "unavailable"
        )
        .is_none()
    );
    assert!(bind_evidence(
        &canonical_evidence_text(body),
        "Gemini 3.7 Flash is generally available (GA): ready for production use across supported Gemini API clients.",
        "low, medium, high"
    ).is_none());
    assert!(bind_evidence(
        "Model Default Thinking Levels Supported gemini-3.7-flash On (medium) low, medium, high",
        "Gemini 3.7 Flash supports tunable thinking levels (low, medium, high).",
        "low, medium, high"
    ).is_some());
    let table_binding = bind_evidence(
        "Model Default Thinking Levels Supported gemini-3.7-flash On (medium) low, medium, high",
        "Gemini 3.7 Flash supports tunable thinking levels (low, medium, high).",
        "tunable thinking levels (low, medium, high)",
    )
    .expect("the longest three-word table value must bind");
    assert_eq!(table_binding.value, "low, medium, high");
    let quoted_binding = bind_evidence(
        "Model Default Thinking Levels Supported gemini-3.7-flash On (medium) low, medium, high",
        "The model supports tunable thinking levels, specifically \"low,\" \"medium,\" and \"high\".",
        "\"low,\" \"medium,\" and \"high\"",
    )
    .expect("quoted list prose must bind to the exact table value");
    assert_eq!(quoted_binding.value, "low, medium, high");
    assert!(
        bind_evidence(
            body,
            "Gemini 3.7 Flash has a fabricated unlimited context window.",
            "fabricated unlimited context window"
        )
        .is_none()
    );
}

#[test]
fn projects_public_snippet_from_the_same_url_body() {
    let mut research: ResearchResponse = serde_json::from_value(serde_json::json!({
        "object": "research",
        "evidence_audit": {
            "candidates": [{
                "scope": "thinking levels",
                "claim": "The supported levels are low, medium, and high.",
                "url": "https://example.com/thinking",
                "value": "low, medium, high",
                "evidence_excerpt": "The model supports configurable thinking levels: low, medium, high."
            }],
            "coverage_complete": true,
            "conclusion": "The levels are documented."
        },
        "title": "Research",
        "summary": "Summary",
        "findings": [{
            "title": "Thinking levels",
            "summary": "The supported levels are low, medium, high.",
            "citations": ["https://example.com/thinking"]
        }],
        "sources": [{
            "title": "Thinking",
            "url": "https://example.com/thinking",
            "snippet": "model-written paraphrase"
        }]
    }))
    .expect("fixture response must deserialize");
    let url = SafeSourceUrl::parse_redirect("https://example.com/thinking")
        .expect("fixture URL must be safe");
    let body = "Gemini documentation introduction. Model Default Thinking Levels Supported gemini-3.7-flash On (medium) low, medium, high. Configure the setting per request.";
    let evidence = SourceEvidenceSnapshot {
        bodies: HashMap::from([(url, body.to_owned())]),
    };

    evidence
        .verify_and_project(&mut research)
        .expect("the exact body value must bind");

    let projected =
        "Model Default Thinking Levels Supported gemini-3.7-flash On (medium) low, medium, high.";
    assert_eq!(
        research
            .sources
            .first()
            .expect("projected source must exist")
            .snippet
            .as_str(),
        projected
    );
    assert_eq!(
        research
            .evidence_audit
            .candidates
            .first()
            .and_then(|candidate| candidate.value.as_ref())
            .expect("bound value must exist")
            .as_str(),
        "low, medium, high"
    );
    assert_eq!(
        research
            .evidence_audit
            .candidates
            .first()
            .expect("projected candidate must exist")
            .evidence_excerpt
            .as_ref()
            .expect("projected excerpt must exist")
            .as_str(),
        projected
    );
    assert_eq!(
        research
            .findings
            .first()
            .expect("projected finding must exist")
            .summary
            .as_str(),
        projected
    );
    assert_eq!(research.summary.as_str(), projected);
}

#[test]
fn extract_projects_query_relevant_same_url_body_content() {
    let url = SafeSourceUrl::parse_redirect("https://example.com/thinking")
        .expect("fixture URL must be safe");
    let body = "Gemini thinking Model Default Thinking Levels Supported gemini-3.7-flash On (medium) low, medium, high. Configure the setting per request.";
    let evidence = SourceEvidenceSnapshot {
        bodies: HashMap::from([(url, body.to_owned())]),
    };
    let response = |content: &str| -> ExtractResponse {
        serde_json::from_value(serde_json::json!({
            "object": "extract",
            "results": [{
                "url": "https://example.com/thinking",
                "title": "Gemini thinking",
                "content": content
            }]
        }))
        .expect("fixture response must deserialize")
    };
    let exact =
        "Model Default Thinking Levels Supported gemini-3.7-flash On (medium) low, medium, high.";
    let mut valid = response(exact);
    evidence
        .verify_and_project_extract(&mut valid, Some("Gemini 3.7 thinking levels"))
        .expect("exact source content must pass");
    assert!(
        valid
            .results
            .first()
            .expect("projected page must exist")
            .content
            .as_str()
            .contains(exact)
    );

    let mut fabricated = response(
        "gemini-3.7-flash supports disabled thinking and a fabricated 64,000 token budget.",
    );
    evidence
        .verify_and_project_extract(&mut fabricated, Some("Gemini 3.7 thinking levels"))
        .expect("model prose is replaced by exact source windows");
    assert!(
        !fabricated
            .results
            .first()
            .expect("projected page must exist")
            .content
            .as_str()
            .contains("fabricated")
    );
}
