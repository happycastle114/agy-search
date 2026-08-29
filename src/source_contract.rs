//! Caller-owned source allowlist and verification facade.

use std::collections::{HashMap, HashSet};

use thiserror::Error;
use tokio::time::Instant;

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
        let documents = fetcher
            .fetch_many(sources, deadline)
            .await?
            .into_iter()
            .map(SourceDocument::parse)
            .collect::<Result<Vec<_>, _>>()?;
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
