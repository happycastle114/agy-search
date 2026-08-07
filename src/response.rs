//! Response document construction and module-level entry points.

use crate::{
    error::AgyError,
    response_models::{
        ModelsObject, ModelsResponse, ResponseDocument, StatusObject, StatusResponse,
    },
};

mod collection_validation;
mod parsing;
mod public_dates;
mod research_validation;
mod schema;
mod scope_schema;
mod source_schema;
#[cfg(test)]
#[path = "response/response_test.rs"]
mod tests;
mod validation;

pub(crate) use scope_schema::temporal_scope_schema;

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

    pub(crate) fn schema(
        operation: crate::types::Operation,
        verification: crate::types::VerificationMode,
        temporal_contract: Option<&crate::temporal_contract::TemporalContract>,
        source_restriction: &crate::source_restriction::SourceRestriction,
        search_result_limit: Option<u16>,
    ) -> Result<String, AgyError> {
        schema::render(
            operation,
            verification,
            temporal_contract,
            source_restriction,
            search_result_limit,
        )
    }

    pub(crate) fn parse(
        operation: crate::types::Operation,
        value: serde_json::Value,
    ) -> Result<Self, AgyError> {
        parsing::parse(operation, value)
    }

    pub(crate) fn validate_request(
        &self,
        request: &crate::request::ContentRequest,
    ) -> Result<(), AgyError> {
        validation::request(self, request)
    }

    pub(crate) fn validate(&self) -> Result<(), AgyError> {
        validation::document(self)
    }

    pub(crate) fn validate_search_document(&self) -> Result<(), validation::SearchDocumentError> {
        match self {
            Self::Search(response) => validation::search_document(response),
            Self::Extract(_)
            | Self::Map(_)
            | Self::Crawl(_)
            | Self::Research(_)
            | Self::Status(_)
            | Self::Models(_) => Err(validation::SearchDocumentError::Invalid),
        }
    }

    pub(crate) fn project_unbound_standard_search_dates(&mut self) -> Result<(), AgyError> {
        match self {
            Self::Search(response) => public_dates::project_unbound_standard_dates(
                &mut response.results,
                &response.evidence_audit,
            ),
            Self::Extract(_)
            | Self::Map(_)
            | Self::Crawl(_)
            | Self::Research(_)
            | Self::Status(_)
            | Self::Models(_) => Err(AgyError::OutputInvalid),
        }
    }
}
