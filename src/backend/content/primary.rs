//! Initial content execution routing.

use crate::{
    error::AgyError, request::ContentRequest, research_source_evidence::SourceEvidenceSnapshot,
    response::Document as ResponseDocument, types::VerificationMode,
};

use super::{
    execution::{ContentExecution, ExecutionContext, run_content_once},
    standard_search::run_standard_search,
};

pub(super) async fn run_primary(
    context: &ExecutionContext,
    request: &ContentRequest,
    execution: ContentExecution,
    request_json: &str,
    prefetched_evidence: Option<&SourceEvidenceSnapshot>,
) -> Result<ResponseDocument, AgyError> {
    if let ContentRequest::Search(search) = request
        && search.verification == VerificationMode::Standard
    {
        return run_standard_search(context, execution, request_json, prefetched_evidence).await;
    }
    run_content_once(context, execution).await
}
