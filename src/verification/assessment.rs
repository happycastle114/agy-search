//! Primary temporal evidence assessment and public-result binding.

use super::{ScopeLabel, TemporalAssessment, TemporalRecoveryPlan, TemporalUpdateBinding};
use crate::{
    response_models::{EvidenceAudit, ScopeEvidence, WebSource},
    temporal_contract::TemporalContract,
    types::{CalendarDate, HttpUrl, NonEmptyText},
};

pub(crate) struct PublicEvidence<'a> {
    pub(crate) title: &'a NonEmptyText,
    pub(crate) snippet: &'a NonEmptyText,
    pub(crate) url: &'a HttpUrl,
    pub(crate) date: Option<&'a str>,
    pub(crate) update_binding: TemporalUpdateBinding,
}

impl PublicEvidence<'_> {
    pub(crate) const fn has_unbound_update(&self) -> bool {
        self.update_binding.is_unbound()
    }
}

pub(super) fn assess_temporal(
    audit: &EvidenceAudit,
    public_sources: &[&WebSource],
    contract: &TemporalContract,
) -> TemporalAssessment {
    let sources = public_sources
        .iter()
        .map(|source| PublicEvidence {
            title: &source.title,
            snippet: &source.snippet,
            url: &source.url,
            date: source.date.as_deref(),
            update_binding: TemporalUpdateBinding::from_last_updated(
                source.last_updated.as_deref(),
            ),
        })
        .collect::<Vec<_>>();
    assess_temporal_evidence(audit, &sources, contract)
}

pub(super) fn assess_temporal_evidence(
    audit: &EvidenceAudit,
    public_sources: &[PublicEvidence<'_>],
    contract: &TemporalContract,
) -> TemporalAssessment {
    let scopes = audit
        .candidates
        .iter()
        .map(|candidate| ScopeLabel::parse(&candidate.scope))
        .collect::<Result<Vec<_>, _>>();
    let Ok(scopes) = scopes else {
        return TemporalAssessment::Invalid;
    };
    if public_sources
        .iter()
        .any(PublicEvidence::has_unbound_update)
        || !contract.has_exact_scopes(&scopes)
        || !audit
            .candidates
            .iter()
            .all(|candidate| contract.allows_source(&candidate.url))
        || !public_sources
            .iter()
            .all(|source| contract.allows_source(source.url))
        || !audit.candidates.iter().all(candidate_has_complete_fields)
    {
        return TemporalAssessment::Invalid;
    }
    if !audit.candidates.iter().all(|candidate| {
        candidate
            .date
            .as_ref()
            .is_some_and(|date| contract.allows_date(date))
    }) {
        return TemporalAssessment::Recoverable(TemporalRecoveryPlan::from_contract(contract));
    }
    let candidates_are_grounded = audit.candidates.iter().all(candidate_is_grounded);
    let public_winner_matches =
        unique_latest_candidate(&audit.candidates).is_some_and(|candidate| {
            let [source] = public_sources else {
                return false;
            };
            public_source_matches_evidence(candidate, source)
        });
    if candidates_are_grounded && public_winner_matches {
        TemporalAssessment::Verified
    } else {
        TemporalAssessment::Recoverable(TemporalRecoveryPlan::from_contract(contract))
    }
}

pub(crate) const fn candidate_has_complete_fields(candidate: &ScopeEvidence) -> bool {
    candidate.date.is_some()
        && candidate.value.is_some()
        && candidate.source_date_text.is_some()
        && candidate.evidence_excerpt.is_some()
}

pub(crate) fn candidate_is_grounded(candidate: &ScopeEvidence) -> bool {
    let (Some(value), Some(source_date_text), Some(excerpt)) = (
        candidate.value.as_ref(),
        candidate.source_date_text.as_ref(),
        candidate.evidence_excerpt.as_ref(),
    ) else {
        return false;
    };
    let normalized = excerpt.as_str().to_lowercase();
    normalized.contains(&source_date_text.as_str().to_lowercase())
        && normalized.contains(&value.as_str().to_lowercase())
}

pub(crate) fn public_source_matches(candidate: &ScopeEvidence, source: &WebSource) -> bool {
    public_source_matches_evidence(
        candidate,
        &PublicEvidence {
            title: &source.title,
            snippet: &source.snippet,
            url: &source.url,
            date: source.date.as_deref(),
            update_binding: TemporalUpdateBinding::from_last_updated(
                source.last_updated.as_deref(),
            ),
        },
    )
}

fn public_source_matches_evidence(candidate: &ScopeEvidence, source: &PublicEvidence<'_>) -> bool {
    let text = format!("{} {}", source.title.as_str(), source.snippet.as_str()).to_lowercase();
    let public_date = source
        .date
        .and_then(|value| CalendarDate::parse(value).ok());
    candidate.date.as_ref() == public_date.as_ref()
        && &candidate.url == source.url
        && candidate
            .value
            .as_ref()
            .is_some_and(|value| text.contains(&value.as_str().to_lowercase()))
}

pub(crate) fn unique_latest_candidate(candidates: &[ScopeEvidence]) -> Option<&ScopeEvidence> {
    let latest = candidates
        .iter()
        .max_by_key(|candidate| candidate.date.as_ref())?;
    let latest_date = latest.date.as_ref()?;
    if candidates
        .iter()
        .filter(|candidate| candidate.date.as_ref() == Some(latest_date))
        .count()
        == 1
    {
        Some(latest)
    } else {
        None
    }
}
