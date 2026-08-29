//! Local response verification and evidence projection.

use std::sync::Arc;

use crate::{
    error::AgyError,
    request::ContentRequest,
    research_source_evidence::SourceEvidenceSnapshot,
    response::Document as ResponseDocument,
    source_verification::VerifiedSources,
    types::{NonEmptyText, VerificationMode},
    verification::{TemporalAssessment, TemporalRecoveryPlan, assess_search},
};

use super::{execution::ExecutionContext, temporal::recover_temporal};

pub(super) async fn verify_response(
    context: ExecutionContext,
    request: ContentRequest,
    mut response: ResponseDocument,
    prefetched_evidence: Option<&SourceEvidenceSnapshot>,
) -> Result<ResponseDocument, AgyError> {
    match &request {
        ContentRequest::Search(search) => {
            let ResponseDocument::Search(result) = &response else {
                return Err(AgyError::OutputInvalid);
            };
            match assess_search(
                result,
                search.verification,
                search.temporal_contract.as_ref(),
            ) {
                TemporalAssessment::Verified => {
                    response.validate_request(&request)?;
                    let Some(contract) = search.temporal_contract.as_ref() else {
                        return Ok(response);
                    };
                    let sources = Arc::new(
                        VerifiedSources::fetch(contract, context.deadline.instant()).await?,
                    );
                    if sources.verify_audit(&result.evidence_audit).is_ok() {
                        Ok(response)
                    } else {
                        recover_temporal(
                            context,
                            search,
                            TemporalRecoveryPlan::from_contract(contract),
                            sources,
                            result.evidence_audit.clone(),
                        )
                        .await
                    }
                }
                TemporalAssessment::Invalid => Err(AgyError::OutputInvalid),
                TemporalAssessment::Recoverable(plan) => {
                    let contract = search
                        .temporal_contract
                        .as_ref()
                        .ok_or(AgyError::OutputInvalid)?;
                    let sources = Arc::new(
                        VerifiedSources::fetch(contract, context.deadline.instant()).await?,
                    );
                    recover_temporal(
                        context,
                        search,
                        plan,
                        sources,
                        result.evidence_audit.clone(),
                    )
                    .await
                }
            }
        }
        ContentRequest::Extract(extract) => {
            response.validate_request(&request)?;
            let ResponseDocument::Extract(result) = &mut response else {
                return Err(AgyError::OutputInvalid);
            };
            prefetched_evidence
                .ok_or(AgyError::OutputInvalid)?
                .verify_and_project_extract(
                    result,
                    extract.query.as_ref().map(NonEmptyText::as_str),
                )?;
            Ok(response)
        }
        ContentRequest::Map(_) | ContentRequest::Crawl(_) => {
            response.validate_request(&request)?;
            Ok(response)
        }
        ContentRequest::Research(research) => {
            if research.verification == VerificationMode::Standard {
                response.project_unbound_standard_research_dates()?;
            }
            response.validate_request(&request)?;
            if research.verification == VerificationMode::Standard {
                let ResponseDocument::Research(result) = &mut response else {
                    return Err(AgyError::OutputInvalid);
                };
                verify_standard_research_sources(
                    &context,
                    research.source_restriction.is_exact_only(),
                    result,
                    prefetched_evidence,
                )
                .await?;
                return Ok(response);
            }
            let Some(contract) = research.temporal_contract.as_ref() else {
                return Ok(response);
            };
            let sources = VerifiedSources::fetch(contract, context.deadline.instant()).await?;
            let ResponseDocument::Research(result) = &response else {
                return Err(AgyError::OutputInvalid);
            };
            sources.verify_audit(&result.evidence_audit)?;
            Ok(response)
        }
    }
}

async fn verify_standard_research_sources(
    context: &ExecutionContext,
    exact_only: bool,
    result: &mut crate::response_models::ResearchResponse,
    prefetched_evidence: Option<&SourceEvidenceSnapshot>,
) -> Result<(), AgyError> {
    if exact_only {
        return prefetched_evidence
            .ok_or(AgyError::OutputInvalid)?
            .verify_and_project(result);
    }
    SourceEvidenceSnapshot::fetch(result, context.deadline.instant())
        .await?
        .verify_and_project(result)
}
