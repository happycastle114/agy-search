//! Caller-owned source allowlist and verification facade.

use std::collections::{HashMap, HashSet};

use thiserror::Error;
use tokio::{task::JoinSet, time::Instant};

use crate::{
    source_document::{CandidateBinding, SourceDocument, SourceDocumentError},
    source_fact::SourceFact,
    source_fetch::{SafeSourceUrl, SourceFetchError, SourceFetcher},
};

#[cfg(test)]
mod facts_test;
#[cfg(test)]
mod verification_test;

#[derive(Debug)]
pub(crate) struct SourceContract {
    documents: HashMap<SafeSourceUrl, SourceDocument>,
}

#[derive(Debug, Error)]
pub(crate) enum SourceContractError {
    #[error("source allowlist was empty or contained duplicates")]
    InvalidAllowlist,
    #[error("source fetch failed")]
    Fetch(#[from] SourceFetchError),
    #[error("source document failed verification")]
    Document(#[from] SourceDocumentError),
    #[error("candidate source was outside the caller allowlist")]
    SourceNotAllowed,
    #[error("source fetch task failed")]
    TaskFailed,
}

impl SourceContract {
    pub(crate) async fn fetch(
        fetcher: &SourceFetcher,
        sources: &[SafeSourceUrl],
        deadline: Instant,
    ) -> Result<Self, SourceContractError> {
        let unique: HashSet<_> = sources.iter().collect();
        if sources.is_empty() || unique.len() != sources.len() {
            return Err(SourceContractError::InvalidAllowlist);
        }
        let mut documents = Vec::with_capacity(sources.len());
        for batch in sources.chunks(4) {
            let mut tasks = JoinSet::new();
            for (index, source) in batch.iter().enumerate() {
                let owned_fetcher = fetcher.clone();
                let owned_source = source.clone();
                tasks.spawn(
                    async move { (index, owned_fetcher.fetch(&owned_source, deadline).await) },
                );
            }
            let mut completed = std::collections::BTreeMap::new();
            let mut task_failed = false;
            while let Some(result) = tasks.join_next().await {
                match result {
                    Ok((index, response)) => {
                        completed.insert(index, response);
                    }
                    Err(_) => task_failed = true,
                }
            }
            if task_failed || completed.len() != batch.len() {
                return Err(SourceContractError::TaskFailed);
            }
            for response in completed.into_values() {
                let response = response?;
                documents.push(SourceDocument::parse(response)?);
            }
        }
        Self::from_documents(documents)
    }

    pub(crate) fn from_documents(
        documents: Vec<SourceDocument>,
    ) -> Result<Self, SourceContractError> {
        if documents.is_empty() {
            return Err(SourceContractError::InvalidAllowlist);
        }
        let mut indexed = HashMap::with_capacity(documents.len());
        for document in documents {
            if indexed.insert(document.url().clone(), document).is_some() {
                return Err(SourceContractError::InvalidAllowlist);
            }
        }
        Ok(Self { documents: indexed })
    }

    pub(crate) fn verify(
        &self,
        candidate: &CandidateBinding<'_>,
    ) -> Result<(), SourceContractError> {
        self.documents
            .get(candidate.source_url())
            .ok_or(SourceContractError::SourceNotAllowed)?
            .verify(candidate)?;
        Ok(())
    }

    pub(crate) fn exact_fact(
        &self,
        scope: &str,
        expected_value: &str,
    ) -> Result<Option<SourceFact>, SourceContractError> {
        Ok(self
            .unique_fact(scope)?
            .filter(|fact| fact.value() == expected_value))
    }

    pub(crate) fn unique_fact(
        &self,
        scope: &str,
    ) -> Result<Option<SourceFact>, SourceContractError> {
        let mut found = None;
        for document in self.documents.values() {
            if let Some(fact) = document.exact_fact(scope)? {
                if found.is_some() {
                    return Err(SourceDocumentError::AmbiguousBinding.into());
                }
                found = Some(fact);
            }
        }
        Ok(found)
    }
}
