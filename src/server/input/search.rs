use schemars::JsonSchema;
use serde::Deserialize;

use super::{bounded_text, output_args};
use crate::server::ServerError;
use crate::{
    cli::{Command, QueryArgument, SearchArgs},
    source_restriction::SourceDomain,
    types::{Effort, HttpUrl, NonEmptyText},
};

const MAX_BATCH_QUERIES: usize = 5;

#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(untagged)]
pub(crate) enum SearchQuery {
    One(NonEmptyText),
    Many(Vec<NonEmptyText>),
}

#[derive(Clone, Copy, Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SearchProfile {
    Standard,
}

#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct SearchInput {
    pub(crate) query: SearchQuery,
    #[serde(default = "default_results")]
    #[schemars(range(min = 1, max = 20))]
    pub(crate) max_results: u16,
    #[serde(default)]
    pub(crate) search_domain_filter: Vec<SourceDomain>,
    #[serde(default)]
    pub(crate) source_urls: Vec<HttpUrl>,
    pub(crate) max_tokens_per_page: Option<u32>,
    pub(crate) country: Option<NonEmptyText>,
    pub(crate) thinking_level: Option<Effort>,
    pub(crate) search_profile: Option<SearchProfile>,
}

const fn default_results() -> u16 {
    10
}

pub(crate) struct PreparedSearch {
    pub(crate) commands: Vec<Command>,
    pub(crate) max_results: usize,
    pub(crate) effort: Option<Effort>,
}

impl SearchInput {
    pub(crate) fn prepare(self) -> Result<PreparedSearch, ServerError> {
        let queries = match self.query {
            SearchQuery::One(query) => vec![query],
            SearchQuery::Many(queries) => queries,
        };
        if queries.is_empty()
            || queries.len() > MAX_BATCH_QUERIES
            || !(1..=20).contains(&self.max_results)
            || self.max_tokens_per_page == Some(0)
            || self
                .country
                .as_ref()
                .is_some_and(|value| value.as_str().len() > 128)
        {
            return Err(ServerError::InvalidInput);
        }
        match self.search_profile {
            Some(SearchProfile::Standard) | None => {}
        }
        let mut commands = Vec::with_capacity(queries.len());
        for query in queries {
            bounded_text(&query)?;
            commands.push(Command::Search(SearchArgs {
                query: QueryArgument::Text(query),
                max_results: self.max_results,
                domains: self.search_domain_filter.clone(),
                source_urls: self.source_urls.clone(),
                country: self.country.clone(),
                max_tokens_per_page: self.max_tokens_per_page,
                scopes: Vec::new(),
                cutoff: None,
                output: output_args(),
            }));
        }
        Ok(PreparedSearch {
            commands,
            max_results: usize::from(self.max_results),
            effort: self.thinking_level,
        })
    }
}
