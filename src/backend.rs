//! Antigravity discovery and schema-constrained content execution.

use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    str::FromStr,
};

use crate::{
    antigravity_version::{self, Deadline},
    error::AgyError,
    invocation::{Invocation, InvocationCommand},
    process::{ProcessRequest, run},
    response::Document as ResponseDocument,
    types::ModelSlug,
};

mod content;

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
            .map(ResponseDocument::models),
        InvocationCommand::Content(request) => {
            antigravity_version::require_supported(&agy_path, cwd.clone(), deadline).await?;
            if let Some(selected) = &model {
                validate_model(&agy_path, &cwd, deadline, selected).await?;
            }
            content::execute(&agy_path, model, effort, deadline, *request).await
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
) -> Result<Vec<String>, AgyError> {
    let output = run(ProcessRequest {
        argv: vec![executable.to_owned(), "models".to_owned()],
        cwd,
        timeout: deadline.remaining()?,
    })
    .await?;
    let models = decode_lines(&output.stdout)?;
    if models.is_empty()
        || models
            .iter()
            .any(|model| ModelSlug::from_str(model).is_err())
        || models.iter().collect::<HashSet<_>>().len() != models.len()
    {
        return Err(AgyError::OutputInvalid);
    }
    Ok(models)
}

async fn validate_model(
    executable: &str,
    cwd: &Path,
    deadline: Deadline,
    selected: &ModelSlug,
) -> Result<(), AgyError> {
    if models(executable, cwd.to_path_buf(), deadline)
        .await?
        .iter()
        .any(|model| model == selected.as_str())
    {
        Ok(())
    } else {
        Err(AgyError::UnknownModel)
    }
}

fn decode_lines(output: &[u8]) -> Result<Vec<String>, AgyError> {
    let text = std::str::from_utf8(output).map_err(|_| AgyError::OutputInvalid)?;
    Ok(text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect())
}
