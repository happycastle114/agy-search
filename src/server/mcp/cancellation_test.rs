use std::{error::Error, fs, io, num::NonZeroU16, os::unix::fs::PermissionsExt, str::FromStr};

use nix::{errno::Errno, sys::wait::waitpid, unistd::Pid};
use rmcp::{
    ServiceExt,
    model::{CallToolRequestParams, JsonObject},
    transport::{
        StreamableHttpClientTransport, streamable_http_client::StreamableHttpClientTransportConfig,
    },
};
use serde_json::json;
use tempfile::TempDir;
use tokio::{io::AsyncReadExt, net::TcpListener};
use tokio_util::sync::CancellationToken;

use super::{McpHttpConfig, McpServer, Runtime, router, wait_for_stdio_shutdown};
use crate::{
    server::config::RuntimeOptions,
    types::{Effort, TimeoutSeconds},
};

struct SlowFixture {
    runtime: Runtime,
    callback: TcpListener,
    _temporary: TempDir,
}

type TestResult<T = ()> = Result<T, Box<dyn Error + Send + Sync>>;

impl SlowFixture {
    async fn new() -> TestResult<Self> {
        let callback = TcpListener::bind("127.0.0.1:0").await?;
        let address = callback.local_addr()?;
        let temporary = TempDir::new()?;
        let executable = temporary.path().join("slow_agy.py");
        let script = format!(
            r#"#!/usr/bin/env python3
import os
import socket
import sys
import time
if sys.argv[1:] == ["--version"]:
    print("9.9.9")
elif sys.argv[1:] == ["models"]:
    print("fixture-model\tFixture Model")
else:
    with socket.create_connection(("{}", {})) as stream:
        stream.sendall(str(os.getpid()).encode() + b"\n")
        time.sleep(30)
"#,
            address.ip(),
            address.port()
        );
        fs::write(&executable, script)?;
        let mut permissions = fs::metadata(&executable)?.permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(&executable, permissions)?;
        let runtime = Runtime::new(RuntimeOptions {
            agy_path: executable.to_string_lossy().into_owned(),
            model: None,
            effort: Effort::Low,
            timeout: TimeoutSeconds::from_str("30").map_err(io::Error::other)?,
            max_concurrency: NonZeroU16::new(1)
                .ok_or_else(|| io::Error::other("fixture concurrency must be non-zero"))?,
            catalog_ttl_seconds: 60,
        })
        .map_err(|error| io::Error::other(error.to_string()))?;
        Ok(Self {
            runtime,
            callback,
            _temporary: temporary,
        })
    }

    async fn running_pid(&self) -> TestResult<(tokio::net::TcpStream, i32)> {
        let (mut stream, _) = self.callback.accept().await?;
        let mut pid_line = Vec::new();
        stream.read_buf(&mut pid_line).await?;
        let pid = String::from_utf8(pid_line)?.trim().parse::<i32>()?;
        Ok((stream, pid))
    }
}

fn map_call() -> CallToolRequestParams {
    CallToolRequestParams::new("agy_map").with_arguments(JsonObject::from_iter([
        ("url".to_owned(), json!("https://example.com")),
        ("limit".to_owned(), json!(1)),
    ]))
}

async fn assert_process_stopped(mut stream: tokio::net::TcpStream, pid: i32) -> TestResult {
    let mut remainder = Vec::new();
    tokio::time::timeout(
        std::time::Duration::from_secs(10),
        stream.read_to_end(&mut remainder),
    )
    .await??;
    match tokio::task::spawn_blocking(move || waitpid(Pid::from_raw(pid), None)).await? {
        Ok(_) | Err(Errno::ECHILD) => {}
        Err(error) => return Err(error.into()),
    }
    Ok(())
}

#[tokio::test]
async fn stdio_disconnect_cancels_active_runtime_process() -> TestResult {
    // Given a stdio client with a tool call blocked in a real child process.
    let fixture = SlowFixture::new().await?;
    let (server_transport, client_transport) = tokio::io::duplex(64 * 1024);
    let server = tokio::spawn({
        let runtime = fixture.runtime.clone();
        async move {
            let running = McpServer::new(runtime).serve(server_transport).await?;
            Ok::<_, Box<dyn Error + Send + Sync>>(running.waiting().await?)
        }
    });
    let client = ().serve(client_transport).await?;
    let peer = client.peer().clone();
    let call = tokio::spawn(async move { peer.call_tool(map_call()).await });
    let (stream, pid) = fixture.running_pid().await?;

    // When the stdio client disconnects while the tool call is active.
    client.cancel().await?;

    // Then the request future and its real child process are torn down before server exit.
    server.await??;
    assert_process_stopped(stream, pid).await?;
    assert!(call.await?.is_err());
    Ok(())
}

#[tokio::test]
async fn stdio_shutdown_signal_cancels_active_runtime_process() -> TestResult {
    // Given a stdio server with a tool call blocked in a real child process.
    let fixture = SlowFixture::new().await?;
    let (server_transport, client_transport) = tokio::io::duplex(64 * 1024);
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    let server = tokio::spawn({
        let runtime = fixture.runtime.clone();
        async move {
            let running = McpServer::new(runtime).serve(server_transport).await?;
            let signalled = wait_for_stdio_shutdown(running, async move {
                shutdown_rx.await.map_err(io::Error::other)
            })
            .await
            .map_err(|error| io::Error::other(error.to_string()))?;
            assert!(signalled);
            Ok::<_, Box<dyn Error + Send + Sync>>(())
        }
    });
    let client = ().serve(client_transport).await?;
    let peer = client.peer().clone();
    let call = tokio::spawn(async move { peer.call_tool(map_call()).await });
    let (stream, pid) = fixture.running_pid().await?;

    // When the same shutdown branch used by SIGINT is triggered.
    shutdown_tx
        .send(())
        .map_err(|()| io::Error::other("shutdown receiver dropped"))?;

    // Then rmcp cleanup completes and reaps the descendant before server exit.
    server.await??;
    assert_process_stopped(stream, pid).await?;
    if let Ok(result) = call.await? {
        assert_eq!(result.is_error, Some(true));
    }
    client.cancel().await?;
    Ok(())
}

#[tokio::test]
async fn http_disconnect_cancels_active_runtime_process() -> TestResult {
    // Given an HTTP client with a tool call blocked in a real child process.
    let fixture = SlowFixture::new().await?;
    let cancellation = CancellationToken::new();
    let app = router(
        fixture.runtime.clone(),
        McpHttpConfig {
            allowed_hosts: vec!["127.0.0.1".to_owned()],
            allowed_origins: Vec::new(),
            cancellation: cancellation.child_token(),
        },
    );
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let server = tokio::spawn({
        let cancellation = cancellation.clone();
        async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(cancellation.cancelled_owned())
                .await
        }
    });
    let transport = StreamableHttpClientTransport::from_config(
        StreamableHttpClientTransportConfig::with_uri(format!("http://{address}/mcp")),
    );
    let client = ().serve(transport).await?;
    let peer = client.peer().clone();
    let call = tokio::spawn(async move { peer.call_tool(map_call()).await });
    let (stream, pid) = fixture.running_pid().await?;

    // When the HTTP client disconnects while its request is active.
    client.cancel().await?;

    // Then RequestContext cancellation drops the runtime future and its process.
    assert_process_stopped(stream, pid).await?;
    assert!(call.await?.is_err());
    cancellation.cancel();
    server.await??;
    Ok(())
}
