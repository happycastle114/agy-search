//! Antigravity discovery and schema-constrained content execution.

use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use crate::{
    antigravity_version::{self, Deadline},
    error::AgyError,
    invocation::{Invocation, InvocationCommand},
    process::{ProcessRequest, run},
    response::Document as ResponseDocument,
    types::{Effort, ModelCatalog, ModelSlug, Operation, PreferredSearchModel, VerificationMode},
};

mod content;

#[cfg(test)]
#[path = "backend/catalog_policy_test.rs"]
mod catalog_policy_test;

const MAX_ADVISORY_CATALOG_DISCOVERY: Duration = Duration::from_secs(5);

pub(crate) async fn execute(invocation: Invocation) -> Result<ResponseDocument, AgyError> {
    let Invocation {
        agy_path,
        model,
        effort,
        timeout,
        command,
        output: _,
    } = invocation;
    let cwd = std::env::current_dir().map_err(|_| AgyError::InvalidCommand)?;
    let deadline = Deadline::after(timeout);
    match command {
        InvocationCommand::Status => status(&agy_path, cwd, deadline).await,
        InvocationCommand::Models => models(&agy_path, cwd, deadline)
            .await
            .map(ModelCatalog::into_strings)
            .map(ResponseDocument::models),
        InvocationCommand::Content(request) => {
            antigravity_version::require_supported(&agy_path, cwd.clone(), deadline).await?;
            let selected_model = match model {
                Some(selected) => {
                    validate_model(&agy_path, &cwd, deadline, &selected).await?;
                    Some(selected)
                }
                None => {
                    select_preferred_search_model(&agy_path, &cwd, deadline, &request, effort)
                        .await?
                }
            };
            content::execute(&agy_path, selected_model, effort, deadline, *request).await
        }
    }
}

async fn status(
    executable: &str,
    cwd: PathBuf,
    deadline: Deadline,
) -> Result<ResponseDocument, AgyError> {
    let version = antigravity_version::require_supported(executable, cwd.clone(), deadline).await?;
    let discovered = models(executable, cwd, deadline).await?;
    Ok(ResponseDocument::status(
        version.to_string(),
        discovered.len(),
    ))
}

async fn models(
    executable: &str,
    cwd: PathBuf,
    deadline: Deadline,
) -> Result<ModelCatalog, AgyError> {
    discover_models(executable, cwd, deadline.remaining()?).await
}

async fn discover_models(
    executable: &str,
    cwd: PathBuf,
    timeout: Duration,
) -> Result<ModelCatalog, AgyError> {
    let output = run(ProcessRequest {
        argv: vec![executable.to_owned(), "models".to_owned()],
        cwd,
        timeout,
    })
    .await?;
    ModelCatalog::parse(&output.stdout).map_err(|_| AgyError::OutputInvalid)
}

async fn validate_model(
    executable: &str,
    cwd: &Path,
    deadline: Deadline,
    selected: &ModelSlug,
) -> Result<(), AgyError> {
    if models(executable, cwd.to_path_buf(), deadline)
        .await?
        .contains(selected)
    {
        Ok(())
    } else {
        Err(AgyError::UnknownModel)
    }
}

async fn select_preferred_search_model(
    executable: &str,
    cwd: &Path,
    deadline: Deadline,
    request: &crate::request::ContentRequest,
    effort: Option<Effort>,
) -> Result<Option<ModelSlug>, AgyError> {
    let Some(preferred) =
        preferred_search_model(request.operation(), request.verification(), effort)
    else {
        return Ok(None);
    };
    let timeout = deadline.remaining()?.min(MAX_ADVISORY_CATALOG_DISCOVERY);
    match discover_models(executable, cwd.to_path_buf(), timeout).await {
        Ok(catalog) => Ok(catalog.preferred(preferred)),
        Err(_) if deadline.remaining().is_ok() => Ok(None),
        Err(_) => Err(AgyError::Timeout),
    }
}

const fn preferred_search_model(
    operation: Operation,
    verification: VerificationMode,
    effort: Option<Effort>,
) -> Option<PreferredSearchModel> {
    match operation {
        Operation::Search => match verification {
            VerificationMode::Standard => match effort {
                Some(Effort::Low) => Some(PreferredSearchModel::Gemini36FlashLow),
                Some(Effort::Medium | Effort::High) | None => None,
            },
            VerificationMode::TemporalComparison => None,
        },
        Operation::Extract | Operation::Map | Operation::Crawl | Operation::Research => None,
    }
}
