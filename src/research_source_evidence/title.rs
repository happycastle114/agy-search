use std::collections::{HashMap, HashSet};

use super::{
    MAXIMUM_PROJECTED_CONTEXT_CHARACTERS, MINIMUM_PAGE_HEADING_CHARACTERS,
    MINIMUM_VALUE_CHARACTERS,
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
    contexts: &HashMap<SafeSourceUrl, Vec<NonEmptyText>>,
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
    let (first, remaining) = contexts.get(&safe)?.split_first()?;
    let mut snippet = first.as_str().to_owned();
    let mut characters = snippet.chars().count();
    for context in remaining {
        let combined_characters = characters + 1 + context.as_str().chars().count();
        if combined_characters <= MAXIMUM_PROJECTED_CONTEXT_CHARACTERS {
            snippet.push('\n');
            snippet.push_str(context.as_str());
            characters = combined_characters;
        }
    }
    source.snippet = NonEmptyText::parse(&snippet).ok()?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projection_skips_whole_contexts_that_exceed_the_budget_without_mixing_urls() {
        // Given: same-page contexts exceed the budget and another URL has distinct evidence.
        let source: WebSource = serde_json::from_value(serde_json::json!({
            "title": "Verified page title", "url": "https://example.com/page",
            "snippet": "model prose"
        }))
        .expect("valid source");
        let url = SafeSourceUrl::parse_redirect(source.url.as_str()).expect("safe URL");
        let other = SafeSourceUrl::parse_redirect("https://example.com/other").expect("safe URL");
        let first = NonEmptyText::parse("First complete verified context.").expect("valid context");
        let last = NonEmptyText::parse("Final complete verified context.").expect("valid context");
        let oversized = NonEmptyText::parse(&"가".repeat(MAXIMUM_PROJECTED_CONTEXT_CHARACTERS))
            .expect("valid context");
        let contexts = HashMap::from([
            (url.clone(), vec![first.clone(), oversized, last.clone()]),
            (
                other,
                vec![NonEmptyText::parse("Other page evidence").expect("valid context")],
            ),
        ]);
        let pages = HashMap::from([(
            url,
            SearchEvidencePage {
                body: String::new(),
                headings: Vec::new(),
                title: Some("Verified page title".to_owned()),
            },
        )]);

        // When: independently verified contexts are projected for one URL.
        let result = project_search_source(source, &pages, &contexts, &HashMap::new(), &[])
            .expect("projected source");

        // Then: complete contexts fit in order without truncation or evidence from another URL.
        assert_eq!(
            result.snippet.as_str(),
            format!("{}\n{}", first.as_str(), last.as_str())
        );
        assert!(result.snippet.as_str().chars().count() <= MAXIMUM_PROJECTED_CONTEXT_CHARACTERS);
    }
}
