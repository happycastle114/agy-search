use std::collections::{HashMap, HashSet};

use tokio::time::Instant;

use super::{binding::bind_evidence, fetch::fetch_bodies};
use crate::{
    error::AgyError,
    response_models::{ExtractResponse, ResearchResponse},
    source_fetch::SafeSourceUrl,
    types::{HttpUrl, NonEmptyText},
};

pub(crate) struct SourceEvidenceSnapshot {
    pub(super) bodies: HashMap<SafeSourceUrl, String>,
}

impl SourceEvidenceSnapshot {
    pub(crate) async fn fetch_exact(
        sources: &[HttpUrl],
        deadline: Instant,
    ) -> Result<Self, AgyError> {
        let sources = sources
            .iter()
            .map(|source| {
                SafeSourceUrl::parse_redirect(source.as_str()).map_err(|_| AgyError::OutputInvalid)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let bodies = fetch_bodies(&sources, deadline).await?;
        Ok(Self { bodies })
    }

    pub(crate) async fn fetch(
        research: &ResearchResponse,
        deadline: Instant,
    ) -> Result<Self, AgyError> {
        let sources = research
            .sources
            .iter()
            .map(|source| {
                SafeSourceUrl::parse_redirect(source.url.as_str())
                    .map_err(|_| AgyError::OutputInvalid)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let bodies = fetch_bodies(&sources, deadline).await?;
        Ok(Self { bodies })
    }

    pub(crate) fn verify_and_project(
        &self,
        research: &mut ResearchResponse,
    ) -> Result<(), AgyError> {
        let mut source_contexts = HashMap::new();
        for candidate in &mut research.evidence_audit.candidates {
            let source = SafeSourceUrl::parse_redirect(candidate.url.as_str())
                .map_err(|_| AgyError::OutputInvalid)?;
            let excerpt = candidate
                .evidence_excerpt
                .as_ref()
                .ok_or(AgyError::OutputInvalid)?;
            let value = candidate.value.as_ref().ok_or(AgyError::OutputInvalid)?;
            let body = self.bodies.get(&source).ok_or(AgyError::OutputInvalid)?;
            let bound = bind_evidence(body, excerpt.as_str(), value.as_str())
                .ok_or(AgyError::OutputInvalid)?;
            let context =
                NonEmptyText::parse(&bound.context).map_err(|_| AgyError::OutputInvalid)?;
            candidate.value =
                Some(NonEmptyText::parse(&bound.value).map_err(|_| AgyError::OutputInvalid)?);
            candidate.evidence_excerpt = Some(context.clone());
            source_contexts.entry(source).or_insert(context);
        }
        for finding in &mut research.findings {
            let mut summary = String::new();
            let mut cited = HashSet::new();
            for citation in &finding.citations {
                let source = SafeSourceUrl::parse_redirect(citation.as_str())
                    .map_err(|_| AgyError::OutputInvalid)?;
                if !cited.insert(source.clone()) {
                    continue;
                }
                let context = source_contexts
                    .get(&source)
                    .ok_or(AgyError::OutputInvalid)?;
                if !summary.is_empty() {
                    summary.push_str("\n\n");
                }
                summary.push_str(context.as_str());
            }
            finding.summary = NonEmptyText::parse(&summary).map_err(|_| AgyError::OutputInvalid)?;
        }
        let mut summary = String::new();
        for source in &mut research.sources {
            let safe = SafeSourceUrl::parse_redirect(source.url.as_str())
                .map_err(|_| AgyError::OutputInvalid)?;
            let context = source_contexts
                .remove(&safe)
                .ok_or(AgyError::OutputInvalid)?;
            if !summary.is_empty() {
                summary.push_str("\n\n");
            }
            summary.push_str(context.as_str());
            source.snippet = context;
        }
        if !source_contexts.is_empty() {
            return Err(AgyError::OutputInvalid);
        }
        research.summary = NonEmptyText::parse(&summary).map_err(|_| AgyError::OutputInvalid)?;
        Ok(())
    }

    pub(crate) fn prompt_context(&self, query: &str) -> Result<String, AgyError> {
        super::prompt::render_prompt_context(&self.bodies, query)
    }

    pub(crate) fn verify_and_project_extract(
        &self,
        extract: &mut ExtractResponse,
        query: Option<&str>,
    ) -> Result<(), AgyError> {
        for page in &mut extract.results {
            let source = SafeSourceUrl::parse_redirect(page.url.as_str())
                .map_err(|_| AgyError::OutputInvalid)?;
            let body = self.bodies.get(&source).ok_or(AgyError::OutputInvalid)?;
            let projected = super::prompt::extract_content(body, query.unwrap_or_default())
                .ok_or(AgyError::OutputInvalid)?;
            page.content = NonEmptyText::parse(&projected).map_err(|_| AgyError::OutputInvalid)?;

            let title = super::text::canonical_evidence_text(page.title.as_str());
            let (title_start, title_end) = super::text::find_ascii_case_insensitive(body, &title)
                .ok_or(AgyError::OutputInvalid)?;
            page.title = NonEmptyText::parse(&body[title_start..title_end])
                .map_err(|_| AgyError::OutputInvalid)?;
        }
        Ok(())
    }
}
