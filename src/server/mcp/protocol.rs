use std::{str::FromStr, sync::Arc};

use rmcp::{
    ErrorData,
    model::{CallToolResult, Tool, ToolAnnotations},
};
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::{Value, json};

use crate::{AgyError, server::ServerError};

#[derive(Clone, Copy, Debug)]
pub(super) enum ToolName {
    Search,
    Extract,
    Map,
    Crawl,
    Research,
}

impl ToolName {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Search => "agy_search",
            Self::Extract => "agy_extract",
            Self::Map => "agy_map",
            Self::Crawl => "agy_crawl",
            Self::Research => "agy_research",
        }
    }

    const fn description(self) -> &'static str {
        match self {
            Self::Search => "Search the web and return source-backed results.",
            Self::Extract => "Extract source-backed content from web pages.",
            Self::Map => "Map links reachable from a website.",
            Self::Crawl => "Crawl and return pages from a website.",
            Self::Research => "Research a topic and return findings with sources.",
        }
    }
}

impl FromStr for ToolName {
    type Err = ErrorData;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "agy_search" => Ok(Self::Search),
            "agy_extract" => Ok(Self::Extract),
            "agy_map" => Ok(Self::Map),
            "agy_crawl" => Ok(Self::Crawl),
            "agy_research" => Ok(Self::Research),
            _ => Err(ErrorData::method_not_found::<
                rmcp::model::CallToolRequestMethod,
            >()),
        }
    }
}

pub(super) fn tool<T: JsonSchema>(name: ToolName) -> Result<Tool, ErrorData> {
    let Value::Object(schema) = serde_json::to_value(schemars::schema_for!(T))
        .map_err(|_| ErrorData::internal_error("tool schema unavailable", None))?
    else {
        return Err(ErrorData::internal_error("tool schema unavailable", None));
    };
    Ok(Tool::new_with_raw(
        name.as_str(),
        Some(name.description().into()),
        Arc::new(schema),
    )
    .with_annotations(
        ToolAnnotations::new()
            .read_only(true)
            .idempotent(true)
            .open_world(true),
    ))
}

pub(super) fn invalid_arguments(error: &serde_json::Error) -> CallToolResult {
    CallToolResult::structured_error(json!({
        "error": {
            "code": McpErrorCode::InvalidArguments,
            "message": format!("invalid tool arguments: {error}"),
        }
    }))
}

pub(super) fn cancelled() -> CallToolResult {
    CallToolResult::structured_error(json!({
        "error": {
            "code": McpErrorCode::Cancelled,
            "message": "request cancelled",
        }
    }))
}

pub(super) fn runtime_result(
    result: Result<crate::response::Document, ServerError>,
) -> CallToolResult {
    match result {
        Ok(document) => serde_json::to_value(document).map_or_else(
            |_| {
                CallToolResult::structured_error(json!({
                    "error": {
                        "code": McpErrorCode::Internal,
                        "message": "response serialization failed",
                    }
                }))
            },
            CallToolResult::structured,
        ),
        Err(error) => {
            if matches!(
                error,
                ServerError::Backend(
                    AgyError::Unavailable
                        | AgyError::Timeout
                        | AgyError::ProcessFailed
                        | AgyError::OutputInvalid
                        | AgyError::OutputWrite
                )
            ) {
                tracing::error!(error = %error, "MCP runtime request failed");
            }
            CallToolResult::structured_error(json!({
                "error": {
                    "code": McpErrorCode::from(&error),
                    "message": error.to_string(),
                }
            }))
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum McpErrorCode {
    InvalidArguments,
    Cancelled,
    Busy,
    UpstreamUnavailable,
    Internal,
}

impl From<&ServerError> for McpErrorCode {
    fn from(error: &ServerError) -> Self {
        match error {
            ServerError::InvalidInput => Self::InvalidArguments,
            ServerError::Busy => Self::Busy,
            ServerError::Backend(AgyError::InvalidCommand | AgyError::UnknownModel) => {
                Self::InvalidArguments
            }
            ServerError::Backend(_) => Self::UpstreamUnavailable,
            ServerError::Configuration
            | ServerError::Unauthorized
            | ServerError::Forbidden
            | ServerError::Transport => Self::Internal,
        }
    }
}
