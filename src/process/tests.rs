use super::*;

#[cfg(unix)]
use nix::{
    errno::Errno,
    sys::signal::{Signal, kill},
};

#[tokio::test]
async fn missing_executable_is_sanitized() {
    // Given: a request whose executable does not exist.
    let request = ProcessRequest {
        argv: vec!["/definitely/missing/agy".to_owned()],
        cwd: std::env::temp_dir(),
        timeout: Duration::from_secs(1),
    };

    // When: the request is executed. Then: no path detail crosses the error boundary.
    assert!(matches!(run(request).await, Err(AgyError::Unavailable)));
}

#[tokio::test]
async fn capture_is_bounded_while_the_reader_is_fully_drained() {
    // Given: a reader with one byte more than the capture limit.
    let reader = tokio::io::repeat(42).take((MAX_CAPTURE_BYTES + 1) as u64);

    // When: the reader is drained. Then: capture stays bounded and records overflow.
    let capture = read_bounded(reader, MAX_CAPTURE_BYTES).await;
    assert!(matches!(
        capture,
        Ok(Capture { bytes, exceeded: true }) if bytes.len() == MAX_CAPTURE_BYTES
    ));
}

#[cfg(unix)]
#[tokio::test]
async fn timed_out_process_group_kills_background_children()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: a shell and background child that outlive the request deadline.
    let temporary = tempfile::tempdir()?;
    let pid_file = temporary.path().join("child.pid");
    let request = background_request(&pid_file, "wait", Duration::from_millis(200));

    // When: execution times out. Then: the whole process group is gone.
    let result = run(request).await;
    assert_child_is_cleaned(read_pid(&pid_file)?).await;
    assert!(matches!(result, Err(AgyError::Timeout)));
    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn cancelled_process_future_kills_background_children()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: a running process tree whose background PID has been published.
    let temporary = tempfile::tempdir()?;
    let pid_file = temporary.path().join("child.pid");
    let request = background_request(&pid_file, "wait", Duration::from_secs(30));
    let execution = tokio::spawn(run(request));
    if let Err(error) = wait_for_pid_file(&pid_file).await {
        execution.abort();
        let _ = execution.await;
        return Err(error);
    }
    let pid = read_pid(&pid_file)?;

    // When: the owning future is cancelled. Then: the background child is gone.
    execution.abort();
    let _ = execution.await;
    assert_child_is_cleaned(pid).await;
    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn successful_process_kills_background_children() -> Result<(), Box<dyn std::error::Error>> {
    // Given: a successful shell that leaves a background child behind.
    let temporary = tempfile::tempdir()?;
    let pid_file = temporary.path().join("child.pid");
    let request = background_request(&pid_file, "exit 0", Duration::from_secs(2));

    // When: execution succeeds. Then: the background child is still cleaned up.
    let result = run(request).await;
    assert_child_is_cleaned(read_pid(&pid_file)?).await;
    assert!(result.is_ok());
    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn bounded_output_error_kills_background_children() -> Result<(), Box<dyn std::error::Error>>
{
    // Given: a process tree whose leader emits more than the configured limit.
    let temporary = tempfile::tempdir()?;
    let pid_file = temporary.path().join("child.pid");
    let request = background_request(&pid_file, "printf 12345", Duration::from_secs(2));

    // When: bounded execution rejects the output. Then: the descendant is gone.
    let result = run_bounded(request, CaptureLimits::new(4, 4)).await;
    assert_child_is_cleaned(read_pid(&pid_file)?).await;
    assert!(matches!(result, Err(AgyError::OutputInvalid)));
    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn failed_process_kills_background_children() -> Result<(), Box<dyn std::error::Error>> {
    // Given: a shell that reports failure after leaving a background child behind.
    let temporary = tempfile::tempdir()?;
    let pid_file = temporary.path().join("child.pid");
    let request = background_request(&pid_file, "exit 7", Duration::from_secs(2));

    // When: execution reports the failure. Then: the descendant is still cleaned up.
    let result = run(request).await;
    assert_child_is_cleaned(read_pid(&pid_file)?).await;
    assert!(matches!(result, Err(AgyError::ProcessFailed)));
    Ok(())
}

#[cfg(unix)]
fn background_request(
    pid_file: &std::path::Path,
    conclusion: &str,
    timeout: Duration,
) -> ProcessRequest {
    ProcessRequest {
        argv: vec![
            "/bin/sh".to_owned(),
            "-c".to_owned(),
            format!("sleep 30 </dev/null >/dev/null 2>&1 & echo $! > \"$1\"; {conclusion}"),
            "agy-search-process-test".to_owned(),
            pid_file.to_string_lossy().into_owned(),
        ],
        cwd: std::env::temp_dir(),
        timeout,
    }
}

#[cfg(unix)]
async fn wait_for_pid_file(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    for _ in 0..100 {
        if path.exists() {
            return Ok(());
        }
        time::sleep(Duration::from_millis(10)).await;
    }
    Err("background PID was not published".into())
}

#[cfg(unix)]
fn read_pid(path: &std::path::Path) -> Result<i32, Box<dyn std::error::Error>> {
    Ok(std::fs::read_to_string(path)?.trim().parse()?)
}

#[cfg(unix)]
async fn assert_child_is_cleaned(pid: i32) {
    let child_is_dead = wait_until_dead(pid).await;
    if !child_is_dead {
        let _ = kill(Pid::from_raw(pid), Signal::SIGKILL);
        let _ = wait_until_dead(pid).await;
    }
    assert!(child_is_dead, "background child {pid} survived cleanup");
}

#[cfg(unix)]
async fn wait_until_dead(pid: i32) -> bool {
    for _ in 0..100 {
        if matches!(kill(Pid::from_raw(pid), None), Err(Errno::ESRCH)) {
            return true;
        }
        time::sleep(Duration::from_millis(10)).await;
    }
    false
}
