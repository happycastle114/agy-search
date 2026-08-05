use crate::{
    calendar_date::CalendarDate,
    source_document::{CandidateBinding, SourceDocument},
    source_fetch::SafeSourceUrl,
};

use super::SourceContract;

#[test]
fn verifies_generic_panel_and_plain_text_bindings_without_borrowing()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: paired generic panels plus one plain-text source.
    let panel_url = SafeSourceUrl::parse("https://example.com/panels")?;
    let panel = SourceDocument::from_text(
        panel_url.clone(),
        "<div data-tab-buttons></div>\
         <button data-tab=\"cli\">Example CLI</button>\
         <button data-tab=\"sdk\">Example SDK</button>\
         <div data-list-panel=\"cli\"><div data-section-row>1.2.3 August 3, 2026</div>\
         <div data-section-row>0.9.0 January 1, 2025</div></div>\
         <div data-list-panel=\"sdk\"><div data-section-row>9.9.9 July 27, 2026</div></div>",
    )?;
    let text_url = SafeSourceUrl::parse("https://example.com/text")?;
    let text = SourceDocument::from_text(
        text_url.clone(),
        "<h2>Plain Track</h2>released 4.5.6 on 2026-08-01.",
    )?;
    let contract = SourceContract::from_documents(vec![panel, text])?;
    let cli_date = CalendarDate::parse("2026-08-03")?;
    let plain_date = CalendarDate::parse("2026-08-01")?;

    // When/Then: exact same-panel and bounded text tuples pass.
    contract.verify(&CandidateBinding::new(
        &panel_url,
        "Example CLI",
        "1.2.3",
        &cli_date,
        "August 3, 2026",
    )?)?;
    contract.verify(&CandidateBinding::new(
        &text_url,
        "Plain Track",
        "4.5.6",
        &plain_date,
        "2026-08-01",
    )?)?;

    // Then: sibling, stale, and partial values are rejected.
    let borrowed = CandidateBinding::new(
        &panel_url,
        "Example CLI",
        "9.9.9",
        &cli_date,
        "August 3, 2026",
    )?;
    assert!(contract.verify(&borrowed).is_err());
    let stale_date = CalendarDate::parse("2025-01-01")?;
    let stale = CandidateBinding::new(
        &panel_url,
        "Example CLI",
        "0.9.0",
        &stale_date,
        "January 1, 2025",
    )?;
    assert!(contract.verify(&stale).is_err());
    let partial_scope =
        CandidateBinding::new(&panel_url, "CLI", "1.2.3", &cli_date, "August 3, 2026")?;
    assert!(contract.verify(&partial_scope).is_err());
    let partial_value = CandidateBinding::new(
        &panel_url,
        "Example CLI",
        "1.2",
        &cli_date,
        "August 3, 2026",
    )?;
    assert!(contract.verify(&partial_value).is_err());
    Ok(())
}

#[test]
fn rejects_ambiguous_swapped_date_and_non_allowlisted_bindings()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: two panels that repeat one exact tuple.
    let allowed = SafeSourceUrl::parse("https://example.com/ambiguous")?;
    let document = SourceDocument::from_text(
        allowed.clone(),
        "<button data-tab=\"one\">Track</button><button data-tab=\"two\">Track</button>\
         <div data-list-panel=\"one\"><div data-section-row>1.0.0 August 3, 2026</div></div>\
         <div data-list-panel=\"two\"><div data-section-row>1.0.0 August 3, 2026</div></div>",
    )?;
    let contract = SourceContract::from_documents(vec![document])?;
    let correct = CalendarDate::parse("2026-08-03")?;
    let swapped = CalendarDate::parse("2026-08-02")?;

    // When/Then: ambiguous occurrence, normalized-date mismatch, and URL escape fail.
    assert!(
        contract
            .verify(&CandidateBinding::new(
                &allowed,
                "Track",
                "1.0.0",
                &correct,
                "August 3, 2026",
            )?)
            .is_err()
    );
    assert!(
        CandidateBinding::new(&allowed, "Track", "1.0.0", &swapped, "August 3, 2026",).is_err()
    );
    let outside = SafeSourceUrl::parse("https://outside.example/release")?;
    assert!(
        contract
            .verify(&CandidateBinding::new(
                &outside,
                "Track",
                "1.0.0",
                &correct,
                "August 3, 2026",
            )?)
            .is_err()
    );
    let split_url = SafeSourceUrl::parse("https://example.com/headings")?;
    let split_document = SourceDocument::from_text(
        split_url.clone(),
        "<h2>Track A</h2>1.0.0<h2>Track B</h2>August 3, 2026",
    )?;
    let split_contract = SourceContract::from_documents(vec![split_document])?;
    assert!(
        split_contract
            .verify(&CandidateBinding::new(
                &split_url,
                "Track A",
                "1.0.0",
                &correct,
                "August 3, 2026",
            )?)
            .is_err()
    );
    Ok(())
}

#[test]
fn rejects_structured_evidence_hidden_in_inert_or_quoted_html()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: syntactically complete release evidence hidden outside rendered markup.
    let url = SafeSourceUrl::parse("https://example.com/inert-markup")?;
    let date = CalendarDate::parse("2026-08-05")?;
    let binding =
        CandidateBinding::new(&url, "hidden-track", "hidden-v2", &date, "August 5, 2026")?;
    let fake_evidence = "<button data-tab=\"hidden\">hidden-track</button>\
        <div data-list-panel=\"hidden\"><div data-section-row>\
        <span data-date-pin>hidden-v2 August 5, 2026</span></div></div>";
    let documents = [
        format!("<script>{fake_evidence}</script>"),
        format!("<style>{fake_evidence}</style>"),
        format!("<template>{fake_evidence}</template>"),
        format!("<noscript>{fake_evidence}</noscript>"),
        format!("<!--{fake_evidence}-->"),
        format!("<div title='{fake_evidence}'></div>"),
        format!("<script>{fake_evidence}</script"),
        format!("<div title='{fake_evidence}</div>"),
    ];

    // When/Then: each inert or quoted payload cannot verify the fake binding.
    for document in documents {
        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            let parsed = SourceDocument::from_text(url.clone(), &document)?;
            let contract = SourceContract::from_documents(vec![parsed])?;
            contract.verify(&binding)?;
            Ok(())
        })();
        assert!(result.is_err(), "inert payload verified: {document}");
    }
    Ok(())
}
