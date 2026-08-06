//! Schema-constrained content request dispatch.

use std::sync::Arc;

use crate::{
    antigravity_version::Deadline,
    error::AgyError,
    prompt::{build_prompt, build_standard_search_retry_prompt},
    request::ContentRequest,
    response::Document as ResponseDocument,
    source_verification::VerifiedSources,
    types::{Effort, ModelSlug, Operation, ResearchToolPolicy},
    verification::{TemporalAssessment, TemporalRecoveryPlan, assess_search},
};

mod execution;
mod temporal;

use execution::{
    ContentExecution, ExecutionContext, StandardSearchRun, run_content_once,
    run_standard_search_unvalidated_once,
};
use temporal::recover_temporal;

#[cfg(test)]
use crate::verification::ScopeLabel;
#[cfg(test)]
use temporal::{MAX_RECOVERY_CONCURRENCY, run_bounded};

#[cfg(test)]
#[path = "content/content_test.rs"]
mod tests;

pub(super) async fn execute(
    executable: &str,
    model: Option<ModelSlug>,
    effort: Option<Effort>,
    deadline: Deadline,
    request: ContentRequest,
) -> Result<ResponseDocument, AgyError> {
    let context = ExecutionContext {
        executable: executable.to_owned(),
        model,
        effort,
        deadline,
    };
    let operation = request.operation();
    let schema = ResponseDocument::schema(
        operation,
        request.verification(),
        request.temporal_contract(),
        request.source_restriction(),
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

async fn run_standard_search(
    context: &ExecutionContext,
    operation: Operation,
    tool_policy: ResearchToolPolicy,
    schema: String,
    prompt: String,
    request_json: &str,
) -> Result<ResponseDocument, AgyError> {
    let first = validate_standard_run(
        run_standard_search_unvalidated_once(
            context,
            ContentExecution {
                operation,
                tool_policy: tool_policy.clone(),
                schema: schema.clone(),
                prompt,
            },
        )
        .await?,
    )?;
    match first {
        StandardSearchRun::Response(response) => Ok(response),
        StandardSearchRun::NoReachableResults | StandardSearchRun::RecoverableUnlistedTool => {
            let second = validate_standard_run(
                run_standard_search_unvalidated_once(
                    context,
                    ContentExecution {
                        operation,
                        tool_policy,
                        schema,
                        prompt: build_standard_search_retry_prompt(request_json),
                    },
                )
                .await?,
            )?;
            match second {
                StandardSearchRun::Response(response) => Ok(response),
                StandardSearchRun::NoReachableResults
                | StandardSearchRun::RecoverableUnlistedTool => Err(AgyError::OutputInvalid),
            }
        }
    }
}

fn validate_standard_run(mut run: StandardSearchRun) -> Result<StandardSearchRun, AgyError> {
    if let StandardSearchRun::Response(response) = &mut run {
        response
            .validate_search_document()
            .map_err(|_| AgyError::OutputInvalid)?;
        response.project_unbound_standard_search_dates()?;
    }
    Ok(run)
}
