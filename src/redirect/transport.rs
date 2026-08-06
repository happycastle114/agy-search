use std::{net::IpAddr, path::PathBuf, time::Duration};

use tokio::time::Instant;

use super::response::{HopResponse, parse_hop_response};
use crate::{
    error::AgyError,
    process::{CaptureLimits, ProcessRequest, run_bounded},
    source_network::{PinnedSource, SafeSourceUrl, SourceNetworkError, resolve},
};

const MAX_HEADER_BYTES: usize = 64 * 1024;
const MAX_STDERR_BYTES: usize = 16 * 1024;

#[derive(Clone, Debug)]
pub(super) struct RedirectTransport {
    executable: String,
    cwd: PathBuf,
    deadline: Instant,
}

impl RedirectTransport {
    pub(super) fn new(executable: &str, cwd: PathBuf, deadline: Instant) -> Self {
        Self {
            executable: executable.to_owned(),
            cwd,
            deadline,
        }
    }

    pub(super) async fn request(&self, source: &SafeSourceUrl) -> Result<HopResponse, AgyError> {
        let pinned = resolve(source.clone(), self.deadline)
            .await
            .map_err(map_network_error)?;
        let timeout = remaining(self.deadline)?;
        let output = run_bounded(
            ProcessRequest {
                argv: curl_argv(&self.executable, &pinned, timeout),
                cwd: self.cwd.clone(),
                timeout,
            },
            CaptureLimits::new(MAX_HEADER_BYTES, MAX_STDERR_BYTES),
        )
        .await
        .map_err(|error| map_transport_error(&error))?;
        parse_hop_response(&output.stdout)
    }
}

fn curl_argv(program: &str, source: &PinnedSource, timeout: Duration) -> Vec<String> {
    let seconds = format!("{:.3}", timeout.as_secs_f64().max(0.001));
    let connect = format!("{:.3}", timeout.as_secs_f64().clamp(0.001, 2.0));
    let address = match source.address() {
        IpAddr::V4(address) => address.to_string(),
        IpAddr::V6(address) => format!("[{address}]"),
    };
    let mut argv = [
        program,
        "--disable",
        "--noproxy",
        "*",
        "--proxy",
        "",
        "--cookie",
        "",
        "--netrc-file",
        "/dev/null",
        "--hsts",
        "",
        "--alt-svc",
        "",
        "--silent",
        "--show-error",
        "--fail",
        "--head",
        "--proto",
        "=https",
        "--proto-redir",
        "=https",
        "--tlsv1.2",
        "--max-redirs",
        "0",
        "--connect-timeout",
    ]
    .map(str::to_owned)
    .to_vec();
    argv.extend([
        connect,
        "--max-time".to_owned(),
        seconds,
        "--resolve".to_owned(),
        format!("{}:443:{address}", source.url().host()),
    ]);
    argv.extend(
        [
            "--dump-header",
            "-",
            "--output",
            "/dev/null",
            "--write-out",
            "\nAGY_REDIRECT_META:%{http_code}:%{num_redirects}\n",
            "--url",
            source.url().as_str(),
        ]
        .map(str::to_owned),
    );
    argv
}

fn remaining(deadline: Instant) -> Result<Duration, AgyError> {
    deadline
        .checked_duration_since(Instant::now())
        .filter(|duration| !duration.is_zero())
        .ok_or(AgyError::Timeout)
}

const fn map_network_error(error: SourceNetworkError) -> AgyError {
    match error {
        SourceNetworkError::Deadline => AgyError::Timeout,
        SourceNetworkError::InvalidUrl
        | SourceNetworkError::UnsafeAddress
        | SourceNetworkError::Dns => AgyError::OutputInvalid,
    }
}

const fn map_transport_error(error: &AgyError) -> AgyError {
    match error {
        AgyError::Timeout => AgyError::Timeout,
        AgyError::Unavailable => AgyError::Unavailable,
        AgyError::InvalidCommand
        | AgyError::ProcessFailed
        | AgyError::OutputInvalid
        | AgyError::UnknownModel
        | AgyError::OutputWrite => AgyError::OutputInvalid,
    }
}
