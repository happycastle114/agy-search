mod protocol;

use std::{future::Future, str::FromStr};

use axum::Router;
use rmcp::{
    ErrorData, RoleServer, ServerHandler, ServiceExt,
    model::{
        CacheScope, CallToolRequestParams, CallToolResponse, Implementation, JsonObject,
        ListToolsResult, PaginatedRequestParams, ServerCapabilities, ServerConfig,
    },
    service::{RequestContext, RunningService},
    transport::streamable_http_server::{
        StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
    },
};
use serde::de::DeserializeOwned;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use super::{
    Runtime, ServerError,
    input::{CrawlInput, ExtractInput, MapInput, ResearchInput, SearchInput},
};
use protocol::{ToolName, cancelled, invalid_arguments, runtime_result, tool};

const STDIO_SHUTDOWN_GRACE: std::time::Duration = std::time::Duration::from_secs(1);
const TOOL_LIST_CACHE_TTL_MS: u64 = 0;

#[derive(Clone, Debug)]
struct McpServer {
    runtime: Runtime,
}

impl McpServer {
    const fn new(runtime: Runtime) -> Self {
        Self { runtime }
    }

    async fn invoke<T, F>(
        &self,
        arguments: Option<JsonObject>,
        context: RequestContext<RoleServer>,
        operation: impl FnOnce(Runtime, T) -> F,
    ) -> CallToolResponse
    where
        T: DeserializeOwned,
        F: Future<Output = Result<crate::response::Document, ServerError>>,
    {
        let input = match serde_json::from_value(Value::Object(arguments.unwrap_or_default())) {
            Ok(input) => input,
            Err(error) => return invalid_arguments(&error).into(),
        };
        let runtime = self.runtime.clone();
        let result = tokio::select! {
            biased;
            () = context.ct.cancelled() => return cancelled().into(),
            result = operation(runtime, input) => result,
        };
        runtime_result(result).into()
    }
}

impl ServerHandler for McpServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("agy-search", env!("CARGO_PKG_VERSION")))
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult::with_all_items(vec![
            tool::<SearchInput>(ToolName::Search)?,
            tool::<ExtractInput>(ToolName::Extract)?,
            tool::<MapInput>(ToolName::Map)?,
            tool::<CrawlInput>(ToolName::Crawl)?,
            tool::<ResearchInput>(ToolName::Research)?,
        ])
        .with_ttl_ms(TOOL_LIST_CACHE_TTL_MS)
        .with_cache_scope(CacheScope::Private))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let tool = ToolName::from_str(request.name.as_ref())?;
        let result = match tool {
            ToolName::Search => {
                self.invoke(request.arguments, context, |runtime, input| async move {
                    runtime.search(input).await
                })
                .await
            }
            ToolName::Extract => {
                self.invoke(request.arguments, context, |runtime, input| async move {
                    runtime.extract(input).await
                })
                .await
            }
            ToolName::Map => {
                self.invoke(request.arguments, context, |runtime, input| async move {
                    runtime.map(input).await
                })
                .await
            }
            ToolName::Crawl => {
                self.invoke(request.arguments, context, |runtime, input| async move {
                    runtime.crawl(input).await
                })
                .await
            }
            ToolName::Research => {
                self.invoke(request.arguments, context, |runtime, input| async move {
                    runtime.research(input).await
                })
                .await
            }
        };
        Ok(result)
    }
}

#[derive(Debug)]
pub(super) struct McpHttpConfig {
    pub(super) allowed_hosts: Vec<String>,
    pub(super) allowed_origins: Vec<String>,
    pub(super) cancellation: CancellationToken,
}

pub(super) async fn serve_stdio(runtime: Runtime) -> Result<(), ServerError> {
    let service = McpServer::new(runtime)
        .serve(rmcp::transport::stdio())
        .await
        .map_err(|_| ServerError::Transport)?;
    if wait_for_stdio_shutdown(service, stdio_shutdown()).await? {
        tracing::info!("stdio MCP server stopped by signal");
    }
    Ok(())
}

async fn stdio_shutdown() -> std::io::Result<()> {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
        tokio::select! {
            result = tokio::signal::ctrl_c() => result,
            _ = terminate.recv() => Ok(()),
        }
    }
    #[cfg(not(unix))]
    tokio::signal::ctrl_c().await
}

async fn wait_for_stdio_shutdown(
    service: RunningService<RoleServer, McpServer>,
    shutdown: impl Future<Output = std::io::Result<()>>,
) -> Result<bool, ServerError> {
    let cancellation = service.cancellation_token();
    let mut waiting = Box::pin(service.waiting());
    let signalled = tokio::select! {
        result = &mut waiting => {
            result.map_err(|_| ServerError::Transport)?;
            false
        }
        result = shutdown => {
            result.map_err(|_| ServerError::Transport)?;
            cancellation.cancel();
            if let Ok(result) = tokio::time::timeout(STDIO_SHUTDOWN_GRACE, &mut waiting).await {
                result.map_err(|_| ServerError::Transport)?;
            }
            true
        }
    };
    Ok(signalled)
}

pub(super) fn router(runtime: Runtime, config: McpHttpConfig) -> Router {
    let mut transport = StreamableHttpServerConfig::default()
        .with_legacy_session_mode(false)
        .with_json_response(true)
        .with_sse_keep_alive(None)
        .with_allowed_origins(config.allowed_origins)
        .enforce_origin_validation()
        .with_cancellation_token(config.cancellation);
    if !config.allowed_hosts.is_empty() {
        transport = transport.with_allowed_hosts(config.allowed_hosts);
    }
    let service: StreamableHttpService<McpServer, LocalSessionManager> = StreamableHttpService::new(
        move || Ok(McpServer::new(runtime.clone())),
        std::sync::Arc::default(),
        transport,
    );
    Router::new().nest_service("/mcp", service)
}

#[cfg(test)]
#[path = "mcp_test.rs"]
mod tests;

#[cfg(all(test, unix))]
#[path = "mcp/cancellation_test.rs"]
mod cancellation_tests;
