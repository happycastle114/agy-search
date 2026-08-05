use std::str::FromStr;

use serde_json::json;

use super::recovery::VerifiedScope;
use super::*;
use crate::{
    request::SearchRequest,
    response::Document as ResponseDocument,
    temporal_contract::TemporalContract,
    types::{
        CalendarDate, DatePolicy, NonEmptyText, Operation, ScopePolicy, SourcePolicy,
        VerificationMode,
    },
};

pub(super) fn temporal_contract(scopes: &[&str], sources: &[&str]) -> TemporalContract {
    temporal_contract_with_cutoff(scopes, sources, None)
}

pub(super) fn temporal_contract_with_cutoff(
    scopes: &[&str],
    sources: &[&str],
    cutoff: Option<&str>,
) -> TemporalContract {
    let scopes = scopes
        .iter()
        .map(|scope| ScopeLabel::from_str(scope).expect("valid caller scope"))
        .collect();
    let sources = sources
        .iter()
        .map(|source| crate::types::HttpUrl::parse(source).expect("valid caller source"))
        .collect();
    let cutoff = cutoff.map(|value| CalendarDate::parse(value).expect("valid cutoff"));
    TemporalContract::parse(
        VerificationMode::TemporalComparison,
        scopes,
        sources,
        cutoff,
    )
    .expect("valid temporal contract")
    .expect("temporal mode creates a contract")
}

pub(super) fn alpha_beta_contract() -> TemporalContract {
    temporal_contract(
        &["alpha", "beta"],
        &["https://example.com/alpha", "https://example.com/beta"],
    )
}

pub(super) fn search_response(scopes: &[&str], coverage_complete: bool) -> SearchResponse {
    let candidates = scopes
        .iter()
        .enumerate()
        .map(|(index, scope)| {
            let value = format!("v{}", scopes.len() - index);
            let date = format!("2026-08-0{}", scopes.len() - index + 1);
            json!({
                "scope": scope,
                "claim": value,
                "url": format!("https://example.com/{scope}"),
                "date": date,
                "value": value,
                "source_date_text": date,
                "evidence_excerpt": format!("{value} released {date}"),
            })
        })
        .collect::<Vec<_>>();
    serde_json::from_value(json!({
        "object": "search",
        "evidence_audit": {
            "candidates": candidates,
            "coverage_complete": coverage_complete,
            "conclusion": "winner",
        },
        "results": [{
            "title": "alpha v2",
            "url": "https://example.com/alpha",
            "snippet": "alpha v2",
            "date": "2026-08-03",
            "last_updated": null,
        }],
    }))
    .expect("test response must deserialize")
}

pub(super) fn search_request() -> SearchRequest {
    SearchRequest {
        query: NonEmptyText::from_str("temporal unit fixture").expect("valid query"),
        source_policy: SourcePolicy::PrimaryFirst,
        scope_policy: ScopePolicy::CompleteRequestedScope,
        date_policy: DatePolicy::ExplicitSourceOnly,
        verification: VerificationMode::TemporalComparison,
        temporal_contract: Some(alpha_beta_contract()),
        source_restriction: crate::source_restriction::SourceRestriction::Unrestricted,
        max_results: 5,
        country: None,
        max_tokens_per_page: None,
    }
}

pub(super) fn verified_scope(scope: &str, value: &str, date: &str) -> VerifiedScope {
    let response = ResponseDocument::parse(
        Operation::Search,
        json!({
            "object": "search",
            "evidence_audit": {
                "candidates": [{
                    "scope": scope,
                    "claim": value,
                    "url": format!("https://example.com/{scope}"),
                    "date": date,
                    "value": value,
                    "source_date_text": date,
                    "evidence_excerpt": format!("{value} released {date}"),
                }],
                "coverage_complete": true,
                "conclusion": value,
            },
            "results": [{
                "title": value,
                "url": format!("https://example.com/{scope}"),
                "snippet": format!("{scope} {value}"),
                "date": date,
                "last_updated": null,
            }],
        }),
    )
    .expect("test response must parse");
    let label =
        ScopeLabel::parse(&NonEmptyText::from_str(scope).expect("test scope must be non-empty"))
            .expect("test scope must be bounded");
    validate_scope_result(label, response, &alpha_beta_contract())
        .expect("test scope response must verify")
}
