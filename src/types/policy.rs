use std::fmt;

use clap::ValueEnum;
use schemars::JsonSchema;
use serde::Serialize;

use super::RequiredSearchQuery;
use crate::source_restriction::SourceRestriction;

#[derive(Clone, Copy, Debug, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SourcePolicy {
    PrimaryFirst,
}

#[derive(Clone, Copy, Debug, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ScopePolicy {
    CompleteRequestedScope,
}

#[derive(Clone, Copy, Debug, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DatePolicy {
    ExplicitSourceOnly,
}

#[derive(Clone, Copy, Debug, Default, Eq, JsonSchema, PartialEq, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub(crate) enum VerificationMode {
    #[default]
    Standard,
    TemporalComparison,
}

impl fmt::Display for VerificationMode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Standard => "standard",
            Self::TemporalComparison => "temporal-comparison",
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Operation {
    Search,
    Extract,
    Map,
    Crawl,
    Research,
}

#[derive(Clone, Copy, Debug, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(transparent)]
pub(crate) struct ResearchAttemptBudget(u16);

impl ResearchAttemptBudget {
    const MAXIMUM: u16 = 12;
    const OVERHEAD: u16 = 2;

    pub(crate) fn from_max_sources(max_sources: u16) -> Self {
        Self(
            max_sources
                .saturating_add(Self::OVERHEAD)
                .min(Self::MAXIMUM),
        )
    }

    pub(crate) fn maximum(self) -> usize {
        usize::from(self.0)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ResearchToolBudget {
    Single,
    StandardSearch,
    TemporalSearch,
    Research(ResearchAttemptBudget),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ResearchToolPolicy {
    Budget(ResearchToolBudget),
    Restricted {
        budget: ResearchToolBudget,
        restriction: Box<SourceRestriction>,
    },
    ScopedTemporalSearch(RequiredSearchQuery),
    RestrictedScopedTemporalSearch {
        required_query: RequiredSearchQuery,
        restriction: Box<SourceRestriction>,
    },
}

impl ResearchToolPolicy {
    pub(crate) fn maximum(&self) -> usize {
        match self {
            Self::Budget(budget)
            | Self::Restricted {
                budget,
                restriction: _,
            } => budget.maximum(),
            Self::ScopedTemporalSearch(_)
            | Self::RestrictedScopedTemporalSearch {
                required_query: _,
                restriction: _,
            } => 2,
        }
    }
}

impl ResearchToolBudget {
    pub(crate) fn maximum(self) -> usize {
        match self {
            Self::Single => 1,
            Self::StandardSearch => 2,
            Self::TemporalSearch => 8,
            Self::Research(budget) => budget.maximum(),
        }
    }
}

impl fmt::Display for Operation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Search => "search",
            Self::Extract => "extract",
            Self::Map => "map",
            Self::Crawl => "crawl",
            Self::Research => "research",
        })
    }
}
