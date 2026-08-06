//! Bounded normalization of Google grounding transport URLs.

mod projection;
mod response;
mod transport;

use std::{collections::HashSet, path::Path, time::Duration};

use tokio::time::Instant;

use self::{response::HttpStatus, transport::RedirectTransport};
use crate::{
    error::AgyError,
    events::{GroundingRequirement, GroundingResolved, ParsedRun, PendingGrounding},
    source_network::SafeSourceUrl,
    types::{NonEmptyText, SourceUrlKind},
};

pub(crate) use projection::{StandardSearchResolution, resolve_standard_search_run};

const RESOLVER_ENVIRONMENT: &str = "AGY_SEARCH_CURL_PATH";
const DEFAULT_RESOLVER: &str = "curl";
const RESOLVER_TIMEOUT: Duration = Duration::from_secs(6);
const MAX_REDIRECT_HOPS: usize = 5;

#[derive(Clone, Debug)]
pub(super) struct RedirectResolver {
    transport: RedirectTransport,
}

pub(crate) async fn resolve_grounding_run(
    mut run: ParsedRun<PendingGrounding>,
    cwd: &Path,
) -> Result<ParsedRun<GroundingResolved>, AgyError> {
    let mut transports = run.response.grounding_redirects();
    let mut seen = transports.iter().cloned().collect::<HashSet<_>>();
    if let GroundingRequirement::Restricted {
        transports: tool_transports,
        restriction: _,
    } = &run.grounding
    {
        transports.extend(
            tool_transports
                .iter()
                .filter(|transport| seen.insert((*transport).clone()))
                .cloned(),
        );
    }
    if transports.is_empty() {
        return Ok(run.mark_resolved());
    }
    let resolver = new_resolver(cwd)?;
    for transport in transports {
        let direct = resolver.resolve_one(&transport).await?;
        if let GroundingRequirement::Restricted {
            transports: tool_transports,
            restriction,
        } = &run.grounding
            && tool_transports.contains(&transport)
            && !restriction.allows(&direct)
        {
            return Err(AgyError::OutputInvalid);
        }
        run.response.replace_url(&transport, &direct);
    }
    Ok(run.mark_resolved())
}

fn new_resolver(cwd: &Path) -> Result<RedirectResolver, AgyError> {
    let executable = curl_executable()?;
    Ok(RedirectResolver {
        transport: RedirectTransport::new(
            &executable,
            cwd.to_path_buf(),
            Instant::now() + RESOLVER_TIMEOUT,
        ),
    })
}

pub(crate) fn curl_executable() -> Result<String, AgyError> {
    match std::env::var(RESOLVER_ENVIRONMENT) {
        Ok(configured) => NonEmptyText::parse(&configured)
            .map(|value| value.as_str().to_owned())
            .map_err(|_| AgyError::OutputInvalid),
        Err(std::env::VarError::NotPresent) => Ok(DEFAULT_RESOLVER.to_owned()),
        Err(std::env::VarError::NotUnicode(_)) => Err(AgyError::OutputInvalid),
    }
}

impl RedirectResolver {
    pub(super) async fn resolve_one(
        &self,
        transport: &crate::types::HttpUrl,
    ) -> Result<crate::types::HttpUrl, AgyError> {
        let mut current = SafeSourceUrl::parse_redirect(transport.as_str())
            .map_err(|_| AgyError::OutputInvalid)?;
        let mut visited = HashSet::with_capacity(MAX_REDIRECT_HOPS + 1);
        for hop in 0..=MAX_REDIRECT_HOPS {
            if !visited.insert(current.as_str().to_owned()) {
                return Err(AgyError::OutputInvalid);
            }
            let response = self.transport.request(&current).await?;
            match response.status {
                HttpStatus::Success => {
                    if response.location.is_some()
                        || current.source().source_kind() != SourceUrlKind::Direct
                    {
                        return Err(AgyError::OutputInvalid);
                    }
                    return Ok(current.source().clone());
                }
                HttpStatus::Redirect => {
                    if hop == MAX_REDIRECT_HOPS {
                        return Err(AgyError::OutputInvalid);
                    }
                    let location = response.location.ok_or(AgyError::OutputInvalid)?;
                    current = current
                        .join_redirect(&location)
                        .map_err(|_| AgyError::OutputInvalid)?;
                }
                HttpStatus::Informational | HttpStatus::Other => {
                    return Err(AgyError::OutputInvalid);
                }
            }
        }
        Err(AgyError::OutputInvalid)
    }
}
