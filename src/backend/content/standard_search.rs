//! Standard Search retry orchestration and response validation.

use crate::{
    error::AgyError,
    prompt::{build_standard_search_final_retry_prompt, build_standard_search_retry_prompt},
    research_source_evidence::SearchSourceEvidence,
    response::Document as ResponseDocument,
    types::{Operation, ResearchToolPolicy},
};

use super::execution::{
    ContentExecution, ExecutionContext, RecoveryStage, StandardSearchRun,
    run_standard_search_unvalidated_once,
};

pub(super) async fn run_standard_search(
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
        context,
    )
    .await?;
    match first {
        StandardSearchRun::Response(response) => Ok(response),
        StandardSearchRun::NoReachableResults
        | StandardSearchRun::RecoverableUnlistedTool
        | StandardSearchRun::RecoverableFailedWebTool => {
            let Some(retry_context) = context.for_recovery(RecoveryStage::First) else {
                return Err(AgyError::OutputInvalid);
            };
            let second = validate_standard_run(
                run_standard_search_unvalidated_once(
                    &retry_context,
                    ContentExecution {
                        operation,
                        tool_policy: tool_policy.clone(),
                        schema: schema.clone(),
                        prompt: build_standard_search_retry_prompt(request_json),
                    },
                )
                .await?,
                &retry_context,
            )
            .await?;
            match second {
                StandardSearchRun::Response(response) => Ok(response),
                StandardSearchRun::NoReachableResults
                | StandardSearchRun::RecoverableUnlistedTool
                | StandardSearchRun::RecoverableFailedWebTool => {
                    let Some(final_retry_context) = context.for_recovery(RecoveryStage::Final)
                    else {
                        return Err(AgyError::OutputInvalid);
                    };
                    let third = validate_standard_run(
                        run_standard_search_unvalidated_once(
                            &final_retry_context,
                            ContentExecution {
                                operation,
                                tool_policy,
                                schema,
                                prompt: build_standard_search_final_retry_prompt(request_json),
                            },
                        )
                        .await?,
                        &final_retry_context,
                    )
                    .await?;
                    match third {
                        StandardSearchRun::Response(response) => Ok(response),
                        StandardSearchRun::NoReachableResults
                        | StandardSearchRun::RecoverableUnlistedTool
                        | StandardSearchRun::RecoverableFailedWebTool => {
                            Err(AgyError::OutputInvalid)
                        }
                    }
                }
            }
        }
    }
}

async fn validate_standard_run(
    mut run: StandardSearchRun,
    context: &ExecutionContext,
) -> Result<StandardSearchRun, AgyError> {
    if let StandardSearchRun::Response(response) = &mut run {
        if response.validate_search_document().is_err()
            || response.project_unbound_standard_search_dates().is_err()
        {
            return Err(AgyError::OutputInvalid);
        }
        let ResponseDocument::Search(search) = response else {
            return Err(AgyError::OutputInvalid);
        };
        let evidence = match SearchSourceEvidence::fetch(search, context.deadline.instant()).await {
            Ok(evidence) => evidence,
            Err(AgyError::OutputInvalid) => return Ok(StandardSearchRun::NoReachableResults),
            Err(error) => return Err(error),
        };
        if evidence.verify_and_project(search).is_err() {
            return Ok(StandardSearchRun::NoReachableResults);
        }
    }
    Ok(run)
}
