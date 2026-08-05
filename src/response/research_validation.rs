//! Research-specific response topology and citation validation.

use std::collections::HashSet;

use crate::{error::AgyError, response_models::ResearchResponse};

use super::{
    collection_validation::{bounded, evidence_audit, unique},
    public_dates,
};

pub(super) fn validate(response: &ResearchResponse) -> Result<(), AgyError> {
    bounded(response.findings.len(), 20)?;
    bounded(response.sources.len(), 20)?;
    for finding in &response.findings {
        bounded(finding.citations.len(), 20)?;
    }
    unique(response.sources.iter().map(|item| &item.url))?;
    let sources: HashSet<_> = response.sources.iter().map(|item| &item.url).collect();
    if !response
        .findings
        .iter()
        .flat_map(|item| &item.citations)
        .all(|url| sources.contains(url))
    {
        return Err(AgyError::OutputInvalid);
    }
    public_dates::validate_syntax(&response.sources)?;
    evidence_audit(&response.evidence_audit, sources.into_iter())
}
