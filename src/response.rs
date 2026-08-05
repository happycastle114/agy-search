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
    ) -> Result<String, AgyError> {
        schema::render(
            operation,
            verification,
            temporal_contract,
            source_restriction,
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
}
