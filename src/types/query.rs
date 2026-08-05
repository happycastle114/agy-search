use serde::Serialize;

use super::{CalendarDate, NonEmptyText};
use crate::source_restriction::SourceDomain;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub(crate) struct RequiredSearchQuery(String);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ScopedQueryKind {
    Initial,
    ExactValueFollowup,
    Invalid,
}

impl RequiredSearchQuery {
    pub(crate) fn for_exact_scope(
        query: &NonEmptyText,
        scope: &str,
        domains: &[SourceDomain],
        country: Option<&NonEmptyText>,
    ) -> Self {
        let mut required = format!(
            "For exact scope \"{scope}\" only, find its latest release, exact version, and source-published date; do not use another scope's value. Original request constraints: {}",
            query.as_str()
        );
        for domain in domains {
            required.push_str(" site:");
            required.push_str(domain.as_str());
        }
        if let Some(country) = country {
            required.push_str(" country:");
            required.push_str(country.as_str());
        }
        Self(required)
    }

    pub(crate) fn classify(&self, query: &str) -> ScopedQueryKind {
        if query == self.0 {
            return ScopedQueryKind::Initial;
        }
        let Some(token) = query
            .strip_prefix(&self.0)
            .and_then(|suffix| suffix.strip_prefix(' '))
        else {
            return ScopedQueryKind::Invalid;
        };
        let token_is_safe = !token.is_empty()
            && token.len() <= 128
            && !token.chars().any(char::is_whitespace)
            && token.chars().any(char::is_alphanumeric)
            && !token.contains("://")
            && CalendarDate::parse(token).is_err();
        if token_is_safe {
            ScopedQueryKind::ExactValueFollowup
        } else {
            ScopedQueryKind::Invalid
        }
    }
}
