use std::collections::{HashMap, HashSet};

use super::{
    MINIMUM_PAGE_HEADING_CHARACTERS, MINIMUM_VALUE_CHARACTERS,
    search::SearchEvidencePage,
    text::{canonical_evidence_text, find_ascii_case_insensitive},
};
use crate::{
    response_models::{ScopeEvidence, WebSource},
    source_fetch::SafeSourceUrl,
    types::NonEmptyText,
};

pub(super) fn project_search_source(
    mut source: WebSource,
    pages: &HashMap<SafeSourceUrl, SearchEvidencePage>,
    contexts: &HashMap<SafeSourceUrl, NonEmptyText>,
    identities: &HashMap<crate::types::HttpUrl, NonEmptyText>,
    candidates: &[ScopeEvidence],
) -> Option<WebSource> {
    let safe = SafeSourceUrl::parse_redirect(source.url.as_str()).ok()?;
    let page = pages.get(&safe)?;
    if source.url.is_structured_collection() {
        source.date = None;
        source.last_updated = None;
    }
    let supports_public_date = source.date.as_ref().is_none_or(|date| {
        candidates.iter().any(|candidate| {
            candidate.url == source.url
                && candidate
                    .date
                    .as_ref()
                    .is_some_and(|candidate_date| candidate_date.as_str() == date)
        })
    });
    if !supports_public_date {
        return None;
    }
    let model_title = canonical_evidence_text(source.title.as_str());
    if let Some(title) = &page.title {
        source.title = NonEmptyText::parse(title).ok()?;
    } else if model_title.chars().count() < MINIMUM_VALUE_CHARACTERS
        || find_ascii_case_insensitive(&page.body, &model_title).is_none()
    {
        if let Some(heading) = matching_page_heading(&page.headings, &model_title) {
            source.title = NonEmptyText::parse(heading).ok()?;
        } else {
            let identity = identities.get(&source.url)?.clone();
            if !model_title
                .to_ascii_lowercase()
                .contains(&identity.as_str().to_ascii_lowercase())
            {
                return None;
            }
            source.title = identity;
        }
    }
    source.snippet = contexts.get(&safe)?.clone();
    Some(source)
}

fn matching_page_heading<'a>(headings: &'a [String], title: &str) -> Option<&'a str> {
    let title_tokens = identity_tokens(title);
    let mut seen = HashSet::new();
    let mut matches = headings
        .iter()
        .filter(|heading| seen.insert(heading.as_str()))
        .filter_map(|heading| {
            let heading_tokens = identity_tokens(heading);
            let shared = heading_tokens
                .iter()
                .filter(|token| title_tokens.contains(*token))
                .count();
            let eligible = heading.chars().count() >= MINIMUM_PAGE_HEADING_CHARACTERS
                && !heading_tokens.is_empty()
                && shared.saturating_mul(3) >= heading_tokens.len().saturating_mul(2);
            eligible.then_some((shared, heading.chars().count(), heading.as_str()))
        })
        .collect::<Vec<_>>();
    matches.sort_unstable_by(|left, right| right.cmp(left));
    let best = matches.first()?;
    if matches
        .get(1)
        .is_some_and(|runner_up| runner_up.0 == best.0 && runner_up.1 == best.1)
    {
        None
    } else {
        Some(best.2)
    }
}

fn identity_tokens(value: &str) -> HashSet<String> {
    value
        .split(|character: char| !character.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(normalize_identity_token)
        .collect()
}

fn normalize_identity_token(token: &str) -> String {
    let mut normalized = token.to_lowercase();
    let has_numeric_date_suffix = normalized
        .chars()
        .last()
        .is_some_and(|suffix| ['년', '월', '일'].contains(&suffix))
        && normalized
            .chars()
            .take(normalized.chars().count().saturating_sub(1))
            .all(|character| character.is_ascii_digit());
    if has_numeric_date_suffix {
        normalized.pop();
    }
    normalized
}
