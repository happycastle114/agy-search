use std::{collections::HashMap, path::PathBuf};

use tokio::time::Instant;

use super::text::canonical_evidence_text;
use crate::{
    error::AgyError,
    redirect::curl_executable,
    source_fetch::{SafeSourceUrl, SourceFetcher},
    source_verification::map_source_fetch_error,
};

pub(super) async fn fetch_bodies(
    sources: &[SafeSourceUrl],
    deadline: Instant,
) -> Result<HashMap<SafeSourceUrl, String>, AgyError> {
    let executable = PathBuf::from(curl_executable()?);
    let fetched = SourceFetcher::new(executable)
        .fetch_many(sources, deadline)
        .await
        .map_err(|error| map_source_fetch_error(&error))?;
    let mut bodies = HashMap::with_capacity(fetched.len());
    for source in fetched {
        let (url, body) = source.into_parts();
        let body = canonical_evidence_text(&body);
        if body.is_empty() || bodies.insert(url, body).is_some() {
            return Err(AgyError::OutputInvalid);
        }
    }
    if bodies.len() != sources.len() {
        return Err(AgyError::OutputInvalid);
    }
    Ok(bodies)
}
