//! Bounded, shell-free child-process execution with deadline cleanup.

use std::{path::PathBuf, process::Stdio, time::Duration};

#[cfg(all(test, unix))]
use nix::{errno::Errno, sys::signal::kill};
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

pub(crate) async fn run(request: ProcessRequest) -> Result<ProcessOutput, AgyError> {
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

    let mut child = command.spawn().map_err(|_| AgyError::Unavailable)?;
    let stdout = child.stdout.take().ok_or(AgyError::Unavailable)?;
    let stderr = child.stderr.take().ok_or(AgyError::Unavailable)?;
    let execution =
        async { tokio::try_join!(read_bounded(stdout), read_bounded(stderr), child.wait()) };

    let completed = if let Ok(result) = time::timeout(request.timeout, execution).await {
        result.map_err(|_| AgyError::Unavailable)?
    } else {
        terminate(&mut child).await;
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

async fn read_bounded<R>(mut reader: R) -> std::io::Result<Capture>
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
        let keep = read.min(MAX_CAPTURE_BYTES.saturating_sub(capture.bytes.len()));
        if let Some(bytes) = chunk.get(..keep) {
            capture.bytes.extend_from_slice(bytes);
        }
        capture.exceeded |= keep < read;
    }
    Ok(capture)
}

async fn terminate(child: &mut Child) {
    #[cfg(unix)]
    if let Some(id) = child.id().and_then(|id| i32::try_from(id).ok()) {
        let _ = killpg(Pid::from_raw(id), Signal::SIGKILL);
    }
    let _ = child.start_kill();
    let _ = child.wait().await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn missing_executable_is_sanitized() {
        let request = ProcessRequest {
            argv: vec!["/definitely/missing/agy".to_owned()],
            cwd: std::env::temp_dir(),
            timeout: Duration::from_secs(1),
        };

        assert!(matches!(run(request).await, Err(AgyError::Unavailable)));
    }

    #[tokio::test]
    async fn capture_is_bounded_while_the_reader_is_fully_drained() {
        let reader = tokio::io::repeat(42).take((MAX_CAPTURE_BYTES + 1) as u64);
        let capture = read_bounded(reader).await;

        assert!(matches!(
            capture,
            Ok(Capture { bytes, exceeded: true }) if bytes.len() == MAX_CAPTURE_BYTES
        ));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn timed_out_process_group_kills_background_children()
    -> Result<(), Box<dyn std::error::Error>> {
        let temporary = tempfile::tempdir()?;
        let pid_file = temporary.path().join("child.pid");
        let request = ProcessRequest {
            argv: vec![
                "/bin/sh".to_owned(),
                "-c".to_owned(),
                "sleep 10 & echo $! > \"$1\"; wait".to_owned(),
                "agy-search-timeout-test".to_owned(),
                pid_file.to_string_lossy().into_owned(),
            ],
            cwd: std::env::temp_dir(),
            timeout: Duration::from_millis(200),
        };

        assert!(matches!(run(request).await, Err(AgyError::Timeout)));
        let pid = std::fs::read_to_string(pid_file)?.trim().parse::<i32>()?;
        let mut child_is_dead = false;
        for _ in 0..50 {
            if matches!(kill(Pid::from_raw(pid), None), Err(Errno::ESRCH)) {
                child_is_dead = true;
                break;
            }
            time::sleep(Duration::from_millis(10)).await;
        }
        assert!(
            child_is_dead,
            "background child survived process-group kill"
        );
        Ok(())
    }
}
