//! Typed request documents sent to Antigravity.

use schemars::JsonSchema;
use serde::Serialize;

use crate::types::{HttpUrl, NonEmptyText, Operation};

#[derive(Clone, Debug, JsonSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SearchRequest {
    pub(crate) query: NonEmptyText,
    pub(crate) max_results: u16,
    pub(crate) domains: Vec<NonEmptyText>,
    pub(crate) country: Option<NonEmptyText>,
    pub(crate) max_tokens_per_page: Option<u32>,
}

#[derive(Clone, Debug, JsonSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExtractRequest {
    pub(crate) urls: Vec<HttpUrl>,
    pub(crate) query: Option<NonEmptyText>,
}

#[derive(Clone, Debug, JsonSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MapRequest {
    pub(crate) url: HttpUrl,
    pub(crate) limit: u16,
    pub(crate) instructions: Option<NonEmptyText>,
    pub(crate) allow_external: bool,
}

#[derive(Clone, Debug, JsonSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CrawlRequest {
    pub(crate) url: HttpUrl,
    pub(crate) limit: u16,
    pub(crate) instructions: Option<NonEmptyText>,
    pub(crate) allow_external: bool,
}

#[derive(Clone, Debug, JsonSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ResearchRequest {
    pub(crate) query: NonEmptyText,
    pub(crate) max_sources: u16,
}

#[derive(Clone, Debug)]
pub(crate) enum ContentRequest {
    Search(SearchRequest),
    Extract(ExtractRequest),
    Map(MapRequest),
    Crawl(CrawlRequest),
    Research(ResearchRequest),
}

impl ContentRequest {
    pub(crate) const fn operation(&self) -> Operation {
        match self {
            Self::Search(_) => Operation::Search,
            Self::Extract(_) => Operation::Extract,
            Self::Map(_) => Operation::Map,
            Self::Crawl(_) => Operation::Crawl,
            Self::Research(_) => Operation::Research,
        }
    }

    pub(crate) fn to_json(&self) -> Result<String, serde_json::Error> {
        match self {
            Self::Search(request) => serde_json::to_string(request),
            Self::Extract(request) => serde_json::to_string(request),
            Self::Map(request) => serde_json::to_string(request),
            Self::Crawl(request) => serde_json::to_string(request),
            Self::Research(request) => serde_json::to_string(request),
        }
    }
}
