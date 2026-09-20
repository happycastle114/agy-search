#![cfg(all(feature = "server", unix))]

//! Spawned-binary HTTP lifecycle regression.

use std::{
    fs,
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    os::unix::{fs::PermissionsExt, process::CommandExt},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use nix::{
    errno::Errno,
    sys::signal::{Signal, kill, killpg},
    unistd::Pid,
};
use serde_json::{Value, json};
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn std::error::Error>>;

struct HttpResponse {
    status: u16,
    body: Vec<u8>,
}

struct ServerGuard {
    child: Option<Child>,
    pgid: Pid,
}

impl ServerGuard {
    fn terminate(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        kill(self.pgid, Signal::SIGTERM)?;
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let child = self.child.as_mut().ok_or("server already stopped")?;
            if child.try_wait()?.is_some() {
                self.child = None;
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err("server did not stop after SIGTERM".into());
            }
            thread::sleep(Duration::from_millis(20));
        }
    }
}

impl Drop for ServerGuard {
    fn drop(&mut self) {
        if let Some(child) = self.child.as_mut() {
            let _ = killpg(self.pgid, Signal::SIGKILL);
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn reserve_address() -> Result<SocketAddr, std::io::Error> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    listener.local_addr()
}

fn fixture_wrapper(temporary: &TempDir) -> Result<(PathBuf, PathBuf, PathBuf), std::io::Error> {
    let executable = temporary.path().join("agy-wrapper");
    let slow = temporary.path().join("slow-next-content");
    let active_pid = temporary.path().join("active-content-pid");
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake_agy_catalog_policy.py");
    fs::write(
        &executable,
        format!(
            "#!/bin/sh\nif [ \"$1\" != '--version' ] && [ \"$1\" != 'models' ]; then\n  printf '%s\\n' \"$$\" > '{}'\n  if [ -f '{}' ]; then rm '{}'; sleep 3; fi\nfi\nexec '{}' \"$@\"\n",
            active_pid.display(),
            slow.display(),
            slow.display(),
            fixture.display(),
        ),
    )?;
    let mut permissions = fs::metadata(&executable)?.permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&executable, permissions)?;
    Ok((executable, slow, active_pid))
}

fn spawn_server(executable: &Path, address: SocketAddr) -> Result<ServerGuard, std::io::Error> {
    let curl = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake_curl.py");
    let mut command = Command::new(env!("CARGO_BIN_EXE_agy-search-server"));
    command
        .args([
            "--agy-path",
            executable
                .to_str()
                .ok_or_else(|| std::io::Error::other("fixture path"))?,
            "--timeout",
            "1",
            "--max-concurrency",
            "1",
            "http",
            "--listen",
            &address.to_string(),
        ])
        .env("AGY_SEARCH_API_KEY", "fixture-token-123456")
        .env("AGY_SEARCH_CURL_PATH", curl)
        .process_group(0)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let child = command.spawn()?;
    let pgid = child
        .id()
        .try_into()
        .map(Pid::from_raw)
        .map_err(|_| std::io::Error::other("server pid overflow"))?;
    Ok(ServerGuard {
        child: Some(child),
        pgid,
    })
}

fn request(
    address: SocketAddr,
    path: &str,
    body: Option<&Value>,
) -> Result<HttpResponse, std::io::Error> {
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(2))?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    let payload = body.map_or_else(Vec::new, |value| value.to_string().into_bytes());
    let method = if body.is_some() { "POST" } else { "GET" };
    write!(
        stream,
        "{method} {path} HTTP/1.1\r\nHost: {address}\r\nAuthorization: Bearer fixture-token-123456\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        payload.len()
    )?;
    stream.write_all(&payload)?;
    let mut response = Vec::new();
    stream.read_to_end(&mut response)?;
    let marker = b"\r\n\r\n";
    let boundary = response
        .windows(marker.len())
        .position(|window| window == marker)
        .ok_or_else(|| std::io::Error::other("HTTP response missing header boundary"))?;
    let (head, body_with_marker) = response.split_at(boundary);
    let status = std::str::from_utf8(head)
        .map_err(std::io::Error::other)?
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse().ok())
        .ok_or_else(|| std::io::Error::other("HTTP response missing status"))?;
    let body = body_with_marker
        .strip_prefix(marker)
        .ok_or_else(|| std::io::Error::other("HTTP body boundary invalid"))?
        .to_vec();
    Ok(HttpResponse { status, body })
}

fn wait_until(deadline: Instant, mut ready: impl FnMut() -> bool) -> Result<(), &'static str> {
    while Instant::now() < deadline {
        if ready() {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(20));
    }
    Err("condition did not become ready before deadline")
}

#[test]
fn spawned_http_server_bounds_requests_and_shuts_down_active_work() -> TestResult {
    // Given: a spawned server binary with one request permit and an owned AGY fixture.
    let temporary = TempDir::new()?;
    let (executable, slow, active_pid) = fixture_wrapper(&temporary)?;
    let address = reserve_address()?;
    let mut server = spawn_server(&executable, address)?;
    wait_until(Instant::now() + Duration::from_secs(5), || {
        request(address, "/healthz", None).is_ok_and(|response| response.status == 200)
    })?;

    // When: a two-query batch returns duplicate fixture sources.
    let batch = request(
        address,
        "/search",
        Some(&json!({"query":["first","second"],"max_results":5})),
    )?;

    // Then: the public response is successful, globally capped, and deduplicated.
    assert_eq!(
        batch.status,
        200,
        "{}",
        String::from_utf8_lossy(&batch.body)
    );
    let document: Value = serde_json::from_slice(&batch.body)?;
    let results = document
        .get("results")
        .and_then(Value::as_array)
        .ok_or("results array")?;
    assert_eq!(results.len(), 2);
    assert_ne!(results.first(), results.get(1));

    // Given: the next request holds the only permit beyond the one-second deadline.
    fs::remove_file(&active_pid).ok();
    fs::write(&slow, [])?;
    let first = thread::spawn(move || {
        request(
            address,
            "/search",
            Some(&json!({"query":"slow","max_results":5})),
        )
    });
    wait_until(Instant::now() + Duration::from_secs(2), || {
        active_pid.exists()
    })?;

    // When: another request arrives while the first owns the permit.
    let busy = request(
        address,
        "/search",
        Some(&json!({"query":"busy","max_results":5})),
    )?;
    let timed_out = first.join().map_err(|_| "request thread panicked")??;

    // Then: excess work fails immediately, timeout maps to 504, and the permit returns.
    assert_eq!(busy.status, 429);
    assert_eq!(timed_out.status, 504);
    let recovered = request(
        address,
        "/search",
        Some(&json!({"query":"recovered","max_results":5})),
    )?;
    assert_eq!(recovered.status, 200);

    // Given: another active AGY process exists when the server receives SIGTERM.
    fs::remove_file(&active_pid).ok();
    fs::write(&slow, [])?;
    let active = thread::spawn(move || {
        request(
            address,
            "/search",
            Some(&json!({"query":"shutdown","max_results":5})),
        )
    });
    wait_until(Instant::now() + Duration::from_secs(2), || {
        active_pid.exists()
    })?;
    let pid: i32 = fs::read_to_string(&active_pid)?.trim().parse()?;

    // When: SIGTERM initiates graceful shutdown.
    server.terminate()?;
    let _ = active
        .join()
        .map_err(|_| "shutdown request thread panicked")?;

    // Then: the listener closes and the active AGY process group is gone.
    assert!(TcpStream::connect_timeout(&address, Duration::from_millis(200)).is_err());
    assert!(matches!(kill(Pid::from_raw(pid), None), Err(Errno::ESRCH)));
    Ok(())
}
