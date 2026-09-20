//! LiteLLM-compatible HTTP endpoints and guarded MCP transport.

use super::{
    Runtime, ServerError,
    config::HttpOptions,
    input::SearchInput,
    mcp::{self, McpHttpConfig},
    security::{self, HttpSecurity},
};
use crate::response::Document;
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Request, State, rejection::JsonRejection},
    http::StatusCode,
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::Serialize;
use std::sync::Arc;
use tokio::{net::TcpListener, signal};
use tokio_util::sync::CancellationToken;
use tower_http::{
    limit::RequestBodyLimitLayer,
    trace::{DefaultOnResponse, TraceLayer},
};

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum HealthStatus {
    Ok,
}

#[derive(Debug, Serialize)]
struct Health {
    status: HealthStatus,
    version: &'static str,
}

async fn health() -> Json<Health> {
    Json(Health {
        status: HealthStatus::Ok,
        version: env!("CARGO_PKG_VERSION"),
    })
}

async fn search(
    State(runtime): State<Runtime>,
    input: Result<Json<SearchInput>, JsonRejection>,
) -> Result<Json<Document>, Response> {
    let Json(input) = input.map_err(|error| {
        if error.status() == StatusCode::PAYLOAD_TOO_LARGE {
            StatusCode::PAYLOAD_TOO_LARGE.into_response()
        } else {
            ServerError::InvalidInput.into_response()
        }
    })?;
    runtime
        .search(input)
        .await
        .map(Json)
        .map_err(IntoResponse::into_response)
}

async fn readiness(State(runtime): State<Runtime>) -> Result<Json<Document>, ServerError> {
    runtime.readiness().await.map(Json)
}

async fn until_shutdown(
    State(cancellation): State<CancellationToken>,
    request: Request,
    next: Next,
) -> Result<Response, ServerError> {
    tokio::select! {
        () = cancellation.cancelled() => Err(ServerError::Transport),
        response = next.run(request) => Ok(response),
    }
}

fn router(
    runtime: Runtime,
    options: &HttpOptions,
    security: HttpSecurity,
    cancellation: CancellationToken,
) -> Result<Router, ServerError> {
    let limit =
        usize::try_from(options.body_limit_bytes).map_err(|_| ServerError::Configuration)?;
    let mcp = mcp::router(
        runtime.clone(),
        McpHttpConfig {
            allowed_hosts: security.hosts(),
            allowed_origins: security.origins(),
            cancellation: cancellation.clone(),
        },
    );
    let protected = Router::new()
        .route("/search", post(search))
        .route("/readyz", get(readiness))
        .with_state(runtime)
        .merge(mcp)
        .layer(DefaultBodyLimit::max(limit))
        .layer(RequestBodyLimitLayer::new(limit))
        .layer(middleware::from_fn_with_state(cancellation, until_shutdown))
        .layer(middleware::from_fn_with_state(
            Arc::new(security),
            security::authorize,
        ));
    let tracing = TraceLayer::new_for_http()
        .on_failure(())
        .on_response(DefaultOnResponse::new().level(tracing::Level::INFO))
        .make_span_with(|request: &Request| {
            tracing::info_span!("http_request", method = %request.method(), path = request.uri().path())
        });
    Ok(Router::new()
        .route("/healthz", get(health))
        .merge(protected)
        .layer(tracing))
}

pub(super) async fn serve(runtime: Runtime, options: HttpOptions) -> Result<(), ServerError> {
    let security = HttpSecurity::load(&options)?;
    let cancellation = CancellationToken::new();
    let app = router(runtime, &options, security, cancellation.clone())?;
    let listener = TcpListener::bind(options.listen)
        .await
        .map_err(|_| ServerError::Transport)?;
    let address = listener.local_addr().map_err(|_| ServerError::Transport)?;
    tracing::info!(%address, "HTTP and MCP server listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown(cancellation))
        .await
        .map_err(|_| ServerError::Transport)
}

async fn shutdown(cancellation: CancellationToken) {
    #[cfg(unix)]
    {
        match signal::unix::signal(signal::unix::SignalKind::terminate()) {
            Ok(mut terminate) => {
                tokio::select! { _ = signal::ctrl_c() => {}, _ = terminate.recv() => {} }
            }
            Err(_) => {
                let _ = signal::ctrl_c().await;
            }
        }
    }
    #[cfg(not(unix))]
    let _ = signal::ctrl_c().await;
    cancellation.cancel();
}

#[cfg(test)]
#[path = "http_test.rs"]
mod tests;
