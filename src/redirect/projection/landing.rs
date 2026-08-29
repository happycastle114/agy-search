//! Safe promotion from a generic landing page to one unique deep title link.

use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use tokio::time::Instant;

use crate::{
    error::AgyError,
    response_models::ResponseDocument,
    source_document_html::{elements_with_attribute_on, normalize_text},
    source_fetch::{SafeSourceUrl, SourceFetcher},
    source_restriction::SourceRestriction,
    types::HttpUrl,
};

use super::super::{RESOLVER_TIMEOUT, curl_executable};

pub(super) async fn promote_search_landings(
    response: &mut ResponseDocument,
    restriction: Option<&SourceRestriction>,
) -> Result<Vec<HttpUrl>, AgyError> {
    let landings = response.search_landing_sources()?;
    if landings.is_empty() {
        return Ok(Vec::new());
    }
    let safe_landings = landings
        .iter()
        .map(|(url, _)| {
            SafeSourceUrl::parse_redirect(url.as_str()).map_err(|_| AgyError::OutputInvalid)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let source_reader = SourceFetcher::new(PathBuf::from(curl_executable()?));
    let Ok(fetched_pages) = source_reader
        .fetch_many(&safe_landings, Instant::now() + RESOLVER_TIMEOUT)
        .await
    else {
        return Ok(Vec::new());
    };
    let mut promoted_sources = Vec::new();
    for ((landing_url, title), page) in landings.into_iter().zip(fetched_pages) {
        let (safe_landing, body) = page.into_parts();
        let Some(target) = unique_deep_title_link(&safe_landing, &body, &title, restriction) else {
            continue;
        };
        response.replace_url(&landing_url, target.source());
        promoted_sources.push(target.source().clone());
    }
    Ok(promoted_sources)
}

fn unique_deep_title_link(
    landing: &SafeSourceUrl,
    body: &str,
    title: &str,
    restriction: Option<&SourceRestriction>,
) -> Option<SafeSourceUrl> {
    let title = normalize_text(title).to_ascii_lowercase();
    if title.chars().count() < 12 {
        return None;
    }
    let links = elements_with_attribute_on(body, Some("a"), "href").ok()?;
    let mut matches: HashMap<SafeSourceUrl, (usize, usize)> = HashMap::new();
    for link in links {
        let label = normalize_text(&link.content).to_ascii_lowercase();
        let score = title_match_score(&title, &label)
            .or_else(|| landing.source().is_latest_landing().then_some((0, 0)));
        let Some(score) = score else { continue };
        let Some(target) = eligible_deep_link(landing, link.value.as_deref(), restriction) else {
            continue;
        };
        matches
            .entry(target)
            .and_modify(|current| *current = (*current).max(score))
            .or_insert(score);
    }
    let best_score = matches.values().max().copied()?;
    let mut best = matches
        .into_iter()
        .filter_map(|(target, score)| (score == best_score).then_some(target));
    let target = best.next()?;
    best.next().is_none().then_some(target)
}

fn title_match_score(title: &str, label: &str) -> Option<(usize, usize)> {
    let title_tokens = identity_tokens(title);
    let label_tokens = identity_tokens(label);
    let label_characters = label.chars().count();
    let shared = label_tokens
        .iter()
        .filter(|token| title_tokens.contains(*token))
        .count();
    (label_characters >= 8
        && label_tokens.len() >= 2
        && shared >= 2
        && shared == title_tokens.len().min(label_tokens.len()))
    .then_some((shared, label_characters))
}

fn identity_tokens(value: &str) -> HashSet<String> {
    value
        .split(|character: char| !character.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(str::to_owned)
        .collect()
}

fn eligible_deep_link(
    landing: &SafeSourceUrl,
    href: Option<&str>,
    restriction: Option<&SourceRestriction>,
) -> Option<SafeSourceUrl> {
    let href = href?.trim();
    if href.starts_with("//") || href.split('/').any(|segment| matches!(segment, "." | "..")) {
        return None;
    }
    let target = if href.starts_with('/') || href.starts_with("https://") {
        landing.join_redirect(href).ok()?
    } else {
        return None;
    };
    (landing.source().same_origin(target.source())
        && !target.source().is_site_root()
        && restriction.is_none_or(|allowed| allowed.allows_evidence_url(target.source())))
    .then_some(target)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn promotes_unique_deep_links_and_rejects_ambiguous_links()
    -> Result<(), Box<dyn std::error::Error>> {
        let landing = SafeSourceUrl::parse_redirect("https://example.com/news-room")?;
        let title = "Second meeting of the example emergency committee";
        let html = "<a href='/news/item/report'>28 August 2026 Statement Second meeting of the example emergency committee</a>";
        assert_eq!(
            unique_deep_title_link(&landing, html, title, None)
                .ok_or(AgyError::OutputInvalid)?
                .as_str(),
            "https://example.com/news/item/report"
        );
        for invalid in [
            "<a href='/one'>Second meeting of the example emergency committee</a><a href='/two'>Second meeting of the example emergency committee</a>",
            "<a href='https://other.example/item'>Second meeting of the example emergency committee</a>",
            "<a href='../item'>Second meeting of the example emergency committee</a>",
        ] {
            assert!(unique_deep_title_link(&landing, invalid, title, None).is_none());
        }
        Ok(())
    }
}
