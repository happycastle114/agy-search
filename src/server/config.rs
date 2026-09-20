//! Typed startup configuration; credentials never become command-line values.

use crate::types::{Effort, ModelSlug, TimeoutSeconds};
use clap::{Args, Parser, Subcommand};
use std::{net::SocketAddr, num::NonZeroU16};

#[derive(Debug, Parser)]
#[command(
    name = "agy-search-server",
    version,
    about = "Source-backed search for LiteLLM and MCP"
)]
/// Command-line options for the optional server binary.
pub struct ServerCli {
    #[command(flatten)]
    pub(crate) runtime: RuntimeOptions,
    #[command(subcommand)]
    pub(crate) command: ServerCommand,
}

#[derive(Clone, Debug, Args)]
pub(crate) struct RuntimeOptions {
    /// Antigravity executable; each request runs in its own isolated directory.
    #[arg(
        long,
        env = "AGY_SEARCH_AGY_PATH",
        default_value = "agy",
        global = true
    )]
    pub(crate) agy_path: String,
    /// Optional exact model slug, always checked against fresh discovery.
    #[arg(long, env = "AGY_SEARCH_MODEL", global = true)]
    pub(crate) model: Option<ModelSlug>,
    /// Default reasoning effort for all transports.
    #[arg(long, env = "AGY_SEARCH_EFFORT", default_value = "low", global = true)]
    pub(crate) effort: Effort,
    /// Total request deadline, including batch queries and discovery.
    #[arg(long, env = "AGY_SEARCH_TIMEOUT", default_value = "120", global = true)]
    pub(crate) timeout: TimeoutSeconds,
    /// Maximum active requests; excess requests fail immediately with busy.
    #[arg(
        long,
        env = "AGY_SEARCH_MAX_CONCURRENCY",
        default_value = "2",
        global = true
    )]
    pub(crate) max_concurrency: NonZeroU16,
    /// Advisory model catalog lifetime; zero disables reuse.
    #[arg(long, env = "AGY_SEARCH_CATALOG_TTL_SECONDS", default_value = "60", value_parser = clap::value_parser!(u64).range(0..=3600), global = true)]
    pub(crate) catalog_ttl_seconds: u64,
}

#[derive(Debug, Subcommand)]
pub(crate) enum ServerCommand {
    /// Serve LiteLLM-compatible POST /search and Streamable HTTP MCP /mcp.
    Http(HttpOptions),
    /// Serve MCP over stdin/stdout; diagnostics stay on stderr.
    Stdio,
}

#[derive(Debug, Args)]
pub(crate) struct HttpOptions {
    /// Socket to listen on. Network exposure requires a bearer token.
    #[arg(long, env = "AGY_SEARCH_LISTEN", default_value = "127.0.0.1:18091")]
    pub(crate) listen: SocketAddr,
    /// Name of the environment variable containing the bearer token.
    #[arg(
        long,
        env = "AGY_SEARCH_API_KEY_ENV",
        default_value = "AGY_SEARCH_API_KEY"
    )]
    pub(crate) api_key_env: String,
    /// Additional allowed HTTP host, without a port; repeat for proxies.
    #[arg(
        long = "allowed-host",
        env = "AGY_SEARCH_ALLOWED_HOSTS",
        value_delimiter = ','
    )]
    pub(crate) allowed_hosts: Vec<String>,
    /// Exact browser Origin permitted to call the server; none by default.
    #[arg(
        long = "allowed-origin",
        env = "AGY_SEARCH_ALLOWED_ORIGINS",
        value_delimiter = ','
    )]
    pub(crate) allowed_origins: Vec<String>,
    /// Maximum body size for both /search and /mcp.
    #[arg(long, env = "AGY_SEARCH_BODY_LIMIT_BYTES", default_value = "131072", value_parser = clap::value_parser!(u64).range(1024..=1048576))]
    pub(crate) body_limit_bytes: u64,
}
