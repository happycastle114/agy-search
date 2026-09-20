use std::{path::PathBuf, time::Duration};

use tokio::{sync::Mutex, time::Instant};

use crate::{
    antigravity_version::{self, Deadline},
    error::AgyError,
    invocation::{Invocation, InvocationCommand},
    response::Document as ResponseDocument,
    types::ModelCatalog,
};

use super::{ContentModels, discover_models};

#[derive(Debug)]
struct CacheEntry {
    executable: String,
    cwd: PathBuf,
    catalog: ModelCatalog,
    expires_at: Instant,
}

#[derive(Debug)]
pub(crate) struct DiscoveryCache {
    ttl: Duration,
    entry: Mutex<Option<CacheEntry>>,
}

impl DiscoveryCache {
    pub(crate) fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            entry: Mutex::new(None),
        }
    }

    pub(super) async fn catalog(
        &self,
        executable: &str,
        cwd: PathBuf,
        timeout: Duration,
    ) -> Result<ModelCatalog, AgyError> {
        let started = Instant::now();
        let mut entry = tokio::time::timeout(timeout, self.entry.lock())
            .await
            .map_err(|_| AgyError::Timeout)?;
        let now = Instant::now();
        if let Some(cached) = entry.as_ref()
            && cached.executable == executable
            && cached.cwd == cwd
            && cached.expires_at > now
        {
            return Ok(cached.catalog.clone());
        }
        *entry = None;
        let remaining = timeout
            .checked_sub(started.elapsed())
            .filter(|remaining| !remaining.is_zero())
            .ok_or(AgyError::Timeout)?;
        let catalog = discover_models(executable, cwd.clone(), remaining).await?;
        *entry = Some(CacheEntry {
            executable: executable.to_owned(),
            cwd,
            catalog: catalog.clone(),
            expires_at: Instant::now() + self.ttl,
        });
        drop(entry);
        Ok(catalog)
    }
}

pub(crate) async fn execute_cached(
    invocation: Invocation,
    cache: &DiscoveryCache,
) -> Result<ResponseDocument, AgyError> {
    let Invocation {
        agy_path,
        model,
        effort,
        timeout,
        command,
        output: _,
    } = invocation;
    let cwd = std::env::current_dir().map_err(|_| AgyError::InvalidCommand)?;
    let deadline = Deadline::after(timeout);
    match command {
        InvocationCommand::Status => super::status(&agy_path, cwd, deadline).await,
        InvocationCommand::Models => super::models(&agy_path, cwd, deadline)
            .await
            .map(ModelCatalog::into_strings)
            .map(ResponseDocument::models),
        InvocationCommand::Content(request) => {
            super::validate_network_targets(&request, deadline).await?;
            antigravity_version::require_supported(&agy_path, cwd.clone(), deadline).await?;
            let selected_models = match model {
                Some(selected) => {
                    super::validate_model(&agy_path, &cwd, deadline, &selected).await?;
                    ContentModels::fixed(Some(selected))
                }
                None => {
                    select_cached_models(cache, &agy_path, cwd, deadline, &request, effort).await?
                }
            };
            super::content::execute(&agy_path, selected_models, effort, deadline, *request).await
        }
    }
}

async fn select_cached_models(
    cache: &DiscoveryCache,
    executable: &str,
    cwd: PathBuf,
    deadline: Deadline,
    request: &crate::request::ContentRequest,
    effort: Option<crate::types::Effort>,
) -> Result<ContentModels, AgyError> {
    let Some(preference) =
        super::preferred_model_policy(request.operation(), request.verification(), effort)
    else {
        return Ok(ContentModels::fixed(None));
    };
    let timeout = deadline
        .remaining()?
        .min(super::MAX_ADVISORY_CATALOG_DISCOVERY);
    match cache.catalog(executable, cwd, timeout).await {
        Ok(catalog) => Ok(super::select_catalog_models(&catalog, preference)),
        Err(_) if deadline.remaining().is_ok() => Ok(ContentModels::fixed(None)),
        Err(_) => Err(AgyError::Timeout),
    }
}

#[cfg(test)]
#[path = "discovery_test.rs"]
mod tests;

#[cfg(test)]
#[path = "discovery_execution_test.rs"]
mod execution_tests;
