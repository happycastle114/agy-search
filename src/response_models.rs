//! Closed serializable documents returned by the CLI.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::types::{CalendarDate, HttpUrl, NonEmptyText};

macro_rules! object_marker {
    ($name:ident, $variant:ident, $value:literal) => {
        #[derive(Clone, Copy, Debug, Deserialize, JsonSchema, Serialize)]
        pub(crate) enum $name {
            #[serde(rename = $value)]
            $variant,
        }
    };
}

object_marker!(StatusObject, Status, "status");
object_marker!(ModelsObject, Models, "models");
object_marker!(SearchObject, Search, "search");
object_marker!(ExtractObject, Extract, "extract");
object_marker!(MapObject, Map, "map");
object_marker!(CrawlObject, Crawl, "crawl");
object_marker!(ResearchObject, Research, "research");

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct WebSource {
    pub(crate) title: NonEmptyText,
    pub(crate) url: HttpUrl,
    pub(crate) snippet: NonEmptyText,
    #[serde(default)]
    pub(crate) date: Option<String>,
    #[serde(default)]
    pub(crate) last_updated: Option<String>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ScopeEvidence {
    pub(crate) scope: NonEmptyText,
    pub(crate) claim: NonEmptyText,
    pub(crate) url: HttpUrl,
    #[serde(default)]
    pub(crate) date: Option<CalendarDate>,
    #[serde(default)]
    pub(crate) value: Option<NonEmptyText>,
    #[serde(default)]
    pub(crate) source_date_text: Option<NonEmptyText>,
    #[serde(default)]
    pub(crate) evidence_excerpt: Option<NonEmptyText>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct EvidenceAudit {
    #[schemars(length(min = 1, max = 20))]
    pub(crate) candidates: Vec<ScopeEvidence>,
    pub(crate) coverage_complete: bool,
    pub(crate) conclusion: NonEmptyText,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExtractPage {
    pub(crate) url: HttpUrl,
    pub(crate) title: NonEmptyText,
    pub(crate) content: NonEmptyText,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MapLink {
    pub(crate) url: HttpUrl,
    pub(crate) title: NonEmptyText,
    pub(crate) depth: u32,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CrawlPage {
    pub(crate) url: HttpUrl,
    pub(crate) title: NonEmptyText,
    pub(crate) content: NonEmptyText,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ResearchFinding {
    pub(crate) title: NonEmptyText,
    pub(crate) summary: NonEmptyText,
    #[schemars(length(min = 1, max = 20))]
    pub(crate) citations: Vec<HttpUrl>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SearchResponse {
    pub(crate) object: SearchObject,
    #[serde(skip_serializing)]
    pub(crate) evidence_audit: EvidenceAudit,
    #[schemars(length(min = 1, max = 20))]
    pub(crate) results: Vec<WebSource>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExtractResponse {
    pub(crate) object: ExtractObject,
    #[schemars(length(min = 1, max = 20))]
    pub(crate) results: Vec<ExtractPage>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MapResponse {
    pub(crate) object: MapObject,
    pub(crate) base_url: HttpUrl,
    #[schemars(length(min = 1, max = 100))]
    pub(crate) results: Vec<MapLink>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CrawlResponse {
    pub(crate) object: CrawlObject,
    pub(crate) base_url: HttpUrl,
    #[schemars(length(min = 1, max = 50))]
    pub(crate) results: Vec<CrawlPage>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ResearchResponse {
    pub(crate) object: ResearchObject,
    #[serde(skip_serializing)]
    pub(crate) evidence_audit: EvidenceAudit,
    pub(crate) title: NonEmptyText,
    pub(crate) summary: NonEmptyText,
    #[schemars(length(min = 1, max = 20))]
    pub(crate) findings: Vec<ResearchFinding>,
    #[schemars(length(min = 1, max = 20))]
    pub(crate) sources: Vec<WebSource>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub(crate) enum ResponseDocument {
    Status(StatusResponse),
    Models(ModelsResponse),
    Search(SearchResponse),
    Extract(ExtractResponse),
    Map(MapResponse),
    Crawl(CrawlResponse),
    Research(ResearchResponse),
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct StatusResponse {
    pub(crate) object: StatusObject,
    pub(crate) available: bool,
    pub(crate) version: String,
    pub(crate) model_count: usize,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct ModelsResponse {
    pub(crate) object: ModelsObject,
    pub(crate) models: Vec<String>,
}
