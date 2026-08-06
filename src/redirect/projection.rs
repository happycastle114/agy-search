//! Best-effort grounding projection for latency-sensitive standard Search.

use std::collections::{BTreeMap, HashSet};

use tokio::task::JoinSet;

use super::{RedirectResolver, new_resolver};
use crate::{
    error::AgyError,
    events::{GroundingRequirement, GroundingResolved, ParsedRun, PendingGrounding},
    types::HttpUrl,
};

const MAX_PROJECTION_CONCURRENCY: usize = 4;

#[derive(Debug)]
pub(crate) enum StandardSearchResolution {
    Resolved(ParsedRun<GroundingResolved>),
    NoReachableResults,
}

#[derive(Debug)]
enum TransportOutcome {
    Reachable(HttpUrl),
    Dead,
}

pub(crate) async fn resolve_standard_search_run(
    mut run: ParsedRun<PendingGrounding>,
    cwd: &std::path::Path,
) -> Result<StandardSearchResolution, AgyError> {
    let mut response_transports = run.response.grounding_redirects();
    let restricted = match &run.grounding {
        GroundingRequirement::None => None,
        GroundingRequirement::Restricted {
            transports,
            restriction,
        } => Some((transports.clone(), restriction.clone())),
    };
    let needs_resolver = !response_transports.is_empty()
        || restricted
            .as_ref()
            .is_some_and(|(transports, _)| !transports.is_empty());
    let resolver = if needs_resolver {
        Some(new_resolver(cwd)?)
    } else {
        None
    };

    if let Some((transports, restriction)) = restricted {
        let restricted_transports = transports.iter().cloned().collect::<HashSet<_>>();
        if !transports.is_empty() {
            let resolver = resolver.as_ref().ok_or(AgyError::OutputInvalid)?;
            for transport in transports {
                let direct = resolver.resolve_one(&transport).await?;
                if !restriction.allows(&direct) {
                    return Err(AgyError::OutputInvalid);
                }
                run.response.replace_url(&transport, &direct);
            }
        }
        response_transports.retain(|transport| !restricted_transports.contains(transport));
    }

    if !response_transports.is_empty() {
        let resolver = resolver.ok_or(AgyError::OutputInvalid)?;
        for (transport, outcome) in resolve_bounded(resolver, response_transports).await? {
            match outcome {
                TransportOutcome::Reachable(direct) => {
                    run.response.replace_url(&transport, &direct);
                }
                TransportOutcome::Dead => run.response.remove_search_url(&transport)?,
            }
        }
    }
    for non_source in run.response.non_source_search_urls()? {
        run.response.remove_search_url(&non_source)?;
    }
    if run.response.search_results_empty()? {
        Ok(StandardSearchResolution::NoReachableResults)
    } else {
        Ok(StandardSearchResolution::Resolved(run.mark_resolved()))
    }
}

async fn resolve_bounded(
    resolver: RedirectResolver,
    transports: Vec<HttpUrl>,
) -> Result<Vec<(HttpUrl, TransportOutcome)>, AgyError> {
    let mut pending = JoinSet::new();
    let mut completed = BTreeMap::new();
    let mut next = 0;
    while next < transports.len() || !pending.is_empty() {
        while next < transports.len() && pending.len() < MAX_PROJECTION_CONCURRENCY {
            let transport = transports
                .get(next)
                .cloned()
                .ok_or(AgyError::OutputInvalid)?;
            let worker = resolver.clone();
            let index = next;
            pending.spawn(async move {
                let result = worker.resolve_one(&transport).await;
                (index, transport, result)
            });
            next += 1;
        }
        match pending.join_next().await {
            Some(Ok((index, transport, Ok(direct)))) => {
                completed.insert(index, (transport, TransportOutcome::Reachable(direct)));
            }
            Some(Ok((index, transport, Err(AgyError::OutputInvalid)))) => {
                completed.insert(index, (transport, TransportOutcome::Dead));
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
    if completed.len() != transports.len() {
        return Err(AgyError::OutputInvalid);
    }
    Ok(completed.into_values().collect())
}

async fn abort_and_drain(pending: &mut JoinSet<(usize, HttpUrl, Result<HttpUrl, AgyError>)>) {
    pending.abort_all();
    while pending.join_next().await.is_some() {}
}
