use std::{fmt, str::FromStr};

use clap::ValueEnum;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct ModelSlug(String);

impl ModelSlug {
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn effort_suffix(&self) -> Option<Effort> {
        <Effort as ValueEnum>::from_str(self.0.rsplit('-').next()?, false).ok()
    }
}

impl FromStr for ModelSlug {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.is_empty() || value.trim() != value || value.chars().any(char::is_whitespace) {
            return Err("model must be one non-empty slug");
        }
        Ok(Self(value.to_owned()))
    }
}

/// One validated model exposed by the live Antigravity catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ResolvedModel {
    slug: ModelSlug,
}

impl ResolvedModel {
    const fn new(slug: ModelSlug) -> Self {
        Self { slug }
    }
}

/// Validated, duplicate-free model discovery for one Antigravity installation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ModelCatalog {
    models: Vec<ResolvedModel>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ModelCatalogError {
    InvalidUtf8,
    InvalidSlug,
    DuplicateModel,
    Empty,
}

impl ModelCatalog {
    pub(crate) fn parse(output: &[u8]) -> Result<Self, ModelCatalogError> {
        let text = std::str::from_utf8(output).map_err(|_| ModelCatalogError::InvalidUtf8)?;
        let mut models = Vec::new();
        for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
            let slug = line
                .split_once('\t')
                .map_or(line, |(slug, _display_name)| slug);
            let resolved = ResolvedModel::new(
                ModelSlug::from_str(slug).map_err(|_| ModelCatalogError::InvalidSlug)?,
            );
            if models.contains(&resolved) {
                return Err(ModelCatalogError::DuplicateModel);
            }
            models.push(resolved);
        }
        if models.is_empty() {
            return Err(ModelCatalogError::Empty);
        }
        Ok(Self { models })
    }

    pub(crate) const fn len(&self) -> usize {
        self.models.len()
    }

    pub(crate) fn contains(&self, selected: &ModelSlug) -> bool {
        self.models.contains(&ResolvedModel::new(selected.clone()))
    }

    pub(crate) fn preferred(&self, preferred: PreferredModel) -> Option<ModelSlug> {
        let preferred = ResolvedModel::from(preferred);
        self.models
            .iter()
            .find(|candidate| *candidate == &preferred)
            .map(|candidate| candidate.slug.clone())
    }

    pub(crate) fn into_strings(self) -> Vec<String> {
        self.models.into_iter().map(|model| model.slug.0).collect()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum GeminiFlashGeneration {
    V3_8,
}

impl fmt::Display for GeminiFlashGeneration {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::V3_8 => "3.8",
        })
    }
}

/// One typed, catalog-negotiated Gemini Flash preference.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct PreferredModel {
    generation: GeminiFlashGeneration,
    effort: Effort,
}

impl PreferredModel {
    pub(crate) const fn gemini_flash(generation: GeminiFlashGeneration, effort: Effort) -> Self {
        Self { generation, effort }
    }

    fn slug(self) -> String {
        format!("gemini-{}-flash-{}", self.generation, self.effort)
    }
}

impl From<PreferredModel> for ResolvedModel {
    fn from(preferred: PreferredModel) -> Self {
        Self::new(ModelSlug(preferred.slug()))
    }
}

#[derive(
    Clone, Copy, Debug, Eq, PartialEq, ValueEnum, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Effort {
    Low,
    Medium,
    High,
}

impl fmt::Display for Effort {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        })
    }
}
