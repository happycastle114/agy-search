//! Public source date syntax and audit provenance validation.

use crate::{
    calendar_date::CalendarDate,
    error::AgyError,
    response_models::{EvidenceAudit, ScopeEvidence, WebSource},
    source_date,
};

pub(super) fn validate_syntax(sources: &[WebSource]) -> Result<(), AgyError> {
    for source in sources {
        validate_update(source.last_updated.as_deref())?;
        if let Some(public_date) = source.date.as_deref() {
            CalendarDate::parse(public_date).map_err(|_| AgyError::OutputInvalid)?;
        }
    }
    Ok(())
}

pub(super) fn validate_standard_provenance(
    sources: &[WebSource],
    audit: &EvidenceAudit,
) -> Result<(), AgyError> {
    for source in sources {
        let Some(public_date) = source.date.as_deref() else {
            continue;
        };
        let public_date = CalendarDate::parse(public_date).map_err(|_| AgyError::OutputInvalid)?;
        if !audit
            .candidates
            .iter()
            .any(|candidate| binds_public_date(candidate, source, &public_date))
        {
            return Err(AgyError::OutputInvalid);
        }
    }
    Ok(())
}

fn validate_update(last_updated: Option<&str>) -> Result<(), AgyError> {
    let Some(last_updated) = last_updated else {
        return Ok(());
    };
    CalendarDate::parse(last_updated)
        .map(|_| ())
        .map_err(|_| AgyError::OutputInvalid)
}

fn binds_public_date(
    candidate: &ScopeEvidence,
    source: &WebSource,
    public_date: &CalendarDate,
) -> bool {
    if candidate.url != source.url || candidate.date.as_ref() != Some(public_date) {
        return false;
    }
    let (Some(source_date_text), Some(evidence_excerpt)) = (
        candidate.source_date_text.as_ref(),
        candidate.evidence_excerpt.as_ref(),
    ) else {
        return false;
    };
    source_date::parse(source_date_text.as_str()).is_ok_and(|date| date == *public_date)
        && evidence_excerpt
            .as_str()
            .contains(source_date_text.as_str())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn source(date: &serde_json::Value, last_updated: &serde_json::Value) -> WebSource {
        serde_json::from_value(json!({
            "title": "RFC",
            "url": "https://www.rfc-editor.org/rfc/rfc2549",
            "snippet": "RFC publication",
            "date": date,
            "last_updated": last_updated
        }))
        .expect("test WebSource must deserialize")
    }

    fn audit(
        date: &serde_json::Value,
        source_date_text: &serde_json::Value,
        evidence_excerpt: &serde_json::Value,
    ) -> EvidenceAudit {
        serde_json::from_value(json!({
            "candidates": [{
                "scope": "RFC publication",
                "claim": "Published",
                "url": "https://www.rfc-editor.org/rfc/rfc2549",
                "date": date,
                "source_date_text": source_date_text,
                "evidence_excerpt": evidence_excerpt
            }],
            "coverage_complete": true,
            "conclusion": "RFC date"
        }))
        .expect("test EvidenceAudit must deserialize")
    }

    #[test]
    fn rejects_public_day_when_source_text_only_names_month_and_year() {
        // Given: an invented public day paired with month-only audit text.
        let sources = [source(&json!("2013-02-20"), &serde_json::Value::Null)];
        let evidence = audit(
            &json!("2013-02-20"),
            &json!("February 2013"),
            &json!("Published February 2013"),
        );

        // When: public date provenance is validated.
        let result = validate_standard_provenance(&sources, &evidence);

        // Then: the month-only text cannot bind an exact day.
        assert!(matches!(result, Err(AgyError::OutputInvalid)));
    }

    #[test]
    fn accepts_public_date_bound_to_unambiguous_english_full_date() {
        // Given: the RFC public date and exact full English source text.
        let sources = [source(&json!("1999-06-01"), &serde_json::Value::Null)];
        let evidence = audit(
            &json!("1999-06-01"),
            &json!("June 1, 1999"),
            &json!("Published June 1, 1999"),
        );

        // When: public date provenance is validated.
        let result = validate_standard_provenance(&sources, &evidence);

        // Then: the exact parsed date binds successfully.
        assert!(result.is_ok());
    }

    #[test]
    fn rejects_malformed_public_update_date() {
        // Given: a source with null publication date and an impossible update date.
        let sources = [source(&serde_json::Value::Null, &json!("2026-13-40"))];

        // When: public date syntax is validated.
        let result = validate_syntax(&sources);

        // Then: the malformed update is rejected independently of date binding.
        assert!(matches!(result, Err(AgyError::OutputInvalid)));
    }

    #[test]
    fn accepts_null_public_date_metadata() {
        // Given: a public source with both date fields explicitly null.
        let sources = [source(&serde_json::Value::Null, &serde_json::Value::Null)];

        // When: public date syntax is validated.
        let result = validate_syntax(&sources);

        // Then: absence remains valid without fabricated provenance.
        assert!(result.is_ok());
    }

    #[test]
    fn syntax_allows_a_recoverable_public_date_before_standard_provenance() {
        // Given: a syntactically valid temporal primary date with an unmatched audit row.
        let sources = [source(&json!("2026-08-03"), &serde_json::Value::Null)];
        let evidence = audit(
            &json!("2026-07-29"),
            &json!("July 29, 2026"),
            &json!("Published July 29, 2026"),
        );

        // When: document syntax and standard provenance are checked independently.
        let syntax = validate_syntax(&sources);
        let provenance = validate_standard_provenance(&sources, &evidence);

        // Then: only the standard-only provenance stage rejects the mismatch.
        assert!(syntax.is_ok());
        assert!(matches!(provenance, Err(AgyError::OutputInvalid)));
    }
}
