//! Clap surface for every public operation.

use std::{path::PathBuf, str::FromStr};

use clap::{Args, Parser, Subcommand};

use crate::{
    source_restriction::SourceDomain,
    temporal_contract::ScopeLabel,
    types::{
        CalendarDate, Effort, HttpUrl, ModelSlug, NonEmptyText, TimeoutSeconds, VerificationMode,
    },
};

#[derive(Debug, Parser)]
#[command(
    name = "agy-search",
    version,
    about = "Source-backed web research through Google Antigravity",
    arg_required_else_help = true
)]
/// Parsed public options for one standalone CLI invocation.
pub struct Cli {
    /// Antigravity executable name or path.
    #[arg(
        long,
        env = "AGY_SEARCH_AGY_PATH",
        default_value = "agy",
        global = true
    )]
    pub(crate) agy_path: String,
    /// Dynamically discovered Antigravity model slug.
    #[arg(long, global = true)]
    pub(crate) model: Option<ModelSlug>,
    /// Antigravity reasoning effort.
    #[arg(long, default_value = "low", global = true)]
    pub(crate) effort: Option<Effort>,
    /// Additional evidence requirements for time-ordered scope comparisons.
    #[arg(long, default_value_t, global = true)]
    pub(crate) verification: VerificationMode,
    /// End-to-end downstream deadline in seconds.
    #[arg(long, default_value = "120", global = true)]
    pub(crate) timeout: TimeoutSeconds,
    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Check Antigravity availability and summarize discovered models.
    Status(OutputArgs),
    /// List models exposed by the local Antigravity installation.
    Models(OutputArgs),
    /// Search the web and return ranked sources.
    Search(SearchArgs),
    /// Extract source-backed content from exact URLs.
    Extract(ExtractArgs),
    /// Discover URLs beneath a site.
    Map(SiteArgs),
    /// Crawl a site and return extracted pages.
    Crawl(CrawlArgs),
    /// Produce a cited multi-source research answer.
    Research(ResearchArgs),
}

#[derive(Clone, Debug, Args)]
pub(crate) struct OutputArgs {
    /// Write canonical JSON to this file instead of stdout.
    #[arg(short = 'o', long)]
    pub(crate) output: Option<PathBuf>,
    /// Emit canonical JSON. JSON is already the default success format.
    #[arg(long)]
    pub(crate) _json: bool,
}

#[derive(Debug, Args)]
pub(crate) struct SearchArgs {
    /// Query text, or `-` to read up to 100 KiB from stdin.
    pub(crate) query: QueryArgument,
    #[arg(short = 'n', long, default_value_t = 5, value_parser = clap::value_parser!(u16).range(1..=20))]
    pub(crate) max_results: u16,
    /// Restrict discovery to a domain; repeat up to 20 times.
    #[arg(long = "domain")]
    pub(crate) domains: Vec<SourceDomain>,
    #[arg(long)]
    pub(crate) country: Option<NonEmptyText>,
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..))]
    pub(crate) max_tokens_per_page: Option<u32>,
    /// Exact caller-owned temporal scope; repeat 1 to 8 times. One requires --as-of.
    #[arg(long = "scope")]
    pub(crate) scopes: Vec<ScopeLabel>,
    /// Canonical HTTPS evidence source; repeat for every permitted source.
    #[arg(long = "source-url")]
    pub(crate) source_urls: Vec<HttpUrl>,
    /// Latest permitted source-published date for temporal evidence.
    #[arg(long = "as-of")]
    pub(crate) cutoff: Option<CalendarDate>,
    #[command(flatten)]
    pub(crate) output: OutputArgs,
}

#[derive(Debug, Args)]
pub(crate) struct ExtractArgs {
    /// One or more explicit HTTP(S) URLs.
    #[arg(required = true, num_args = 1..=20)]
    pub(crate) urls: Vec<HttpUrl>,
    #[arg(long)]
    pub(crate) query: Option<NonEmptyText>,
    #[command(flatten)]
    pub(crate) output: OutputArgs,
}

#[derive(Debug, Args)]
pub(crate) struct SiteArgs {
    pub(crate) url: HttpUrl,
    #[arg(long, default_value_t = 50, value_parser = clap::value_parser!(u16).range(1..=100))]
    pub(crate) limit: u16,
    #[arg(long)]
    pub(crate) instructions: Option<NonEmptyText>,
    #[arg(long)]
    pub(crate) allow_external: bool,
    #[command(flatten)]
    pub(crate) output: OutputArgs,
}

#[derive(Debug, Args)]
pub(crate) struct CrawlArgs {
    pub(crate) url: HttpUrl,
    #[arg(long, default_value_t = 20, value_parser = clap::value_parser!(u16).range(1..=50))]
    pub(crate) limit: u16,
    #[arg(long)]
    pub(crate) instructions: Option<NonEmptyText>,
    #[arg(long)]
    pub(crate) allow_external: bool,
    #[command(flatten)]
    pub(crate) output: OutputArgs,
}

#[derive(Debug, Args)]
pub(crate) struct ResearchArgs {
    /// Question text, or `-` to read up to 100 KiB from stdin.
    pub(crate) query: QueryArgument,
    #[arg(long, default_value_t = 10, value_parser = clap::value_parser!(u16).range(1..=20))]
    pub(crate) max_sources: u16,
    /// Restrict sources to a domain tree; repeat up to 20 times.
    #[arg(long = "domain")]
    pub(crate) domains: Vec<SourceDomain>,
    /// Exact caller-owned temporal scope; repeat 1 to 8 times. One requires --as-of.
    #[arg(long = "scope")]
    pub(crate) scopes: Vec<ScopeLabel>,
    /// Canonical HTTPS evidence source; repeat for every permitted source.
    #[arg(long = "source-url")]
    pub(crate) source_urls: Vec<HttpUrl>,
    /// Latest permitted source-published date for temporal evidence.
    #[arg(long = "as-of")]
    pub(crate) cutoff: Option<CalendarDate>,
    #[command(flatten)]
    pub(crate) output: OutputArgs,
}

#[derive(Clone, Debug)]
pub(crate) enum QueryArgument {
    Stdin,
    Text(NonEmptyText),
}

impl FromStr for QueryArgument {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "-" => Ok(Self::Stdin),
            text => NonEmptyText::parse(text).map(Self::Text),
        }
    }
}
