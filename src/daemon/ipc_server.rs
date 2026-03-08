//! Daemon IPC server: newline-delimited JSON-RPC over a local socket.
//!
//! Listens on the workspace-scoped IPC endpoint (Unix domain socket on Linux/macOS,
//! Windows named pipe on Windows), reads exactly one JSON-RPC request per
//! connection, dispatches to [`crate::tools::dispatch`], and writes the response.
//!
//! # Endpoint naming
//!
//! | Platform | Format |
//! |----------|--------|
//! | Unix     | `{workspace}/.engram/run/engram.sock` |
//! | Windows  | `\\.\pipe\engram-{sha256_first_16hex}` |

use std::path::Path;
use std::sync::Arc;

use interprocess::local_socket::{
    ListenerOptions,
    tokio::{Listener, Stream, prelude::*},
};
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::sync::watch;
use tracing::{debug, error, info, instrument, warn};

use crate::daemon::protocol::{IpcError as WireError, IpcRequest, IpcResponse};
use crate::daemon::ttl::TtlTimer;
use crate::errors::{EngramError, IpcError as DomainIpcError};
use crate::server::state::{AppState, SharedState};
use crate::tools;

// ── Endpoint naming ──────────────────────────────────────────────────────────

/// Compute the IPC endpoint string for the given workspace.
///
/// - **Unix**: `{workspace}/.engram/run/engram.sock`
/// - **Windows**: `\\.\pipe\engram-{sha256_first_16hex}` where the hash is
///   the SHA-256 of the canonical workspace path encoded as lowercase hex.
///
/// # Errors
///
/// Returns [`EngramError::Ipc`] if the workspace path contains non-UTF-8
/// characters or if the platform is unsupported.
pub fn ipc_endpoint(workspace: &Path) -> Result<String, EngramError> {
    ipc_endpoint_impl(workspace)
}

#[cfg(unix)]
fn ipc_endpoint_impl(workspace: &Path) -> Result<String, EngramError> {
    use sha2::{Digest, Sha256};

    let sock_path = workspace
        .join(".engram")
        .join("run")
        .join("engram.sock");

    let path_str = sock_path.to_str().ok_or_else(|| {
        EngramError::Ipc(DomainIpcError::ConnectionFailed {
            address: workspace.display().to_string(),
            reason: "workspace path is not valid UTF-8".to_owned(),
        })
    })?;

    // Unix domain socket paths are limited to 108 bytes on Linux (UNIX_PATH_MAX)
    // and 104 bytes on macOS.  108 is used as the conservative cross-platform
    // limit.  If the workspace-scoped path exceeds this, fall back to a
    // hash-derived path under /tmp/ to avoid ENAMETOOLONG on bind() (S119).
    if path_str.len() <= 108 {
        return Ok(path_str.to_owned());
    }

    // Fallback: /tmp/engram-{sha256_first_16hex}.sock
    // Permissions (0o600) are applied by run_with_shutdown after bind, using
    // the endpoint string returned here, so the fallback path is also secured.
    let canonical_str = workspace.to_str().ok_or_else(|| {
        EngramError::Ipc(DomainIpcError::ConnectionFailed {
            address: workspace.display().to_string(),
            reason: "workspace path is not valid UTF-8 for fallback hash".to_owned(),
        })
    })?;

    let mut hasher = Sha256::new();
    hasher.update(canonical_str.as_bytes());
    let hash = hasher.finalize();
    let prefix = hex::encode(&hash[..8]);
    let fallback = format!("/tmp/engram-{prefix}.sock");

    tracing::warn!(
        workspace = %workspace.display(),
        fallback = %fallback,
        path_len = path_str.len(),
        "Unix socket path exceeds 108 bytes — using /tmp/ fallback (S119)"
    );

    Ok(fallback)
}

#[cfg(windows)]
fn ipc_endpoint_impl(workspace: &Path) -> Result<String, EngramError> {
    use sha2::{Digest, Sha256};

    let canonical_str = workspace.to_str().ok_or_else(|| {
        EngramError::Ipc(DomainIpcError::ConnectionFailed {
            address: workspace.display().to_string(),
            reason: "workspace path is not valid UTF-8".to_owned(),
        })
    })?;

    let mut hasher = Sha256::new();
    hasher.update(canonical_str.as_bytes());
    let hash = hasher.finalize();
    // First 8 bytes → 16 lowercase hex characters
    let prefix = hex::encode(&hash[..8]);
    Ok(format!(r"\\.\pipe\engram-{prefix}"))
}

#[cfg(not(any(unix, windows)))]
fn ipc_endpoint_impl(workspace: &Path) -> Result<String, EngramError> {
    Err(EngramError::Ipc(DomainIpcError::ConnectionFailed {
        address: workspace.display().to_string(),
        reason: "unsupported platform for IPC".to_owned(),
    }))
}

// ── Listener binding ─────────────────────────────────────────────────────────

/// Bind a [`Listener`] at `endpoint`, creating the local socket or named pipe.
///
/// On Unix, any stale socket file at the path is removed before binding.
///
/// # Errors
///
/// Returns [`EngramError::Ipc`] if binding fails.
fn bind_listener(endpoint: &str) -> Result<Listener, EngramError> {
    bind_listener_impl(endpoint)
}

#[cfg(unix)]
fn bind_listener_impl(endpoint: &str) -> Result<Listener, EngramError> {
    use interprocess::local_socket::{GenericFilePath, ToFsName};

    // Remove stale socket file before binding so we don't get EADDRINUSE.
    // Propagate errors other than "not found" — they indicate permission or
    // ownership problems that would cause the subsequent bind to fail anyway,
    // and the diagnostic is clearer here than in create_tokio().
    match std::fs::remove_file(endpoint) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => {
            return Err(ipc_err(
                endpoint,
                format!("failed to remove stale socket: {e}"),
            ));
        }
    }

    let name = endpoint
        .to_fs_name::<GenericFilePath>()
        .map_err(|e| ipc_err(endpoint, e.to_string()))?;

    ListenerOptions::new()
        .name(name)
        .create_tokio()
        .map_err(|e| ipc_err(endpoint, e.to_string()))
}

#[cfg(windows)]
fn bind_listener_impl(endpoint: &str) -> Result<Listener, EngramError> {
    use interprocess::local_socket::{GenericNamespaced, ToNsName};

    // `GenericNamespaced` on Windows expects the pipe name WITHOUT `\\.\pipe\`.
    let pipe_name = endpoint.strip_prefix(r"\\.\pipe\").unwrap_or(endpoint);

    let name = pipe_name
        .to_ns_name::<GenericNamespaced>()
        .map_err(|e| ipc_err(endpoint, e.to_string()))?;

    ListenerOptions::new()
        .name(name)
        .create_tokio()
        .map_err(|e| ipc_err(endpoint, e.to_string()))
}

#[cfg(not(any(unix, windows)))]
fn bind_listener_impl(endpoint: &str) -> Result<Listener, EngramError> {
    Err(ipc_err(endpoint, "unsupported platform for IPC".to_owned()))
}

fn ipc_err(address: &str, reason: String) -> EngramError {
    EngramError::Ipc(DomainIpcError::ConnectionFailed {
        address: address.to_owned(),
        reason,
    })
}

// ── Connection handling ──────────────────────────────────────────────────────

/// Maximum IPC request size (1 MiB). Requests exceeding this are rejected with
/// a parse error to prevent a slow-write client from causing unbounded allocation.
const MAX_REQUEST_BYTES: usize = 1024 * 1024;

/// Process a single IPC connection: read one request line, dispatch, write response.
///
/// Errors are logged but not propagated; the accept loop continues after each
/// connection regardless of outcome.
#[instrument(skip(stream, state, shutdown_tx))]
async fn handle_connection(
    stream: Stream,
    state: SharedState,
    shutdown_tx: Arc<watch::Sender<bool>>,
) {
    let (recv_half, mut send_half) = stream.split();
    // Cap reads to MAX_REQUEST_BYTES + 1 bytes before buffering so that an
    // adversarial local client cannot force unbounded allocation before the
    // size check on line 221 triggers. `take` limits the underlying AsyncRead
    // to at most MAX_REQUEST_BYTES + 1 bytes; read_line then returns with
    // n == MAX_REQUEST_BYTES + 1 which the Err arm below rejects cleanly.
    let mut reader = BufReader::new(recv_half.take((MAX_REQUEST_BYTES + 1) as u64));
    let mut line = String::new();

    let response = match reader.read_line(&mut line).await {
        Ok(0) => {
            debug!("IPC connection closed before sending a request (EOF)");
            return;
        }
        Ok(n) if n > MAX_REQUEST_BYTES => {
            IpcResponse::parse_error(format!("request exceeds {MAX_REQUEST_BYTES} byte limit"))
        }
        Ok(_) => process_request(&line, &state, &shutdown_tx).await,
        Err(e) => {
            warn!(error = %e, "failed to read IPC request line");
            return;
        }
    };

    match response.to_line() {
        Ok(line_str) => {
            if let Err(e) = send_half.write_all(line_str.as_bytes()).await {
                error!(error = %e, "failed to write IPC response");
            } else if let Err(e) = send_half.flush().await {
                error!(error = %e, "failed to flush IPC response");
            }
        }
        Err(e) => {
            error!(error = %e, "failed to serialize IPC response");
        }
    }
}

/// Deserialize and dispatch a single raw request line, returning an [`IpcResponse`].
async fn process_request(
    line: &str,
    state: &SharedState,
    shutdown_tx: &Arc<watch::Sender<bool>>,
) -> IpcResponse {
    let request = match IpcRequest::from_line(line.trim()) {
        Ok(r) => r,
        Err(err_response) => return err_response,
    };

    if let Err(err_response) = request.validate() {
        return err_response;
    }

    // Safe to unwrap: validate() ensures id is Some.
    let id = request.id.clone().unwrap_or(Value::Null);

    match request.method.as_str() {
        "_health" => IpcResponse::success(
            id,
            json!({
                "status": "ready",
                "uptime_seconds": state.uptime_seconds(),
                "workspace": state.snapshot_workspace().await.map(|s| s.path),
                "active_connections": state.active_connections(),
            }),
        ),
        // T052: `_shutdown` triggers the shared shutdown channel so the accept
        // loop exits after returning this response (S022, S037).
        "_shutdown" => {
            info!("daemon received _shutdown IPC request — initiating graceful shutdown");
            let _ = shutdown_tx.send(true);
            IpcResponse::success(
                id,
                json!({ "status": "shutting_down", "flush_started": true }),
            )
        }
        method => match tools::dispatch(Arc::clone(state), method, request.params).await {
            Ok(result) => IpcResponse::success(id, result),
            Err(e) => {
                let resp = e.to_response();
                IpcResponse::error(
                    id,
                    WireError {
                        code: -32_603,
                        message: resp.error.message,
                        data: Some(json!({ "engram_code": resp.error.code })),
                    },
                )
            }
        },
    }
}

// ── Daemon entry point ───────────────────────────────────────────────────────

/// Run the daemon accept loop with graceful shutdown support.
///
/// Steps:
/// 1. Canonicalize and validate the workspace path.
/// 2. Create `.engram/run/` if needed.
/// 3. Acquire the daemon lockfile.
/// 4. Build [`AppState`] and set the active workspace.
/// 5. Compute and bind the IPC endpoint.
/// 6. Enter the accept loop; exit when `shutdown_rx` becomes `true`.
///
/// # Errors
///
/// Returns [`EngramError`] if path validation, lock acquisition, or listener
/// binding fails.
pub async fn run_with_shutdown(
    workspace: &str,
    ttl: Arc<TtlTimer>,
    shutdown_tx: Arc<watch::Sender<bool>>,
    shutdown_rx: watch::Receiver<bool>,
) -> Result<(), EngramError> {
    let workspace_path = std::fs::canonicalize(workspace).map_err(|e| {
        EngramError::Ipc(DomainIpcError::ConnectionFailed {
            address: workspace.to_owned(),
            reason: format!("cannot canonicalize workspace path: {e}"),
        })
    })?;

    // Ensure .engram/run/ exists before acquiring the lock.
    let run_dir = workspace_path.join(".engram").join("run");
    std::fs::create_dir_all(&run_dir).map_err(|e| {
        EngramError::Ipc(DomainIpcError::ConnectionFailed {
            address: run_dir.display().to_string(),
            reason: e.to_string(),
        })
    })?;

    // Lock is already acquired by `daemon::mod::run()` which holds it for the
    // daemon's entire lifetime. No re-acquisition needed here.

    let state: SharedState = Arc::new(AppState::new(1));

    // Hydrate the workspace into the shared state.
    crate::tools::lifecycle::set_workspace(state.as_ref(), workspace.to_owned()).await?;

    let endpoint = ipc_endpoint(&workspace_path)?;
    let listener = bind_listener(&endpoint)?;
    info!(endpoint = %endpoint, "IPC listener bound");

    // T077 / S097: Set Unix socket permissions to 0o600 (owner read/write only).
    // Windows named pipes inherit the creating user's security context via OS ACL —
    // no explicit permission setting is required on that platform.
    //
    // We use `endpoint` (already computed above) rather than a hardcoded path so
    // that the /tmp/ fallback sockets introduced in T093 are also secured (S119).
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let socket_path = std::path::Path::new(&endpoint);
        if socket_path.exists() {
            std::fs::set_permissions(socket_path, std::fs::Permissions::from_mode(0o600))
                .map_err(|e| {
                    EngramError::Ipc(DomainIpcError::ConnectionFailed {
                        address: endpoint.clone(),
                        reason: format!("failed to set socket permissions: {e}"),
                    })
                })?;
            debug!(
                socket = %socket_path.display(),
                mode = "0o600",
                "Unix socket permissions set to owner-only"
            );
        }
    }

    // T049 / S046: Reset the idle deadline so the TTL window starts from when
    // the daemon is ready to serve requests, not from when it started.  On a
    // slow machine SurrealDB init may consume several hundred milliseconds;
    // without this reset a short idle timeout (e.g. 500 ms in tests) would
    // fire before the readiness probe even connects.
    ttl.reset();

    // T045: Spawn the TTL expiry task now that the daemon is ready to serve.
    // Spawning here (after bind) rather than at process startup ensures the
    // idle window begins from "daemon ready", preventing false expiry during
    // slow SurrealDB initialization.
    {
        let ttl_task = Arc::clone(&ttl);
        let tx_for_ttl = Arc::clone(&shutdown_tx);
        tokio::spawn(async move {
            ttl_task.run_until_expired(tx_for_ttl).await;
        });
    }

    accept_loop(listener, state, ttl, shutdown_tx, shutdown_rx).await;
    Ok(())
}

/// Run the daemon accept loop for the given workspace path (legacy API).
///
/// Delegates to [`run_with_shutdown`] with a no-op TTL and a one-time
/// Ctrl-C shutdown. New code should call [`run_with_shutdown`] directly.
///
/// # Errors
///
/// Returns [`EngramError`] if path validation, lock acquisition, or listener
/// binding fails.
pub async fn run(workspace: &str) -> Result<(), EngramError> {
    let ttl = TtlTimer::new(std::time::Duration::ZERO); // no auto-shutdown
    let (tx, rx) = watch::channel(false);
    run_with_shutdown(workspace, ttl, Arc::new(tx), rx).await
}

// ── Accept loop ──────────────────────────────────────────────────────────────

/// Drive the main accept loop until the shutdown channel fires.
///
/// On each accepted connection the idle TTL is reset (S046). The `_shutdown`
/// IPC handler and the TTL expiry task both write `true` to `shutdown_tx`,
/// which causes `shutdown_rx.changed()` to fire and exit this loop.
async fn accept_loop(
    listener: Listener,
    state: SharedState,
    ttl: Arc<TtlTimer>,
    shutdown_tx: Arc<watch::Sender<bool>>,
    mut shutdown_rx: watch::Receiver<bool>,
) {
    loop {
        tokio::select! {
            accept_result = listener.accept() => {
                match accept_result {
                    Ok(stream) => {
                        // T049: every accepted connection resets the idle timer (S046).
                        ttl.reset();

                        let state = Arc::clone(&state);
                        let tx = Arc::clone(&shutdown_tx);
                        tokio::spawn(handle_connection(stream, state, tx));
                    }
                    Err(e) => {
                        error!(error = %e, "IPC listener accept error");
                    }
                }
            }
            // Watch for shutdown signal from TTL expiry, _shutdown handler, or signal.
            changed = shutdown_rx.changed() => {
                match changed {
                    Ok(()) if *shutdown_rx.borrow() => {
                        info!("shutdown signal received — stopping IPC listener");
                        break;
                    }
                    Ok(()) => {}   // value changed to false — ignore
                    Err(_) => {
                        // Sender dropped; treat as shutdown.
                        info!("shutdown channel closed — stopping IPC listener");
                        break;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    #[cfg(unix)]
    fn short_workspace_path_uses_engram_sock() {
        // "/tmp/ws" + "/.engram/run/engram.sock" (24 chars) = 31 bytes ≤ 108.
        let ws = Path::new("/tmp/ws");
        let ep = ipc_endpoint(ws).unwrap();
        assert!(
            ep.ends_with("/.engram/run/engram.sock"),
            "expected standard path, got {ep}"
        );
    }

    #[test]
    #[cfg(unix)]
    fn long_workspace_path_uses_tmp_fallback() {
        // "/.engram/run/engram.sock" = 24 chars; workspace needs > 84 chars so
        // total exceeds 108 bytes.
        let long_ws = "/tmp/".to_owned() + &"a".repeat(90); // 95 chars → total 119
        let ws = Path::new(&long_ws);
        let ep = ipc_endpoint(ws).unwrap();
        assert!(
            ep.starts_with("/tmp/engram-"),
            "expected /tmp/ fallback, got {ep}"
        );
        assert!(ep.ends_with(".sock"), "expected .sock suffix, got {ep}");
        // The fallback path must itself be short enough to bind.
        assert!(
            ep.len() <= 108,
            "fallback path {ep} still exceeds 108 bytes"
        );
    }

    #[test]
    #[cfg(unix)]
    fn boundary_path_exactly_108_bytes_uses_standard() {
        // "/.engram/run/engram.sock" = 24 chars.  Workspace must be 84 chars
        // for the total to be exactly 108 (≤ 108 → standard path taken).
        // "/tmp/" = 5 chars + 79 'a's = 84.
        let prefix = "/tmp/";
        let padding = "a".repeat(84 - prefix.len()); // 79 'a's
        let ws_str = format!("{prefix}{padding}");
        assert_eq!(ws_str.len(), 84, "workspace path should be 84 bytes");
        let ws = Path::new(&ws_str);
        let ep = ipc_endpoint(ws).unwrap();
        assert!(
            ep.ends_with("/.engram/run/engram.sock"),
            "expected standard path at boundary, got {ep}"
        );
        assert_eq!(ep.len(), 108, "boundary path should be exactly 108 bytes");
    }

    #[test]
    #[cfg(windows)]
    fn windows_endpoint_uses_named_pipe() {
        let ws = Path::new(r"C:\Users\test\project");
        let ep = ipc_endpoint(ws).unwrap();
        assert!(
            ep.starts_with(r"\\.\pipe\engram-"),
            "expected named pipe, got {ep}"
        );
    }
}
