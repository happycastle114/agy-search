use super::*;
use crate::response_models::SearchResponse;

#[test]
fn search_preserves_distinct_verified_contexts_for_the_same_url() {
    // Given: two disjoint source facts, a repeated candidate, and an unsupported claim.
    let first = "Rust 1.98.0 is the latest stable release available today.";
    let second = "The compiler supports the portable widget target on all platforms.";
    let mut search = search_fixture("Announcing Rust 1.98.0");
    let original = search
        .evidence_audit
        .candidates
        .first()
        .expect("candidate")
        .clone();
    let mut additional = original.clone();
    additional.value = Some(NonEmptyText::parse("portable widget target").expect("valid value"));
    additional.evidence_excerpt = Some(NonEmptyText::parse(second).expect("valid excerpt"));
    let mut invalid = original.clone();
    invalid.value = Some(NonEmptyText::parse("invented unsupported feature").expect("valid value"));
    invalid.evidence_excerpt = Some(
        NonEmptyText::parse("This release includes an invented unsupported feature.")
            .expect("valid excerpt"),
    );
    search
        .evidence_audit
        .candidates
        .extend([original, additional, invalid]);
    let evidence = SearchSourceEvidence {
        pages: HashMap::from([(
            SafeSourceUrl::parse_redirect("https://example.com/releases/1.98.0").expect("safe URL"),
            SearchEvidencePage {
                body: format!("{first} Unrelated intervening sentence. {second}"),
                headings: Vec::new(),
                title: Some("Announcing Rust 1.98.0".to_owned()),
            },
        )]),
    };

    // When: candidates are independently verified and projected into the public result.
    evidence
        .verify_and_project(&mut search)
        .expect("verified result");

    // Then: source order survives, repeated context appears once, and model prose is absent.
    assert_eq!(
        search.results.first().expect("result").snippet.as_str(),
        format!("{first}\n{second}")
    );
    assert_eq!(search.evidence_audit.candidates.len(), 3);
}

fn search_fixture(title: &str) -> SearchResponse {
    serde_json::from_value(serde_json::json!({
        "object": "search",
        "evidence_audit": {
            "candidates": [{
                "scope": "latest stable release",
                "claim": "Rust 1.98.0 is the latest stable release.",
                "url": "https://example.com/releases/1.98.0",
                "value": "latest stable release",
                "evidence_excerpt": "Rust 1.98.0 is the latest stable release available today."
            }],
            "coverage_complete": true,
            "conclusion": "Rust 1.98.0 is current."
        },
        "results": [{
            "title": title,
            "url": "https://example.com/releases/1.98.0",
            "snippet": "model-written paraphrase"
        }]
    }))
    .expect("fixture response must deserialize")
}

#[test]
fn search_requires_page_identity_and_projects_same_url_body_context() {
    let url = SafeSourceUrl::parse_redirect("https://example.com/releases/1.98.0")
        .expect("fixture URL must be safe");
    let body = "Announcing Rust 1.98.0. Rust 1.98.0 is the latest stable release available today.";
    let evidence = SearchSourceEvidence {
        pages: HashMap::from([(
            url,
            SearchEvidencePage {
                body: body.to_owned(),
                headings: vec!["Announcing Rust 1.98.0".to_owned()],
                title: None,
            },
        )]),
    };
    let mut valid = search_fixture("Announcing Rust 1.98.0");
    evidence
        .verify_and_project(&mut valid)
        .expect("exact page identity and body evidence must pass");
    assert_eq!(
        valid
            .results
            .first()
            .expect("search result must remain")
            .snippet
            .as_str(),
        "Rust 1.98.0 is the latest stable release available today."
    );

    let mut fabricated = search_fixture("Rust current downloads");
    assert!(matches!(
        evidence.verify_and_project(&mut fabricated),
        Err(AgyError::OutputInvalid)
    ));

    let mut prefixed = search_fixture("Official latest stable release announcement");
    evidence
        .verify_and_project(&mut prefixed)
        .expect("a substantial exact body identity may remove a publisher prefix");
    assert_eq!(
        prefixed
            .results
            .first()
            .expect("projected result must exist")
            .title
            .as_str(),
        "latest stable release"
    );
}

#[test]
fn search_replaces_model_title_with_fetched_document_title() {
    // Given: the model title is a paraphrase while the fetched page exposes a canonical title.
    let url = SafeSourceUrl::parse_redirect("https://example.com/releases/1.98.0")
        .expect("fixture URL must be safe");
    let evidence = SearchSourceEvidence {
        pages: HashMap::from([(
            url,
            SearchEvidencePage {
                body: "Rust 1.98.0 is the latest stable release available today.".to_owned(),
                headings: Vec::new(),
                title: Some("Announcing Rust 1.98.0".to_owned()),
            },
        )]),
    };
    let mut search = search_fixture("Latest compiler update");

    // When: the independently fetched page is verified and projected.
    evidence
        .verify_and_project(&mut search)
        .expect("canonical document identity must replace model title prose");

    // Then: public output uses the fetched title, not an invented heading.
    assert_eq!(
        search
            .results
            .first()
            .expect("result must remain")
            .title
            .as_str(),
        "Announcing Rust 1.98.0"
    );
}

#[test]
fn search_normalizes_the_nearest_visible_date_variant() {
    let body = "Announcing Rust 1.98.0 20 August 2026 Rust 1.98.0 is now stable. A prior release was 16 July 2026.";
    let date = crate::types::CalendarDate::parse("2026-08-20").expect("fixture date must be valid");
    let title = body
        .find("Announcing Rust 1.98.0")
        .expect("fixture title must exist");
    let title_end = title + "Announcing Rust 1.98.0".len();

    let (start, end) = nearest_date_binding(body, &date, title, title_end)
        .expect("day-first source date must bind");

    assert_eq!(&body[start..end], "20 August 2026");
}

#[test]
fn search_binds_a_compact_percentage_without_borrowing_other_numbers() {
    let body = "2008/12/11 3.00 2026/08/27 3.00";
    let date = crate::types::CalendarDate::parse("2026-08-27").expect("fixture date must be valid");
    let (start, end) = find_compact_percentage(body, "연 3.00%, 2026년 8월 27일 변경", Some(&date))
        .expect("the exact percentage token must bind");

    assert_eq!(&body[start..end], "3.00");
    assert!(find_compact_percentage(body, "연 4.00%", Some(&date)).is_none());
}

#[test]
fn search_binds_percentage_and_date_when_model_connective_differs() {
    // Given: the model excerpt and value contain the same percentage and date facts, but use
    // different connective prose around them.
    let body = "변경일자 기준금리 2026 08월 27일 3.00 2026 07월 16일 2.75";
    let excerpt = "현재 기준금리는 연 3.00%이며 가장 최근 변경일은 2026년 8월 27일입니다.";
    let value = "연 3.00% (2026년 8월 27일 변경)";
    let date = crate::types::CalendarDate::parse("2026-08-27").expect("fixture date must be valid");

    // When: Search binds the candidate against the independently fetched page body.
    let bound = bind_search_candidate_evidence(body, excerpt, value, Some(&date))
        .expect("the source-bound percentage and date must survive connective variation");

    // Then: only the literal source-body percentage becomes public evidence.
    assert_eq!(bound.value, "3.00");
}

#[test]
fn search_uses_source_date_text_only_to_disambiguate_body_value() {
    // Given: an official rate table contains repeated historical values and the model keeps
    // public source metadata null while supplying a complete source date as audit evidence.
    let url = SafeSourceUrl::parse_redirect("https://example.com/rates/list.do?series=base")
        .expect("fixture URL must be safe");
    let evidence = SearchSourceEvidence {
        pages: HashMap::from([(
            url,
            SearchEvidencePage {
                body:
                    "변경일자 기준금리 2026 08월 27일 3.00 2026 07월 16일 2.75 2008 12월 11일 3.00"
                        .to_owned(),
                headings: vec!["한국은행 기준금리 추이".to_owned()],
                title: None,
            },
        )]),
    };
    let mut search: SearchResponse = serde_json::from_value(serde_json::json!({
        "object": "search",
        "evidence_audit": {
            "candidates": [{
                "scope": "current policy rate",
                "claim": "The current policy rate is 3.00 percent.",
                "url": "https://example.com/rates/list.do?series=base",
                "date": null,
                "source_date_text": "2026년 8월 27일",
                "value": "연 3.00%",
                "evidence_excerpt": "현재 기준금리는 연 3.00%이며 최근 변경일은 2026년 8월 27일입니다."
            }],
            "coverage_complete": true,
            "conclusion": "The official table proves the current rate."
        },
        "results": [{
            "title": "한국은행 기준금리 추이 공식 페이지",
            "url": "https://example.com/rates/list.do?series=base",
            "snippet": "model prose",
            "date": "2026-08-27"
        }]
    }))
    .expect("fixture response must deserialize");

    // When: the Search evidence boundary independently verifies and projects the result.
    evidence
        .verify_and_project(&mut search)
        .expect("source date evidence must identify the correct same-page value");

    // Then: the verified body value is retained without inventing public date metadata.
    let result = search.results.first().expect("verified result must remain");
    assert!(result.snippet.as_str().contains("2026 08월 27일 3.00"));
    assert!(result.date.is_none());
}
