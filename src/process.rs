//! Bounded, shell-free child-process execution with deadline cleanup.

use std::{path::PathBuf, process::Stdio, time::Duration};

#[cfg(unix)]
use nix::{
    sys::signal::{Signal, killpg},
    unistd::Pid,
};
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    process::{Child, Command},
    time,
};

use crate::error::AgyError;

const MAX_CAPTURE_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone, Copy, Debug)]
pub(crate) struct CaptureLimits {
    stdout: usize,
    stderr: usize,
}

impl CaptureLimits {
    pub(crate) const fn new(stdout: usize, stderr: usize) -> Self {
        Self { stdout, stderr }
    }
}

impl Default for CaptureLimits {
    fn default() -> Self {
        Self::new(MAX_CAPTURE_BYTES, MAX_CAPTURE_BYTES)
    }
}

#[derive(Debug)]
pub(crate) struct ProcessRequest {
    pub(crate) argv: Vec<String>,
    pub(crate) cwd: PathBuf,
    pub(crate) timeout: Duration,
}

#[derive(Debug)]
pub(crate) struct ProcessOutput {
    pub(crate) stdout: Vec<u8>,
}

#[derive(Debug)]
struct Capture {
    bytes: Vec<u8>,
    exceeded: bool,
}

#[derive(Debug)]
struct ProcessGroup {
    child: Child,
    #[cfg(unix)]
    pgid: Option<Pid>,
}

impl ProcessGroup {
    fn new(child: Child) -> Self {
        #[cfg(unix)]
        let pgid = child
            .id()
            .and_then(|id| i32::try_from(id).ok())
            .map(Pid::from_raw);
        Self {
            child,
            #[cfg(unix)]
            pgid,
        }
    }

    async fn terminate(&mut self) {
        self.kill_group();
        let _ = self.child.start_kill();
        let _ = self.child.wait().await;
    }

    fn kill_group(&self) {
        #[cfg(unix)]
        if let Some(pgid) = self.pgid {
            let _ = killpg(pgid, Signal::SIGKILL);
        }
    }
}

impl Drop for ProcessGroup {
    fn drop(&mut self) {
        self.kill_group();
        let _ = self.child.start_kill();
    }
}

pub(crate) async fn run(request: ProcessRequest) -> Result<ProcessOutput, AgyError> {
    run_bounded(request, CaptureLimits::default()).await
}

pub(crate) async fn run_bounded(
    request: ProcessRequest,
    limits: CaptureLimits,
) -> Result<ProcessOutput, AgyError> {
    let Some((program, arguments)) = request.argv.split_first() else {
        return Err(AgyError::InvalidCommand);
    };
    let mut command = Command::new(program);
    command
        .args(arguments)
        .current_dir(request.cwd)
        .kill_on_drop(true)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(unix)]
    command.process_group(0);

    let child = command.spawn().map_err(|_| AgyError::Unavailable)?;
    let mut group = ProcessGroup::new(child);
    let stdout = group.child.stdout.take().ok_or(AgyError::Unavailable)?;
    let stderr = group.child.stderr.take().ok_or(AgyError::Unavailable)?;
    let execution = async {
        tokio::try_join!(
            read_bounded(stdout, limits.stdout),
            read_bounded(stderr, limits.stderr),
            group.child.wait()
        )
    };

    let completed = if let Ok(result) = time::timeout(request.timeout, execution).await {
        result.map_err(|_| AgyError::Unavailable)?
    } else {
        group.terminate().await;
        return Err(AgyError::Timeout);
    };
    let (stdout, stderr, status) = completed;
    if !status.success() {
        return Err(AgyError::ProcessFailed);
    }
    if stdout.exceeded || stderr.exceeded {
        return Err(AgyError::OutputInvalid);
    }
    Ok(ProcessOutput {
        stdout: stdout.bytes,
    })
}

async fn read_bounded<R>(mut reader: R, limit: usize) -> std::io::Result<Capture>
where
    R: AsyncRead + Unpin,
{
    let mut capture = Capture {
        bytes: Vec::new(),
        exceeded: false,
    };
    let mut chunk = vec![0_u8; 8 * 1024];
    loop {
        let read = reader.read(&mut chunk).await?;
        if read == 0 {
            break;
        }
        let keep = read.min(limit.saturating_sub(capture.bytes.len()));
        if let Some(bytes) = chunk.get(..keep) {
            capture.bytes.extend_from_slice(bytes);
        }
        capture.exceeded |= keep < read;
    }
    Ok(capture)
}

#[cfg(test)]
mod tests;
