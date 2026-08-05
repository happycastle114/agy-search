//! Deterministic source-document tuple verification.

use thiserror::Error;

use crate::{
    calendar_date::CalendarDate,
    source_date,
    source_document_heading::heading_sections,
    source_document_html::elements_with_attribute,
    source_fact::{SourceFact, SourceFactCandidate},
    source_fetch::{FetchedSource, SafeSourceUrl},
};

#[path = "source_document_sections.rs"]
mod source_document_sections;

use source_document_sections::{SourceSection, paired_first_rows};

#[derive(Debug)]
pub(crate) struct SourceDocument {
    url: SafeSourceUrl,
    sections: Vec<SourceSection>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct CandidateBinding<'a> {
    source_url: &'a SafeSourceUrl,
    scope: &'a str,
    value: &'a str,
    source_date_text: &'a str,
}

#[derive(Debug, Error)]
pub(crate) enum SourceDocumentError {
    #[error("source document structure was invalid")]
    InvalidStructure,
    #[error("source candidate fields were invalid")]
    InvalidCandidate,
    #[error("source date text did not equal the normalized candidate date")]
    DateMismatch,
    #[error("source candidate tuple was not bound in one source section")]
    MissingBinding,
    #[error("source candidate tuple was ambiguous across source sections")]
    AmbiguousBinding,
}

impl<'a> CandidateBinding<'a> {
    pub(crate) fn new(
        source_url: &'a SafeSourceUrl,
        scope: &'a str,
        value: &'a str,
        date: &'a CalendarDate,
        source_date_text: &'a str,
    ) -> Result<Self, SourceDocumentError> {
        if [scope, value, source_date_text]
            .iter()
            .any(|field| field.trim().is_empty() || field.trim() != *field)
        {
            return Err(SourceDocumentError::InvalidCandidate);
        }
        let parsed_date = source_date::parse(source_date_text)
            .map_err(|_| SourceDocumentError::InvalidCandidate)?;
        if &parsed_date != date {
            return Err(SourceDocumentError::DateMismatch);
        }
        Ok(Self {
            source_url,
            scope,
            value,
            source_date_text,
        })
    }

    pub(crate) const fn source_url(&self) -> &SafeSourceUrl {
        self.source_url
    }
}

impl SourceDocument {
    pub(crate) fn parse(fetched: FetchedSource) -> Result<Self, SourceDocumentError> {
        let (url, body) = fetched.into_parts();
        Self::from_text(url, &body)
    }

    pub(crate) fn from_text(url: SafeSourceUrl, body: &str) -> Result<Self, SourceDocumentError> {
        if body.trim().is_empty() {
            return Err(SourceDocumentError::InvalidStructure);
        }
        let tabs = elements_with_attribute(body, "data-tab")
            .map_err(|_| SourceDocumentError::InvalidStructure)?;
        let panels = elements_with_attribute(body, "data-list-panel")
            .map_err(|_| SourceDocumentError::InvalidStructure)?;
        let sections = if panels.is_empty() {
            let headings =
                heading_sections(body).map_err(|_| SourceDocumentError::InvalidStructure)?;
            if headings.is_empty() {
                return Err(SourceDocumentError::InvalidStructure);
            }
            headings
                .into_iter()
                .map(|(scope, evidence)| SourceSection::weak(scope, evidence))
                .collect::<Result<Vec<_>, _>>()?
        } else {
            paired_first_rows(&url, tabs, panels)?
        };
        Ok(Self { url, sections })
    }

    pub(crate) const fn url(&self) -> &SafeSourceUrl {
        &self.url
    }

    pub(crate) fn verify(
        &self,
        candidate: &CandidateBinding<'_>,
    ) -> Result<(), SourceDocumentError> {
        let matches = self
            .sections
            .iter()
            .filter(|section| {
                section.scope == candidate.scope
                    && contains_exact_field(&section.evidence, candidate.value)
                    && contains_exact_field(&section.evidence, candidate.source_date_text)
            })
            .count();
        match matches {
            1 => Ok(()),
            0 => Err(SourceDocumentError::MissingBinding),
            _ => Err(SourceDocumentError::AmbiguousBinding),
        }
    }

    pub(crate) fn exact_fact(
        &self,
        scope: &str,
    ) -> Result<Option<SourceFact>, SourceDocumentError> {
        let mut found = None;
        for section in self
            .sections
            .iter()
            .filter(|section| section.scope == scope)
        {
            match &section.fact {
                SourceFactCandidate::Fact(fact) if found.is_none() => found = Some(fact.clone()),
                SourceFactCandidate::Fact(_) | SourceFactCandidate::Ambiguous => {
                    return Err(SourceDocumentError::AmbiguousBinding);
                }
                SourceFactCandidate::NotApplicable => {}
            }
        }
        Ok(found)
    }
}

fn contains_exact_field(haystack: &str, needle: &str) -> bool {
    haystack.match_indices(needle).any(|(start, _)| {
        let end = start + needle.len();
        let before = haystack.get(..start).unwrap_or_default();
        let after = haystack.get(end..).unwrap_or_default();
        !continues_before(before) && !continues_after(after)
    })
}

fn continues_before(prefix: &str) -> bool {
    let mut reversed = prefix.chars().rev();
    reversed.next().is_some_and(|character| {
        character.is_alphanumeric()
            || (is_joiner(character) && reversed.next().is_some_and(char::is_alphanumeric))
    })
}

fn continues_after(suffix: &str) -> bool {
    let mut characters = suffix.chars();
    characters.next().is_some_and(|character| {
        character.is_alphanumeric()
            || (is_joiner(character) && characters.next().is_some_and(char::is_alphanumeric))
    })
}

const fn is_joiner(character: char) -> bool {
    matches!(character, '.' | '-' | '_' | '+')
}
