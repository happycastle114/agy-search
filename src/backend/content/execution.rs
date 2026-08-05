//! Isolated Antigravity execution for one content request.

use std::time::Duration;

use tempfile::Builder;
use tokio::time;

use crate::{
    antigravity_version::Deadline,
    error::AgyError,
    events::parse_structured_run,
    process::{ProcessRequest, run},
    redirect::resolve_grounding_run,
    response::Document as ResponseDocument,
    types::{Effort, ModelSlug, Operation, OutputFormat, ResearchToolPolicy},
};

const ISOLATION_DIRECTORY: &str = "agy-search";
const ISOLATION_PREFIX: &str = "agy-search-";

#[derive(Clone)]
pub(super) struct ExecutionContext {
    pub(super) executable: String,
    pub(super) model: Option<ModelSlug>,
    pub(super) effort: Option<Effort>,
    pub(super) deadline: Deadline,
}

pub(super) async fn run_content_once(
    context: &ExecutionContext,
    operation: Operation,
    tool_policy: ResearchToolPolicy,
    schema: String,
    prompt: String,
) -> Result<ResponseDocument, AgyError> {
    let remaining = context.deadline.remaining()?;
    let isolation_base = std::env::temp_dir().join(ISOLATION_DIRECTORY);
    std::fs::create_dir_all(&isolation_base).map_err(|_| AgyError::InvalidCommand)?;
    let isolated = Builder::new()
        .prefix(ISOLATION_PREFIX)
        .tempdir_in(isolation_base)
        .map_err(|_| AgyError::InvalidCommand)?;
    let output = run(ProcessRequest {
        argv: print_argv(context, remaining, schema, prompt),
        cwd: isolated.path().to_path_buf(),
        timeout: remaining,
    })
    .await?;
    let parsed = parse_structured_run(&output.stdout, operation, &tool_policy)?;
    let normalization_remaining = context.deadline.remaining()?;
    let response = time::timeout(
        normalization_remaining,
        resolve_grounding_run(parsed, isolated.path()),
    )
    .await
    .map_err(|_| AgyError::Timeout)??
    .into_response();
    response.validate()?;
    Ok(response)
}

fn print_argv(
    context: &ExecutionContext,
    remaining: Duration,
    schema: String,
    prompt: String,
) -> Vec<String> {
    let mut argv = vec![
        context.executable.clone(),
        "--disable-slash-commands".to_owned(),
        "--print-timeout".to_owned(),
        format!("{}s", remaining.as_secs_f64()),
        "--output-format".to_owned(),
        OutputFormat::StreamJson.to_string(),
        "--json-schema".to_owned(),
        schema,
    ];
    if let Some(selected) = &context.model {
        argv.extend(["--model".to_owned(), selected.as_str().to_owned()]);
    }
    if let Some(selected) = context.effort {
        argv.extend(["--effort".to_owned(), selected.to_string()]);
    }
    argv.extend(["-p".to_owned(), prompt]);
    argv
}
