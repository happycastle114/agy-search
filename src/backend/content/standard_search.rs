//! Standard Search retry orchestration and response validation.

use crate::{
    error::AgyError,
    prompt::{build_standard_search_final_retry_prompt, build_standard_search_retry_prompt},
    response::Document as ResponseDocument,
    types::{Operation, ResearchToolPolicy},
};

use super::execution::{
    ContentExecution, ExecutionContext, StandardSearchRun, run_standard_search_unvalidated_once,
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
    )?;
    match first {
        StandardSearchRun::Response(response) => Ok(response),
        StandardSearchRun::NoReachableResults | StandardSearchRun::RecoverableUnlistedTool => {
            let retry_context = context.for_standard_retry();
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
            )?;
            match second {
                StandardSearchRun::Response(response) => Ok(response),
                StandardSearchRun::NoReachableResults
                | StandardSearchRun::RecoverableUnlistedTool => {
                    let third = validate_standard_run(
                        run_standard_search_unvalidated_once(
                            &retry_context,
                            ContentExecution {
                                operation,
                                tool_policy,
                                schema,
                                prompt: build_standard_search_final_retry_prompt(request_json),
                            },
                        )
                        .await?,
                    )?;
                    match third {
                        StandardSearchRun::Response(response) => Ok(response),
                        StandardSearchRun::NoReachableResults
                        | StandardSearchRun::RecoverableUnlistedTool => {
                            Err(AgyError::OutputInvalid)
                        }
                    }
                }
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
