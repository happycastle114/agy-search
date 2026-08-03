//! Response parsing, schema generation, and cross-field validation.

use std::collections::HashSet;

use schemars::schema_for;
use serde_json::Value;

use crate::{
    error::AgyError,
    request::ContentRequest,
    response_models::{
        CrawlResponse, ExtractResponse, MapResponse, ModelsObject, ModelsResponse,
        ResearchResponse, ResponseDocument, SearchResponse, StatusObject, StatusResponse,
    },
    types::{HttpUrl, Operation},
};

pub(crate) use crate::response_models::ResponseDocument as Document;

impl ResponseDocument {
    pub(crate) const fn status(version: String, model_count: usize) -> Self {
        Self::Status(StatusResponse {
            object: StatusObject::Status,
            available: true,
            version,
            model_count,
        })
    }

    pub(crate) const fn models(models: Vec<String>) -> Self {
        Self::Models(ModelsResponse {
            object: ModelsObject::Models,
            models,
        })
    }

    pub(crate) fn schema(operation: Operation) -> Result<String, AgyError> {
        let schema = match operation {
            Operation::Search => serde_json::to_value(schema_for!(SearchResponse)),
            Operation::Extract => serde_json::to_value(schema_for!(ExtractResponse)),
            Operation::Map => serde_json::to_value(schema_for!(MapResponse)),
            Operation::Crawl => serde_json::to_value(schema_for!(CrawlResponse)),
            Operation::Research => serde_json::to_value(schema_for!(ResearchResponse)),
        }
        .map_err(|_| AgyError::InvalidCommand)?;
        serde_json::to_string(&schema).map_err(|_| AgyError::InvalidCommand)
    }

    pub(crate) fn parse(operation: Operation, value: Value) -> Result<Self, AgyError> {
        let response = match operation {
            Operation::Search => serde_json::from_value(value).map(Self::Search),
            Operation::Extract => serde_json::from_value(value).map(Self::Extract),
            Operation::Map => serde_json::from_value(value).map(Self::Map),
            Operation::Crawl => serde_json::from_value(value).map(Self::Crawl),
            Operation::Research => serde_json::from_value(value).map(Self::Research),
        }
        .map_err(|_| AgyError::OutputInvalid)?;
        response.validate()?;
        Ok(response)
    }

    pub(crate) fn validate_request(&self, request: &ContentRequest) -> Result<(), AgyError> {
        match (self, request) {
            (Self::Search(response), ContentRequest::Search(request)) => {
                bounded(response.results.len(), usize::from(request.max_results))
            }
            (Self::Extract(response), ContentRequest::Extract(request)) => {
                let actual: HashSet<_> = response.results.iter().map(|item| &item.url).collect();
                let expected: HashSet<_> = request.urls.iter().collect();
                if actual == expected {
                    Ok(())
                } else {
                    Err(AgyError::OutputInvalid)
                }
            }
            (Self::Map(response), ContentRequest::Map(request)) => validate_site(
                &request.url,
                &response.base_url,
                response.results.iter().map(|item| &item.url),
                usize::from(request.limit),
                request.allow_external,
            ),
            (Self::Crawl(response), ContentRequest::Crawl(request)) => validate_site(
                &request.url,
                &response.base_url,
                response.results.iter().map(|item| &item.url),
                usize::from(request.limit),
                request.allow_external,
            ),
            (Self::Research(response), ContentRequest::Research(request)) => {
                bounded(response.sources.len(), usize::from(request.max_sources))
            }
            _ => Err(AgyError::OutputInvalid),
        }
    }

    fn validate(&self) -> Result<(), AgyError> {
        match self {
            Self::Search(value) => validate_urls(&value.results, 20, |item| &item.url),
            Self::Extract(value) => validate_urls(&value.results, 20, |item| &item.url),
            Self::Map(value) => validate_urls(&value.results, 100, |item| &item.url),
            Self::Crawl(value) => validate_urls(&value.results, 50, |item| &item.url),
            Self::Research(value) => validate_research(value),
            Self::Status(_) | Self::Models(_) => Ok(()),
        }
    }
}

fn bounded(length: usize, maximum: usize) -> Result<(), AgyError> {
    if (1..=maximum).contains(&length) {
        Ok(())
    } else {
        Err(AgyError::OutputInvalid)
    }
}

fn unique<'a>(urls: impl Iterator<Item = &'a HttpUrl>) -> Result<(), AgyError> {
    let mut observed = HashSet::new();
    if urls.into_iter().all(|url| observed.insert(url)) {
        Ok(())
    } else {
        Err(AgyError::OutputInvalid)
    }
}

fn validate_urls<'a, Item: 'a>(
    items: &'a [Item],
    maximum: usize,
    url: impl Fn(&'a Item) -> &'a HttpUrl,
) -> Result<(), AgyError> {
    bounded(items.len(), maximum)?;
    unique(items.iter().map(url))
}

fn validate_site<'a>(
    requested: &HttpUrl,
    base: &HttpUrl,
    urls: impl Iterator<Item = &'a HttpUrl>,
    limit: usize,
    allow_external: bool,
) -> Result<(), AgyError> {
    let urls: Vec<_> = urls.collect();
    bounded(urls.len(), limit)?;
    if !requested.same_origin(base)
        || (!allow_external && urls.iter().any(|url| !requested.same_origin(url)))
    {
        return Err(AgyError::OutputInvalid);
    }
    Ok(())
}

fn validate_research(response: &ResearchResponse) -> Result<(), AgyError> {
    bounded(response.findings.len(), 20)?;
    bounded(response.sources.len(), 20)?;
    for finding in &response.findings {
        bounded(finding.citations.len(), 20)?;
    }
    unique(response.sources.iter().map(|item| &item.url))?;
    let sources: HashSet<_> = response.sources.iter().map(|item| &item.url).collect();
    if response
        .findings
        .iter()
        .flat_map(|item| &item.citations)
        .all(|url| sources.contains(url))
    {
        Ok(())
    } else {
        Err(AgyError::OutputInvalid)
    }
}
