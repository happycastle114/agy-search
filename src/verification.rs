//! Request-aware temporal evidence validation and recovery merge.

use crate::{
    error::AgyError,
    response_models::{EvidenceAudit, SearchResponse},
    temporal_contract::TemporalContract,
    types::{HttpUrl, NonEmptyText, VerificationMode},
};

mod assessment;
mod recovery;
mod research;
mod schema;
#[cfg(test)]
mod temporal_assessment_test;
#[cfg(test)]
mod temporal_fixtures_test;
#[cfg(test)]
mod temporal_recovery_test;

pub(crate) use crate::temporal_contract::ScopeLabel;
pub(super) use assessment::{
    PublicEvidence, candidate_has_complete_fields, candidate_is_grounded, public_source_matches,
    unique_latest_candidate,
};
use assessment::{assess_temporal, assess_temporal_evidence};
pub(crate) use recovery::{
    TemporalRecoveryPlan, merge_verified_scopes, validate_scope_result,
    verified_scope_from_source_fact,
};
use research::validate_research_temporal;
pub(crate) use schema::{require_temporal_fields, require_temporal_schema_for_operation};
#[derive(Debug)]
pub(crate) enum TemporalAssessment {
    Verified,
    Recoverable(TemporalRecoveryPlan),
    Invalid,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum TemporalEvidenceOperation {
    Search,
    Research,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TemporalUpdateBinding {
    Absent,
    Unbound,
}

impl TemporalUpdateBinding {
    pub(crate) const fn from_last_updated(last_updated: Option<&str>) -> Self {
        match last_updated {
            Some(_) => Self::Unbound,
            None => Self::Absent,
        }
    }

    pub(crate) const fn is_unbound(self) -> bool {
        matches!(self, Self::Unbound)
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct TemporalValidation<'a> {
    verification: VerificationMode,
    contract: Option<&'a TemporalContract>,
    operation: TemporalEvidenceOperation,
}

impl<'a> TemporalValidation<'a> {
    pub(crate) const fn new(
        verification: VerificationMode,
        contract: Option<&'a TemporalContract>,
        operation: TemporalEvidenceOperation,
    ) -> Self {
        Self {
            verification,
            contract,
            operation,
        }
    }
}

pub(crate) fn assess_search(
    response: &SearchResponse,
    verification: VerificationMode,
    temporal_contract: Option<&TemporalContract>,
) -> TemporalAssessment {
    match (verification, temporal_contract) {
        (VerificationMode::Standard, None) => TemporalAssessment::Verified,
        (VerificationMode::TemporalComparison, Some(contract)) => assess_temporal(
            &response.evidence_audit,
            response.results.iter().collect::<Vec<_>>().as_slice(),
            contract,
        ),
        (VerificationMode::Standard, Some(_)) | (VerificationMode::TemporalComparison, None) => {
            TemporalAssessment::Invalid
        }
    }
}

pub(crate) fn validate_verification<'a>(
    audit: &EvidenceAudit,
    public_sources: impl Iterator<
        Item = (
            &'a NonEmptyText,
            &'a NonEmptyText,
            &'a HttpUrl,
            Option<&'a str>,
            Option<&'a str>,
        ),
    >,
    validation: TemporalValidation<'_>,
) -> Result<(), AgyError> {
    match (validation.verification, validation.contract) {
        (VerificationMode::Standard, None) => Ok(()),
        (VerificationMode::TemporalComparison, Some(contract)) => {
            let sources = public_sources
                .map(|(title, snippet, url, date, last_updated)| PublicEvidence {
                    title,
                    snippet,
                    url,
                    date,
                    update_binding: TemporalUpdateBinding::from_last_updated(last_updated),
                })
                .collect::<Vec<_>>();
            match validation.operation {
                TemporalEvidenceOperation::Search => {
                    match assess_temporal_evidence(audit, &sources, contract) {
                        TemporalAssessment::Verified => Ok(()),
                        TemporalAssessment::Recoverable(_) | TemporalAssessment::Invalid => {
                            Err(AgyError::OutputInvalid)
                        }
                    }
                }
                TemporalEvidenceOperation::Research => {
                    validate_research_temporal(audit, &sources, contract)
                }
            }
        }
        (VerificationMode::Standard, Some(_)) | (VerificationMode::TemporalComparison, None) => {
            Err(AgyError::OutputInvalid)
        }
    }
}
