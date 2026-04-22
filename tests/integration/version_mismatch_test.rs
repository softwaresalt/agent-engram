use std::fs;
use std::time::Duration;

use engram::daemon::ipc_server::ipc_endpoint;
use engram::daemon::protocol::{HealthCheckResult, IpcRequest, IpcResponse};
use engram::db::workspace::{canonicalize_workspace, workspace_hash};
use engram::errors::{EngramError, IpcError};
use engram::shim::ipc_client::send_request;
use engram::shim::lifecycle::{check_health, ensure_daemon_running_with_endpoint};
use engram::shim::version::{ENGRAM_PROTOCOL_VERSION, ensure_protocol_compatible};
use interprocess::local_socket::ListenerOptions;
use interprocess::local_socket::tokio::Listener;
use interprocess::local_socket::tokio::prelude::*;
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[tokio::test]
async fn shim_respawns_on_stale_daemon() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    let git_dir = workspace.path().join(".git");
    fs::create_dir(&git_dir).expect("create .git");
    fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").expect("write HEAD");

    let stale_daemon_version = ENGRAM_PROTOCOL_VERSION.saturating_add(1);
    let typed_mismatch = IpcError::VersionMismatch {
        expected: ENGRAM_PROTOCOL_VERSION,
        actual: stale_daemon_version,
    };

    assert!(
        typed_mismatch.to_string().contains("Restart the daemon"),
        "typed mismatch should document the remediation path"
    );

    assert!(matches!(
        ensure_protocol_compatible(stale_daemon_version),
        Err(EngramError::Ipc(IpcError::VersionMismatch { expected, actual }))
            if expected == ENGRAM_PROTOCOL_VERSION && actual == stale_daemon_version
    ));

    let canonical_workspace = canonicalize_workspace(&workspace.path().display().to_string())
        .expect("workspace should canonicalize");
    let legacy_endpoint = legacy_endpoint(&canonical_workspace);
    let server = tokio::spawn(run_stale_daemon(legacy_endpoint.clone()));
    tokio::time::sleep(Duration::from_millis(100)).await;

    tokio::time::timeout(
        Duration::from_secs(15),
        ensure_daemon_running_with_endpoint(workspace.path(), legacy_endpoint.clone()),
    )
    .await
    .expect("respawn should complete before timeout")
    .expect("shim should replace the stale daemon");

    let endpoint = ipc_endpoint(workspace.path()).expect("endpoint should be recomputed");
    #[cfg(windows)]
    assert_ne!(
        legacy_endpoint, endpoint,
        "respawn should switch from the legacy endpoint to the persisted workspace-id endpoint"
    );
    #[cfg(windows)]
    assert!(
        !check_health(&legacy_endpoint).await,
        "legacy endpoint should stop responding after the stale daemon is shut down"
    );
    assert!(
        check_health(&endpoint).await,
        "recomputed endpoint should be healthy after respawn"
    );

    server
        .await
        .expect("fake stale daemon task should join cleanly")
        .expect("fake stale daemon task should exit cleanly");
    shutdown_daemon(&endpoint).await;
}

#[cfg(unix)]
fn bind_fake_listener(
    endpoint: &str,
) -> Result<Listener, Box<dyn std::error::Error + Send + Sync>> {
    use interprocess::local_socket::{GenericFilePath, ToFsName};

    if fs::metadata(endpoint).is_ok() {
        fs::remove_file(endpoint)?;
    }

    let name = endpoint.to_fs_name::<GenericFilePath>()?;
    Ok(ListenerOptions::new().name(name).create_tokio()?)
}

#[cfg(windows)]
fn bind_fake_listener(
    endpoint: &str,
) -> Result<Listener, Box<dyn std::error::Error + Send + Sync>> {
    use interprocess::local_socket::{GenericNamespaced, ToNsName};

    let pipe_name = endpoint.strip_prefix(r"\\.\pipe\").unwrap_or(endpoint);
    let name = pipe_name.to_ns_name::<GenericNamespaced>()?;
    Ok(ListenerOptions::new().name(name).create_tokio()?)
}

async fn run_stale_daemon(
    endpoint: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let listener = bind_fake_listener(&endpoint)?;

    loop {
        let stream = listener.accept().await?;
        let (recv_half, mut send_half) = stream.split();
        let mut reader = BufReader::new(recv_half);
        let mut request_line = String::new();
        reader.read_line(&mut request_line).await?;
        let request = IpcRequest::from_line(&request_line).map_err(|response| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                response
                    .error
                    .map_or_else(|| "invalid request".to_owned(), |error| error.message),
            )
        })?;
        let request_id = request.id.unwrap_or(Value::Null);

        let (response, should_exit) = match request.method.as_str() {
            "_health" => (
                IpcResponse::success(
                    request_id,
                    json!(HealthCheckResult {
                        status: "ready".to_owned(),
                        uptime_seconds: 0,
                        workspace: Some(endpoint.clone()),
                        active_connections: 1,
                        protocol_version: ENGRAM_PROTOCOL_VERSION.saturating_add(1),
                        build_hash: "stale-build".to_owned(),
                    }),
                ),
                false,
            ),
            "_shutdown" => (
                IpcResponse::success(
                    request_id,
                    json!({ "status": "shutting_down", "flush_started": true }),
                ),
                true,
            ),
            _ => (
                IpcResponse::error(
                    request_id,
                    engram::daemon::protocol::IpcError {
                        code: -32_601,
                        message: "method not supported by fake stale daemon".to_owned(),
                        data: None,
                    },
                ),
                false,
            ),
        };

        send_half.write_all(response.to_line()?.as_bytes()).await?;
        send_half.flush().await?;

        if should_exit {
            break;
        }
    }

    Ok(())
}

#[cfg(windows)]
fn legacy_endpoint(workspace: &std::path::Path) -> String {
    format!(r"\\.\pipe\engram-{}", workspace_hash(workspace, "main"))
}

#[cfg(not(windows))]
fn legacy_endpoint(workspace: &std::path::Path) -> String {
    workspace
        .join(".engram")
        .join("run")
        .join("engram.sock")
        .display()
        .to_string()
}

async fn shutdown_daemon(endpoint: &str) {
    let request = IpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: Some(Value::Number(serde_json::Number::from(1))),
        method: "_shutdown".to_owned(),
        params: None,
    };

    let _ = send_request(endpoint, &request, Duration::from_secs(5)).await;

    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while check_health(endpoint).await && tokio::time::Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
