//! URL traversal and replacement for structured response documents.

use std::collections::HashSet;

use crate::{
    error::AgyError,
    response_models::ResponseDocument,
    types::{HttpUrl, SourceUrlKind},
};

impl ResponseDocument {
    pub(crate) fn grounding_redirects(&self) -> Vec<HttpUrl> {
        let mut urls = Vec::new();
        match self {
            Self::Search(value) => {
                urls.extend(value.results.iter().map(|item| &item.url));
                urls.extend(value.evidence_audit.candidates.iter().map(|item| &item.url));
            }
            Self::Extract(value) => urls.extend(value.results.iter().map(|item| &item.url)),
            Self::Map(value) => {
                urls.push(&value.base_url);
                urls.extend(value.results.iter().map(|item| &item.url));
            }
            Self::Crawl(value) => {
                urls.push(&value.base_url);
                urls.extend(value.results.iter().map(|item| &item.url));
            }
            Self::Research(value) => {
                urls.extend(value.sources.iter().map(|item| &item.url));
                urls.extend(value.findings.iter().flat_map(|item| &item.citations));
                urls.extend(value.evidence_audit.candidates.iter().map(|item| &item.url));
            }
            Self::Status(_) | Self::Models(_) => {}
        }
        let mut seen = HashSet::new();
        urls.into_iter()
            .filter(|url| url.source_kind() == SourceUrlKind::GroundingRedirect)
            .filter(|url| seen.insert((*url).clone()))
            .cloned()
            .collect()
    }

    pub(crate) fn replace_url(&mut self, from: &HttpUrl, to: &HttpUrl) {
        let replace = |url: &mut HttpUrl| {
            if url == from {
                url.clone_from(to);
            }
        };
        match self {
            Self::Search(value) => {
                value
                    .results
                    .iter_mut()
                    .for_each(|item| replace(&mut item.url));
                value
                    .evidence_audit
                    .candidates
                    .iter_mut()
                    .for_each(|item| replace(&mut item.url));
            }
            Self::Extract(value) => value
                .results
                .iter_mut()
                .for_each(|item| replace(&mut item.url)),
            Self::Map(value) => {
                replace(&mut value.base_url);
                value
                    .results
                    .iter_mut()
                    .for_each(|item| replace(&mut item.url));
            }
            Self::Crawl(value) => {
                replace(&mut value.base_url);
                value
                    .results
                    .iter_mut()
                    .for_each(|item| replace(&mut item.url));
            }
            Self::Research(value) => {
                value
                    .sources
                    .iter_mut()
                    .for_each(|item| replace(&mut item.url));
                value
                    .findings
                    .iter_mut()
                    .flat_map(|item| &mut item.citations)
                    .for_each(replace);
                value
                    .evidence_audit
                    .candidates
                    .iter_mut()
                    .for_each(|item| replace(&mut item.url));
            }
            Self::Status(_) | Self::Models(_) => {}
        }
    }

    pub(crate) fn non_source_search_urls(&self) -> Result<Vec<HttpUrl>, AgyError> {
        let Self::Search(value) = self else {
            return Err(AgyError::OutputInvalid);
        };
        let mut seen = HashSet::new();
        Ok(value
            .results
            .iter()
            .map(|item| &item.url)
            .chain(value.evidence_audit.candidates.iter().map(|item| &item.url))
            .filter(|url| url.source_kind() == SourceUrlKind::NonSource)
            .filter(|url| seen.insert((*url).clone()))
            .cloned()
            .collect())
    }

    pub(crate) fn direct_search_urls(&self) -> Result<Vec<HttpUrl>, AgyError> {
        let Self::Search(value) = self else {
            return Err(AgyError::OutputInvalid);
        };
        let mut seen = HashSet::new();
        Ok(value
            .results
            .iter()
            .map(|item| &item.url)
            .chain(value.evidence_audit.candidates.iter().map(|item| &item.url))
            .filter(|url| url.source_kind() == SourceUrlKind::Direct)
            .filter(|url| seen.insert((*url).clone()))
            .cloned()
            .collect())
    }

    pub(crate) fn search_landing_sources(&self) -> Result<Vec<(HttpUrl, String)>, AgyError> {
        let Self::Search(value) = self else {
            return Err(AgyError::OutputInvalid);
        };
        Ok(value
            .results
            .iter()
            .filter(|item| item.url.is_site_root())
            .map(|item| (item.url.clone(), item.title.as_str().to_owned()))
            .collect())
    }

    pub(crate) fn remove_search_url(&mut self, removed: &HttpUrl) -> Result<(), AgyError> {
        let Self::Search(value) = self else {
            return Err(AgyError::OutputInvalid);
        };
        value.results.retain(|item| &item.url != removed);
        value
            .evidence_audit
            .candidates
            .retain(|item| &item.url != removed);
        Ok(())
    }

    pub(crate) fn deduplicate_search_urls(&mut self) -> Result<(), AgyError> {
        let Self::Search(value) = self else {
            return Err(AgyError::OutputInvalid);
        };
        let mut result_urls = HashSet::new();
        value
            .results
            .retain(|item| result_urls.insert(item.url.clone()));
        Ok(())
    }

    pub(crate) fn reject_duplicate_search_urls(&self) -> Result<(), AgyError> {
        let Self::Search(value) = self else {
            return Err(AgyError::OutputInvalid);
        };
        let mut result_urls = HashSet::new();
        if value
            .results
            .iter()
            .all(|item| result_urls.insert(&item.url))
        {
            Ok(())
        } else {
            Err(AgyError::OutputInvalid)
        }
    }

    pub(crate) const fn search_results_empty(&self) -> Result<bool, AgyError> {
        match self {
            Self::Search(value) => Ok(value.results.is_empty()),
            Self::Extract(_)
            | Self::Map(_)
            | Self::Crawl(_)
            | Self::Research(_)
            | Self::Status(_)
            | Self::Models(_) => Err(AgyError::OutputInvalid),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::response_models::{MapLink, MapObject, MapResponse, SearchResponse};
    use crate::types::NonEmptyText;

    fn http_url(value: &str) -> HttpUrl {
        HttpUrl::parse(value).expect("test URL must parse")
    }

    fn link(url: &str) -> MapLink {
        MapLink {
            url: http_url(url),
            title: NonEmptyText::parse("result").expect("test title must parse"),
            depth: 0,
        }
    }

    #[test]
    fn grounding_redirects_deduplicate_in_first_seen_order() {
        let first = "https://vertexaisearch.cloud.google.com/grounding-api-redirect/first#fragment";
        let second = "https://vertexaisearch.cloud.google.com/grounding-api-redirect/second";
        let response = ResponseDocument::Map(MapResponse {
            object: MapObject::Map,
            base_url: http_url("https://example.com"),
            results: vec![link(first), link(second), link(first)],
        });

        assert_eq!(
            response.grounding_redirects(),
            vec![http_url(first), http_url(second)]
        );
    }

    #[test]
    fn resolved_search_urls_deduplicate_in_first_seen_order() -> Result<(), AgyError> {
        let search: SearchResponse = serde_json::from_value(serde_json::json!({
            "object": "search",
            "evidence_audit": {
                "candidates": [
                    {"scope":"first","claim":"first claim","url":"https://example.com/item"},
                    {"scope":"duplicate","claim":"duplicate claim","url":"https://example.com/item"}
                ],
                "coverage_complete": true,
                "conclusion": "complete"
            },
            "results": [
                {"title":"first","url":"https://example.com/item","snippet":"first snippet"},
                {"title":"duplicate","url":"https://example.com/item","snippet":"duplicate snippet"}
            ]
        }))
        .expect("search fixture must deserialize");
        let mut response = ResponseDocument::Search(search);

        response
            .deduplicate_search_urls()
            .expect("search URLs must deduplicate");

        let ResponseDocument::Search(search) = response else {
            return Err(AgyError::OutputInvalid);
        };
        assert_eq!(search.results.len(), 1);
        assert_eq!(search.evidence_audit.candidates.len(), 2);
        let first = search.results.first().ok_or(AgyError::OutputInvalid)?;
        assert_eq!(first.title.as_str(), "first");
        Ok(())
    }
}
