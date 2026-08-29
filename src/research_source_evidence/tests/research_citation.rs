use super::*;
use crate::response_models::ResearchResponse;

#[test]
fn rejects_a_finding_bound_to_another_sources_verified_value() {
    let first = SafeSourceUrl::parse_redirect("https://example.com/first")
        .expect("fixture URL must be safe");
    let second = SafeSourceUrl::parse_redirect("https://example.com/second")
        .expect("fixture URL must be safe");
    let mut research: ResearchResponse = serde_json::from_value(serde_json::json!({
        "object": "research",
        "evidence_audit": {
            "candidates": [
                {
                    "scope": "availability",
                    "claim": "The model is generally available.",
                    "url": "https://example.com/first",
                    "value": "generally available",
                    "evidence_excerpt": "The model is generally available for production workloads today."
                },
                {
                    "scope": "modalities",
                    "claim": "The model accepts text, image, video, and audio.",
                    "url": "https://example.com/second",
                    "value": "text, image, video, and audio",
                    "evidence_excerpt": "Supported input modalities include text, image, video, and audio."
                }
            ],
            "coverage_complete": true,
            "conclusion": "The requested claims were checked."
        },
        "title": "Research",
        "summary": "Summary",
        "findings": [{
            "title": "Supported modalities",
            "summary": "The model accepts text, image, video, and audio.",
            "citations": ["https://example.com/first"]
        }],
        "sources": [
            {"title": "Availability", "url": "https://example.com/first", "snippet": "pending"},
            {"title": "Modalities", "url": "https://example.com/second", "snippet": "pending"}
        ]
    }))
    .expect("fixture response must deserialize");
    let evidence = SourceEvidenceSnapshot {
        bodies: HashMap::from([
            (
                first,
                "The model is generally available for production workloads today.".to_owned(),
            ),
            (
                second,
                "Supported input modalities include text, image, video, and audio.".to_owned(),
            ),
        ]),
    };

    assert!(matches!(
        evidence.verify_and_project(&mut research),
        Err(AgyError::OutputInvalid)
    ));
}
