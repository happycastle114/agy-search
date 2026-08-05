//! Conservative extraction of one local release fact from a strong source row.

use crate::{
    calendar_date::CalendarDate,
    source_date,
    source_document_html::{elements_with_attribute, normalize_text},
    source_fetch::SafeSourceUrl,
};

const MAX_VALUE_BYTES: usize = 128;
const MAX_DATE_TEXT_BYTES: usize = 32;
const MAX_SCOPE_BYTES: usize = 128;

#[derive(Clone, Debug)]
pub(crate) struct SourceFact {
    source_url: SafeSourceUrl,
    scope: String,
    value: String,
    date: CalendarDate,
    source_date_text: String,
}

#[derive(Clone, Debug)]
pub(crate) enum SourceFactCandidate {
    Fact(SourceFact),
    NotApplicable,
    Ambiguous,
}

impl SourceFact {
    pub(crate) fn from_first_row(
        source_url: &SafeSourceUrl,
        scope: &str,
        row: &str,
    ) -> SourceFactCandidate {
        if scope.is_empty() || scope.len() > MAX_SCOPE_BYTES || scope.trim() != scope {
            return SourceFactCandidate::NotApplicable;
        }
        let Ok(pins) = elements_with_attribute(row, "data-date-pin") else {
            return SourceFactCandidate::NotApplicable;
        };
        let [pin] = pins.as_slice() else {
            return if pins.len() > 1 {
                SourceFactCandidate::Ambiguous
            } else {
                SourceFactCandidate::NotApplicable
            };
        };
        let normalized = normalize_text(&pin.content);
        let Some((value, source_date_text)) = normalized.split_once(' ') else {
            return SourceFactCandidate::NotApplicable;
        };
        if !valid_value(value) || !valid_date_text(source_date_text) {
            return SourceFactCandidate::NotApplicable;
        }
        let Ok(date) = source_date::parse(source_date_text) else {
            return SourceFactCandidate::NotApplicable;
        };
        SourceFactCandidate::Fact(Self {
            source_url: source_url.clone(),
            scope: scope.to_owned(),
            value: value.to_owned(),
            date,
            source_date_text: source_date_text.to_owned(),
        })
    }

    pub(crate) const fn source_url(&self) -> &SafeSourceUrl {
        &self.source_url
    }

    pub(crate) fn scope(&self) -> &str {
        &self.scope
    }

    pub(crate) fn value(&self) -> &str {
        &self.value
    }

    pub(crate) const fn date(&self) -> &CalendarDate {
        &self.date
    }

    pub(crate) fn source_date_text(&self) -> &str {
        &self.source_date_text
    }
}

fn valid_value(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_VALUE_BYTES
        && !value.chars().any(char::is_whitespace)
        && value.chars().any(char::is_alphanumeric)
        && !value.chars().all(|character| character.is_ascii_digit())
        && !value.contains("://")
        && !value.starts_with("http:")
        && !value.starts_with("https:")
        && CalendarDate::parse(value).is_err()
}

fn valid_date_text(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_DATE_TEXT_BYTES
        && (value.split_ascii_whitespace().count() == 1
            || value.split_ascii_whitespace().count() == 3)
}
