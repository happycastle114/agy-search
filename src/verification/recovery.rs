//! Typed scope validation and all-or-nothing temporal merging.

use super::{candidate_has_complete_fields, candidate_is_grounded, public_source_matches};
use crate::{
    error::AgyError,
    request::{ContentRequest, SearchRequest},
    response::Document as ResponseDocument,
    response_models::{EvidenceAudit, ScopeEvidence, SearchObject, SearchResponse, WebSource},
    source_fact::SourceFact,
    temporal_contract::{ScopeLabel, TemporalContract},
    types::{HttpUrl, NonEmptyText},
};

#[derive(Clone, Debug)]
pub(crate) struct TemporalRecoveryPlan {
    scopes: Vec<ScopeLabel>,
}

impl TemporalRecoveryPlan {
    #[cfg(test)]
    pub(super) const fn new(scopes: Vec<ScopeLabel>) -> Self {
        Self { scopes }
    }

    pub(crate) fn from_contract(contract: &TemporalContract) -> Self {
        Self {
            scopes: contract.expected_scopes().to_vec(),
        }
    }

    pub(crate) fn scopes(&self) -> &[ScopeLabel] {
        &self.scopes
    }
}

#[derive(Clone, Debug)]
pub(crate) struct VerifiedScope {
    scope: ScopeLabel,
    candidate: ScopeEvidence,
    source: WebSource,
}

pub(crate) fn validate_scope_result(
    requested_scope: ScopeLabel,
    response: ResponseDocument,
    contract: &TemporalContract,
) -> Result<VerifiedScope, AgyError> {
    let ResponseDocument::Search(response) = response else {
        return Err(AgyError::OutputInvalid);
    };
    let [candidate] = response.evidence_audit.candidates.as_slice() else {
        return Err(AgyError::OutputInvalid);
    };
    let [source] = response.results.as_slice() else {
        return Err(AgyError::OutputInvalid);
    };
    let exact_scope = ScopeLabel::parse(&candidate.scope)?;
    if exact_scope != requested_scope
        || !contract.allows_source(&candidate.url)
        || !contract.allows_source(&source.url)
        || !candidate_has_complete_fields(candidate)
        || !candidate
            .date
            .as_ref()
            .is_some_and(|date| contract.allows_date(date))
        || !candidate_is_grounded(candidate)
        || !public_source_matches(candidate, source)
    {
        return Err(AgyError::OutputInvalid);
    }
    Ok(VerifiedScope {
        scope: requested_scope,
        candidate: candidate.clone(),
        source: source.clone(),
    })
}

pub(crate) fn verified_scope_from_source_fact(
    requested_scope: ScopeLabel,
    fact: &SourceFact,
    contract: &TemporalContract,
) -> Result<VerifiedScope, AgyError> {
    if fact.scope() != requested_scope.as_str() {
        return Err(AgyError::OutputInvalid);
    }
    if !contract.allows_date(fact.date()) {
        return Err(AgyError::OutputInvalid);
    }
    let url = HttpUrl::parse(fact.source_url().as_str()).map_err(|_| AgyError::OutputInvalid)?;
    if !contract.allows_source(&url) {
        return Err(AgyError::OutputInvalid);
    }
    let value = NonEmptyText::parse(fact.value()).map_err(|_| AgyError::OutputInvalid)?;
    let source_date_text =
        NonEmptyText::parse(fact.source_date_text()).map_err(|_| AgyError::OutputInvalid)?;
    let evidence_text = format!("{} {}", fact.value(), fact.source_date_text());
    let evidence = NonEmptyText::parse(&evidence_text).map_err(|_| AgyError::OutputInvalid)?;
    let title_text = format!("{} {}", fact.scope(), fact.value());
    let title = NonEmptyText::parse(&title_text).map_err(|_| AgyError::OutputInvalid)?;
    let snippet_text = format!(
        "{} {} {}",
        fact.scope(),
        fact.value(),
        fact.source_date_text()
    );
    let snippet = NonEmptyText::parse(&snippet_text).map_err(|_| AgyError::OutputInvalid)?;
    let candidate = ScopeEvidence {
        scope: NonEmptyText::parse(fact.scope()).map_err(|_| AgyError::OutputInvalid)?,
        claim: snippet.clone(),
        url: url.clone(),
        date: Some(fact.date().clone()),
        value: Some(value),
        source_date_text: Some(source_date_text),
        evidence_excerpt: Some(evidence),
    };
    let source = WebSource {
        title,
        url,
        snippet,
        date: Some(fact.date().to_string()),
        last_updated: None,
    };
    if !candidate_has_complete_fields(&candidate)
        || !candidate_is_grounded(&candidate)
        || !public_source_matches(&candidate, &source)
    {
        return Err(AgyError::OutputInvalid);
    }
    Ok(VerifiedScope {
        scope: requested_scope,
        candidate,
        source,
    })
}

pub(crate) fn merge_verified_scopes(
    request: &SearchRequest,
    plan: &TemporalRecoveryPlan,
    scopes: &[VerifiedScope],
) -> Result<ResponseDocument, AgyError> {
    if scopes.len() != plan.scopes.len()
        || !scopes
            .iter()
            .zip(&plan.scopes)
            .all(|(verified, planned)| &verified.scope == planned)
        || !scopes.iter().all(|verified| {
            verified.candidate.date.as_ref().is_some_and(|date| {
                request
                    .temporal_contract
                    .as_ref()
                    .is_some_and(|contract| contract.allows_date(date))
            })
        })
    {
        return Err(AgyError::OutputInvalid);
    }
    let latest = scopes
        .iter()
        .max_by_key(|verified| verified.candidate.date.as_ref())
        .ok_or(AgyError::OutputInvalid)?;
    let latest_date = latest
        .candidate
        .date
        .as_ref()
        .ok_or(AgyError::OutputInvalid)?;
    if scopes
        .iter()
        .filter(|verified| verified.candidate.date.as_ref() == Some(latest_date))
        .count()
        != 1
    {
        return Err(AgyError::OutputInvalid);
    }
    let document = ResponseDocument::Search(SearchResponse {
        object: SearchObject::Search,
        evidence_audit: EvidenceAudit {
            candidates: scopes
                .iter()
                .map(|verified| verified.candidate.clone())
                .collect(),
            coverage_complete: true,
            conclusion: latest.candidate.claim.clone(),
        },
        results: vec![latest.source.clone()],
    });
    document.validate()?;
    document.validate_request(&ContentRequest::Search(request.clone()))?;
    Ok(document)
}
