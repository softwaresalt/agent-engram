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
//! | Windows  | `\\.\pipe\engram-{workspace_key}` |

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use chrono::Utc;
use interprocess::local_socket::{
    ListenerOptions,
    tokio::{Listener, Stream, prelude::*},
};
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, watch};
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

use crate::daemon::protocol::{HealthCheckResult, IpcError as WireError, IpcRequest, IpcResponse};
use crate::daemon::ttl::TtlTimer;
use crate::daemon::watcher::{WatcherConfig, start_watcher};
use crate::db::workspace::daemon_key_for_workspace;
use crate::errors::{EngramError, IpcError as DomainIpcError};
use crate::models::WatcherEvent;
use crate::models::config::WorkspaceConfig;
use crate::models::health::ScanProgress;
use crate::server::state::{
    AppState, CompletionOutcome, CoordinatorCell, DispatchSnapshot, DriverTaskGuard, OwnerKind,
    OwnerPermit, OwnerProgressScope, SharedState, WorkspaceSnapshot,
};
use crate::shim::version::{ENGRAM_BUILD_HASH, ENGRAM_PROTOCOL_VERSION};
use crate::tools;

// ── Endpoint naming ──────────────────────────────────────────────────────────

/// Compute the IPC endpoint string for the given workspace.
///
/// - **Unix**: `{workspace}/.engram/run/engram.sock`
/// - **Windows**: `\\.\pipe\engram-{workspace_key}` where the key is the
///   persisted `.workspace-id`, or the legacy path hash while a pre-upgrade
///   daemon is still live.
///
/// # Errors
///
/// Returns [`EngramError::Ipc`] if the workspace path contains non-UTF-8
/// characters or if the platform is unsupported.
pub fn ipc_endpoint(workspace: &Path) -> Result<String, EngramError> {
    ipc_endpoint_impl(workspace)
}

#[cfg(unix)]
#[cfg(target_os = "macos")]
const MAX_UNIX_SOCKET_PATH_LEN: usize = 103;
#[cfg(unix)]
#[cfg(not(target_os = "macos"))]
const MAX_UNIX_SOCKET_PATH_LEN: usize = 107;
#[cfg(unix)]
fn ipc_endpoint_impl(workspace: &Path) -> Result<String, EngramError> {
    let sock_path = workspace.join(".engram").join("run").join("engram.sock");

    let path_str = sock_path.to_str().ok_or_else(|| {
        EngramError::Ipc(DomainIpcError::ConnectionFailed {
            address: workspace.display().to_string(),
            reason: "workspace path is not valid UTF-8".to_owned(),
        })
    })?;

    // Unix domain socket paths are limited by `sockaddr_un::sun_path`, including
    // the terminating NUL byte: Linux exposes 108 bytes and macOS exposes 104.
    // Use the maximum string length that still leaves room for the terminator.
    if path_str.len() <= MAX_UNIX_SOCKET_PATH_LEN {
        return Ok(path_str.to_owned());
    }

    // Fallback: create a private /tmp/engram-{key}/ directory (0o700) and
    // place the socket at /tmp/engram-{key}/engram.sock.
    //
    // The directory is created at construction time with DirBuilder::mode(0o700)
    // to avoid a TOCTOU window between creation and permission assignment.
    let key = daemon_key_for_workspace(workspace)?;
    let dir = format!("/tmp/engram-{key}");

    {
        use std::fs::DirBuilder;
        use std::os::unix::fs::DirBuilderExt;
        use std::os::unix::fs::PermissionsExt as _;
        DirBuilder::new()
            .mode(0o700)
            .recursive(true)
            .create(&dir)
            .map_err(|e| {
                EngramError::Ipc(DomainIpcError::ConnectionFailed {
                    address: dir.clone(),
                    reason: format!("cannot create private socket directory: {e}"),
                })
            })?;
        // Verify the directory has exactly 0o700 permissions.  If another
        // process pre-created the directory with insecure permissions, refuse
        // to use it rather than trusting a potentially-compromised path.
        let meta = std::fs::metadata(&dir).map_err(|e| {
            EngramError::Ipc(DomainIpcError::ConnectionFailed {
                address: dir.clone(),
                reason: format!("cannot stat private socket directory: {e}"),
            })
        })?;
        let mode = meta.permissions().mode() & 0o777;
        if mode != 0o700 {
            return Err(EngramError::Ipc(DomainIpcError::ConnectionFailed {
                address: dir.clone(),
                reason: format!(
                    "private socket directory has insecure permissions {mode:#o}; expected 0o700"
                ),
            }));
        }
    }

    let fallback = format!("{dir}/engram.sock");

    tracing::warn!(
        workspace = %workspace.display(),
        fallback = %fallback,
        path_len = path_str.len(),
        "Unix socket path exceeds platform limit — using /tmp/ fallback (S119)"
    );

    Ok(fallback)
}

#[cfg(windows)]
fn ipc_endpoint_impl(workspace: &Path) -> Result<String, EngramError> {
    let key = daemon_key_for_workspace(workspace)?;
    Ok(format!(r"\\.\pipe\engram-{key}"))
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
    // T042: Connection health span – log establishment and closure for every
    // IPC connection so that long-running sessions can be traced end-to-end.
    let connection_id = Uuid::new_v4().to_string();
    debug!(connection_id = %connection_id, "ipc_connection_established");

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
            debug!(connection_id = %connection_id, "ipc_connection_closed");
            debug!("IPC connection closed before sending a request (EOF)");
            return;
        }
        Ok(n) if n > MAX_REQUEST_BYTES => (
            IpcResponse::parse_error(format!("request exceeds {MAX_REQUEST_BYTES} byte limit")),
            false,
        ),
        Ok(_) => process_request(&line, &state).await,
        Err(e) => {
            warn!(error = %e, "failed to read IPC request line");
            debug!(connection_id = %connection_id, "ipc_connection_closed");
            return;
        }
    };

    let shutdown_requested = response.1;
    let frame_outcome = match response.0.to_line() {
        Ok(line_str) => {
            if let Err(e) = send_half.write_all(line_str.as_bytes()).await {
                error!(error = %e, "failed to write IPC response");
                "write_error"
            } else if let Err(e) = send_half.flush().await {
                error!(error = %e, "failed to flush IPC response");
                "flush_error"
            } else {
                "flushed"
            }
        }
        Err(e) => {
            error!(error = %e, "failed to serialize IPC response");
            "serialize_error"
        }
    };

    emit_response_frame_result(&connection_id, &response.0.id, frame_outcome);
    debug!(connection_id = %connection_id, "ipc_connection_closed");
    if shutdown_requested {
        // Signal only after the shutdown response has been fully attempted.
        // The accept loop may now abort/join every tracked handler, including
        // this one, without truncating the required response or self-joining.
        let _ = shutdown_tx.send(true);
    }
}

/// Emit the single terminal response-frame event through the configured tracing
/// subscriber.
///
/// JSON-RPC string, integer, floating-point, and Boolean IDs retain their JSON
/// scalar type in JSON tracing output. Null and composite IDs fall back to
/// their JSON text because `tracing` fields do not support those value types.
fn emit_response_frame_result(connection_id: &str, response_id: &Value, outcome: &str) {
    macro_rules! emit {
        ($response_id:expr) => {{
            info!(
                event_type = "response_frame_result",
                connection_id,
                response_id = $response_id,
                outcome,
                "response_frame_result"
            )
        }};
    }

    match response_id {
        Value::String(response_id) => emit!(response_id.as_str()),
        Value::Number(response_id) => {
            if let Some(response_id) = response_id.as_i64() {
                emit!(response_id);
            } else if let Some(response_id) = response_id.as_u64() {
                emit!(response_id);
            } else if let Some(response_id) = response_id.as_f64() {
                emit!(response_id);
            } else {
                let response_id = response_id.to_string();
                emit!(response_id.as_str());
            }
        }
        Value::Bool(response_id) => emit!(*response_id),
        Value::Null | Value::Array(_) | Value::Object(_) => {
            let response_id = response_id.to_string();
            emit!(response_id.as_str());
        }
    }
}

/// Deserialize and dispatch a single raw request line, returning an [`IpcResponse`].
async fn process_request(line: &str, state: &SharedState) -> (IpcResponse, bool) {
    let request = match IpcRequest::from_line(line.trim()) {
        Ok(r) => r,
        Err(err_response) => return (err_response, false),
    };

    if let Err(err_response) = request.validate() {
        return (err_response, false);
    }

    // Safe to unwrap: validate() ensures id is Some.
    let id = request.id.clone().unwrap_or(Value::Null);

    match request.method.as_str() {
        "_health" => {
            // Return "starting" while workspace hydration is in progress so
            // the shim keeps polling rather than treating the daemon as healthy
            // before it can serve real tool calls.
            let snapshot = state.snapshot_workspace().await;
            let status = if snapshot.is_some() && state.is_hydration_ready() {
                "ready"
            } else {
                "starting"
            };
            (
                IpcResponse::success(
                    id,
                    json!(HealthCheckResult {
                        status: status.to_owned(),
                        uptime_seconds: state.uptime_seconds(),
                        workspace: snapshot.map(|s| s.path),
                        active_connections: state.active_connections(),
                        protocol_version: ENGRAM_PROTOCOL_VERSION,
                        build_hash: ENGRAM_BUILD_HASH.to_owned(),
                    }),
                ),
                false,
            )
        }
        // T052: `_shutdown` is signalled by `handle_connection` only after
        // this response has been written and flushed (S022, S037).
        "_shutdown" => {
            info!("daemon received _shutdown IPC request — initiating graceful shutdown");
            (
                IpcResponse::success(
                    id,
                    json!({ "status": "shutting_down", "flush_started": true }),
                ),
                true,
            )
        }
        method => (
            match tools::dispatch(Arc::clone(state), method, request.params).await {
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
            false,
        ),
    }
}

// ── Daemon entry point ───────────────────────────────────────────────────────

/// Atomically snapshot a daemon driver's immutable payload and admission guard.
///
/// This is the single acquisition seam shared by the auto-sync and file-watcher
/// closures in [`run_with_shutdown`] and [`run_with_shutdown_v2`] (092.003-T).
/// Routing all four sites through one helper keeps the atomicity guarantee
/// testable: [`AppState::guarded_workspace_and_config`] holds both read locks
/// together, so a concurrent [`AppState::set_workspace_and_config`] cannot yield
/// a torn `(workspace_i, config_j)` pair. Reverting this body to two separate
/// reads would reopen that window — the `daemon_sync_context_never_tears_pair`
/// unit test guards against exactly that regression.
async fn guarded_daemon_sync_context(
    state: &AppState,
) -> Option<(
    crate::server::state::AdmissionGuard,
    WorkspaceSnapshot,
    WorkspaceConfig,
)> {
    state.guarded_workspace_and_config().await
}

fn same_workspace_instance(left: &WorkspaceSnapshot, right: &WorkspaceSnapshot) -> bool {
    left.workspace_uuid == right.workspace_uuid && left.path == right.path
}

impl AppState {
    async fn prepare_daemon_mutation(
        &self,
        permit: OwnerPermit,
        context: DispatchSnapshot,
        kind: OwnerKind,
    ) -> Result<Option<(OwnerPermit, DispatchSnapshot)>, EngramError> {
        crate::tools::write::prepare_branch_owner(self, permit, context, kind).await
    }
}

async fn flush_daemon_snapshot(snapshot: &WorkspaceSnapshot) -> Result<(), EngramError> {
    let _guard = crate::services::dehydration::acquire_flush_lock().await;
    let db = crate::db::connect_db(&snapshot.data_dir, &snapshot.branch).await?;
    let queries = crate::db::queries::CodeGraphQueries::new(db);
    crate::services::dehydration::dehydrate_code_graph(
        &queries,
        &snapshot.data_dir,
        &snapshot.branch,
    )
    .await?;
    if let Err(error) = crate::services::metrics::compute_and_write_summary(
        std::path::Path::new(&snapshot.path),
        &snapshot.branch,
    )
    .await
    {
        if !matches!(
            error,
            EngramError::Metrics(crate::errors::MetricsError::NotFound { .. })
        ) {
            warn!(
                %error,
                branch = %snapshot.branch,
                "metrics summary write failed during flush"
            );
        }
    }
    Ok(())
}

fn spawn_daemon_driver<F>(future: F) -> DriverTaskGuard
where
    F: std::future::Future<Output = ()> + Send + 'static,
{
    DriverTaskGuard {
        task: Some(tokio::spawn(future)),
    }
}

async fn join_retained_hydration(state: &AppState) -> Result<(), EngramError> {
    if let Some(driver) = state.take_hydration_driver() {
        driver.join().await.map_err(|error| {
            EngramError::System(crate::errors::SystemError::DatabaseError {
                reason: format!("retained hydration failed before safe terminal: {error}"),
            })
        })?;
    }
    Ok(())
}

/// Run the daemon accept loop with graceful shutdown support.
///
/// Steps:
/// 1. Canonicalize and validate the workspace path.
/// 2. Create `.engram/run/` if needed.
/// 3. Build [`AppState`] and set the active workspace.
/// 4. Compute and bind the IPC endpoint.
/// 5. Hydrate workspace state asynchronously, then auto-sync the code graph.
/// 6. Run the watcher event loop for file-change auto-sync.
/// 7. Enter the accept loop; exit when `shutdown_rx` becomes `true`.
///
/// `event_rx` receives debounced file-change events from the workspace watcher
/// started in [`crate::daemon::run`]. The auto-sync loop in this function
/// batches events and triggers [`crate::services::code_graph::sync_workspace`]
/// after a 2-second quiet window.
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
    event_rx: mpsc::UnboundedReceiver<WatcherEvent>,
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

    // ── Bind the IPC endpoint BEFORE hydrating the workspace ─────────────────
    //
    // Workspace hydration (connect_db, hydrate_into_db, hydrate_code_graph)
    // can take several seconds on large workspaces. If we hydrate first and
    // bind after, the shim's health-check poll times out before the pipe even
    // exists. By binding first we let the shim connect immediately; the
    // `_health` handler returns `"starting"` until hydration completes, then
    // switches to `"ready"`.
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
            std::fs::set_permissions(socket_path, std::fs::Permissions::from_mode(0o600)).map_err(
                |e| {
                    EngramError::Ipc(DomainIpcError::ConnectionFailed {
                        address: endpoint.clone(),
                        reason: format!("failed to set socket permissions: {e}"),
                    })
                },
            )?;
            debug!(
                socket = %socket_path.display(),
                mode = "0o600",
                "Unix socket permissions set to owner-only"
            );
        }
    }

    let mut startup_shutdown = shutdown_rx.clone();
    let startup_state = Arc::clone(&state);
    let startup_workspace = workspace.to_owned();
    let startup_ttl = Arc::clone(&ttl);
    let startup_tx = Arc::clone(&shutdown_tx);
    let startup_driver = spawn_daemon_driver(async move {
        tokio::select! {
            () = run_startup_driver(
                startup_state,
                startup_workspace,
                startup_ttl,
                startup_tx,
            ) => {}
            () = async {
                if *startup_shutdown.borrow() {
                    return;
                }
                while startup_shutdown.changed().await.is_ok() {
                    if *startup_shutdown.borrow() {
                        return;
                    }
                }
            } => {}
        }
    });

    let mut watcher_shutdown = shutdown_rx.clone();
    let watcher_state = Arc::clone(&state);
    let watcher_ttl = Arc::clone(&ttl);
    let watcher_driver = spawn_daemon_driver(async move {
        tokio::select! {
            () = run_watcher_driver(watcher_state, watcher_ttl, event_rx, false) => {}
            () = async {
                if *watcher_shutdown.borrow() {
                    return;
                }
                while watcher_shutdown.changed().await.is_ok() {
                    if *watcher_shutdown.borrow() {
                        return;
                    }
                }
            } => {}
        }
    });

    accept_loop(listener, Arc::clone(&state), ttl, shutdown_tx, shutdown_rx).await;
    if let Err(error) = startup_driver.join().await {
        warn!(%error, "startup driver join failed");
    }
    if let Err(error) = watcher_driver.join().await {
        warn!(%error, "legacy watcher driver join failed");
    }
    join_retained_hydration(&state).await?;
    crate::services::metrics::shutdown().await?;
    crate::services::dehydration::flush_all_workspaces(&state).await?;
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
    // Legacy callers have no file watcher; pass a channel that is immediately
    // closed so the auto-sync loop in run_with_shutdown exits cleanly.
    let (event_tx, event_rx) = mpsc::unbounded_channel::<WatcherEvent>();
    drop(event_tx);
    run_with_shutdown(workspace, ttl, Arc::new(tx), rx, event_rx).await
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
    let mut connection_handlers = tokio::task::JoinSet::new();
    loop {
        if *shutdown_rx.borrow() {
            info!("shutdown signal already set — stopping IPC listener");
            break;
        }
        tokio::select! {
            biased;
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
            joined = connection_handlers.join_next(), if !connection_handlers.is_empty() => {
                if let Some(Err(error)) = joined {
                    if !error.is_cancelled() {
                        warn!(%error, "IPC connection handler join failed");
                    }
                }
            }
            accept_result = listener.accept() => {
                match accept_result {
                    Ok(stream) => {
                        // T049: every accepted connection resets the idle timer (S046).
                        ttl.reset();

                        let state = Arc::clone(&state);
                        let tx = Arc::clone(&shutdown_tx);
                        connection_handlers.spawn(handle_connection(stream, state, tx));
                    }
                    Err(e) => {
                        error!(error = %e, "IPC listener accept error");
                    }
                }
            }
        }
    }

    // Admission has stopped. Cancel and join every accepted handler before
    // startup/hydration/watchers are drained and the final flush begins.
    connection_handlers.abort_all();
    while let Some(result) = connection_handlers.join_next().await {
        if let Err(error) = result {
            if !error.is_cancelled() {
                warn!(%error, "IPC connection handler join failed during shutdown");
            }
        }
    }
}

/// Run the daemon IPC server using the refactored startup order where the file
/// watcher is initialised *after* the IPC listener binds.
///
/// This is the new entry point introduced by task 025.002-T, replacing
/// [`run_with_shutdown`]. Unlike its predecessor, this function does **not**
/// receive an `event_rx` channel from the caller; instead the channel is created
/// internally immediately after [`bind_listener`] succeeds.  Moving watcher
/// initialisation past the bind point prevents a slow `ReadDirectoryChangesW`
/// (Windows) or `inotify_add_watch` (Linux) registration from delaying the
/// moment the shim can send its first health probe.
///
/// # Errors
///
/// Returns [`EngramError`] if path validation, run-directory creation, or
/// listener binding fails.
pub async fn run_with_shutdown_v2(
    workspace: &str,
    ttl: Arc<TtlTimer>,
    shutdown_tx: Arc<watch::Sender<bool>>,
    shutdown_rx: watch::Receiver<bool>,
    watcher_config: WatcherConfig,
) -> Result<(), EngramError> {
    let workspace_path = std::fs::canonicalize(workspace).map_err(|e| {
        EngramError::Ipc(DomainIpcError::ConnectionFailed {
            address: workspace.to_owned(),
            reason: format!("cannot canonicalize workspace path: {e}"),
        })
    })?;

    // Ensure .engram/run/ exists before binding the listener.
    // The daemon lock is already held by `daemon::run()` for the entire daemon
    // lifetime; this function does not acquire it.
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

    // ── Bind the IPC endpoint BEFORE starting the file watcher ───────────────
    //
    // This is the core fix for the daemon startup hang (025.002-T).
    // `start_watcher` calls `new_debouncer` + `debouncer.watch(…, Recursive)`
    // which blocks the calling thread while the OS registers watch handles for
    // every directory in the workspace.  On a large workspace this takes several
    // seconds and previously blocked the tokio executor before the IPC listener
    // was ever bound.  Binding first lets the shim connect immediately; the
    // `_health` handler returns `"starting"` until hydration completes.
    let endpoint = ipc_endpoint(&workspace_path)?;
    let listener = bind_listener(&endpoint)?;
    info!(endpoint = %endpoint, "IPC listener bound");

    // T077 / S097: Set Unix socket permissions to 0o600 (owner read/write only).
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let socket_path = std::path::Path::new(&endpoint);
        if socket_path.exists() {
            std::fs::set_permissions(socket_path, std::fs::Permissions::from_mode(0o600)).map_err(
                |e| {
                    EngramError::Ipc(DomainIpcError::ConnectionFailed {
                        address: endpoint.clone(),
                        reason: format!("failed to set socket permissions: {e}"),
                    })
                },
            )?;
            debug!(
                socket = %socket_path.display(),
                mode = "0o600",
                "Unix socket permissions set to owner-only"
            );
        }
    }

    let mut startup_shutdown = shutdown_rx.clone();
    let startup_state = Arc::clone(&state);
    let startup_workspace = workspace.to_owned();
    let startup_ttl = Arc::clone(&ttl);
    let startup_tx = Arc::clone(&shutdown_tx);
    let startup_driver = spawn_daemon_driver(async move {
        tokio::select! {
            () = run_startup_driver(
                startup_state,
                startup_workspace,
                startup_ttl,
                startup_tx,
            ) => {}
            () = async {
                if *startup_shutdown.borrow() {
                    return;
                }
                while startup_shutdown.changed().await.is_ok() {
                    if *startup_shutdown.borrow() {
                        return;
                    }
                }
            } => {}
        }
    });

    // ── Start file watcher AFTER IPC bind, using spawn_blocking ──────────────
    //
    // `new_debouncer` + `debouncer.watch(…, RecursiveMode::Recursive)` are
    // synchronous and block until the OS has registered watch handles for every
    // sub-directory.  On a large workspace this can exceed the shim's health-
    // probe timeout.  Using `spawn_blocking` offloads the blocking work to the
    // thread pool so the tokio executor and the already-bound IPC listener remain
    // responsive. The blocking task is joined rather than detached so a timed
    // out registration cannot create a watcher after its parent has moved on.
    //
    // The mpsc channel is created inside the blocking task so that `event_tx` is
    // dropped together with the closure on the error path.
    //
    // This block is intentionally placed AFTER the set_workspace tokio::spawn above
    // so that workspace hydration begins concurrently with watcher registration.
    let workspace_for_watcher = workspace_path.clone();
    let watcher_init_handle = tokio::task::spawn_blocking(move || {
        let (event_tx, event_rx) = mpsc::unbounded_channel::<WatcherEvent>();
        // start_watcher logs its own error/warning details; we only need to
        // surface the result here.
        let handle =
            start_watcher(&workspace_for_watcher, watcher_config, event_tx).unwrap_or(None);
        handle.map(|h| (h, event_rx))
    });
    let mut watcher_shutdown = shutdown_rx.clone();
    let watcher_state = Arc::clone(&state);
    let watcher_ttl = Arc::clone(&ttl);
    let watcher_driver = spawn_daemon_driver(async move {
        let watcher_init = match watcher_init_handle.await {
            Ok(watcher_init) => watcher_init,
            Err(join_error) => {
                error!(
                    error = %join_error,
                    "watcher spawn_blocking panicked; daemon continues degraded"
                );
                return;
            }
        };
        if let Some((_watcher_handle, event_rx)) = watcher_init {
            tokio::select! {
                () = run_watcher_driver(watcher_state, watcher_ttl, event_rx, true) => {}
                () = async {
                    if *watcher_shutdown.borrow() {
                        return;
                    }
                    while watcher_shutdown.changed().await.is_ok() {
                        if *watcher_shutdown.borrow() {
                            return;
                        }
                    }
                } => {}
            }
        }
    });

    accept_loop(listener, Arc::clone(&state), ttl, shutdown_tx, shutdown_rx).await;
    if let Err(error) = startup_driver.join().await {
        warn!(%error, "startup driver join failed");
    }
    if let Err(error) = watcher_driver.join().await {
        warn!(%error, "v2 watcher driver join failed");
    }
    join_retained_hydration(&state).await?;
    crate::services::metrics::shutdown().await?;
    crate::services::dehydration::flush_all_workspaces(&state).await?;
    Ok(())
}

async fn run_startup_driver(
    state: SharedState,
    workspace: String,
    ttl: Arc<TtlTimer>,
    shutdown_tx: Arc<watch::Sender<bool>>,
) {
    if let Err(error) = crate::tools::lifecycle::set_workspace(Arc::clone(&state), workspace).await
    {
        error!(%error, "workspace hydration failed — initiating shutdown");
        let _ = shutdown_tx.send(true);
        return;
    }

    info!("workspace hydration complete — daemon ready to serve");
    ttl.reset();
    let ttl_task = Arc::clone(&ttl);
    tokio::spawn(async move {
        ttl_task.run_until_expired(shutdown_tx).await;
    });

    let Some((admission, snapshot, workspace_config)) = guarded_daemon_sync_context(&state).await
    else {
        return;
    };
    let permit = match admission.acquire_background(OwnerKind::Startup).await {
        Ok(Some(permit)) => permit,
        Ok(None) => return,
        Err(error) => {
            error!(%error, "startup coordinator admission failed");
            return;
        }
    };
    let context = DispatchSnapshot {
        workspace: snapshot,
        config: workspace_config,
    };
    let Some((mut permit, context)) = (match state
        .prepare_daemon_mutation(permit, context, OwnerKind::Startup)
        .await
    {
        Ok(prepared) => prepared,
        Err(error) => {
            error!(%error, "startup branch preparation failed");
            return;
        }
    }) else {
        return;
    };
    let snapshot = context.workspace;
    let workspace_config = context.config;
    let Some(progress_scope) = permit.progress_scope() else {
        error!("startup permit lost ownership before progress relay creation");
        return;
    };
    let (mut backfill_progress, progress_tx) =
        StartupBackfillProgressRelay::spawn(Arc::clone(&state), progress_scope.clone());

    let operation = async {
        let mut backfill_result = None;
        let workspace_path = std::path::PathBuf::from(&snapshot.path);
        let should_flush = match crate::services::code_graph::sync_workspace(
            &workspace_path,
            &snapshot.data_dir,
            &snapshot.branch,
            &workspace_config.code_graph,
        )
        .await
        {
            Ok(result) => {
                info!(
                    files_added = result.files_added,
                    files_modified = result.files_modified,
                    files_unchanged = result.files_unchanged,
                    "startup auto-sync complete"
                );
                true
            }
            Err(error) => {
                warn!(%error, "startup auto-sync failed");
                false
            }
        };

        match crate::db::connect_db(&snapshot.data_dir, &snapshot.branch).await {
            Ok(db) => {
                let queries = crate::db::queries::CodeGraphQueries::new(db);
                let registry_path = workspace_path.join(".engram").join("registry.yaml");
                match crate::services::registry::load_registry(&registry_path) {
                    Ok(Some(mut config)) => {
                        let _ = crate::services::registry::validate_sources(
                            &mut config,
                            &workspace_path,
                        );
                        match crate::services::ingestion::ingest_all_sources(
                            &config,
                            &workspace_path,
                            &queries,
                        )
                        .await
                        {
                            Ok(summary) => {
                                info!(
                                    ingested = summary.ingested,
                                    unchanged = summary.unchanged,
                                    total = summary.total_files,
                                    "startup registry ingestion complete"
                                );
                            }
                            Err(error) => {
                                warn!(%error, "startup registry ingestion failed");
                            }
                        }
                    }
                    Ok(None) => {
                        debug!("no registry.yaml — skipping content ingestion");
                    }
                    Err(error) => {
                        warn!(%error, "startup registry load failed");
                    }
                }

                backfill_result = Some(backfill_with_scan_progress(&queries, &progress_tx).await);
            }
            Err(error) => {
                warn!(%error, "startup ingestion: failed to connect to database");
            }
        }

        if should_flush {
            if let Err(error) = flush_daemon_snapshot(&snapshot).await {
                warn!(%error, "startup auto-flush failed");
            }
        }
        backfill_result
    };

    let backfill_result = permit.run_until_cancelled(operation).await;
    // The cancellable future owns a producer clone. Close it before joining so
    // normal completion cannot wait forever for its own progress channel.
    drop(progress_tx);
    if backfill_result.is_none() {
        backfill_progress.abort_and_join().await;
        return;
    }
    backfill_progress.join().await;
    if let Some(result) = backfill_result.flatten() {
        match result {
            Ok(updated) => {
                if let Some(snapshot) = backfill_completion_snapshot(
                    backfill_progress.relayed_running(),
                    updated,
                    Utc::now().to_rfc3339(),
                ) {
                    let _ = state
                        .set_scan_progress_for_owner(&progress_scope, Some(snapshot))
                        .await;
                }
                if updated != 0 {
                    info!(updated, "startup content embedding backfill complete");
                }
            }
            Err(error) => {
                warn!(%error, "startup content embedding backfill failed");
            }
        }
    }
    let transferred = match CoordinatorCell::complete(permit) {
        CompletionOutcome::Transferred(successor) => Some(successor),
        CompletionOutcome::Released
        | CompletionOutcome::RetirementAcknowledged
        | CompletionOutcome::SequenceExhausted(_)
        | CompletionOutcome::Stale => None,
    };
    if let Some(successor) = transferred {
        drive_daemon_transferred_syncs(
            &state,
            &snapshot,
            &workspace_config,
            successor,
            "startup",
            #[cfg(test)]
            None,
        )
        .await;
    }
}

async fn drive_daemon_transferred_syncs(
    state: &AppState,
    snapshot: &WorkspaceSnapshot,
    workspace_config: &WorkspaceConfig,
    successor: crate::server::state::OwnerPermit,
    driver: &'static str,
    #[cfg(test)] operation_reached: Option<&AtomicBool>,
) {
    let mut successor = successor;
    let mut current_ctx = crate::server::state::DispatchSnapshot {
        workspace: snapshot.clone(),
        config: workspace_config.clone(),
    };
    loop {
        let prepared =
            crate::tools::write::prepare_transferred_sync(state, successor, current_ctx).await;
        let Some((prepared_successor, prepared_ctx)) = (match prepared {
            Ok(prepared) => prepared,
            Err(error) => {
                warn!(%error, driver, "daemon transferred branch preparation failed");
                return;
            }
        }) else {
            return;
        };
        successor = prepared_successor;
        current_ctx = prepared_ctx;
        let work_bits = successor.work_bits();
        let operation = async {
            #[cfg(test)]
            if let Some(operation_reached) = operation_reached {
                operation_reached.store(true, Ordering::SeqCst);
            }
            let workspace_path = std::path::PathBuf::from(&current_ctx.workspace.path);
            let result = crate::services::code_graph::sync_workspace_with_progress(
                &workspace_path,
                &current_ctx.workspace.data_dir,
                &current_ctx.workspace.branch,
                &current_ctx.config.code_graph,
                work_bits & 0b100 != 0,
                work_bits & 0b010 != 0,
                None,
            )
            .await;
            match &result {
                Ok(result) => {
                    let unfulfilled_work_bits =
                        crate::tools::write::unfulfilled_work_bits(&result.errors, work_bits);
                    info!(
                        driver,
                        files_added = result.files_added,
                        files_modified = result.files_modified,
                        unfulfilled_work_bits,
                        "daemon transferred sync complete"
                    );
                    if let Err(error) = flush_daemon_snapshot(&current_ctx.workspace).await {
                        warn!(%error, driver, "daemon transferred flush failed");
                    }
                    unfulfilled_work_bits == 0
                }
                Err(error) => {
                    warn!(%error, driver, "daemon transferred sync failed");
                    false
                }
            }
        };
        if !matches!(successor.run_until_cancelled(operation).await, Some(true)) {
            return;
        }
        let _ = state.set_hydration_ready_for_permit(&successor);
        successor = match CoordinatorCell::complete(successor) {
            CompletionOutcome::Transferred(next) => next,
            CompletionOutcome::Released
            | CompletionOutcome::RetirementAcknowledged
            | CompletionOutcome::SequenceExhausted(_)
            | CompletionOutcome::Stale => return,
        };
    }
}

async fn run_watcher_driver(
    state: SharedState,
    ttl: Arc<TtlTimer>,
    mut event_rx: mpsc::UnboundedReceiver<WatcherEvent>,
    reactive_markdown: bool,
) {
    while let Some(event) = event_rx.recv().await {
        ttl.reset();
        let Some((mut admission, mut snapshot, mut workspace_config)) =
            guarded_daemon_sync_context(&state).await
        else {
            continue;
        };
        let mut pending_reingest = std::collections::BTreeSet::new();
        let mut pending_reindex = match crate::daemon::debounce::adapt_event(&event) {
            crate::daemon::debounce::ServiceAction::ReindexFile { .. } => true,
            crate::daemon::debounce::ServiceAction::ReingestContent { path }
                if reactive_markdown =>
            {
                pending_reingest.insert(path);
                false
            }
            crate::daemon::debounce::ServiceAction::ReingestContent { .. }
            | crate::daemon::debounce::ServiceAction::Skip => false,
        };

        while let Ok(Some(event)) =
            tokio::time::timeout(Duration::from_secs(2), event_rx.recv()).await
        {
            ttl.reset();
            match crate::daemon::debounce::adapt_event(&event) {
                crate::daemon::debounce::ServiceAction::ReindexFile { .. } => {
                    pending_reindex = true;
                }
                crate::daemon::debounce::ServiceAction::ReingestContent { path }
                    if reactive_markdown =>
                {
                    pending_reingest.insert(path);
                }
                crate::daemon::debounce::ServiceAction::ReingestContent { .. }
                | crate::daemon::debounce::ServiceAction::Skip => {}
            }
        }
        if !pending_reindex && pending_reingest.is_empty() {
            continue;
        }

        'reconcile_batch: loop {
            let mut permit = match admission.acquire_background(OwnerKind::Watcher).await {
                Ok(Some(permit)) => permit,
                Ok(None) => {
                    let Some((next_admission, next_snapshot, next_config)) =
                        guarded_daemon_sync_context(&state).await
                    else {
                        break 'reconcile_batch;
                    };
                    if !same_workspace_instance(&snapshot, &next_snapshot) {
                        break 'reconcile_batch;
                    }
                    admission = next_admission;
                    snapshot = next_snapshot;
                    workspace_config = next_config;
                    continue 'reconcile_batch;
                }
                Err(error) => {
                    error!(%error, "watcher coordinator admission failed");
                    break 'reconcile_batch;
                }
            };
            let context = DispatchSnapshot {
                workspace: snapshot,
                config: workspace_config,
            };
            let Some((prepared_permit, context)) = (match state
                .prepare_daemon_mutation(permit, context, OwnerKind::Watcher)
                .await
            {
                Ok(prepared) => prepared,
                Err(error) => {
                    error!(%error, "watcher branch preparation failed");
                    break 'reconcile_batch;
                }
            }) else {
                break 'reconcile_batch;
            };
            permit = prepared_permit;
            snapshot = context.workspace;
            workspace_config = context.config;
            let operation = async {
                let workspace_path = std::path::PathBuf::from(&snapshot.path);
                let should_flush = if pending_reindex {
                    match crate::services::code_graph::sync_workspace(
                        &workspace_path,
                        &snapshot.data_dir,
                        &snapshot.branch,
                        &workspace_config.code_graph,
                    )
                    .await
                    {
                        Ok(result) => {
                            info!(
                                files_added = result.files_added,
                                files_modified = result.files_modified,
                                "file-change auto-sync complete"
                            );
                            true
                        }
                        Err(error) => {
                            warn!(%error, "file-change auto-sync failed");
                            false
                        }
                    }
                } else {
                    false
                };

                if !pending_reingest.is_empty() {
                    crate::services::reactive_sync::reingest_pending_markdown(
                        &workspace_path,
                        &snapshot.data_dir,
                        &snapshot.branch,
                        &pending_reingest,
                    )
                    .await;
                }

                if should_flush {
                    if let Err(error) = flush_daemon_snapshot(&snapshot).await {
                        warn!(%error, "file-change auto-flush failed");
                    }
                }
            };
            if permit.run_until_cancelled(operation).await.is_none() {
                drop(permit);
                let Some((next_admission, next_snapshot, next_config)) =
                    guarded_daemon_sync_context(&state).await
                else {
                    break 'reconcile_batch;
                };
                if !same_workspace_instance(&snapshot, &next_snapshot) {
                    break 'reconcile_batch;
                }
                admission = next_admission;
                snapshot = next_snapshot;
                workspace_config = next_config;
                continue 'reconcile_batch;
            }

            if let CompletionOutcome::Transferred(successor) = CoordinatorCell::complete(permit) {
                drive_daemon_transferred_syncs(
                    &state,
                    &snapshot,
                    &workspace_config,
                    successor,
                    "watcher",
                    #[cfg(test)]
                    None,
                )
                .await;
            }
            break 'reconcile_batch;
        }
    }
}

/// Build a `running` scan-status snapshot reflecting embedding-backfill progress.
fn backfill_running_progress(done: usize, total: usize) -> ScanProgress {
    ScanProgress {
        running: true,
        files_scanned: done as u64,
        files_total: total as u64,
        last_completed_at: None,
    }
}

/// Build a completed scan-status snapshot for a finished embedding backfill.
fn backfill_completed_progress(done: usize, completed_at: String) -> ScanProgress {
    ScanProgress {
        running: false,
        files_scanned: done as u64,
        files_total: done as u64,
        last_completed_at: Some(completed_at),
    }
}

/// Decide the `scan_status` snapshot to write once the backfill finishes.
///
/// Returns a `running: false` completed snapshot whenever any `running`
/// progress was relayed — even if `embedded == 0` (model unavailable or every
/// write-back failed). This clears a `running: true` status that would
/// otherwise persist forever, since owner completion does not touch
/// `scan_progress`. Returns `None` when no running progress was relayed (there
/// was nothing to clear).
fn backfill_completion_snapshot(
    relayed_running: bool,
    embedded: usize,
    completed_at: String,
) -> Option<ScanProgress> {
    if relayed_running {
        Some(backfill_completed_progress(embedded, completed_at))
    } else {
        None
    }
}

/// Run the content-embedding backfill using the startup owner's progress relay.
async fn backfill_with_scan_progress(
    queries: &crate::db::queries::CodeGraphQueries,
    progress_tx: &mpsc::UnboundedSender<crate::services::ingestion::BackfillProgress>,
) -> Result<usize, EngramError> {
    crate::services::ingestion::backfill_content_embeddings(queries, Some(progress_tx)).await
}

/// Parent-owned progress child for the startup embedding phase.
///
/// The relay is created outside the cancellable startup future so cancellation
/// can abort and join it before the startup permit acknowledges retirement.
/// Every publication is fenced to the exact startup owner.
struct StartupBackfillProgressRelay {
    tx: Option<mpsc::UnboundedSender<crate::services::ingestion::BackfillProgress>>,
    relayed_running: Arc<AtomicBool>,
    child: Option<DriverTaskGuard>,
}

impl StartupBackfillProgressRelay {
    fn spawn(
        state: SharedState,
        scope: OwnerProgressScope,
    ) -> (
        Self,
        mpsc::UnboundedSender<crate::services::ingestion::BackfillProgress>,
    ) {
        let (tx, mut rx) =
            mpsc::unbounded_channel::<crate::services::ingestion::BackfillProgress>();
        let relayed_running = Arc::new(AtomicBool::new(false));
        let relayed_for_updater = Arc::clone(&relayed_running);
        let child = tokio::spawn(async move {
            while let Some(progress) = rx.recv().await {
                if state
                    .set_scan_progress_for_owner(
                        &scope,
                        Some(backfill_running_progress(progress.done, progress.total)),
                    )
                    .await
                {
                    relayed_for_updater.store(true, Ordering::Relaxed);
                }
            }
        });
        let producer = tx.clone();
        (
            Self {
                tx: Some(tx),
                relayed_running,
                child: Some(DriverTaskGuard { task: Some(child) }),
            },
            producer,
        )
    }

    fn relayed_running(&self) -> bool {
        self.relayed_running.load(Ordering::Relaxed)
    }

    async fn join(&mut self) {
        let _ = self.tx.take();
        if let Some(child) = self.child.take() {
            if let Err(error) = child.join().await {
                warn!(%error, "startup backfill progress updater failed");
            }
        }
    }

    async fn abort_and_join(&mut self) {
        let _ = self.tx.take();
        if let Some(child) = self.child.take() {
            let _ = child.abort_and_join().await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::state::{
        AppState, CoordinatorCell, DispatchSnapshot, OwnerKind, RequestOutcome, WorkMask,
    };
    #[cfg(unix)]
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;

    #[derive(Clone, Default)]
    struct CapturedJson(Arc<Mutex<Vec<u8>>>);

    impl std::io::Write for CapturedJson {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0
                .lock()
                .expect("capture lock")
                .extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl<'writer> tracing_subscriber::fmt::MakeWriter<'writer> for CapturedJson {
        type Writer = Self;

        fn make_writer(&'writer self) -> Self::Writer {
            self.clone()
        }
    }

    #[test]
    fn response_frame_capture_exercises_production_event_and_preserves_id_types() {
        let captured = CapturedJson::default();
        let subscriber = tracing_subscriber::fmt()
            .json()
            .with_writer(captured.clone())
            .finish();

        tracing::subscriber::with_default(subscriber, || {
            emit_response_frame_result("connection-string", &json!("62046B37-cold-1"), "flushed");
            emit_response_frame_result("connection-number", &json!(62_046), "write_error");
        });

        let bytes = captured.0.lock().expect("capture lock").clone();
        let records = String::from_utf8(bytes)
            .expect("capture is UTF-8")
            .lines()
            .map(|line| serde_json::from_str::<Value>(line).expect("capture line is JSON"))
            .collect::<Vec<_>>();

        assert_eq!(records.len(), 2);
        assert_eq!(
            records[0]["fields"],
            json!({
                "event_type": "response_frame_result",
                "connection_id": "connection-string",
                "response_id": "62046B37-cold-1",
                "outcome": "flushed",
                "message": "response_frame_result",
            })
        );
        assert_eq!(records[1]["fields"]["response_id"], json!(62_046));
        assert_eq!(records[1]["fields"]["outcome"], "write_error");
    }

    #[cfg(unix)]
    async fn connect_test_stream(endpoint: &str) -> Stream {
        use interprocess::local_socket::{GenericFilePath, ToFsName};

        let name = endpoint
            .to_fs_name::<GenericFilePath>()
            .expect("convert test socket path");
        Stream::connect(name).await.expect("connect test socket")
    }

    #[cfg(windows)]
    async fn connect_test_stream(endpoint: &str) -> Stream {
        use interprocess::local_socket::{GenericNamespaced, ToNsName};

        let pipe_name = endpoint.strip_prefix(r"\\.\pipe\").unwrap_or(endpoint);
        let name = pipe_name
            .to_ns_name::<GenericNamespaced>()
            .expect("convert test pipe name");
        Stream::connect(name).await.expect("connect test pipe")
    }

    fn coordinator_snapshot(
        workspace_id: &str,
        workspace_uuid: &str,
        path: &std::path::Path,
        data_dir: std::path::PathBuf,
    ) -> WorkspaceSnapshot {
        WorkspaceSnapshot {
            workspace_id: workspace_id.to_owned(),
            workspace_uuid: workspace_uuid.to_owned(),
            branch: "main".to_owned(),
            data_dir,
            path: path.display().to_string(),
            last_flush: None,
            stale_files: false,
            connection_count: 0,
            file_mtimes: std::collections::HashMap::new(),
        }
    }

    /// 092.003-T: the shared daemon `(workspace, config)` acquisition seam must
    /// never expose a torn pair to a background-sync closure while a concurrent
    /// bind is flipping the active workspace. This is the regression guard for
    /// [`snapshot_daemon_sync_context`]: reverting its body to two separate
    /// reads reopens the tear window and makes this test fail.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn daemon_sync_context_never_tears_pair() {
        use crate::models::config::CodeGraphConfig;
        use std::collections::HashMap;
        use std::sync::atomic::{AtomicBool, Ordering};

        const LARGE: u64 = 8_000_000;
        const SMALL: u64 = 1_024;

        fn snapshot_for(path: &str) -> WorkspaceSnapshot {
            WorkspaceSnapshot {
                workspace_id: format!("ws-{path}"),
                workspace_uuid: format!("uuid-{path}"),
                branch: "main".to_owned(),
                data_dir: std::path::PathBuf::from("unused"),
                path: path.to_owned(),
                last_flush: None,
                stale_files: false,
                connection_count: 0,
                file_mtimes: HashMap::new(),
            }
        }
        fn config_with(max_file_size_bytes: u64) -> WorkspaceConfig {
            WorkspaceConfig {
                code_graph: CodeGraphConfig {
                    max_file_size_bytes,
                    ..CodeGraphConfig::default()
                },
                ..WorkspaceConfig::default()
            }
        }

        // Two internally-consistent (workspace, config) states. A torn read
        // pairs one state's `path` with the other's `max_file_size_bytes`.
        let snapshot_a = snapshot_for("/ws/a");
        let snapshot_b = snapshot_for("/ws/b");
        let config_a = config_with(LARGE);
        let config_b = config_with(SMALL);

        let state = Arc::new(AppState::new(10));
        state
            .set_workspace_and_config(snapshot_a.clone(), Some(config_a.clone()))
            .await
            .expect("seed bind A");

        let stop = Arc::new(AtomicBool::new(false));
        let writer_state = state.clone();
        let writer_stop = stop.clone();
        let writer = tokio::spawn(async move {
            while !writer_stop.load(Ordering::Relaxed) {
                writer_state
                    .set_workspace_and_config(snapshot_b.clone(), Some(config_b.clone()))
                    .await
                    .expect("bind B");
                tokio::task::yield_now().await;
                writer_state
                    .set_workspace_and_config(snapshot_a.clone(), Some(config_a.clone()))
                    .await
                    .expect("bind A");
                tokio::task::yield_now().await;
            }
        });

        let mut torn = 0u32;
        let mut observed_a = 0u32;
        let mut observed_b = 0u32;
        let mut iterations = 0u32;
        while (iterations < 2_000 || observed_a == 0 || observed_b == 0) && iterations < 50_000 {
            iterations += 1;
            let Some((_admission, snap, cfg)) = guarded_daemon_sync_context(&state).await else {
                continue;
            };
            let max = cfg.code_graph.max_file_size_bytes;
            if snap.path == "/ws/a" {
                observed_a += 1;
                if max != LARGE {
                    torn += 1;
                }
            } else if snap.path == "/ws/b" {
                observed_b += 1;
                if max != SMALL {
                    torn += 1;
                }
            }
            tokio::task::yield_now().await;
        }

        stop.store(true, Ordering::Relaxed);
        writer.await.expect("writer task joins");

        assert!(
            observed_a > 0 && observed_b > 0,
            "vacuous test: must observe both bound states (A={observed_a}, B={observed_b})"
        );
        assert_eq!(
            torn, 0,
            "daemon sync context tore {torn} (workspace, config) pair(s) across \
             {iterations} samples (A={observed_a}, B={observed_b})"
        );
    }

    #[cfg(unix)]
    const SOCKET_SUFFIX_LEN: usize = "/.engram/run/engram.sock".len();

    #[cfg(unix)]
    fn create_workspace_for_socket_len(
        target_socket_len: usize,
        needs_git: bool,
    ) -> (tempfile::TempDir, PathBuf) {
        let root = tempfile::tempdir().expect("workspace tempdir");
        let root_str = root.path().to_string_lossy();
        let target_workspace_len = target_socket_len - SOCKET_SUFFIX_LEN;
        assert!(
            root_str.len() < target_workspace_len,
            "tempdir root {} exceeds target workspace length {}",
            root_str.len(),
            target_workspace_len
        );

        let padding = "a".repeat(target_workspace_len - root_str.len() - 1);
        let workspace = root.path().join(padding);
        std::fs::create_dir_all(&workspace).expect("create padded workspace");

        if needs_git {
            std::fs::create_dir(workspace.join(".git")).expect("create .git");
            std::fs::write(
                workspace.join(".git").join("HEAD"),
                "ref: refs/heads/main\n",
            )
            .expect("write HEAD");
        }

        (root, workspace)
    }

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
        let (_root, workspace) =
            create_workspace_for_socket_len(MAX_UNIX_SOCKET_PATH_LEN + 1, true);
        let ep = ipc_endpoint(&workspace).unwrap();
        assert!(
            ep.starts_with("/tmp/engram-"),
            "expected /tmp/ fallback, got {ep}"
        );
        assert!(
            std::path::Path::new(&ep)
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("sock")),
            "expected .sock suffix, got {ep}"
        );
        // The fallback path must itself be short enough to bind.
        assert!(
            ep.len() <= MAX_UNIX_SOCKET_PATH_LEN,
            "fallback path {ep} still exceeds platform limit"
        );
    }

    #[test]
    #[cfg(unix)]
    fn boundary_path_at_platform_limit_uses_standard() {
        let (_root, workspace) = create_workspace_for_socket_len(MAX_UNIX_SOCKET_PATH_LEN, false);
        let ep = ipc_endpoint(&workspace).unwrap();
        assert!(
            ep.ends_with("/.engram/run/engram.sock"),
            "expected standard path at boundary, got {ep}"
        );
        assert_eq!(
            ep.len(),
            MAX_UNIX_SOCKET_PATH_LEN,
            "boundary path should match the platform limit"
        );
    }

    #[test]
    #[cfg(windows)]
    fn windows_endpoint_uses_named_pipe() {
        let workspace = tempfile::tempdir().expect("workspace tempdir");
        std::fs::create_dir(workspace.path().join(".git")).expect("create .git");
        let ep = ipc_endpoint(workspace.path()).unwrap();
        assert!(
            ep.starts_with(r"\\.\pipe\engram-"),
            "expected named pipe, got {ep}"
        );
    }

    #[tokio::test]
    async fn daemon_driver_handle_loss_cannot_detach_mutation() {
        struct Termination(Option<tokio::sync::oneshot::Sender<()>>);

        impl Drop for Termination {
            fn drop(&mut self) {
                if let Some(terminated) = self.0.take() {
                    let _ = terminated.send(());
                }
            }
        }

        let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = tokio::sync::oneshot::channel();
        let (terminated_tx, terminated_rx) = tokio::sync::oneshot::channel();
        let writes = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let writes_for_driver = Arc::clone(&writes);
        let driver = spawn_daemon_driver(async move {
            let _termination = Termination(Some(terminated_tx));
            let _ = entered_tx.send(());
            let _ = release_rx.await;
            writes_for_driver.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        });

        entered_rx.await.expect("driver should enter");
        drop(driver);
        let _ = release_tx.send(());
        terminated_rx.await.expect("driver should terminate");
        assert_eq!(
            writes.load(std::sync::atomic::Ordering::SeqCst),
            0,
            "a lost parent handle must abort before later mutation"
        );
    }

    #[tokio::test]
    async fn normal_shutdown_joins_retained_hydration_to_its_safe_terminal() {
        let state = AppState::new(1);
        let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = tokio::sync::oneshot::channel();
        let reached_terminal = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let terminal_for_driver = Arc::clone(&reached_terminal);
        let driver = spawn_daemon_driver(async move {
            let _ = entered_tx.send(());
            let _ = release_rx.await;
            terminal_for_driver.store(true, std::sync::atomic::Ordering::SeqCst);
        });
        assert!(state.retain_hydration_driver(1, driver).is_none());
        entered_rx.await.expect("hydration should enter");
        let releaser = tokio::spawn(async move {
            let _ = release_tx.send(());
        });

        join_retained_hydration(&state)
            .await
            .expect("hydration should reach its safe terminal");

        releaser.await.expect("hydration releaser should join");
        assert!(
            reached_terminal.load(std::sync::atomic::Ordering::SeqCst),
            "normal shutdown must not abort hydration before its safe terminal"
        );
        assert!(state.take_hydration_driver().is_none());
    }

    #[tokio::test]
    async fn startup_prepares_current_head_before_database_or_file_mutation() {
        let metrics_guard = crate::services::metrics::test_writer_guard().await;
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = temp.path().join("workspace");
        std::fs::create_dir_all(workspace.join(".git")).expect("create git metadata");
        std::fs::write(
            workspace.join(".git").join("HEAD"),
            "ref: refs/heads/main\n",
        )
        .expect("write git HEAD");
        let state = AppState::new(1);
        let mut stale_snapshot = coordinator_snapshot(
            "id-stale",
            "uuid-worktree",
            &workspace,
            temp.path().join("data"),
        );
        stale_snapshot.branch = "captured-before-checkout".to_owned();
        crate::services::metrics::configure_test_disabled_writer(
            &metrics_guard,
            &workspace,
            &stale_snapshot.branch,
        )
        .await
        .expect("configure disabled startup metrics");
        let _ = state
            .publish_workspace_generation(stale_snapshot, Some(WorkspaceConfig::default()))
            .await
            .expect("publish stale capture");
        let (admission, workspace, config) = guarded_daemon_sync_context(&state)
            .await
            .expect("guarded startup context");
        let permit = admission
            .acquire_background(OwnerKind::Startup)
            .await
            .expect("startup admission")
            .expect("startup permit");

        let (_, prepared) = state
            .prepare_daemon_mutation(
                permit,
                DispatchSnapshot { workspace, config },
                OwnerKind::Startup,
            )
            .await
            .expect("prepare startup mutation")
            .expect("same worktree remains active");

        assert_eq!(
            prepared.workspace.branch, "main",
            "startup must refresh HEAD before its first DB/file mutation"
        );
        assert_eq!(
            state
                .snapshot_workspace()
                .await
                .expect("published workspace")
                .branch,
            "main",
            "the refreshed branch must be coherently published"
        );
        crate::services::metrics::shutdown()
            .await
            .expect("reset startup metrics");
    }

    #[tokio::test]
    async fn shared_accept_loop_quiesces_accepted_handlers_before_returning() {
        let workspace = tempfile::tempdir().expect("workspace tempdir");
        std::fs::create_dir_all(workspace.path().join(".git")).expect("create git metadata");
        std::fs::create_dir_all(workspace.path().join(".engram").join("run"))
            .expect("create IPC runtime directory");
        std::fs::write(
            workspace.path().join(".git").join("HEAD"),
            "ref: refs/heads/main\n",
        )
        .expect("write git HEAD");
        let endpoint = ipc_endpoint(workspace.path()).expect("IPC endpoint");
        let listener = bind_listener(&endpoint).expect("bind IPC listener");
        let state = Arc::new(AppState::new(1));
        let ttl = TtlTimer::new(Duration::ZERO);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let shutdown_tx = Arc::new(shutdown_tx);
        let server_state = Arc::clone(&state);
        let server_tx = Arc::clone(&shutdown_tx);
        let server = tokio::spawn(async move {
            accept_loop(listener, server_state, ttl, server_tx, shutdown_rx).await;
        });

        let blocker = connect_test_stream(&endpoint).await;
        let (blocker_recv, mut blocker_send) = blocker.split();
        blocker_send
            .write_all(
                br#"{"jsonrpc":"2.0","id":1,"method":"blocked_until_shutdown","params":null}"#,
            )
            .await
            .expect("write partial blocking request");
        blocker_send
            .flush()
            .await
            .expect("flush partial blocking request");

        // A later accepted witness proves the partial connection's handler was
        // already admitted before shutdown; no timing or sleep is involved.
        let witness = connect_test_stream(&endpoint).await;
        let (witness_recv, mut witness_send) = witness.split();
        witness_send
            .write_all(b"{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"_health\"}\n")
            .await
            .expect("write witness request");
        witness_send.flush().await.expect("flush witness request");
        let mut witness_reader = BufReader::new(witness_recv);
        let mut witness_response = String::new();
        assert!(
            witness_reader
                .read_line(&mut witness_response)
                .await
                .expect("read witness response")
                > 0,
            "witness connection must complete"
        );

        shutdown_tx.send(true).expect("signal shutdown");
        server.await.expect("accept loop joins");

        let _ = blocker_send.write_all(b"\n").await;
        let _ = blocker_send.flush().await;
        drop(blocker_send);
        let mut blocker_reader = BufReader::new(blocker_recv);
        let mut blocker_response = String::new();
        let bytes = blocker_reader
            .read_line(&mut blocker_response)
            .await
            .expect("read blocker terminal");

        assert_eq!(
            state.tool_call_count(),
            0,
            "an accepted handler must not dispatch after accept-loop shutdown"
        );
        assert_eq!(
            bytes, 0,
            "quiesced handler must close without processing its partial request"
        );
    }

    #[tokio::test]
    async fn shutdown_request_flushes_exact_response_before_handler_quiescence() {
        let workspace = tempfile::tempdir().expect("workspace tempdir");
        std::fs::create_dir_all(workspace.path().join(".git")).expect("create git metadata");
        std::fs::create_dir_all(workspace.path().join(".engram").join("run"))
            .expect("create IPC runtime directory");
        std::fs::write(
            workspace.path().join(".git").join("HEAD"),
            "ref: refs/heads/main\n",
        )
        .expect("write git HEAD");
        let endpoint = ipc_endpoint(workspace.path()).expect("IPC endpoint");
        let listener = bind_listener(&endpoint).expect("bind IPC listener");
        let state = Arc::new(AppState::new(1));
        let ttl = TtlTimer::new(Duration::ZERO);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let shutdown_tx = Arc::new(shutdown_tx);
        let server_tx = Arc::clone(&shutdown_tx);
        let server = tokio::spawn(async move {
            accept_loop(listener, state, ttl, server_tx, shutdown_rx).await;
        });

        let stream = connect_test_stream(&endpoint).await;
        let (recv, mut send) = stream.split();
        send.write_all(b"{\"jsonrpc\":\"2.0\",\"id\":9,\"method\":\"_shutdown\"}\n")
            .await
            .expect("write shutdown request");
        send.flush().await.expect("flush shutdown request");
        let mut reader = BufReader::new(recv);
        let mut response = String::new();
        assert!(
            reader
                .read_line(&mut response)
                .await
                .expect("read shutdown response")
                > 0,
            "shutdown response must be flushed before quiescence"
        );
        let response: Value =
            serde_json::from_str(response.trim()).expect("parse shutdown response");
        assert_eq!(
            response,
            json!({
                "jsonrpc": "2.0",
                "id": 9,
                "result": {
                    "status": "shutting_down",
                    "flush_started": true
                }
            })
        );

        server
            .await
            .expect("shutdown handler must not self-join or deadlock");
        assert!(*shutdown_tx.borrow());
    }

    #[tokio::test]
    async fn daemon_transferred_failure_recovers_full_mask() {
        let metrics_guard = crate::services::metrics::test_writer_guard().await;
        crate::services::metrics::shutdown()
            .await
            .expect("reset metrics writer");
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = temp.path().join("workspace");
        std::fs::create_dir_all(&workspace).expect("create workspace");
        let invalid_data_dir = temp.path().join("not-a-directory");
        std::fs::write(&invalid_data_dir, b"file blocks database directory")
            .expect("create invalid data path");
        let snapshot =
            coordinator_snapshot("id-transfer", "uuid-transfer", &workspace, invalid_data_dir);
        crate::services::metrics::configure_test_disabled_writer(
            &metrics_guard,
            &workspace,
            &snapshot.branch,
        )
        .await
        .expect("configure fixture metrics writer");
        let config = WorkspaceConfig::default();
        let state = AppState::new(1);
        let _ = state
            .publish_workspace_generation(snapshot.clone(), Some(config.clone()))
            .await
            .expect("publish binding");
        let owner = match CoordinatorCell::request(
            state.coordinator.admission(),
            WorkMask::default(),
            OwnerKind::Startup,
        )
        .expect("request startup owner")
        {
            RequestOutcome::Acquired(permit) => permit,
            RequestOutcome::Waiting(_) | RequestOutcome::Enqueued | RequestOutcome::Stale => {
                panic!("startup owner should acquire")
            }
        };
        assert!(matches!(
            CoordinatorCell::request(
                state.coordinator.admission(),
                WorkMask::from_bits(0b111),
                OwnerKind::Sync,
            ),
            Ok(RequestOutcome::Enqueued)
        ));
        let successor = match CoordinatorCell::complete(owner) {
            CompletionOutcome::Transferred(successor) => successor,
            CompletionOutcome::Released
            | CompletionOutcome::RetirementAcknowledged
            | CompletionOutcome::SequenceExhausted(_)
            | CompletionOutcome::Stale => panic!("full mask should transfer"),
        };
        let operation_reached = AtomicBool::new(false);

        drive_daemon_transferred_syncs(
            &state,
            &snapshot,
            &config,
            successor,
            "test",
            Some(&operation_reached),
        )
        .await;

        let operation_was_reached = operation_reached.load(Ordering::SeqCst);
        let coordinator_is_idle = state.coordinator.test_is_idle();
        let pending_bits = state.coordinator.test_pending_bits();
        crate::services::metrics::shutdown()
            .await
            .expect("clean fixture metrics writer");

        assert!(
            operation_was_reached,
            "fixture must reach the intended database failure"
        );
        assert!(coordinator_is_idle);
        assert_eq!(pending_bits, 0b111);
    }

    #[tokio::test]
    async fn daemon_transferred_partial_file_errors_recover_full_mask() {
        let metrics_guard = crate::services::metrics::test_writer_guard().await;
        crate::services::metrics::shutdown()
            .await
            .expect("reset metrics writer");
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = temp.path().join("workspace");
        let data_dir = temp.path().join("data");
        std::fs::create_dir_all(&workspace).expect("create workspace");
        std::fs::write(workspace.join("broken.py"), [0xff]).expect("write invalid UTF-8 fixture");
        let snapshot = coordinator_snapshot(
            "id-transfer-partial",
            "uuid-transfer-partial",
            &workspace,
            data_dir,
        );
        crate::services::metrics::configure_test_disabled_writer(
            &metrics_guard,
            &workspace,
            &snapshot.branch,
        )
        .await
        .expect("configure fixture metrics writer");
        let config = WorkspaceConfig::default();
        let state = AppState::new(1);
        let _ = state
            .publish_workspace_generation(snapshot.clone(), Some(config.clone()))
            .await
            .expect("publish binding");
        let owner = match CoordinatorCell::request(
            state.coordinator.admission(),
            WorkMask::default(),
            OwnerKind::Startup,
        )
        .expect("request startup owner")
        {
            RequestOutcome::Acquired(permit) => permit,
            RequestOutcome::Waiting(_) | RequestOutcome::Enqueued | RequestOutcome::Stale => {
                panic!("startup owner should acquire")
            }
        };
        assert!(matches!(
            CoordinatorCell::request(
                state.coordinator.admission(),
                WorkMask::from_bits(0b111),
                OwnerKind::Sync,
            ),
            Ok(RequestOutcome::Enqueued)
        ));
        let successor = match CoordinatorCell::complete(owner) {
            CompletionOutcome::Transferred(successor) => successor,
            CompletionOutcome::Released
            | CompletionOutcome::RetirementAcknowledged
            | CompletionOutcome::SequenceExhausted(_)
            | CompletionOutcome::Stale => panic!("full mask should transfer"),
        };
        let operation_reached = AtomicBool::new(false);

        drive_daemon_transferred_syncs(
            &state,
            &snapshot,
            &config,
            successor,
            "test",
            Some(&operation_reached),
        )
        .await;

        let operation_was_reached = operation_reached.load(Ordering::SeqCst);
        let coordinator_is_idle = state.coordinator.test_is_idle();
        let pending_bits = state.coordinator.test_pending_bits();
        crate::services::metrics::shutdown()
            .await
            .expect("clean fixture metrics writer");

        assert!(
            operation_was_reached,
            "fixture must reach the intended per-file failure"
        );
        assert!(coordinator_is_idle);
        assert_eq!(
            pending_bits, 0b111,
            "a partial daemon transfer must retain heavy work for retry"
        );
    }

    #[test]
    fn watcher_batch_retries_only_for_same_worktree_path_and_uuid() {
        let root = std::path::Path::new("C:/workspace");
        let original = coordinator_snapshot(
            "id-original",
            "uuid-original",
            root,
            std::path::PathBuf::from("data/original"),
        );
        let mut checked_out_branch = original.clone();
        checked_out_branch.branch = "feature".to_owned();
        checked_out_branch.workspace_id = "id-feature".to_owned();
        let mut different_path = checked_out_branch.clone();
        different_path.path = "C:/workspace/rebound".to_owned();
        let mut different_uuid = checked_out_branch.clone();
        different_uuid.workspace_uuid = "uuid-replacement".to_owned();

        assert!(same_workspace_instance(&original, &checked_out_branch));
        assert!(!same_workspace_instance(&original, &different_path));
        assert!(!same_workspace_instance(&original, &different_uuid));
    }

    #[test]
    fn backfill_running_progress_marks_scan_active_with_counts() {
        let progress = backfill_running_progress(128, 2441);
        assert!(progress.running, "backfill in flight must report running");
        assert_eq!(progress.files_scanned, 128);
        assert_eq!(progress.files_total, 2441);
        assert!(
            progress.last_completed_at.is_none(),
            "an in-flight backfill has no completion timestamp"
        );
    }

    #[test]
    fn backfill_completed_progress_marks_scan_finished() {
        let progress = backfill_completed_progress(2441, "2026-07-06T07:28:21Z".to_owned());
        assert!(!progress.running, "finished backfill must clear running");
        assert_eq!(progress.files_scanned, 2441);
        assert_eq!(
            progress.files_total, progress.files_scanned,
            "completed snapshot reports the embedded record count as the total"
        );
        assert_eq!(
            progress.last_completed_at.as_deref(),
            Some("2026-07-06T07:28:21Z")
        );
    }

    #[test]
    fn backfill_completion_snapshot_clears_running_even_when_nothing_embedded() {
        // Regression: when running progress was relayed but the model was
        // unavailable (embedded == 0), status must still be cleared to
        // `running: false` — otherwise it reports indexing forever.
        let snapshot = backfill_completion_snapshot(true, 0, "2026-07-06T07:28:21Z".to_owned())
            .expect("relayed progress must yield a completion snapshot");
        assert!(!snapshot.running, "status must be cleared to not-running");
        assert_eq!(snapshot.files_scanned, 0);
    }

    #[test]
    fn backfill_completion_snapshot_reports_embedded_count() {
        let snapshot = backfill_completion_snapshot(true, 42, "2026-07-06T07:28:21Z".to_owned())
            .expect("relayed progress must yield a completion snapshot");
        assert!(!snapshot.running);
        assert_eq!(snapshot.files_scanned, 42);
    }

    #[test]
    fn backfill_completion_snapshot_is_none_when_no_progress_relayed() {
        // Nothing was set to running (no pending records), so there is nothing
        // to clear and the existing scan_status must be left untouched.
        assert!(
            backfill_completion_snapshot(false, 0, "2026-07-06T07:28:21Z".to_owned()).is_none(),
            "no relayed progress means no completion snapshot"
        );
    }

    #[tokio::test]
    async fn backfill_progress_relay_updates_scan_status() {
        let state: SharedState = Arc::new(AppState::new(1));
        let workspace = tempfile::tempdir().expect("workspace tempdir");
        let data = tempfile::tempdir().expect("data tempdir");
        std::fs::create_dir_all(workspace.path().join(".git")).expect("create git metadata");
        std::fs::write(
            workspace.path().join(".git").join("HEAD"),
            "ref: refs/heads/main\n",
        )
        .expect("write git HEAD");
        let _ = state
            .publish_workspace_generation(
                coordinator_snapshot(
                    "id-main",
                    "uuid-worktree",
                    workspace.path(),
                    data.path().to_path_buf(),
                ),
                Some(WorkspaceConfig::default()),
            )
            .await
            .expect("publish workspace");
        let (admission, _, _) = guarded_daemon_sync_context(&state)
            .await
            .expect("guarded startup context");
        let permit = admission
            .acquire_background(OwnerKind::Startup)
            .await
            .expect("startup admission")
            .expect("startup permit");
        let scope = permit.progress_scope().expect("progress scope");
        let (mut relay, sender) = StartupBackfillProgressRelay::spawn(Arc::clone(&state), scope);

        sender
            .send(crate::services::ingestion::BackfillProgress {
                done: 256,
                total: 1000,
            })
            .expect("send progress");
        drop(sender);
        relay.join().await;

        let snapshot = state
            .scan_progress_snapshot()
            .await
            .expect("scan status populated by relay");
        assert!(snapshot.running);
        assert_eq!(snapshot.files_scanned, 256);
        assert_eq!(snapshot.files_total, 1000);
    }

    #[tokio::test]
    async fn cancelled_backfill_progress_relay_quiesces_and_rejects_stale_writes() {
        let state: SharedState = Arc::new(AppState::new(1));
        let workspace = tempfile::tempdir().expect("workspace tempdir");
        let data = tempfile::tempdir().expect("data tempdir");
        std::fs::create_dir_all(workspace.path().join(".git")).expect("create git metadata");
        std::fs::write(
            workspace.path().join(".git").join("HEAD"),
            "ref: refs/heads/main\n",
        )
        .expect("write git HEAD");
        let _ = state
            .publish_workspace_generation(
                coordinator_snapshot(
                    "id-main",
                    "uuid-worktree",
                    workspace.path(),
                    data.path().to_path_buf(),
                ),
                Some(WorkspaceConfig::default()),
            )
            .await
            .expect("publish workspace");
        let (admission, _, _) = guarded_daemon_sync_context(&state)
            .await
            .expect("guarded startup context");
        let permit = admission
            .acquire_background(OwnerKind::Startup)
            .await
            .expect("startup admission")
            .expect("startup permit");
        let scope = permit.progress_scope().expect("progress scope");
        let (mut relay, sender) = StartupBackfillProgressRelay::spawn(Arc::clone(&state), scope);

        relay.abort_and_join().await;
        assert!(
            sender
                .send(crate::services::ingestion::BackfillProgress {
                    done: 999,
                    total: 1000,
                })
                .is_err(),
            "joined cancellation must close the progress receiver"
        );
        assert!(matches!(
            CoordinatorCell::complete(permit),
            CompletionOutcome::Released
        ));
        assert!(
            state.scan_progress_snapshot().await.is_none(),
            "a quiesced stale child must not publish progress after retirement"
        );
    }
}
