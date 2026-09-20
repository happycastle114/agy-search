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
    source_network::{SafeSourceUrl, SourceNetworkError, resolve},
    types::{
        Effort, GeminiFlashGeneration, ModelCatalog, ModelSlug, Operation, PreferredModel,
        VerificationMode,
    },
};

mod content;

#[cfg(feature = "server")]
mod discovery;
#[cfg(feature = "server")]
pub(crate) use discovery::{DiscoveryCache, execute_cached};

#[cfg(test)]
#[path = "backend/catalog_policy_test.rs"]
mod catalog_policy_test;

const MAX_ADVISORY_CATALOG_DISCOVERY: Duration = Duration::from_secs(5);

struct ContentModels {
    primary: Option<ModelSlug>,
    recoveries: [RecoveryModel; 2],
}

#[derive(Clone)]
enum RecoveryModel {
    Inherit,
    Selected(ModelSlug),
    Disabled,
}

impl ContentModels {
    const fn fixed(model: Option<ModelSlug>) -> Self {
        Self {
            primary: model,
            recoveries: [RecoveryModel::Inherit, RecoveryModel::Inherit],
        }
    }
}

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
            validate_network_targets(&request, deadline).await?;
            antigravity_version::require_supported(&agy_path, cwd.clone(), deadline).await?;
            let selected_models = match model {
                Some(selected) => {
                    validate_model(&agy_path, &cwd, deadline, &selected).await?;
                    ContentModels::fixed(Some(selected))
                }
                None => {
                    select_preferred_content_models(&agy_path, &cwd, deadline, &request, effort)
                        .await?
                }
            };
            content::execute(&agy_path, selected_models, effort, deadline, *request).await
        }
    }
}

async fn validate_network_targets(
    request: &crate::request::ContentRequest,
    deadline: Deadline,
) -> Result<(), AgyError> {
    let targets = match request {
        crate::request::ContentRequest::Map(request) => std::slice::from_ref(&request.url),
        crate::request::ContentRequest::Crawl(request) => std::slice::from_ref(&request.url),
        crate::request::ContentRequest::Search(request) => request.source_restriction.exact_urls(),
        crate::request::ContentRequest::Extract(request) => request.urls.as_slice(),
        crate::request::ContentRequest::Research(request) => {
            request.source_restriction.exact_urls()
        }
    };
    for target in targets {
        let safe =
            SafeSourceUrl::parse_redirect(target.as_str()).map_err(|_| AgyError::InvalidCommand)?;
        resolve(safe, deadline.instant())
            .await
            .map_err(|error| match error {
                SourceNetworkError::Deadline => AgyError::Timeout,
                SourceNetworkError::InvalidUrl
                | SourceNetworkError::UnsafeAddress
                | SourceNetworkError::Dns => AgyError::InvalidCommand,
            })?;
    }
    Ok(())
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

async fn select_preferred_content_models(
    executable: &str,
    cwd: &Path,
    deadline: Deadline,
    request: &crate::request::ContentRequest,
    effort: Option<Effort>,
) -> Result<ContentModels, AgyError> {
    let Some(preference) =
        preferred_model_policy(request.operation(), request.verification(), effort)
    else {
        return Ok(ContentModels::fixed(None));
    };
    let timeout = deadline.remaining()?.min(MAX_ADVISORY_CATALOG_DISCOVERY);
    match discover_models(executable, cwd.to_path_buf(), timeout).await {
        Ok(catalog) => Ok(select_catalog_models(&catalog, preference)),
        Err(_) if deadline.remaining().is_ok() => Ok(ContentModels::fixed(None)),
        Err(_) => Err(AgyError::Timeout),
    }
}

fn select_catalog_models(catalog: &ModelCatalog, preference: ModelPreference) -> ContentModels {
    let primary = catalog.preferred(preference.primary);
    let Some(primary_model) = primary.clone() else {
        return ContentModels::fixed(None);
    };
    let Some(recovery_generation) = preference.recovery_generation else {
        return ContentModels {
            primary,
            recoveries: [
                RecoveryModel::Selected(primary_model.clone()),
                RecoveryModel::Selected(primary_model),
            ],
        };
    };
    let first_recovery = catalog.preferred(PreferredModel::gemini_flash(
        recovery_generation,
        Effort::Medium,
    ));
    let final_recovery = catalog.preferred(PreferredModel::gemini_flash(
        recovery_generation,
        Effort::High,
    ));
    let recoveries = match (first_recovery, final_recovery) {
        (Some(medium), Some(high)) => [
            RecoveryModel::Selected(medium),
            RecoveryModel::Selected(high),
        ],
        (Some(model), None) | (None, Some(model)) => {
            [RecoveryModel::Selected(model), RecoveryModel::Disabled]
        }
        (None, None) => [RecoveryModel::Disabled, RecoveryModel::Disabled],
    };
    ContentModels {
        primary,
        recoveries,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ModelPreference {
    primary: PreferredModel,
    recovery_generation: Option<GeminiFlashGeneration>,
}

const fn preferred_model_policy(
    operation: Operation,
    verification: VerificationMode,
    effort: Option<Effort>,
) -> Option<ModelPreference> {
    let Some(effort) = effort else {
        return None;
    };
    let (generation, recovery_generation) = match (operation, verification, effort) {
        (Operation::Search, VerificationMode::Standard, Effort::Low) => (
            GeminiFlashGeneration::V3_8,
            Some(GeminiFlashGeneration::V3_8),
        ),
        (Operation::Search, VerificationMode::Standard, Effort::Medium | Effort::High) => {
            (GeminiFlashGeneration::V3_8, None)
        }
        (
            Operation::Search,
            VerificationMode::TemporalComparison,
            Effort::Low | Effort::Medium | Effort::High,
        )
        | (
            Operation::Extract | Operation::Map | Operation::Crawl | Operation::Research,
            VerificationMode::Standard | VerificationMode::TemporalComparison,
            Effort::Low | Effort::Medium | Effort::High,
        ) => (GeminiFlashGeneration::V3_8, None),
    };
    Some(ModelPreference {
        primary: PreferredModel::gemini_flash(generation, effort),
        recovery_generation,
    })
}
