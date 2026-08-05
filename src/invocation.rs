//! Conversion from parsed arguments into validated runtime requests.

use std::{
    io::{self, Read},
    path::{Path, PathBuf},
};

use crate::{
    cli::{Cli, Command, QueryArgument},
    error::AgyError,
    request::{
        ContentRequest, CrawlRequest, ExtractRequest, MapRequest, ResearchRequest, SearchRequest,
    },
    source_restriction::SourceRestriction,
    temporal_contract::TemporalContract,
    types::{
        DatePolicy, Effort, ModelSlug, NonEmptyText, ResearchAttemptBudget, ScopePolicy,
        SourcePolicy, TimeoutSeconds, VerificationMode,
    },
};

const STDIN_LIMIT_BYTES: u64 = 100 * 1024;

#[derive(Debug)]
pub(crate) struct Invocation {
    pub(crate) agy_path: String,
    pub(crate) model: Option<ModelSlug>,
    pub(crate) effort: Option<Effort>,
    pub(crate) timeout: TimeoutSeconds,
    pub(crate) command: InvocationCommand,
    pub(crate) output: Option<PathBuf>,
}

#[derive(Debug)]
pub(crate) enum InvocationCommand {
    Status,
    Models,
    Content(Box<ContentRequest>),
}

impl Cli {
    pub(crate) fn into_invocation(self) -> Result<Invocation, AgyError> {
        validate_model_effort(self.model.as_ref(), self.effort)?;
        let (command, output) = convert_command(self.command, self.verification)?;
        Ok(Invocation {
            agy_path: resolve_executable(&self.agy_path)?,
            model: self.model,
            effort: self.effort,
            timeout: self.timeout,
            command,
            output,
        })
    }
}

fn validate_model_effort(
    model: Option<&ModelSlug>,
    effort: Option<Effort>,
) -> Result<(), AgyError> {
    let Some(model_effort) = model.and_then(ModelSlug::effort_suffix) else {
        return Ok(());
    };
    let Some(effort) = effort else {
        return Ok(());
    };
    if model_effort == effort {
        Ok(())
    } else {
        Err(AgyError::InvalidCommand)
    }
}

fn convert_command(
    command: Command,
    verification: VerificationMode,
) -> Result<(InvocationCommand, Option<PathBuf>), AgyError> {
    let converted = match command {
        Command::Status(output) => (InvocationCommand::Status, output.output),
        Command::Models(output) => (InvocationCommand::Models, output.output),
        Command::Search(arguments) => {
            let source_urls = arguments.source_urls;
            let source_restriction =
                SourceRestriction::parse(arguments.domains, source_urls.clone())?;
            let temporal_source_urls = match verification {
                VerificationMode::Standard => Vec::new(),
                VerificationMode::TemporalComparison => source_urls,
            };
            let temporal_contract = TemporalContract::parse(
                verification,
                arguments.scopes,
                temporal_source_urls,
                arguments.cutoff,
            )?;
            let request = SearchRequest {
                query: resolve_query(arguments.query)?,
                source_policy: SourcePolicy::PrimaryFirst,
                scope_policy: ScopePolicy::CompleteRequestedScope,
                date_policy: DatePolicy::ExplicitSourceOnly,
                verification,
                temporal_contract,
                source_restriction,
                max_results: arguments.max_results,
                country: arguments.country,
                max_tokens_per_page: arguments.max_tokens_per_page,
            };
            (
                InvocationCommand::Content(Box::new(ContentRequest::Search(request))),
                arguments.output.output,
            )
        }
        Command::Extract(arguments) => {
            let request = ExtractRequest {
                urls: arguments.urls,
                query: arguments.query,
            };
            (
                InvocationCommand::Content(Box::new(ContentRequest::Extract(request))),
                arguments.output.output,
            )
        }
        Command::Map(arguments) => {
            let request = MapRequest {
                url: arguments.url,
                limit: arguments.limit,
                instructions: arguments.instructions,
                allow_external: arguments.allow_external,
            };
            (
                InvocationCommand::Content(Box::new(ContentRequest::Map(request))),
                arguments.output.output,
            )
        }
        Command::Crawl(arguments) => {
            let request = CrawlRequest {
                url: arguments.url,
                limit: arguments.limit,
                instructions: arguments.instructions,
                allow_external: arguments.allow_external,
            };
            (
                InvocationCommand::Content(Box::new(ContentRequest::Crawl(request))),
                arguments.output.output,
            )
        }
        Command::Research(arguments) => {
            let source_urls = arguments.source_urls;
            let source_restriction =
                SourceRestriction::parse(arguments.domains, source_urls.clone())?;
            let temporal_source_urls = match verification {
                VerificationMode::Standard => Vec::new(),
                VerificationMode::TemporalComparison => source_urls,
            };
            let temporal_contract = TemporalContract::parse(
                verification,
                arguments.scopes,
                temporal_source_urls,
                arguments.cutoff,
            )?;
            let request = ResearchRequest {
                query: resolve_query(arguments.query)?,
                source_policy: SourcePolicy::PrimaryFirst,
                scope_policy: ScopePolicy::CompleteRequestedScope,
                date_policy: DatePolicy::ExplicitSourceOnly,
                verification,
                temporal_contract,
                source_restriction,
                max_sources: arguments.max_sources,
                tool_call_budget: ResearchAttemptBudget::from_max_sources(arguments.max_sources),
            };
            (
                InvocationCommand::Content(Box::new(ContentRequest::Research(request))),
                arguments.output.output,
            )
        }
    };
    Ok(converted)
}

fn resolve_query(argument: QueryArgument) -> Result<NonEmptyText, AgyError> {
    match argument {
        QueryArgument::Text(text) => Ok(text),
        QueryArgument::Stdin => read_query_from_stdin(),
    }
}

fn read_query_from_stdin() -> Result<NonEmptyText, AgyError> {
    let mut bytes = Vec::new();
    io::stdin()
        .take(STDIN_LIMIT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| AgyError::InvalidCommand)?;
    let maximum = usize::try_from(STDIN_LIMIT_BYTES).map_err(|_| AgyError::InvalidCommand)?;
    if bytes.len() > maximum {
        return Err(AgyError::InvalidCommand);
    }
    let text = String::from_utf8(bytes).map_err(|_| AgyError::InvalidCommand)?;
    NonEmptyText::parse(&text).map_err(|_| AgyError::InvalidCommand)
}

fn resolve_executable(value: &str) -> Result<String, AgyError> {
    if value.trim().is_empty() {
        return Err(AgyError::InvalidCommand);
    }
    let path = Path::new(value);
    if path.is_absolute() || path.components().count() > 1 {
        let absolute = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()
                .map_err(|_| AgyError::InvalidCommand)?
                .join(path)
        };
        return Ok(absolute.to_string_lossy().into_owned());
    }
    Ok(value.to_owned())
}
