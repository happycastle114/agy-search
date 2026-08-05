//! Research-only temporal evidence-set validation without Search recovery semantics.

use crate::{
    error::AgyError,
    response_models::{EvidenceAudit, ScopeEvidence},
    temporal_contract::TemporalContract,
};

use super::{
    PublicEvidence, ScopeLabel, candidate_has_complete_fields, candidate_is_grounded,
    unique_latest_candidate,
};

pub(super) fn validate_research_temporal(
    audit: &EvidenceAudit,
    sources: &[PublicEvidence<'_>],
    contract: &TemporalContract,
) -> Result<(), AgyError> {
    let evidence = ResearchEvidenceSet::parse(audit, sources, contract)?;
    if evidence.candidates_have_public_text()
        && evidence.sources_have_audited_dates()
        && evidence.latest_has_exact_public_source()
    {
        Ok(())
    } else {
        Err(AgyError::OutputInvalid)
    }
}

struct ResearchEvidenceSet<'candidate, 'source> {
    candidates: &'candidate [ScopeEvidence],
    sources: &'source [PublicEvidence<'source>],
    latest: &'candidate ScopeEvidence,
}

impl<'candidate, 'source> ResearchEvidenceSet<'candidate, 'source> {
    fn parse(
        audit: &'candidate EvidenceAudit,
        sources: &'source [PublicEvidence<'source>],
        contract: &TemporalContract,
    ) -> Result<Self, AgyError> {
        let scopes = audit
            .candidates
            .iter()
            .map(|candidate| ScopeLabel::parse(&candidate.scope))
            .collect::<Result<Vec<_>, _>>()?;
        let latest = unique_latest_candidate(&audit.candidates).ok_or(AgyError::OutputInvalid)?;
        if sources.iter().any(PublicEvidence::has_unbound_update)
            || !contract.has_exact_scopes(&scopes)
            || !audit
                .candidates
                .iter()
                .all(|candidate| contract.allows_source(&candidate.url))
            || !sources
                .iter()
                .all(|source| contract.allows_source(source.url))
            || !audit.candidates.iter().all(candidate_has_complete_fields)
            || !audit.candidates.iter().all(candidate_is_grounded)
            || !audit.candidates.iter().all(|candidate| {
                candidate
                    .date
                    .as_ref()
                    .is_some_and(|date| contract.allows_date(date))
            })
        {
            return Err(AgyError::OutputInvalid);
        }
        Ok(Self {
            candidates: &audit.candidates,
            sources,
            latest,
        })
    }

    fn candidates_have_public_text(&self) -> bool {
        self.candidates.iter().all(|candidate| {
            self.sources.iter().any(|source| {
                source.url == &candidate.url
                    && source_text_contains(source, candidate.value.as_ref())
                    && source_text_contains(source, candidate.source_date_text.as_ref())
            })
        })
    }

    fn sources_have_audited_dates(&self) -> bool {
        self.sources.iter().all(|source| {
            source
                .date
                .and_then(|date| crate::types::CalendarDate::parse(date).ok())
                .is_some_and(|date| {
                    self.candidates.iter().any(|candidate| {
                        source.url == &candidate.url
                            && candidate.date.as_ref() == Some(&date)
                            && source_text_contains(source, candidate.value.as_ref())
                    })
                })
        })
    }

    fn latest_has_exact_public_source(&self) -> bool {
        self.sources.iter().any(|source| {
            source.url == &self.latest.url
                && source
                    .date
                    .and_then(|date| crate::types::CalendarDate::parse(date).ok())
                    .as_ref()
                    == self.latest.date.as_ref()
                && source_text_contains(source, self.latest.value.as_ref())
        })
    }
}

fn source_text_contains(
    source: &PublicEvidence<'_>,
    expected: Option<&crate::types::NonEmptyText>,
) -> bool {
    let Some(expected) = expected else {
        return false;
    };
    let text = format!("{} {}", source.title.as_str(), source.snippet.as_str()).to_lowercase();
    text.contains(&expected.as_str().to_lowercase())
}
