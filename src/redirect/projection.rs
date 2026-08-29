//! Best-effort grounding projection for latency-sensitive standard Search.

use std::{
    collections::{BTreeMap, HashMap, HashSet},
    path::PathBuf,
};

use tokio::task::JoinSet;
use tokio::time::Instant;

use super::{RESOLVER_TIMEOUT, RedirectResolver, curl_executable, new_resolver};
use crate::{
    error::AgyError,
    events::{GroundingRequirement, GroundingResolved, ParsedRun, PendingGrounding},
    response_models::ResponseDocument,
    source_document_html::{elements_with_attribute_on, normalize_text},
    source_fetch::{SafeSourceUrl, SourceFetcher},
    source_restriction::SourceRestriction,
    types::HttpUrl,
};

const MAX_PROJECTION_CONCURRENCY: usize = 4;

#[derive(Debug)]
pub(crate) enum StandardSearchResolution {
    Resolved(ParsedRun<GroundingResolved>),
    NoReachableResults,
}

#[derive(Debug)]
pub(super) enum SourceOutcome {
    Reachable(HttpUrl),
    Dead,
}

pub(crate) async fn resolve_standard_search_run(
    mut run: ParsedRun<PendingGrounding>,
    cwd: &std::path::Path,
) -> Result<StandardSearchResolution, AgyError> {
    run.response.reject_duplicate_search_urls()?;
    let mut response_transports = run.response.grounding_redirects();
    let restricted = match &run.grounding {
        GroundingRequirement::None => None,
        GroundingRequirement::Restricted {
            transports,
            restriction,
        } => Some((transports.clone(), restriction.clone())),
    };
    let has_direct_sources = !run.response.direct_search_urls()?.is_empty();
    let needs_resolver = !response_transports.is_empty()
        || has_direct_sources
        || restricted
            .as_ref()
            .is_some_and(|(transports, _)| !transports.is_empty());
    let resolver = if needs_resolver {
        Some(new_resolver(cwd)?)
    } else {
        None
    };
    let mut verified_sources = HashSet::new();

    if let Some((transports, restriction)) = &restricted {
        let restricted_transports = transports.iter().cloned().collect::<HashSet<_>>();
        if !transports.is_empty() {
            let resolver = resolver.as_ref().ok_or(AgyError::OutputInvalid)?;
            for transport in transports {
                let direct = resolver.resolve_restricted(transport, restriction).await?;
                if !restriction.allows(&direct) {
                    return Err(AgyError::OutputInvalid);
                }
                run.response.replace_url(transport, &direct);
                verified_sources.insert(direct);
            }
        }
        response_transports.retain(|transport| !restricted_transports.contains(transport));
    }

    if !response_transports.is_empty() {
        let resolver = resolver.as_ref().ok_or(AgyError::OutputInvalid)?.clone();
        let restriction = restricted
            .as_ref()
            .map(|(_, restriction)| restriction.clone());
        for (transport, outcome) in
            resolve_bounded(resolver, response_transports, restriction).await?
        {
            match outcome {
                SourceOutcome::Reachable(direct) => {
                    run.response.replace_url(&transport, &direct);
                    verified_sources.insert(direct);
                }
                SourceOutcome::Dead => run.response.remove_search_url(&transport)?,
            }
        }
    }
    let evidence_restriction = restricted
        .as_ref()
        .map(|(_, restriction)| restriction.as_ref());
    run.response.deduplicate_search_urls()?;
    for promoted in promote_search_landings(&mut run.response, evidence_restriction).await? {
        verified_sources.insert(promoted);
    }
    prune_ineligible_sources(&mut run, evidence_restriction)?;
    let direct_sources = run
        .response
        .direct_search_urls()?
        .into_iter()
        .filter(|source| !verified_sources.contains(source))
        .collect::<Vec<_>>();
    if !direct_sources.is_empty() {
        let resolver = resolver.ok_or(AgyError::OutputInvalid)?;
        let restriction = restricted.map(|(_, restriction)| restriction);
        for (source, outcome) in resolve_bounded(resolver, direct_sources, restriction).await? {
            match outcome {
                SourceOutcome::Reachable(direct) => run.response.replace_url(&source, &direct),
                SourceOutcome::Dead => run.response.remove_search_url(&source)?,
            }
        }
    }
    run.response.deduplicate_search_urls()?;
    if run.response.search_results_empty()? {
        Ok(StandardSearchResolution::NoReachableResults)
    } else {
        Ok(StandardSearchResolution::Resolved(run.mark_resolved()))
    }
}

async fn promote_search_landings(
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
        let Some(score) = title_match_score(&title, &label) else {
            if !landing.source().is_latest_landing() {
                continue;
            }
            let score = (0, 0);
            let Some(target) = eligible_deep_link(landing, link.value.as_deref(), restriction)
            else {
                continue;
            };
            matches
                .entry(target)
                .and_modify(|current| *current = (*current).max(score))
                .or_insert(score);
            continue;
        };
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

fn prune_ineligible_sources(
    run: &mut ParsedRun<PendingGrounding>,
    restriction: Option<&SourceRestriction>,
) -> Result<(), AgyError> {
    for non_source in run.response.non_source_search_urls()? {
        run.response.remove_search_url(&non_source)?;
    }
    if restriction.is_none() {
        for news_portal in run
            .response
            .direct_search_urls()?
            .into_iter()
            .filter(HttpUrl::is_news_portal)
        {
            run.response.remove_search_url(&news_portal)?;
        }
    }
    let restriction = restriction.unwrap_or(&SourceRestriction::Unrestricted);
    for site_root in run
        .response
        .direct_search_urls()?
        .into_iter()
        .filter(HttpUrl::is_site_root)
        .filter(|source| !restriction.allows_evidence_url(source))
    {
        run.response.remove_search_url(&site_root)?;
    }
    Ok(())
}

pub(super) async fn resolve_bounded(
    resolver: RedirectResolver,
    sources: Vec<HttpUrl>,
    restriction: Option<Box<SourceRestriction>>,
) -> Result<Vec<(HttpUrl, SourceOutcome)>, AgyError> {
    let mut pending = JoinSet::new();
    let mut completed = BTreeMap::new();
    let mut next = 0;
    while next < sources.len() || !pending.is_empty() {
        while next < sources.len() && pending.len() < MAX_PROJECTION_CONCURRENCY {
            let source = sources.get(next).cloned().ok_or(AgyError::OutputInvalid)?;
            let worker = resolver.clone();
            let worker_restriction = restriction.clone();
            let index = next;
            pending.spawn(async move {
                let result = match worker_restriction {
                    Some(restriction) => worker.resolve_restricted(&source, &restriction).await,
                    None => worker.resolve_one(&source).await,
                };
                (index, source, result)
            });
            next += 1;
        }
        match pending.join_next().await {
            Some(Ok((index, source, Ok(direct)))) => {
                completed.insert(index, (source, SourceOutcome::Reachable(direct)));
            }
            Some(Ok((index, source, Err(AgyError::OutputInvalid)))) => {
                completed.insert(index, (source, SourceOutcome::Dead));
            }
            Some(Ok((_, _, Err(error)))) => {
                abort_and_drain(&mut pending).await;
                return Err(error);
            }
            Some(Err(_)) => {
                abort_and_drain(&mut pending).await;
                return Err(AgyError::OutputInvalid);
            }
            None => {}
        }
    }
    if completed.len() != sources.len() {
        return Err(AgyError::OutputInvalid);
    }
    Ok(completed.into_values().collect())
}

async fn abort_and_drain(pending: &mut JoinSet<(usize, HttpUrl, Result<HttpUrl, AgyError>)>) {
    pending.abort_all();
    while pending.join_next().await.is_some() {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn promotes_one_exact_same_origin_deep_title_link() -> Result<(), Box<dyn std::error::Error>> {
        let landing = SafeSourceUrl::parse_redirect("https://example.com/news-room")?;
        let title = "Second meeting of the example emergency committee";
        let html = "<html><head><link href='/feed'></head><body>\
                    <a href='/news/item/report'>28 August 2026 Statement \
                    Second meeting of the example emergency committee</a></body></html>";

        let promoted =
            unique_deep_title_link(&landing, html, title, None).ok_or(AgyError::OutputInvalid)?;

        assert_eq!(promoted.as_str(), "https://example.com/news/item/report");

        let bank = SafeSourceUrl::parse_redirect("https://example.com/")?;
        let bank_html = "<a href='/rate'>Example Bank base rate</a>\
                         <a href='/rate/history'>Example Bank base rate history</a>";
        let promoted = unique_deep_title_link(
            &bank,
            bank_html,
            "Example Bank base rate history - example.com",
            None,
        )
        .ok_or(AgyError::OutputInvalid)?;
        assert_eq!(promoted.as_str(), "https://example.com/rate/history");

        let latest = SafeSourceUrl::parse_redirect("https://example.com/releases/latest/")?;
        let redirect = "<a href='/2026/release'>Click here</a> to view the latest release.";
        let promoted = unique_deep_title_link(&latest, redirect, "Announcing Release 2.0", None)
            .ok_or(AgyError::OutputInvalid)?;
        assert_eq!(promoted.as_str(), "https://example.com/2026/release");
        Ok(())
    }

    #[test]
    fn rejects_ambiguous_external_or_parent_traversal_links()
    -> Result<(), crate::source_network::SourceNetworkError> {
        let landing = SafeSourceUrl::parse_redirect("https://example.com/news-room")?;
        let title = "Second meeting of the example emergency committee";
        for html in [
            "<a href='/one'>Second meeting of the example emergency committee</a>\
             <a href='/two'>Second meeting of the example emergency committee</a>",
            "<a href='https://other.example/item'>Second meeting of the example emergency committee</a>",
            "<a href='../item'>Second meeting of the example emergency committee</a>",
        ] {
            assert!(unique_deep_title_link(&landing, html, title, None).is_none());
        }
        Ok(())
    }
}
