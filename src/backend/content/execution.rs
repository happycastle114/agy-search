//! Isolated Antigravity execution for one content request.

use std::{path::Path, time::Duration};

use tempfile::Builder;
use tokio::time;

use crate::{
    antigravity_version::Deadline,
    error::AgyError,
    events::{StructuredRunError, parse_structured_run},
    process::{ProcessRequest, run},
    redirect::{StandardSearchResolution, resolve_grounding_run, resolve_standard_search_run},
    response::Document as ResponseDocument,
    types::{Effort, ModelSlug, Operation, OutputFormat, ResearchToolPolicy},
};

const ISOLATION_DIRECTORY: &str = "agy-search";
const ISOLATION_PREFIX: &str = "agy-search-";
const AGENT_NAME: &str = "agy-search";
const AGENT_DIRECTORY: &str = ".agents/agents/agy-search";
const AGENT_DEFINITION: &str = r"---
name: agy-search
description: Isolated source-backed web search for one schema-constrained request.
tools:
  - search_web
  - read_url_content
  - view_file
  - grep_search
mainAgent: true
subagent: false
inheritMcp: false
model: inherit
commandExecutionPolicy: off
---

Follow the caller's operation, source policy, tool budget, and output schema exactly. Use only the tools listed above.
";

#[derive(Clone)]
pub(super) struct ExecutionContext {
    pub(super) executable: String,
    pub(super) model: Option<ModelSlug>,
    pub(super) retry_model: Option<ModelSlug>,
    pub(super) effort: Option<Effort>,
    pub(super) deadline: Deadline,
}

impl ExecutionContext {
    pub(super) fn for_standard_retry(&self) -> Self {
        let model = self.retry_model.clone().or_else(|| self.model.clone());
        let effort = model
            .as_ref()
            .and_then(ModelSlug::effort_suffix)
            .or(self.effort);
        Self {
            executable: self.executable.clone(),
            retry_model: model.clone(),
            model,
            effort,
            deadline: self.deadline,
        }
    }
}

pub(super) struct ContentExecution {
    pub(super) operation: Operation,
    pub(super) tool_policy: ResearchToolPolicy,
    pub(super) schema: String,
    pub(super) prompt: String,
}

#[derive(Debug)]
pub(super) enum StandardSearchRun {
    Response(ResponseDocument),
    NoReachableResults,
    RecoverableUnlistedTool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum GroundingResolutionMode {
    Strict,
    StandardSearchProjection,
}

pub(super) async fn run_content_once(
    context: &ExecutionContext,
    execution: ContentExecution,
) -> Result<ResponseDocument, AgyError> {
    let StandardSearchRun::Response(response) =
        run_content(context, execution, GroundingResolutionMode::Strict).await?
    else {
        return Err(AgyError::OutputInvalid);
    };
    response.validate()?;
    Ok(response)
}

pub(super) async fn run_standard_search_unvalidated_once(
    context: &ExecutionContext,
    execution: ContentExecution,
) -> Result<StandardSearchRun, AgyError> {
    run_content(
        context,
        execution,
        GroundingResolutionMode::StandardSearchProjection,
    )
    .await
}

async fn run_content(
    context: &ExecutionContext,
    execution: ContentExecution,
    resolution: GroundingResolutionMode,
) -> Result<StandardSearchRun, AgyError> {
    let ContentExecution {
        operation,
        tool_policy,
        schema,
        prompt,
    } = execution;
    let remaining = context.deadline.remaining()?;
    let isolation_base = std::env::temp_dir().join(ISOLATION_DIRECTORY);
    std::fs::create_dir_all(&isolation_base).map_err(|_| AgyError::InvalidCommand)?;
    let isolated = Builder::new()
        .prefix(ISOLATION_PREFIX)
        .tempdir_in(isolation_base)
        .map_err(|_| AgyError::InvalidCommand)?;
    install_agent(isolated.path())?;
    let output = run(ProcessRequest {
        argv: print_argv(context, remaining, schema, prompt),
        cwd: isolated.path().to_path_buf(),
        timeout: remaining,
    })
    .await?;
    let parsed = match parse_structured_run(&output.stdout, operation, &tool_policy) {
        Ok(parsed) => parsed,
        Err(StructuredRunError::RecoverableUnlistedTool(response))
            if resolution == GroundingResolutionMode::StandardSearchProjection =>
        {
            response
                .validate_search_document()
                .map_err(|_| AgyError::OutputInvalid)?;
            return Ok(StandardSearchRun::RecoverableUnlistedTool);
        }
        Err(error) => return Err(error.into_public_error()),
    };
    let normalization_remaining = context.deadline.remaining()?;
    match resolution {
        GroundingResolutionMode::Strict => {
            let run = time::timeout(
                normalization_remaining,
                resolve_grounding_run(parsed, isolated.path()),
            )
            .await
            .map_err(|_| AgyError::Timeout)??;
            Ok(StandardSearchRun::Response(run.into_response()))
        }
        GroundingResolutionMode::StandardSearchProjection => {
            let resolution = time::timeout(
                normalization_remaining,
                resolve_standard_search_run(parsed, isolated.path()),
            )
            .await
            .map_err(|_| AgyError::Timeout)??;
            match resolution {
                StandardSearchResolution::Resolved(run) => {
                    Ok(StandardSearchRun::Response(run.into_response()))
                }
                StandardSearchResolution::NoReachableResults => {
                    Ok(StandardSearchRun::NoReachableResults)
                }
            }
        }
    }
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
        "--agent".to_owned(),
        AGENT_NAME.to_owned(),
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

fn install_agent(root: &Path) -> Result<(), AgyError> {
    let directory = root.join(AGENT_DIRECTORY);
    std::fs::create_dir_all(&directory).map_err(|_| AgyError::InvalidCommand)?;
    std::fs::write(directory.join("agent.md"), AGENT_DEFINITION)
        .map_err(|_| AgyError::InvalidCommand)
}
