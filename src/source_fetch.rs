//! Bounded curl retrieval for a DNS-pinned caller source.

use std::{future::Future, path::PathBuf};

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
        let requests = sources.iter().map(|source| {
            let worker = self.clone();
            let source = source.clone();
            async move { worker.fetch(&source, deadline).await }
        });
        collect_bounded(requests).await
    }

    pub(crate) async fn fetch_pinned(
        &self,
        source: &PinnedSource,
        deadline: Instant,
    ) -> Result<FetchedSource, SourceFetchError> {
        transport::fetch_pinned(&self.curl_path, source, deadline).await
    }
}

async fn collect_bounded<F, T>(
    requests: impl Iterator<Item = F>,
) -> Result<Vec<T>, SourceFetchError>
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    let mut remaining = requests.enumerate();
    let mut tasks = JoinSet::new();
    let mut completed = std::collections::BTreeMap::new();
    loop {
        while tasks.len() < MAX_FETCH_CONCURRENCY {
            let Some((index, request)) = remaining.next() else {
                break;
            };
            tasks.spawn(async move { (index, request.await) });
        }
        let Some(result) = tasks.join_next().await else {
            break;
        };
        let (index, response) = result.map_err(|_| SourceFetchError::TaskFailed)?;
        completed.insert(index, response);
    }
    Ok(completed.into_values().collect())
}
