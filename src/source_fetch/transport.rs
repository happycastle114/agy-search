//! One DNS-pinned curl transport and its bounded wire parser.

use std::{net::IpAddr, path::Path, time::Duration};

use tokio::time::Instant;

use super::{FetchedSource, PinnedSource, SourceFetchError};
use crate::{
    error::AgyError,
    process::{self, CaptureLimits, ProcessRequest},
};

const MAX_BODY_BYTES: usize = 512 * 1024;
const MAX_METADATA_BYTES: usize = 128;
const MAX_STDERR_BYTES: usize = 64 * 1024;
const STATUS_SENTINEL: &[u8] = b"\nAGY_SOURCE_META:";

pub(super) async fn fetch_pinned(
    curl_path: &Path,
    source: &PinnedSource,
    deadline: Instant,
) -> Result<FetchedSource, SourceFetchError> {
    let timeout = remaining(deadline)?;
    let program = curl_path.to_str().ok_or(SourceFetchError::Unavailable)?;
    let output = process::run_bounded(
        ProcessRequest {
            argv: curl_argv(program, source, timeout),
            cwd: std::env::current_dir().map_err(|_| SourceFetchError::Unavailable)?,
            timeout,
        },
        CaptureLimits::new(MAX_BODY_BYTES + MAX_METADATA_BYTES, MAX_STDERR_BYTES),
    )
    .await
    .map_err(|error| map_process_error(&error))?;
    let body = split_status(&output.stdout)?;
    let body = String::from_utf8(body.to_vec()).map_err(|_| SourceFetchError::InvalidUtf8)?;
    if body.trim().is_empty() {
        return Err(SourceFetchError::EmptyBody);
    }
    Ok(FetchedSource {
        url: source.url().clone(),
        body,
    })
}

fn curl_argv(program: &str, source: &PinnedSource, timeout: Duration) -> Vec<String> {
    let seconds = format!("{:.3}", timeout.as_secs_f64().max(0.001));
    let connect = format!("{:.3}", timeout.as_secs_f64().clamp(0.001, 5.0));
    let address = match source.address() {
        IpAddr::V4(address) => address.to_string(),
        IpAddr::V6(address) => format!("[{address}]"),
    };
    [
        program.to_owned(),
        "--disable".to_owned(),
        "--noproxy".to_owned(),
        "*".to_owned(),
        "--proxy".to_owned(),
        String::new(),
        "--cookie".to_owned(),
        String::new(),
        "--netrc-file".to_owned(),
        "/dev/null".to_owned(),
        "--hsts".to_owned(),
        String::new(),
        "--alt-svc".to_owned(),
        String::new(),
        "--silent".to_owned(),
        "--show-error".to_owned(),
        "--fail".to_owned(),
        "--proto".to_owned(),
        "=https".to_owned(),
        "--proto-redir".to_owned(),
        "=https".to_owned(),
        "--tlsv1.2".to_owned(),
        "--compressed".to_owned(),
        "--max-redirs".to_owned(),
        "0".to_owned(),
        "--connect-timeout".to_owned(),
        connect,
        "--max-time".to_owned(),
        seconds,
        "--max-filesize".to_owned(),
        MAX_BODY_BYTES.to_string(),
        "--resolve".to_owned(),
        format!("{}:443:{address}", source.url().host()),
        "--write-out".to_owned(),
        "\nAGY_SOURCE_META:%{http_code}:%{num_redirects}\n".to_owned(),
        "--url".to_owned(),
        source.url().as_str().to_owned(),
    ]
    .into()
}

fn split_status(output: &[u8]) -> Result<&[u8], SourceFetchError> {
    let marker = output
        .windows(STATUS_SENTINEL.len())
        .rposition(|window| window == STATUS_SENTINEL)
        .ok_or(SourceFetchError::InvalidResponse)?;
    let body = output
        .get(..marker)
        .ok_or(SourceFetchError::InvalidResponse)?;
    if body
        .windows(STATUS_SENTINEL.len())
        .any(|window| window == STATUS_SENTINEL)
    {
        return Err(SourceFetchError::InvalidResponse);
    }
    let metadata = output
        .get(marker + STATUS_SENTINEL.len()..)
        .ok_or(SourceFetchError::InvalidResponse)?;
    if body.len() > MAX_BODY_BYTES {
        return Err(SourceFetchError::Oversize);
    }
    let metadata = std::str::from_utf8(metadata)
        .map_err(|_| SourceFetchError::InvalidResponse)?
        .trim_end();
    let mut fields = metadata.split(':');
    let status = fields
        .next()
        .and_then(|field| field.parse::<u16>().ok())
        .map(HttpStatus::parse)
        .ok_or(SourceFetchError::InvalidResponse)?;
    let redirects = fields
        .next()
        .and_then(|field| field.parse::<u32>().ok())
        .map(RedirectCount)
        .ok_or(SourceFetchError::InvalidResponse)?;
    if fields.next().is_some()
        || !matches!(status, HttpStatus::Ok)
        || redirects != RedirectCount::NONE
    {
        return Err(SourceFetchError::InvalidResponse);
    }
    Ok(body)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HttpStatus {
    Ok,
    Other(u16),
}

impl HttpStatus {
    const fn parse(value: u16) -> Self {
        match value {
            200 => Self::Ok,
            other => Self::Other(other),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RedirectCount(u32);

impl RedirectCount {
    const NONE: Self = Self(0);
}

const fn map_process_error(error: &AgyError) -> SourceFetchError {
    match error {
        AgyError::Timeout => SourceFetchError::Deadline,
        AgyError::Unavailable => SourceFetchError::Unavailable,
        AgyError::OutputInvalid => SourceFetchError::Oversize,
        AgyError::ProcessFailed => SourceFetchError::ProcessFailed,
        AgyError::InvalidCommand | AgyError::UnknownModel | AgyError::OutputWrite => {
            SourceFetchError::InvalidResponse
        }
    }
}

fn remaining(deadline: Instant) -> Result<Duration, SourceFetchError> {
    deadline
        .checked_duration_since(Instant::now())
        .filter(|duration| !duration.is_zero())
        .ok_or(SourceFetchError::Deadline)
}
