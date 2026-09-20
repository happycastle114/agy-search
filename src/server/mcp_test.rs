use std::{num::NonZeroU16, path::Path, str::FromStr};

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use rmcp::{
    ServiceExt,
    model::{CallToolRequestParams, JsonObject},
    transport::{
        StreamableHttpClientTransport, streamable_http_client::StreamableHttpClientTransportConfig,
    },
};
use serde_json::json;
use tower::ServiceExt as _;

use super::*;
use crate::{
    server::config::RuntimeOptions,
    types::{Effort, TimeoutSeconds},
};

const TOOL_NAMES: [&str; 5] = [
    "agy_search",
    "agy_extract",
    "agy_map",
    "agy_crawl",
    "agy_research",
];

fn runtime() -> Runtime {
    let executable = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake_agy.py");
    Runtime::new(RuntimeOptions {
        agy_path: executable.to_string_lossy().into_owned(),
        model: None,
        effort: Effort::Low,
        timeout: TimeoutSeconds::from_str("10").expect("valid fixture timeout"),
        max_concurrency: NonZeroU16::new(2).expect("non-zero fixture concurrency"),
        catalog_ttl_seconds: 60,
    })
    .expect("fixture runtime")
}

#[tokio::test]
async fn stdio_initializes_lists_and_rejects_invalid_tool_arguments() {
    // Given an official SDK client connected to the MCP server through a duplex stdio stream.
    let (server_transport, client_transport) = tokio::io::duplex(64 * 1024);
    let server = tokio::spawn(async move {
        McpServer::new(runtime())
            .serve(server_transport)
            .await
            .expect("server initializes")
            .waiting()
            .await
    });
    let client = ().serve(client_transport).await.expect("client initializes");
    assert_server_identity(&client);

    // When the client lists tools and calls search with a schema-invalid payload.
    let listed = client.list_tools(None).await.expect("tools/list succeeds");
    let invalid = client
        .call_tool(
            CallToolRequestParams::new("agy_search")
                .with_arguments(JsonObject::from_iter([("query".to_owned(), json!(" "))])),
        )
        .await
        .expect("invalid arguments are a tool result");
    let mapped = client
        .call_tool(
            CallToolRequestParams::new("agy_map").with_arguments(JsonObject::from_iter([
                ("url".to_owned(), json!("https://example.com")),
                ("limit".to_owned(), json!(1)),
            ])),
        )
        .await
        .expect("fixture tool call succeeds");

    // Then all exact tools carry safe hints and invalid input is a caller-visible tool error.
    let names = listed
        .tools
        .iter()
        .map(|tool| tool.name.as_ref())
        .collect::<Vec<_>>();
    assert_eq!(names, TOOL_NAMES);
    assert_eq!(listed.ttl_ms, Some(0));
    assert_eq!(listed.cache_scope, Some(rmcp::model::CacheScope::Private));
    assert!(listed.tools.iter().all(|tool| {
        tool.annotations.as_ref().is_some_and(|annotations| {
            annotations.read_only_hint == Some(true)
                && annotations.idempotent_hint == Some(true)
                && annotations.open_world_hint == Some(true)
        })
    }));
    assert_eq!(invalid.is_error, Some(true));
    assert!(invalid.structured_content.is_some());
    assert_eq!(mapped.is_error, Some(false));
    assert_eq!(
        mapped
            .structured_content
            .as_ref()
            .and_then(|value| value.get("object")),
        Some(&json!("map"))
    );

    client.cancel().await.expect("client teardown");
    server
        .await
        .expect("server task joins")
        .expect("server teardown");
}

#[tokio::test]
async fn streamable_http_initializes_lists_and_calls_fixture_tool() {
    // Given a stateless JSON MCP router served on an ephemeral loopback listener.
    let cancellation = CancellationToken::new();
    let app = router(
        runtime(),
        McpHttpConfig {
            allowed_hosts: vec!["127.0.0.1".to_owned()],
            allowed_origins: Vec::new(),
            cancellation: cancellation.child_token(),
        },
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("ephemeral listener binds");
    let address = listener.local_addr().expect("listener address");
    let server = tokio::spawn({
        let cancellation = cancellation.clone();
        async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(cancellation.cancelled_owned())
                .await
        }
    });
    let transport = StreamableHttpClientTransport::from_config(
        StreamableHttpClientTransportConfig::with_uri(format!("http://{address}/mcp")),
    );
    let client = ().serve(transport).await.expect("HTTP client initializes");
    assert_server_identity(&client);

    // When the official client lists tools and calls a deterministic map fixture.
    let listed = client.list_tools(None).await.expect("tools/list succeeds");
    let mapped = client
        .call_tool(
            CallToolRequestParams::new("agy_map").with_arguments(JsonObject::from_iter([
                ("url".to_owned(), json!("https://example.com")),
                ("limit".to_owned(), json!(1)),
            ])),
        )
        .await
        .expect("tools/call succeeds");

    // Then HTTP exposes the same five tools and structured result as stdio.
    let names = listed
        .tools
        .iter()
        .map(|tool| tool.name.as_ref())
        .collect::<Vec<_>>();
    assert_eq!(names, TOOL_NAMES);
    assert_eq!(listed.ttl_ms, Some(0));
    assert_eq!(listed.cache_scope, Some(rmcp::model::CacheScope::Private));
    assert_eq!(mapped.is_error, Some(false));
    assert_eq!(
        mapped
            .structured_content
            .as_ref()
            .and_then(|value| value.get("object")),
        Some(&json!("map"))
    );

    client.cancel().await.expect("HTTP client teardown");
    cancellation.cancel();
    server
        .await
        .expect("HTTP server task joins")
        .expect("HTTP server teardown");
}

#[tokio::test]
async fn streamable_http_rejects_unlisted_host_and_origin() {
    // Given explicit production host and browser-origin allowlists.
    let app = router(
        runtime(),
        McpHttpConfig {
            allowed_hosts: vec!["127.0.0.1".to_owned()],
            allowed_origins: vec!["https://allowed.example".to_owned()],
            cancellation: CancellationToken::new(),
        },
    );
    let initialize = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {"name": "security-test", "version": "1"}
        }
    });

    // When requests carry either an unlisted Host or an unlisted Origin.
    let unlisted_host = app
        .clone()
        .oneshot(mcp_request(
            &initialize,
            "unlisted.example",
            "https://allowed.example",
        ))
        .await
        .expect("host rejection response");
    let unlisted_origin = app
        .oneshot(mcp_request(
            &initialize,
            "127.0.0.1",
            "https://unlisted.example",
        ))
        .await
        .expect("origin rejection response");

    // Then the SDK protection rejects both before MCP initialization.
    assert_eq!(unlisted_host.status(), StatusCode::FORBIDDEN);
    assert_eq!(unlisted_origin.status(), StatusCode::FORBIDDEN);
}

fn mcp_request(payload: &serde_json::Value, host: &str, origin: &str) -> Request<Body> {
    Request::post("/mcp")
        .header("host", host)
        .header("origin", origin)
        .header("content-type", "application/json")
        .header("accept", "application/json, text/event-stream")
        .body(Body::from(payload.to_string()))
        .expect("valid MCP request")
}

fn assert_server_identity(client: &rmcp::service::RunningService<rmcp::RoleClient, ()>) {
    let info = client.peer_info().expect("server initialization result");
    let identity = info.server_info.as_ref().expect("server identity");
    assert_eq!(identity.name, "agy-search");
    assert_eq!(identity.version, env!("CARGO_PKG_VERSION"));
}
