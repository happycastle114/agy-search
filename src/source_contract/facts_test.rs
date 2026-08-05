use std::{path::PathBuf, time::Duration};

use tokio::time::Instant;

use super::SourceContract;
use crate::{
    calendar_date::CalendarDate,
    source_document::{CandidateBinding, SourceDocument},
    source_fetch::{SafeSourceUrl, SourceFetcher},
};

#[test]
fn extracts_only_unambiguous_strongly_structured_local_facts()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: exact tab/panel/first-row/date-pin structure for two caller scopes.
    let url = SafeSourceUrl::parse("https://example.com/local")?;
    let document = SourceDocument::from_text(
        url.clone(),
        "<button data-tab=\"alpha\">alpha</button>\
         <button data-tab=\"beta\">beta</button>\
         <div data-list-panel=\"alpha\"><div data-section-row>\
         <span data-date-pin>alpha-v2 August 5, 2026</span></div></div>\
         <div data-list-panel=\"beta\"><div data-section-row>\
         <span data-date-pin>beta-v1 2026-08-04</span></div></div>",
    )?;
    let contract = SourceContract::from_documents(vec![document])?;

    // When/Then: exact lookup yields typed values and normalized dates.
    let alpha = contract
        .exact_fact("alpha", "alpha-v2")?
        .ok_or("alpha fact missing")?;
    assert_eq!(alpha.scope(), "alpha");
    assert_eq!(alpha.value(), "alpha-v2");
    assert_eq!(alpha.date().as_str(), "2026-08-05");
    assert_eq!(alpha.source_date_text(), "August 5, 2026");
    assert_eq!(alpha.source_url(), &url);
    assert!(contract.exact_fact("alpha", "alpha-v1")?.is_none());
    let beta = contract
        .exact_fact("beta", "beta-v1")?
        .ok_or("beta fact missing")?;
    assert_eq!(beta.date().as_str(), "2026-08-04");
    Ok(())
}

#[test]
fn local_fact_lookup_falls_back_for_weak_or_contaminated_rows_and_rejects_duplicates()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: no pin, numeric-only value, and duplicated exact scope facts.
    let weak_url = SafeSourceUrl::parse("https://example.com/weak")?;
    let weak = SourceDocument::from_text(
        weak_url,
        "<button data-tab=\"plain\">plain</button>\
         <button data-tab=\"numeric\">numeric</button>\
         <div data-list-panel=\"plain\"><div data-section-row>v2 August 5, 2026</div></div>\
         <div data-list-panel=\"numeric\"><div data-section-row>\
         <span data-date-pin>20260805 August 5, 2026</span></div></div>",
    )?;
    let weak_contract = SourceContract::from_documents(vec![weak])?;
    assert!(weak_contract.exact_fact("plain", "v2")?.is_none());
    assert!(weak_contract.exact_fact("numeric", "20260805")?.is_none());

    let duplicate_url = SafeSourceUrl::parse("https://example.com/duplicate")?;
    let duplicate = SourceDocument::from_text(
        duplicate_url,
        "<button data-tab=\"one\">same</button><button data-tab=\"two\">same</button>\
         <div data-list-panel=\"one\"><div data-section-row>\
         <span data-date-pin>v1 August 5, 2026</span></div></div>\
         <div data-list-panel=\"two\"><div data-section-row>\
         <span data-date-pin>v2 August 6, 2026</span></div></div>",
    )?;
    let duplicate_contract = SourceContract::from_documents(vec![duplicate])?;

    // When/Then: weak rows have no fact and duplicate scopes are ambiguous.
    assert!(duplicate_contract.exact_fact("same", "v1").is_err());
    Ok(())
}

#[tokio::test]
#[ignore = "live official source proof"]
async fn live_official_panel_verifies_current_cli_tuple() -> Result<(), Box<dyn std::error::Error>>
{
    // Given: the caller-owned official Antigravity changelog source.
    let source = SafeSourceUrl::parse("https://antigravity.google/changelog")?;
    let fetcher = SourceFetcher::new(PathBuf::from("curl"));

    // When: the real page is safely fetched and its current CLI tuple is checked.
    let contract = SourceContract::fetch(
        &fetcher,
        std::slice::from_ref(&source),
        Instant::now() + Duration::from_secs(10),
    )
    .await?;
    let date = CalendarDate::parse("2026-08-03")?;
    let binding = CandidateBinding::new(
        &source,
        "Antigravity CLI",
        "1.1.10",
        &date,
        "August 3, 2026",
    )?;

    // Then: the exact tuple is bound inside the CLI panel's first release row.
    contract.verify(&binding)?;
    let fact = contract
        .exact_fact("Antigravity CLI", "1.1.10")?
        .ok_or("official CLI source fact missing")?;
    assert_eq!(fact.date().as_str(), "2026-08-03");
    assert_eq!(fact.source_date_text(), "August 3, 2026");
    Ok(())
}
