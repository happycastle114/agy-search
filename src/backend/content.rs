//! Schema-constrained content request dispatch.

use crate::{
    antigravity_version::Deadline,
    error::AgyError,
    prompt::{
        build_exact_research_retry_prompt, build_extract_retry_prompt, build_prompt,
        build_standard_research_retry_prompt,
    },
    request::ContentRequest,
    response::Document as ResponseDocument,
    source_restriction::SourceRestriction,
    types::Effort,
};

mod agent;
mod evidence;
mod execution;
mod extract;
mod primary;
mod standard_search;
mod temporal;
mod verification;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ResearchRecoveryMode {
    ExactPrefetched,
    SearchDiscovery,
}

const fn research_recovery_mode(restriction: &SourceRestriction) -> ResearchRecoveryMode {
    if restriction.is_exact_only() {
        ResearchRecoveryMode::ExactPrefetched
    } else {
        ResearchRecoveryMode::SearchDiscovery
    }
}

fn build_research_recovery_prompt(mode: ResearchRecoveryMode, request_json: &str) -> String {
    match mode {
        ResearchRecoveryMode::ExactPrefetched => build_exact_research_retry_prompt(request_json),
        ResearchRecoveryMode::SearchDiscovery => build_standard_research_retry_prompt(request_json),
    }
}

use evidence::{append_prefetched_evidence, is_standard_research, prefetch_exact_evidence};
use execution::{ContentExecution, ExecutionContext, RecoveryStage, run_content_once};
use extract::execute_extract_many;
use primary::run_primary;
use verification::verify_response;

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
    match request {
        ContentRequest::Extract(extract) if extract.urls.len() > 1 => {
            execute_extract_many(context, extract).await
        }
        request => execute_one(context, request).await,
    }
}

async fn execute_one(
    context: ExecutionContext,
    request: ContentRequest,
) -> Result<ResponseDocument, AgyError> {
    let operation = request.operation();
    let schema = ResponseDocument::schema(
        operation,
        request.verification(),
        request.temporal_contract(),
        request.source_restriction(),
        request.search_result_limit(),
    )?;
    let request_json = request.to_json().map_err(|_| AgyError::InvalidCommand)?;
    let prefetched_evidence = prefetch_exact_evidence(&context, &request).await?;
    let prompt = append_prefetched_evidence(
        build_prompt(operation, request.verification(), &request_json),
        prefetched_evidence.as_deref(),
        &request,
    )?;
    let first = async {
        let response = run_primary(
            &context,
            &request,
            ContentExecution {
                operation,
                tool_policy: request.tool_policy(),
                schema: schema.clone(),
                prompt,
            },
            &request_json,
            prefetched_evidence.as_deref(),
        )
        .await?;
        verify_response(
            context.clone(),
            request.clone(),
            response,
            prefetched_evidence.as_deref(),
        )
        .await
    }
    .await;
    match first {
        Err(AgyError::OutputInvalid) if matches!(request, ContentRequest::Extract(_)) => {
            let Some(retry_context) = context.for_recovery(RecoveryStage::Final) else {
                return Err(AgyError::OutputInvalid);
            };
            let prompt = append_prefetched_evidence(
                build_extract_retry_prompt(&request_json),
                prefetched_evidence.as_deref(),
                &request,
            )?;
            let response = run_content_once(
                &retry_context,
                ContentExecution {
                    operation,
                    tool_policy: request.tool_policy(),
                    schema,
                    prompt,
                },
            )
            .await?;
            verify_response(
                retry_context,
                request,
                response,
                prefetched_evidence.as_deref(),
            )
            .await
        }
        Err(AgyError::OutputInvalid) if is_standard_research(&request) => {
            let Some(retry_context) = context.for_recovery(RecoveryStage::Final) else {
                return Err(AgyError::OutputInvalid);
            };
            let prompt = build_research_recovery_prompt(
                research_recovery_mode(request.source_restriction()),
                &request_json,
            );
            let prompt =
                append_prefetched_evidence(prompt, prefetched_evidence.as_deref(), &request)?;
            let response = run_content_once(
                &retry_context,
                ContentExecution {
                    operation,
                    tool_policy: request.tool_policy(),
                    schema,
                    prompt,
                },
            )
            .await?;
            verify_response(
                retry_context,
                request,
                response,
                prefetched_evidence.as_deref(),
            )
            .await
        }
        result => result,
    }
}
