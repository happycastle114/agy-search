//! Sanitized failures shared by HTTP and MCP transports.

use crate::AgyError;
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
/// Runtime failures without subprocess output or credentials.
pub enum ServerError {
    /// Invalid wire-level request.
    #[error("invalid request")]
    InvalidInput,
    /// Required runtime configuration is missing or invalid.
    #[error("invalid server configuration")]
    Configuration,
    /// Authentication failed.
    #[error("authentication required")]
    Unauthorized,
    /// Host or browser origin is not allowed.
    #[error("request origin is not allowed")]
    Forbidden,
    /// The configured concurrent request limit has been reached.
    #[error("server is busy; retry later")]
    Busy,
    /// Existing source-verification or downstream failure.
    #[error(transparent)]
    Backend(#[from] AgyError),
    /// Listener or transport failed.
    #[error("server transport failed")]
    Transport,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum ErrorCode {
    InvalidRequest,
    Configuration,
    Unauthorized,
    Forbidden,
    Busy,
    UpstreamUnavailable,
    Timeout,
    InvalidUpstreamResponse,
    Internal,
}

#[derive(Serialize)]
struct Problem {
    error: ErrorBody,
}
#[derive(Serialize)]
struct ErrorBody {
    code: ErrorCode,
    message: String,
}

impl ServerError {
    const fn classification(&self) -> (StatusCode, ErrorCode) {
        match self {
            Self::InvalidInput => (StatusCode::BAD_REQUEST, ErrorCode::InvalidRequest),
            Self::Configuration => (StatusCode::INTERNAL_SERVER_ERROR, ErrorCode::Configuration),
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, ErrorCode::Unauthorized),
            Self::Forbidden => (StatusCode::FORBIDDEN, ErrorCode::Forbidden),
            Self::Busy => (StatusCode::TOO_MANY_REQUESTS, ErrorCode::Busy),
            Self::Transport => (StatusCode::INTERNAL_SERVER_ERROR, ErrorCode::Internal),
            Self::Backend(error) => match error {
                AgyError::InvalidCommand | AgyError::UnknownModel => {
                    (StatusCode::BAD_REQUEST, ErrorCode::InvalidRequest)
                }
                AgyError::Unavailable => (
                    StatusCode::SERVICE_UNAVAILABLE,
                    ErrorCode::UpstreamUnavailable,
                ),
                AgyError::Timeout => (StatusCode::GATEWAY_TIMEOUT, ErrorCode::Timeout),
                AgyError::ProcessFailed => {
                    (StatusCode::BAD_GATEWAY, ErrorCode::UpstreamUnavailable)
                }
                AgyError::OutputInvalid => {
                    (StatusCode::BAD_GATEWAY, ErrorCode::InvalidUpstreamResponse)
                }
                AgyError::OutputWrite => (StatusCode::INTERNAL_SERVER_ERROR, ErrorCode::Internal),
            },
        }
    }
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let (status, code) = self.classification();
        if status.is_server_error() {
            tracing::error!(error = %self, status = status.as_u16(), "request failed");
        }
        let mut response = (
            status,
            Json(Problem {
                error: ErrorBody {
                    code,
                    message: self.to_string(),
                },
            }),
        )
            .into_response();
        if status == StatusCode::UNAUTHORIZED {
            response.headers_mut().insert(
                axum::http::header::WWW_AUTHENTICATE,
                axum::http::HeaderValue::from_static("Bearer"),
            );
        }
        if status == StatusCode::TOO_MANY_REQUESTS {
            response.headers_mut().insert(
                axum::http::header::RETRY_AFTER,
                axum::http::HeaderValue::from_static("1"),
            );
        }
        response
    }
}
