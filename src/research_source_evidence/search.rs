use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use tokio::time::Instant;

use super::{
    MINIMUM_FALLBACK_VALUE_CHARACTERS,
    binding::{bind_search_candidate_evidence, nearest_date_binding},
    research::SourceEvidenceSnapshot,
    text::{canonical_evidence_text, forward_evidence_context},
    title::project_search_source,
};
use crate::{
    error::AgyError,
    redirect::curl_executable,
    response_models::{ScopeEvidence, SearchResponse},
    source_fetch::{SafeSourceUrl, SourceFetchError, SourceFetcher},
    source_verification::map_source_fetch_error,
    types::NonEmptyText,
};

pub(crate) struct SearchSourceEvidence {
    pub(super) pages: HashMap<SafeSourceUrl, SearchEvidencePage>,
}

#[derive(Debug)]
pub(super) struct SearchEvidencePage {
    pub(super) body: String,
    pub(super) headings: Vec<String>,
    pub(super) title: Option<String>,
}

struct VerifiedSearchCandidate {
    candidate: ScopeEvidence,
    context: NonEmptyText,
    identity: Option<NonEmptyText>,
    source: SafeSourceUrl,
}

impl SearchSourceEvidence {
    pub(crate) fn from_prefetched(evidence: &SourceEvidenceSnapshot) -> Self {
        let pages = evidence
            .bodies
            .iter()
            .map(|(url, body)| {
                (
                    url.clone(),
                    SearchEvidencePage {
                        body: body.clone(),
                        headings: Vec::new(),
                        title: None,
                    },
                )
            })
            .collect();
        Self { pages }
    }

    pub(crate) async fn fetch(
        search: &SearchResponse,
        deadline: Instant,
    ) -> Result<Self, AgyError> {
        let sources = search
            .results
            .iter()
            .map(|source| {
                SafeSourceUrl::parse_redirect(source.url.as_str())
                    .map_err(|_| AgyError::OutputInvalid)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let executable = PathBuf::from(curl_executable()?);
        let fetched = SourceFetcher::new(executable)
            .fetch_many_available(&sources, deadline)
            .await
            .map_err(|error| map_source_fetch_error(&error))?;
        let mut pages = HashMap::with_capacity(fetched.len());
        for source in fetched {
            let source = match source {
                Ok(source) => source,
                Err(SourceFetchError::Deadline) => return Err(AgyError::Timeout),
                Err(_) => continue,
            };
            let (url, raw_body) = source.into_parts();
            let body = canonical_evidence_text(&raw_body);
            let mut headings = crate::source_document_heading::heading_sections(&raw_body)
                .unwrap_or_default()
                .into_iter()
                .map(|(heading, _)| heading)
                .filter(|heading| !heading.trim().is_empty())
                .collect::<Vec<_>>();
            let title = crate::source_document_heading::document_title(&raw_body)
                .ok()
                .flatten()
                .and_then(|title| {
                    title
                        .split('|')
                        .map(str::trim)
                        .find(|segment| !segment.is_empty())
                        .map(ToOwned::to_owned)
                });
            if let Some(page_identity) = &title {
                headings.push(page_identity.clone());
            }
            if body.is_empty()
                || pages
                    .insert(
                        url,
                        SearchEvidencePage {
                            body,
                            headings,
                            title,
                        },
                    )
                    .is_some()
            {
                return Err(AgyError::OutputInvalid);
            }
        }
        if pages.is_empty() {
            return Err(AgyError::OutputInvalid);
        }
        Ok(Self { pages })
    }

    pub(crate) fn verify_and_project(&self, search: &mut SearchResponse) -> Result<(), AgyError> {
        let mut source_contexts: HashMap<SafeSourceUrl, Vec<NonEmptyText>> = HashMap::new();
        let mut source_identities = HashMap::new();
        let candidates = std::mem::take(&mut search.evidence_audit.candidates);
        for verified in candidates
            .into_iter()
            .filter_map(|candidate| self.verify_candidate(candidate))
        {
            let contexts = source_contexts.entry(verified.source).or_default();
            if !contexts.contains(&verified.context) {
                contexts.push(verified.context);
            }
            if let Some(identity) = verified.identity {
                source_identities
                    .entry(verified.candidate.url.clone())
                    .or_insert(identity);
            }
            search.evidence_audit.candidates.push(verified.candidate);
        }
        let candidates = &search.evidence_audit.candidates;
        search.results = std::mem::take(&mut search.results)
            .into_iter()
            .filter_map(|source| {
                project_search_source(
                    source,
                    &self.pages,
                    &source_contexts,
                    &source_identities,
                    candidates,
                )
            })
            .collect();
        if search.results.is_empty() {
            return Err(AgyError::OutputInvalid);
        }
        let retained = search
            .results
            .iter()
            .map(|source| &source.url)
            .collect::<HashSet<_>>();
        search
            .evidence_audit
            .candidates
            .retain(|candidate| retained.contains(&candidate.url));
        Ok(())
    }

    fn verify_candidate(&self, mut candidate: ScopeEvidence) -> Option<VerifiedSearchCandidate> {
        let source = SafeSourceUrl::parse_redirect(candidate.url.as_str()).ok()?;
        let excerpt = candidate.evidence_excerpt.as_ref()?;
        let value = candidate.value.as_ref()?;
        let body = &self.pages.get(&source)?.body;
        let binding_date = candidate.date.clone().or_else(|| {
            candidate
                .source_date_text
                .as_ref()
                .and_then(|date| crate::source_date::parse(date.as_str()).ok())
        });
        let mut bound = bind_search_candidate_evidence(
            body,
            excerpt.as_str(),
            value.as_str(),
            binding_date.as_ref(),
        )?;
        if let Some(date) = &candidate.date {
            let (date_start, date_end) =
                nearest_date_binding(body, date, bound.value_start, bound.value_end)?;
            bound.context = forward_evidence_context(
                body,
                bound.value_start.min(date_start),
                bound.value_end.max(date_end),
            );
            candidate.source_date_text = NonEmptyText::parse(&body[date_start..date_end]).ok();
        }
        let context = NonEmptyText::parse(&bound.context).ok()?;
        let bound_value = NonEmptyText::parse(&bound.value).ok()?;
        candidate.value = Some(bound_value.clone());
        candidate.evidence_excerpt = Some(context.clone());
        let identity = (crate::source_date::parse(&bound.value).is_err()
            && bound.value.chars().count() >= MINIMUM_FALLBACK_VALUE_CHARACTERS)
            .then_some(bound_value);
        Some(VerifiedSearchCandidate {
            candidate,
            context,
            identity,
            source,
        })
    }
}
