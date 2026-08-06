//! Shared response collection and evidence-audit invariants.

use std::collections::HashSet;

use crate::{
    error::AgyError,
    response_models::EvidenceAudit,
    types::{HttpUrl, SourceUrlKind},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum EvidenceAuditError {
    CandidateInvalid,
    PublicUrlMissing,
}

pub(super) fn bounded(length: usize, maximum: usize) -> Result<(), AgyError> {
    if (1..=maximum).contains(&length) {
        Ok(())
    } else {
        Err(AgyError::OutputInvalid)
    }
}

pub(super) fn unique<'a>(urls: impl Iterator<Item = &'a HttpUrl>) -> Result<(), AgyError> {
    let mut observed = HashSet::new();
    if urls.into_iter().all(|url| observed.insert(url)) {
        Ok(())
    } else {
        Err(AgyError::OutputInvalid)
    }
}

pub(super) fn evidence_audit<'a>(
    audit: &EvidenceAudit,
    mut public_urls: impl Iterator<Item = &'a HttpUrl>,
) -> Result<(), EvidenceAuditError> {
    if !(1..=20).contains(&audit.candidates.len())
        || audit
            .candidates
            .iter()
            .any(|item| item.url.source_kind() != SourceUrlKind::Direct)
    {
        return Err(EvidenceAuditError::CandidateInvalid);
    }
    let audited: HashSet<_> = audit.candidates.iter().map(|item| &item.url).collect();
    if public_urls.all(|url| audited.contains(url)) {
        Ok(())
    } else {
        Err(EvidenceAuditError::PublicUrlMissing)
    }
}
