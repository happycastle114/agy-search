//! Bounded curl retrieval for a DNS-pinned caller source.

use std::path::PathBuf;

use thiserror::Error;
use tokio::{task::JoinSet, time::Instant};

use crate::source_network::{SourceNetworkError, resolve};

#[cfg(test)]
mod source_fetch_test;
mod transport;

pub(crate) use crate::source_network::{PinnedSource, SafeSourceUrl};

const MAX_FETCH_CONCURRENCY: usize = 4;

#[derive(Debug)]
pub(crate) struct FetchedSource {
    url: SafeSourceUrl,
    body: String,
}

#[derive(Clone, Debug)]
pub(crate) struct SourceFetcher {
    curl_path: PathBuf,
}

#[derive(Debug, Error)]
pub(crate) enum SourceFetchError {
    #[error("source URL or DNS answer was unsafe")]
    Network(#[from] SourceNetworkError),
    #[error("source deadline elapsed")]
    Deadline,
    #[error("source transport was unavailable")]
    Unavailable,
    #[error("source transport failed")]
    ProcessFailed,
    #[error("source response exceeded its capture bound")]
    Oversize,
    #[error("source transport response was invalid")]
    InvalidResponse,
    #[error("source body was not UTF-8")]
    InvalidUtf8,
    #[error("source body was empty")]
    EmptyBody,
    #[error("source fetch task failed")]
    TaskFailed,
}

impl FetchedSource {
    pub(crate) fn into_parts(self) -> (SafeSourceUrl, String) {
        (self.url, self.body)
    }
}

impl SourceFetcher {
    pub(crate) const fn new(curl_path: PathBuf) -> Self {
        Self { curl_path }
    }

    pub(crate) async fn fetch(
        &self,
        url: &SafeSourceUrl,
        deadline: Instant,
    ) -> Result<FetchedSource, SourceFetchError> {
        let pinned = resolve(url.clone(), deadline).await?;
        self.fetch_pinned(&pinned, deadline).await
    }

    pub(crate) async fn fetch_many(
        &self,
        sources: &[SafeSourceUrl],
        deadline: Instant,
    ) -> Result<Vec<FetchedSource>, SourceFetchError> {
        self.fetch_many_available(sources, deadline)
            .await?
            .into_iter()
            .collect()
    }

    pub(crate) async fn fetch_many_available(
        &self,
        sources: &[SafeSourceUrl],
        deadline: Instant,
    ) -> Result<Vec<Result<FetchedSource, SourceFetchError>>, SourceFetchError> {
        let mut fetched = Vec::with_capacity(sources.len());
        for batch in sources.chunks(MAX_FETCH_CONCURRENCY) {
            let mut tasks = JoinSet::new();
            for (index, source) in batch.iter().enumerate() {
                let worker = self.clone();
                let source = source.clone();
                tasks.spawn(async move { (index, worker.fetch(&source, deadline).await) });
            }
            let mut completed = std::collections::BTreeMap::new();
            while let Some(result) = tasks.join_next().await {
                let (index, response) = result.map_err(|_| SourceFetchError::TaskFailed)?;
                completed.insert(index, response);
            }
            if completed.len() != batch.len() {
                return Err(SourceFetchError::TaskFailed);
            }
            for response in completed.into_values() {
                fetched.push(response);
            }
        }
        Ok(fetched)
    }

    pub(crate) async fn fetch_pinned(
        &self,
        source: &PinnedSource,
        deadline: Instant,
    ) -> Result<FetchedSource, SourceFetchError> {
        transport::fetch_pinned(&self.curl_path, source, deadline).await
    }
}
