//! Schema-constrained content request dispatch.

use std::sync::Arc;

use crate::{
    antigravity_version::Deadline,
    error::AgyError,
    prompt::build_prompt,
    request::ContentRequest,
    response::Document as ResponseDocument,
    source_verification::VerifiedSources,
    types::{Effort, Operation},
    verification::{TemporalAssessment, TemporalRecoveryPlan, assess_search},
};

mod execution;
mod standard_search;
mod temporal;

use execution::{ContentExecution, ExecutionContext, run_content_once};
use standard_search::run_standard_search;
use temporal::recover_temporal;

use super::ContentModels;
#[cfg(test)]
use crate::verification::ScopeLabel;
#[cfg(test)]
use temporal::{MAX_RECOVERY_CONCURRENCY, run_bounded};

#[cfg(test)]
#[path = "content/content_test.rs"]
mod tests;

pub(super) async fn execute(
    executable: &str,
    models: ContentModels,
    effort: Option<Effort>,
    deadline: Deadline,
    request: ContentRequest,
) -> Result<ResponseDocument, AgyError> {
    let ContentModels {
        primary,
        recoveries,
    } = models;
    let context = ExecutionContext {
        executable: executable.to_owned(),
        model: primary,
        recoveries,
        effort,
        deadline,
    };
    let operation = request.operation();
    let schema = ResponseDocument::schema(
        operation,
        request.verification(),
        request.temporal_contract(),
        request.source_restriction(),
        request.search_result_limit(),
    )?;
    let request_json = request.to_json().map_err(|_| AgyError::InvalidCommand)?;
    let prompt = build_prompt(operation, request.verification(), &request_json);
    let response =
        run_primary(&context, &request, operation, schema, prompt, &request_json).await?;
    verify_response(context, request, response).await
}

async fn run_primary(
    context: &ExecutionContext,
    request: &ContentRequest,
    operation: Operation,
    schema: String,
    prompt: String,
    request_json: &str,
) -> Result<ResponseDocument, AgyError> {
    match request {
        ContentRequest::Search(search) => match search.verification {
            crate::types::VerificationMode::Standard => {
                run_standard_search(
                    context,
                    operation,
                    request.tool_policy(),
                    schema,
                    prompt,
                    request_json,
                )
                .await
            }
            crate::types::VerificationMode::TemporalComparison => {
                run_content_once(
                    context,
                    ContentExecution {
                        operation,
                        tool_policy: request.tool_policy(),
                        schema,
                        prompt,
                    },
                )
                .await
            }
        },
        ContentRequest::Extract(_)
        | ContentRequest::Map(_)
        | ContentRequest::Crawl(_)
        | ContentRequest::Research(_) => {
            run_content_once(
                context,
                ContentExecution {
                    operation,
                    tool_policy: request.tool_policy(),
                    schema,
                    prompt,
                },
            )
            .await
        }
    }
}

async fn verify_response(
    context: ExecutionContext,
    request: ContentRequest,
    response: ResponseDocument,
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
        ContentRequest::Extract(_) | ContentRequest::Map(_) | ContentRequest::Crawl(_) => {
            response.validate_request(&request)?;
            Ok(response)
        }
        ContentRequest::Research(research) => {
            response.validate_request(&request)?;
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
