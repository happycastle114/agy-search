//! Bounded all-or-nothing multi-URL extraction.

use std::collections::BTreeMap;

use tokio::task::JoinSet;

use crate::{
    error::AgyError,
    request::{ContentRequest, ExtractRequest},
    response::Document as ResponseDocument,
    response_models::{ExtractObject, ExtractResponse},
};

use super::{execute_one, execution::ExecutionContext};

const MAX_EXTRACT_CONCURRENCY: usize = 4;

pub(super) async fn execute_extract_many(
    context: ExecutionContext,
    request: ExtractRequest,
) -> Result<ResponseDocument, AgyError> {
    let mut pages = BTreeMap::new();
    for (batch_index, batch) in request.urls.chunks(MAX_EXTRACT_CONCURRENCY).enumerate() {
        let mut tasks = JoinSet::new();
        for (offset, url) in batch.iter().cloned().enumerate() {
            let worker = context.clone();
            let query = request.query.clone();
            let index = batch_index * MAX_EXTRACT_CONCURRENCY + offset;
            tasks.spawn(async move {
                let response = execute_one(
                    worker,
                    ContentRequest::Extract(ExtractRequest {
                        urls: vec![url],
                        query,
                    }),
                )
                .await?;
                let ResponseDocument::Extract(mut extract) = response else {
                    return Err(AgyError::OutputInvalid);
                };
                if extract.results.len() != 1 {
                    return Err(AgyError::OutputInvalid);
                }
                Ok((index, extract.results.remove(0)))
            });
        }
        while let Some(result) = tasks.join_next().await {
            let (index, page) = result.map_err(|_| AgyError::OutputInvalid)??;
            if pages.insert(index, page).is_some() {
                return Err(AgyError::OutputInvalid);
            }
        }
    }
    if pages.len() != request.urls.len() {
        return Err(AgyError::OutputInvalid);
    }
    Ok(ResponseDocument::Extract(ExtractResponse {
        object: ExtractObject::Extract,
        results: pages.into_values().collect(),
    }))
}
