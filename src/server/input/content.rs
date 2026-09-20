use schemars::JsonSchema;
use serde::Deserialize;

use super::{bounded_text, output_args};
use crate::server::ServerError;
use crate::{
    cli::{Command, CrawlArgs, ExtractArgs, QueryArgument, ResearchArgs, SiteArgs},
    source_restriction::SourceDomain,
    types::{HttpUrl, NonEmptyText},
};

#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExtractInput {
    #[schemars(length(min = 1, max = 20))]
    pub(crate) urls: Vec<HttpUrl>,
    pub(crate) query: Option<NonEmptyText>,
}

impl ExtractInput {
    pub(crate) fn into_command(self) -> Result<Command, ServerError> {
        if !(1..=20).contains(&self.urls.len()) {
            return Err(ServerError::InvalidInput);
        }
        if let Some(query) = &self.query {
            bounded_text(query)?;
        }
        Ok(Command::Extract(ExtractArgs {
            urls: self.urls,
            query: self.query,
            output: output_args(),
        }))
    }
}

#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct MapInput {
    pub(crate) url: HttpUrl,
    #[serde(default = "default_map_limit")]
    #[schemars(range(min = 1, max = 100))]
    pub(crate) limit: u16,
    pub(crate) instructions: Option<NonEmptyText>,
    #[serde(default)]
    pub(crate) allow_external: bool,
}

const fn default_map_limit() -> u16 {
    50
}

impl MapInput {
    pub(crate) fn into_command(self) -> Result<Command, ServerError> {
        if !(1..=100).contains(&self.limit) {
            return Err(ServerError::InvalidInput);
        }
        if let Some(text) = &self.instructions {
            bounded_text(text)?;
        }
        Ok(Command::Map(SiteArgs {
            url: self.url,
            limit: self.limit,
            instructions: self.instructions,
            allow_external: self.allow_external,
            output: output_args(),
        }))
    }
}

#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct CrawlInput {
    pub(crate) url: HttpUrl,
    #[serde(default = "default_crawl_limit")]
    #[schemars(range(min = 1, max = 50))]
    pub(crate) limit: u16,
    pub(crate) instructions: Option<NonEmptyText>,
    #[serde(default)]
    pub(crate) allow_external: bool,
}

const fn default_crawl_limit() -> u16 {
    20
}

impl CrawlInput {
    pub(crate) fn into_command(self) -> Result<Command, ServerError> {
        if !(1..=50).contains(&self.limit) {
            return Err(ServerError::InvalidInput);
        }
        if let Some(text) = &self.instructions {
            bounded_text(text)?;
        }
        Ok(Command::Crawl(CrawlArgs {
            url: self.url,
            limit: self.limit,
            instructions: self.instructions,
            allow_external: self.allow_external,
            output: output_args(),
        }))
    }
}

#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ResearchInput {
    pub(crate) query: NonEmptyText,
    #[serde(default = "default_research_limit")]
    #[schemars(range(min = 1, max = 20))]
    pub(crate) max_sources: u16,
    #[serde(default)]
    pub(crate) domains: Vec<SourceDomain>,
    #[serde(default)]
    pub(crate) source_urls: Vec<HttpUrl>,
}

const fn default_research_limit() -> u16 {
    10
}

impl ResearchInput {
    pub(crate) fn into_command(self) -> Result<Command, ServerError> {
        if !(1..=20).contains(&self.max_sources) {
            return Err(ServerError::InvalidInput);
        }
        bounded_text(&self.query)?;
        Ok(Command::Research(ResearchArgs {
            query: QueryArgument::Text(self.query),
            max_sources: self.max_sources,
            domains: self.domains,
            source_urls: self.source_urls,
            scopes: Vec::new(),
            cutoff: None,
            output: output_args(),
        }))
    }
}
