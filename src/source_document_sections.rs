//! Source-document section construction for paired tabs and panels.

use std::collections::{HashMap, HashSet};

use crate::{
    source_document::SourceDocumentError,
    source_document_html::{AttributedElement, elements_with_attribute, normalize_text},
    source_fact::{SourceFact, SourceFactCandidate},
    source_fetch::SafeSourceUrl,
};

const MAX_SECTION_BYTES: usize = 8 * 1024;

#[derive(Debug)]
pub(super) struct SourceSection {
    pub(super) scope: String,
    pub(super) evidence: String,
    pub(super) fact: SourceFactCandidate,
}

pub(super) fn paired_first_rows(
    source_url: &SafeSourceUrl,
    tabs: Vec<AttributedElement>,
    panels: Vec<AttributedElement>,
) -> Result<Vec<SourceSection>, SourceDocumentError> {
    let mut labels = HashMap::new();
    for tab in tabs {
        let key = tab.value.ok_or(SourceDocumentError::InvalidStructure)?;
        let label = normalize_text(&tab.content);
        if label.is_empty() || labels.insert(key, label).is_some() {
            return Err(SourceDocumentError::InvalidStructure);
        }
    }
    let mut seen = HashSet::new();
    let mut sections = Vec::new();
    for panel in panels {
        let key = panel.value.ok_or(SourceDocumentError::InvalidStructure)?;
        if !seen.insert(key.clone()) {
            return Err(SourceDocumentError::InvalidStructure);
        }
        let Some(label) = labels.get(&key) else {
            continue;
        };
        let rows = elements_with_attribute(&panel.content, "data-section-row")
            .map_err(|_| SourceDocumentError::InvalidStructure)?;
        let first = rows.first().ok_or(SourceDocumentError::InvalidStructure)?;
        sections.push(SourceSection::strong(
            source_url,
            label.clone(),
            &first.content,
        )?);
    }
    if sections.is_empty() {
        return Err(SourceDocumentError::InvalidStructure);
    }
    Ok(sections)
}

impl SourceSection {
    pub(super) fn weak(scope: String, evidence: String) -> Result<Self, SourceDocumentError> {
        Self::new(scope, evidence, SourceFactCandidate::NotApplicable)
    }

    fn strong(
        source_url: &SafeSourceUrl,
        scope: String,
        row: &str,
    ) -> Result<Self, SourceDocumentError> {
        let evidence = normalize_text(row);
        let fact = SourceFact::from_first_row(source_url, &scope, row);
        Self::new(scope, evidence, fact)
    }

    fn new(
        scope: String,
        evidence: String,
        fact: SourceFactCandidate,
    ) -> Result<Self, SourceDocumentError> {
        if scope.is_empty()
            || evidence.is_empty()
            || scope.len().saturating_add(evidence.len()) > MAX_SECTION_BYTES
        {
            return Err(SourceDocumentError::InvalidStructure);
        }
        Ok(Self {
            scope,
            evidence,
            fact,
        })
    }
}
