use super::*;
use crate::response_models::SearchResponse;
#[tokio::test]
#[ignore = "live WHO page evidence proof"]
async fn live_search_evidence_proves_a_current_who_item() -> Result<(), Box<dyn std::error::Error>>
{
    let title = "Second meeting of the IHR Emergency Committee on the epidemic of Ebola \
                 Bundibugyo virus disease in the Democratic Republic of the Congo – Meeting report";
    let url = "https://www.who.int/news/item/28-08-2026-second-meeting-of-the-ihr-emergency-committee-on-the-epidemic-of-ebola-bundibugyo-virus-disease-in-the-democratic-republic-of-the-congo---meeting-report";
    let mut search: SearchResponse = serde_json::from_value(serde_json::json!({
        "object": "search",
        "evidence_audit": {
            "candidates": [{
                "scope": "latest WHO headline and publication date",
                "claim": title,
                "url": url,
                "date": "2026-08-28",
                "value": title,
                "source_date_text": "August 28, 2026",
                "evidence_excerpt": format!("{title}. This statement was published on August 28, 2026.")
            }],
            "coverage_complete": true,
            "conclusion": title
        },
        "results": [{
            "title": title,
            "url": url,
            "snippet": title,
            "date": "2026-08-28"
        }]
    }))?;

    let evidence =
        SearchSourceEvidence::fetch(&search, Instant::now() + std::time::Duration::from_secs(15))
            .await?;
    evidence.verify_and_project(&mut search)?;
    assert_eq!(
        search
            .evidence_audit
            .candidates
            .first()
            .and_then(|candidate| candidate.source_date_text.as_ref())
            .map(NonEmptyText::as_str),
        Some("28 August 2026")
    );
    Ok(())
}

#[tokio::test]
#[ignore = "live Rust release evidence proof"]
async fn live_search_evidence_proves_the_current_rust_release()
-> Result<(), Box<dyn std::error::Error>> {
    let url = "https://blog.rust-lang.org/2026/08/20/Rust-1.98.0/";
    let mut search: SearchResponse = serde_json::from_value(serde_json::json!({
        "object": "search",
        "evidence_audit": {
            "candidates": [{
                "scope": "latest stable Rust release and release date",
                "claim": "Rust 1.98.0 was released on August 20, 2026.",
                "url": url,
                "date": "2026-08-20",
                "value": "Rust 1.98.0",
                "source_date_text": "August 20, 2026",
                "evidence_excerpt": "The latest stable release is Rust 1.98.0, released on August 20, 2026."
            }],
            "coverage_complete": true,
            "conclusion": "Rust 1.98.0 is current."
        },
        "results": [{
            "title": "Announcing Rust 1.98.0",
            "url": url,
            "snippet": "model-written summary",
            "date": "2026-08-20"
        }]
    }))?;

    let evidence =
        SearchSourceEvidence::fetch(&search, Instant::now() + std::time::Duration::from_secs(15))
            .await?;
    evidence.verify_and_project(&mut search)?;

    assert_eq!(
        search.results.first().map(|source| source.url.as_str()),
        Some(url)
    );
    Ok(())
}

#[tokio::test]
#[ignore = "live Bank of Korea page evidence proof"]
async fn live_search_evidence_proves_a_current_bok_release()
-> Result<(), Box<dyn std::error::Error>> {
    let url = "https://www.bok.or.kr/portal/bbs/P0000559/view.do?nttId=11064191&menuNo=200690&programType=newsData&relate=Y&depth=200690";
    let mut search: SearchResponse = serde_json::from_value(serde_json::json!({
        "object": "search",
        "evidence_audit": {
            "candidates": [{
                "scope": "한국은행 통화정책방향 결정 회의",
                "claim": "한국은행은 2026년 8월 27일 통화정책방향 결정회의 자료를 게시하였다.",
                "url": url,
                "date": "2026-08-27",
                "value": "통화정책방향(2026.8.27)",
                "source_date_text": "2026.8.27",
                "evidence_excerpt": "한국은행 통화정책방향(2026.8.27): 통화정책방향 결정회의 공식 보도자료 원문입니다."
            }],
            "coverage_complete": true,
            "conclusion": "한국은행 기준금리는 3.00%이다."
        },
        "results": [{
            "title": "한국은행 통화정책방향(2026.8.27)",
            "url": url,
            "snippet": "한국은행 기준금리는 3.00%이다.",
            "date": "2026-08-27"
        }]
    }))?;

    let evidence =
        SearchSourceEvidence::fetch(&search, Instant::now() + std::time::Duration::from_secs(15))
            .await?;
    evidence.verify_and_project(&mut search)?;
    assert_eq!(
        search.results.first().map(|source| source.title.as_str()),
        Some("통화정책방향(2026.8.27)")
    );
    Ok(())
}

#[tokio::test]
#[ignore = "live Bank of Korea partial-candidate projection proof"]
async fn live_search_evidence_keeps_only_the_proven_bok_candidate()
-> Result<(), Box<dyn std::error::Error>> {
    let rate_url = "https://www.bok.or.kr/portal/singl/baseRate/list.do?dataSeCd=01&menuNo=200643";
    let release_url = "https://www.bok.or.kr/portal/bbs/P0000559/view.do?nttId=11064191&menuNo=200690&programType=newsData&relate=Y&depth=200690";
    let mut search: SearchResponse = serde_json::from_value(serde_json::json!({
        "object": "search",
        "evidence_audit": {
            "candidates": [{
                "scope": "기준금리 및 최근 변경일",
                "claim": "현재 기준금리는 3.00%이다.",
                "url": rate_url,
                "date": "2026-08-27",
                "value": "연 3.00%, 2026년 8월 27일",
                "source_date_text": "2026년 8월 27일",
                "evidence_excerpt": "가장 최근의 금리 변경일은 2026년 8월 27일로 3.00%입니다."
            }, {
                "scope": "최근 변경일 상세",
                "claim": "최근 변경일은 2026년 8월 27일이다.",
                "url": release_url,
                "date": "2026-08-27",
                "value": "2026년 8월 27일",
                "source_date_text": "2026년 8월 27일",
                "evidence_excerpt": "가장 최근의 금리 변경일은 2026년 8월 27일이다."
            }],
            "coverage_complete": true,
            "conclusion": "기준금리는 3.00%이다."
        },
        "results": [{
            "title": "한국은행 기준금리 안내 (연 3.00%, 2026년 8월 27일 변경)",
            "url": rate_url,
            "snippet": "모델 요약",
            "date": "2026-08-27"
        }, {
            "title": "통화정책방향 결정회의 결과 (2026년 8월 27일)",
            "url": release_url,
            "snippet": "모델 요약",
            "date": "2026-08-27"
        }]
    }))?;

    let evidence =
        SearchSourceEvidence::fetch(&search, Instant::now() + std::time::Duration::from_secs(15))
            .await?;
    evidence.verify_and_project(&mut search)?;

    assert!(search.results.iter().all(|source| {
        source.url.as_str() != rate_url || source.snippet.as_str() != "모델 요약"
    }));
    assert!(
        search
            .results
            .iter()
            .any(|source| source.url.as_str() == release_url)
    );
    Ok(())
}
