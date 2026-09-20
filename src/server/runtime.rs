//! Shared bounded execution for both server transports.

use super::{
    ServerError,
    config::RuntimeOptions,
    input::{CrawlInput, ExtractInput, MapInput, ResearchInput, SearchInput},
};
use crate::{
    AgyError,
    backend::{self, DiscoveryCache},
    cli::{Cli, Command, OutputArgs},
    response::Document,
    response_models::SearchResponse,
    types::{Effort, VerificationMode},
};
use std::{collections::HashSet, sync::Arc, time::Duration};
use tokio::{sync::Semaphore, time};

#[derive(Clone, Debug)]
pub(crate) struct Runtime {
    inner: Arc<RuntimeState>,
}

#[derive(Debug)]
struct RuntimeState {
    options: RuntimeOptions,
    cache: DiscoveryCache,
    permits: Semaphore,
}

impl Runtime {
    pub(crate) fn new(options: RuntimeOptions) -> Result<Self, ServerError> {
        if options.agy_path.trim().is_empty() || options.max_concurrency.get() > 64 {
            return Err(ServerError::Configuration);
        }
        let state = RuntimeState {
            cache: DiscoveryCache::new(Duration::from_secs(options.catalog_ttl_seconds)),
            permits: Semaphore::new(usize::from(options.max_concurrency.get())),
            options,
        };
        Ok(Self {
            inner: Arc::new(state),
        })
    }

    pub(crate) async fn search(&self, input: SearchInput) -> Result<Document, ServerError> {
        let prepared = input.prepare()?;
        let _permit = self
            .inner
            .permits
            .try_acquire()
            .map_err(|_| ServerError::Busy)?;
        time::timeout(self.inner.options.timeout.duration(), async {
            let mut combined: Option<SearchResponse> = None;
            let mut retained = HashSet::new();
            for command in prepared.commands {
                let result = self.execute(command, prepared.effort).await?;
                let Document::Search(mut response) = result else {
                    return Err(ServerError::Backend(AgyError::OutputInvalid));
                };
                response
                    .results
                    .retain(|source| retained.insert(source.url.clone()));
                match &mut combined {
                    Some(combined) => combined.results.extend(response.results),
                    None => combined = Some(response),
                }
            }
            let mut response = combined.ok_or(ServerError::InvalidInput)?;
            response.results.truncate(prepared.max_results);
            Ok(Document::Search(response))
        })
        .await
        .map_err(|_| ServerError::Backend(AgyError::Timeout))?
    }

    pub(crate) async fn extract(&self, input: ExtractInput) -> Result<Document, ServerError> {
        self.run(input.into_command()?).await
    }
    pub(crate) async fn map(&self, input: MapInput) -> Result<Document, ServerError> {
        self.run(input.into_command()?).await
    }
    pub(crate) async fn crawl(&self, input: CrawlInput) -> Result<Document, ServerError> {
        self.run(input.into_command()?).await
    }
    pub(crate) async fn research(&self, input: ResearchInput) -> Result<Document, ServerError> {
        self.run(input.into_command()?).await
    }
    pub(crate) async fn readiness(&self) -> Result<Document, ServerError> {
        self.run(Command::Status(OutputArgs {
            output: None,
            _json: true,
        }))
        .await
    }

    async fn run(&self, command: Command) -> Result<Document, ServerError> {
        let _permit = self
            .inner
            .permits
            .try_acquire()
            .map_err(|_| ServerError::Busy)?;
        time::timeout(
            self.inner.options.timeout.duration(),
            self.execute(command, None),
        )
        .await
        .map_err(|_| ServerError::Backend(AgyError::Timeout))?
    }

    async fn execute(
        &self,
        command: Command,
        effort: Option<Effort>,
    ) -> Result<Document, ServerError> {
        let options = &self.inner.options;
        let invocation = Cli {
            agy_path: options.agy_path.clone(),
            model: options.model.clone(),
            effort: Some(effort.unwrap_or(options.effort)),
            timeout: options.timeout,
            verification: VerificationMode::Standard,
            command,
        }
        .into_invocation()?;
        backend::execute_cached(invocation, &self.inner.cache)
            .await
            .map_err(Into::into)
    }
}
