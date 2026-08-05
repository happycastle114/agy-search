//! Request-owned temporal source fetching and audit tuple verification.

use std::path::PathBuf;

use tokio::time::Instant;

use crate::{
    error::AgyError,
    redirect::curl_executable,
    response_models::{EvidenceAudit, ScopeEvidence},
    source_contract::{SourceContract, SourceContractError},
    source_document::CandidateBinding,
    source_fact::SourceFact,
    source_fetch::{SafeSourceUrl, SourceFetchError, SourceFetcher},
    source_network::SourceNetworkError,
    temporal_contract::TemporalContract,
    verification::TemporalRecoveryPlan,
};

#[derive(Debug)]
pub(crate) struct VerifiedSources {
    contract: SourceContract,
}

#[derive(Debug)]
pub(crate) enum LocalFactRecovery {
    Complete(Vec<SourceFact>),
    Unsupported,
}

impl VerifiedSources {
    pub(crate) async fn fetch(
        temporal: &TemporalContract,
        deadline: Instant,
    ) -> Result<Self, AgyError> {
        let executable = PathBuf::from(curl_executable()?);
        let fetcher = SourceFetcher::new(executable);
        let contract = SourceContract::fetch(&fetcher, temporal.safe_source_urls(), deadline)
            .await
            .map_err(|error| map_source_error(&error))?;
        Ok(Self { contract })
    }

    pub(crate) fn verify_audit(&self, audit: &EvidenceAudit) -> Result<(), AgyError> {
        audit
            .candidates
            .iter()
            .try_for_each(|candidate| self.verify_candidate(candidate))
    }

    pub(crate) fn recover_local_facts(
        &self,
        plan: &TemporalRecoveryPlan,
        audit: &EvidenceAudit,
        cutoff: Option<&crate::types::CalendarDate>,
    ) -> Result<LocalFactRecovery, AgyError> {
        let mut facts = Vec::with_capacity(plan.scopes().len());
        for scope in plan.scopes() {
            let fact_result = if cutoff.is_some() {
                self.contract.unique_fact(scope.as_str())
            } else {
                let mut matching = audit
                    .candidates
                    .iter()
                    .filter(|candidate| candidate.scope.as_str() == scope.as_str());
                let candidate = matching.next().ok_or(AgyError::OutputInvalid)?;
                if matching.next().is_some() {
                    return Err(AgyError::OutputInvalid);
                }
                let value = candidate.value.as_ref().ok_or(AgyError::OutputInvalid)?;
                self.contract.exact_fact(scope.as_str(), value.as_str())
            };
            let fact = match fact_result {
                Ok(fact) => fact,
                Err(SourceContractError::Document(
                    crate::source_document::SourceDocumentError::AmbiguousBinding,
                )) => return Ok(LocalFactRecovery::Unsupported),
                Err(error) => return Err(map_source_error(&error)),
            };
            let Some(fact) = fact else {
                return Ok(LocalFactRecovery::Unsupported);
            };
            if cutoff.is_some_and(|cutoff| fact.date() > cutoff) {
                return Ok(LocalFactRecovery::Unsupported);
            }
            facts.push(fact);
        }
        Ok(LocalFactRecovery::Complete(facts))
    }

    fn verify_candidate(&self, candidate: &ScopeEvidence) -> Result<(), AgyError> {
        let source_url =
            SafeSourceUrl::parse(candidate.url.as_str()).map_err(|_| AgyError::OutputInvalid)?;
        let value = candidate.value.as_ref().ok_or(AgyError::OutputInvalid)?;
        let date = candidate.date.as_ref().ok_or(AgyError::OutputInvalid)?;
        let source_date_text = candidate
            .source_date_text
            .as_ref()
            .ok_or(AgyError::OutputInvalid)?;
        let binding = CandidateBinding::new(
            &source_url,
            candidate.scope.as_str(),
            value.as_str(),
            date,
            source_date_text.as_str(),
        )
        .map_err(|_| AgyError::OutputInvalid)?;
        self.contract
            .verify(&binding)
            .map_err(|error| map_source_error(&error))
    }
}

const fn map_source_error(error: &SourceContractError) -> AgyError {
    match error {
        SourceContractError::Fetch(fetch_error) => match fetch_error {
            SourceFetchError::Deadline
            | SourceFetchError::Network(SourceNetworkError::Deadline) => AgyError::Timeout,
            SourceFetchError::Unavailable => AgyError::Unavailable,
            SourceFetchError::Network(
                SourceNetworkError::InvalidUrl
                | SourceNetworkError::UnsafeAddress
                | SourceNetworkError::Dns,
            )
            | SourceFetchError::ProcessFailed
            | SourceFetchError::Oversize
            | SourceFetchError::InvalidResponse
            | SourceFetchError::InvalidUtf8
            | SourceFetchError::EmptyBody => AgyError::OutputInvalid,
        },
        SourceContractError::InvalidAllowlist
        | SourceContractError::Document(_)
        | SourceContractError::SourceNotAllowed
        | SourceContractError::TaskFailed => AgyError::OutputInvalid,
    }
}
