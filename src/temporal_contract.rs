//! Caller-owned scope and source inventory for relative temporal comparisons.

use std::{collections::HashSet, str::FromStr};

use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    error::AgyError,
    source_network::SafeSourceUrl,
    types::{CalendarDate, HttpScheme, HttpUrl, VerificationMode},
};

const MIN_SCOPES: usize = 1;
const MAX_SCOPES: usize = 8;
const MAX_SOURCES: usize = 8;
const MAX_SCOPE_BYTES: usize = 128;

#[derive(Clone, Debug, Eq, Hash, JsonSchema, PartialEq, Serialize)]
#[serde(transparent)]
pub(crate) struct ScopeLabel(String);

impl ScopeLabel {
    pub(crate) fn parse(value: &crate::types::NonEmptyText) -> Result<Self, AgyError> {
        Self::from_str(value.as_str()).map_err(|_| AgyError::OutputInvalid)
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for ScopeLabel {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.is_empty() || value.trim() != value {
            return Err("scope must be non-empty without surrounding whitespace");
        }
        if value.len() > MAX_SCOPE_BYTES {
            return Err("scope must be at most 128 bytes");
        }
        Ok(Self(value.to_owned()))
    }
}

#[derive(Clone, Debug, JsonSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TemporalContract {
    expected_scopes: Vec<ScopeLabel>,
    source_urls: Vec<HttpUrl>,
    cutoff: Option<CalendarDate>,
    #[serde(skip)]
    #[schemars(skip)]
    safe_source_urls: Vec<SafeSourceUrl>,
}

impl TemporalContract {
    pub(crate) fn parse(
        verification: VerificationMode,
        expected_scopes: Vec<ScopeLabel>,
        source_urls: Vec<HttpUrl>,
        cutoff: Option<CalendarDate>,
    ) -> Result<Option<Self>, AgyError> {
        match verification {
            VerificationMode::Standard => {
                if expected_scopes.is_empty() && source_urls.is_empty() && cutoff.is_none() {
                    Ok(None)
                } else {
                    Err(AgyError::InvalidCommand)
                }
            }
            VerificationMode::TemporalComparison => {
                let safe_source_urls = source_urls
                    .iter()
                    .map(|url| SafeSourceUrl::parse(url.as_str()))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|_| AgyError::InvalidCommand)?;
                if !(MIN_SCOPES..=MAX_SCOPES).contains(&expected_scopes.len())
                    || (expected_scopes.len() == 1 && cutoff.is_none())
                    || !(1..=MAX_SOURCES).contains(&source_urls.len())
                    || expected_scopes.iter().collect::<HashSet<_>>().len() != expected_scopes.len()
                    || source_urls.iter().collect::<HashSet<_>>().len() != source_urls.len()
                    || source_urls
                        .iter()
                        .any(|url| url.scheme() != Some(HttpScheme::Https))
                {
                    return Err(AgyError::InvalidCommand);
                }
                Ok(Some(Self {
                    expected_scopes,
                    source_urls,
                    cutoff,
                    safe_source_urls,
                }))
            }
        }
    }

    pub(crate) fn expected_scopes(&self) -> &[ScopeLabel] {
        &self.expected_scopes
    }

    pub(crate) fn source_urls(&self) -> &[HttpUrl] {
        &self.source_urls
    }

    pub(crate) fn safe_source_urls(&self) -> &[SafeSourceUrl] {
        &self.safe_source_urls
    }

    pub(crate) const fn cutoff(&self) -> Option<&CalendarDate> {
        self.cutoff.as_ref()
    }

    pub(crate) fn allows_date(&self, date: &CalendarDate) -> bool {
        self.cutoff.as_ref().is_none_or(|cutoff| date <= cutoff)
    }

    pub(crate) fn allows_source(&self, source: &HttpUrl) -> bool {
        self.source_urls.contains(source)
    }

    pub(crate) fn has_exact_scopes(&self, scopes: &[ScopeLabel]) -> bool {
        scopes.len() == self.expected_scopes.len()
            && scopes.iter().collect::<HashSet<_>>().len() == scopes.len()
            && scopes
                .iter()
                .all(|scope| self.expected_scopes.contains(scope))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_rejects_temporal_only_values() {
        let scope = ScopeLabel::from_str("alpha").expect("valid scope");
        let source = HttpUrl::parse("https://example.com/releases").expect("valid URL");
        assert!(matches!(
            TemporalContract::parse(VerificationMode::Standard, vec![scope], vec![source], None),
            Err(AgyError::InvalidCommand)
        ));
    }
}
