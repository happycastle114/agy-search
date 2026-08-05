use std::{fmt, str::FromStr};

use clap::ValueEnum;

#[derive(Clone, Debug, Eq, PartialEq)]
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
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
