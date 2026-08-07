//! Cross-field validation for parsed response documents.

use std::collections::HashSet;

use crate::{
    error::AgyError,
    request::{ContentRequest, ResearchRequest, SearchRequest},
    response_models::{EvidenceAudit, ResearchResponse, ResponseDocument, WebSource},
    source_restriction::SourceRestriction,
    types::{HttpUrl, SourceUrlKind, VerificationMode},
    verification::{TemporalEvidenceOperation, TemporalValidation, validate_verification},
};

use super::{
    collection_validation::{EvidenceAuditError, bounded, evidence_audit, unique},
    public_dates, research_validation,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SearchDocumentError {
    AuditCoverageMissing,
    Invalid,
}

pub(super) fn request(
    response: &ResponseDocument,
    request: &ContentRequest,
) -> Result<(), AgyError> {
    match (response, request) {
        (ResponseDocument::Search(response), ContentRequest::Search(request)) => {
            validate_search_request(response, request)
        }
        (ResponseDocument::Extract(response), ContentRequest::Extract(request)) => {
            let actual: HashSet<_> = response.results.iter().map(|item| &item.url).collect();
            let expected: HashSet<_> = request.urls.iter().collect();
            if actual == expected {
                Ok(())
            } else {
                Err(AgyError::OutputInvalid)
            }
        }
        (ResponseDocument::Map(response), ContentRequest::Map(request)) => validate_site(
            &request.url,
            &response.base_url,
            response.results.iter().map(|item| &item.url),
            usize::from(request.limit),
            request.allow_external,
        ),
        (ResponseDocument::Crawl(response), ContentRequest::Crawl(request)) => validate_site(
            &request.url,
            &response.base_url,
            response.results.iter().map(|item| &item.url),
            usize::from(request.limit),
            request.allow_external,
        ),
        (ResponseDocument::Research(response), ContentRequest::Research(request)) => {
            validate_research_request(response, request)
        }
        _ => Err(AgyError::OutputInvalid),
    }
}

fn validate_search_request(
    response: &crate::response_models::SearchResponse,
    request: &SearchRequest,
) -> Result<(), AgyError> {
    bounded(response.results.len(), usize::from(request.max_results))?;
    validate_public_dates_for_request(
        request.verification,
        &response.results,
        &response.evidence_audit,
    )?;
    validate_caller_owned_portals(
        &request.source_restriction,
        response.results.iter().map(|item| &item.url).chain(
            response
                .evidence_audit
                .candidates
                .iter()
                .map(|item| &item.url),
        ),
    )?;
    validate_source_restriction(
        &request.source_restriction,
        response.results.iter().map(|item| &item.url).chain(
            response
                .evidence_audit
                .candidates
                .iter()
                .map(|item| &item.url),
        ),
    )?;
    validate_verification(
        &response.evidence_audit,
        response.results.iter().map(|item| {
            (
                &item.title,
                &item.snippet,
                &item.url,
                item.date.as_deref(),
                item.last_updated.as_deref(),
            )
        }),
        TemporalValidation::new(
            request.verification,
            request.temporal_contract.as_ref(),
            TemporalEvidenceOperation::Search,
        ),
    )
}

fn validate_research_request(
    response: &ResearchResponse,
    request: &ResearchRequest,
) -> Result<(), AgyError> {
    bounded(response.sources.len(), usize::from(request.max_sources))?;
    validate_public_dates_for_request(
        request.verification,
        &response.sources,
        &response.evidence_audit,
    )?;
    validate_caller_owned_portals(
        &request.source_restriction,
        response
            .sources
            .iter()
            .map(|item| &item.url)
            .chain(response.findings.iter().flat_map(|item| &item.citations))
            .chain(
                response
                    .evidence_audit
                    .candidates
                    .iter()
                    .map(|item| &item.url),
            ),
    )?;
    validate_source_restriction(
        &request.source_restriction,
        response
            .sources
            .iter()
            .map(|item| &item.url)
            .chain(response.findings.iter().flat_map(|item| &item.citations))
            .chain(
                response
                    .evidence_audit
                    .candidates
                    .iter()
                    .map(|item| &item.url),
            ),
    )?;
    validate_verification(
        &response.evidence_audit,
        response.sources.iter().map(|item| {
            (
                &item.title,
                &item.snippet,
                &item.url,
                item.date.as_deref(),
                item.last_updated.as_deref(),
            )
        }),
        TemporalValidation::new(
            request.verification,
            request.temporal_contract.as_ref(),
            TemporalEvidenceOperation::Research,
        ),
    )
}

pub(super) fn document(response: &ResponseDocument) -> Result<(), AgyError> {
    match response {
        ResponseDocument::Search(value) => {
            search_document(value).map_err(|_| AgyError::OutputInvalid)
        }
        ResponseDocument::Extract(value) => validate_urls(&value.results, 20, |item| &item.url),
        ResponseDocument::Map(value) => validate_urls(&value.results, 100, |item| &item.url),
        ResponseDocument::Crawl(value) => validate_urls(&value.results, 50, |item| &item.url),
        ResponseDocument::Research(value) => research_validation::validate(value),
        ResponseDocument::Status(_) | ResponseDocument::Models(_) => Ok(()),
    }
}

pub(crate) fn search_document(
    response: &crate::response_models::SearchResponse,
) -> Result<(), SearchDocumentError> {
    validate_urls(&response.results, 20, |item| &item.url)
        .map_err(|_| SearchDocumentError::Invalid)?;
    public_dates::validate_syntax(&response.results).map_err(|_| SearchDocumentError::Invalid)?;
    evidence_audit(
        &response.evidence_audit,
        response.results.iter().map(|item| &item.url),
    )
    .map_err(|error| match error {
        EvidenceAuditError::CandidateInvalid => SearchDocumentError::Invalid,
        EvidenceAuditError::PublicUrlMissing => SearchDocumentError::AuditCoverageMissing,
    })
}

fn validate_source_restriction<'a>(
    restriction: &SourceRestriction,
    mut urls: impl Iterator<Item = &'a HttpUrl>,
) -> Result<(), AgyError> {
    if urls.all(|url| restriction.allows(url)) {
        Ok(())
    } else {
        Err(AgyError::OutputInvalid)
    }
}

fn validate_caller_owned_portals<'a>(
    restriction: &SourceRestriction,
    mut urls: impl Iterator<Item = &'a HttpUrl>,
) -> Result<(), AgyError> {
    if restriction.is_unrestricted() && urls.any(HttpUrl::is_news_portal) {
        Err(AgyError::OutputInvalid)
    } else {
        Ok(())
    }
}

fn validate_urls<'a, Item: 'a>(
    items: &'a [Item],
    maximum: usize,
    url: impl Fn(&'a Item) -> &'a HttpUrl,
) -> Result<(), AgyError> {
    bounded(items.len(), maximum)?;
    let urls: Vec<_> = items.iter().map(url).collect();
    if urls
        .iter()
        .any(|url| url.source_kind() != SourceUrlKind::Direct)
    {
        return Err(AgyError::OutputInvalid);
    }
    unique(urls.into_iter())
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

fn validate_public_dates_for_request(
    verification: VerificationMode,
    sources: &[WebSource],
    audit: &EvidenceAudit,
) -> Result<(), AgyError> {
    match verification {
        VerificationMode::Standard => public_dates::validate_standard_provenance(sources, audit),
        VerificationMode::TemporalComparison => Ok(()),
    }
}
