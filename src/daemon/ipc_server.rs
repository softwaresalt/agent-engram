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
use crate::server::state::{AppState, SharedState, WorkspaceSnapshot};
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
        Ok(n) if n > MAX_REQUEST_BYTES => {
            IpcResponse::parse_error(format!("request exceeds {MAX_REQUEST_BYTES} byte limit"))
        }
        Ok(_) => process_request(&line, &state, &shutdown_tx).await,
        Err(e) => {
            warn!(error = %e, "failed to read IPC request line");
            debug!(connection_id = %connection_id, "ipc_connection_closed");
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

    debug!(connection_id = %connection_id, "ipc_connection_closed");
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
            )
        }
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

/// Atomically snapshot the active `(workspace, config)` pair for a daemon
/// background-sync closure, returning `None` when either value is absent.
///
/// This is the single acquisition seam shared by the auto-sync and file-watcher
/// closures in [`run_with_shutdown`] and [`run_with_shutdown_v2`] (092.003-T).
/// Routing all four sites through one helper keeps the atomicity guarantee
/// testable: [`AppState::snapshot_workspace_and_config`] holds both read locks
/// together, so a concurrent [`AppState::set_workspace_and_config`] cannot yield
/// a torn `(workspace_i, config_j)` pair. Reverting this body to two separate
/// reads would reopen that window — the `daemon_sync_context_never_tears_pair`
/// unit test guards against exactly that regression.
async fn snapshot_daemon_sync_context(
    state: &AppState,
) -> Option<(WorkspaceSnapshot, WorkspaceConfig)> {
    state.snapshot_workspace_and_config().await
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
    mut event_rx: mpsc::UnboundedReceiver<WatcherEvent>,
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

    // ── Hydrate workspace in a background task ────────────────────────────────
    //
    // Running set_workspace asynchronously unblocks the accept loop so health
    // probes from the shim are answered immediately (with "starting") rather
    // than timing out waiting for CozoDB init + file hydration.
    //
    // On success: reset the TTL idle deadline and spawn the TTL expiry task
    //   (T049/S046, T045) so the idle window begins from "daemon ready".
    // On failure: send the shutdown signal so the daemon exits cleanly and the
    //   shim's poll eventually gives up with NotReady.
    {
        let state_init = Arc::clone(&state);
        let workspace_str = workspace.to_owned();
        let ttl_init = Arc::clone(&ttl);
        let tx_init = Arc::clone(&shutdown_tx);
        tokio::spawn(async move {
            match crate::tools::lifecycle::set_workspace(Arc::clone(&state_init), workspace_str)
                .await
            {
                Ok(_) => {
                    info!("workspace hydration complete — daemon ready to serve");
                    // T049 / S046: Reset idle deadline from "daemon ready", not
                    // "daemon starting", to avoid false TTL expiry during slow init.
                    ttl_init.reset();
                    // T045: Start TTL expiry task after the socket is bound and
                    // workspace is hydrated so idle counting begins at readiness.
                    let ttl_task = Arc::clone(&ttl_init);
                    tokio::spawn(async move {
                        ttl_task.run_until_expired(tx_init).await;
                    });
                    // Auto-sync on startup: picks up any changes since the last
                    // flush, or performs a full index if the code graph is empty.
                    let state_auto = Arc::clone(&state_init);
                    tokio::spawn(async move {
                        if !try_start_startup_sync(&state_auto) {
                            return;
                        }
                        // Retry on SQLITE_BUSY: background_db_hydration and this
                        // auto-sync task both call connect_db concurrently.  The
                        // fd-lock serialises the open+bootstrap phase, but the
                        // write transactions from the two resulting handles can
                        // still race (U015-FLK1 intra-process residual).
                        // Ten attempts with 50 ms → 500 ms exponential back-off
                        // give ≈ 3 s of headroom for the hydration writer to
                        // finish its transaction.
                        let should_flush = 'sync: {
                            // 092.003-T: single atomic (workspace, config) read via
                            // the shared daemon seam; skips when either value is absent.
                            let Some((snapshot, ws_config)) =
                                snapshot_daemon_sync_context(&state_auto).await
                            else {
                                break 'sync false;
                            };
                            let ws_path = std::path::PathBuf::from(&snapshot.path);
                            let mut sync_delay = Duration::from_millis(50);
                            let mut synced = false;
                            for attempt in 0..10_u32 {
                                match crate::services::code_graph::sync_workspace(
                                    &ws_path,
                                    &snapshot.data_dir,
                                    &snapshot.branch,
                                    &ws_config.code_graph,
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
                                        synced = true;
                                        break;
                                    }
                                    Err(e) => {
                                        let msg = e.to_string().to_lowercase();
                                        if (msg.contains("locked") || msg.contains("busy"))
                                            && attempt + 1 < 10
                                        {
                                            tokio::time::sleep(sync_delay).await;
                                            sync_delay =
                                                (sync_delay * 2).min(Duration::from_millis(500));
                                            continue;
                                        }
                                        warn!(error = %e, "startup auto-sync failed");
                                        break;
                                    }
                                }
                            }
                            synced
                        };

                        // ── Registry content ingestion ────────────────────────────
                        //
                        // Load the content registry from disk and ingest all active
                        // sources. Runs after the code-graph sync so the DB connection
                        // is warm. Non-fatal: failures are logged and do not prevent
                        // the daemon serving tool calls.
                        //
                        // After ingestion, backfill any content records that lack an
                        // embedding vector — covering both newly ingested files and
                        // records persisted in previous daemon runs.
                        if let Some(snapshot) = state_auto.snapshot_workspace().await {
                            let ws_path = std::path::PathBuf::from(&snapshot.path);
                            match crate::db::connect_db(&snapshot.data_dir, &snapshot.branch).await
                            {
                                Ok(db) => {
                                    let queries = crate::db::queries::CodeGraphQueries::new(db);

                                    // Run registry ingestion.
                                    let registry_path =
                                        ws_path.join(".engram").join("registry.yaml");
                                    match crate::services::registry::load_registry(&registry_path) {
                                        Ok(Some(mut config)) => {
                                            let _ = crate::services::registry::validate_sources(
                                                &mut config,
                                                &ws_path,
                                            );
                                            match crate::services::ingestion::ingest_all_sources(
                                                &config, &ws_path, &queries,
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
                                                Err(e) => warn!(
                                                    error = %e,
                                                    "startup registry ingestion failed"
                                                ),
                                            }
                                        }
                                        Ok(None) => {
                                            debug!("no registry.yaml — skipping content ingestion");
                                        }
                                        Err(e) => {
                                            warn!(error = %e, "startup registry load failed");
                                        }
                                    }

                                    // Backfill embeddings for content records with no vector.
                                    match backfill_with_scan_progress(&state_auto, &queries).await {
                                        Ok(0) => {}
                                        Ok(n) => {
                                            info!(
                                                updated = n,
                                                "startup content embedding backfill complete"
                                            );
                                        }
                                        Err(e) => warn!(
                                            error = %e,
                                            "startup content embedding backfill failed"
                                        ),
                                    }
                                }
                                Err(e) => {
                                    warn!(
                                        error = %e,
                                        "startup ingestion: failed to connect to database"
                                    );
                                }
                            }
                        }

                        // finish_indexing MUST come before flush_state —
                        // flush_state rejects calls while indexing is in progress.
                        finish_indexing_and_drain_pending_sync(&state_auto).await;
                        if should_flush {
                            if let Err(e) =
                                crate::tools::write::flush_state(Arc::clone(&state_auto), None)
                                    .await
                            {
                                warn!(error = %e, "startup auto-flush failed");
                            }
                        }
                    });
                }
                Err(e) => {
                    error!(error = %e, "workspace hydration failed — initiating shutdown");
                    let _ = tx_init.send(true);
                }
            }
        });
    }

    // ── File-change auto-sync loop ────────────────────────────────────────────
    //
    // Receives debounced file events from the workspace watcher (started in
    // daemon::run). Batches events for a 2-second quiet window, then triggers
    // sync_workspace so the code graph stays current without explicit MCP calls.
    // Follows each sync with a flush_state to persist updated graph to disk.
    //
    // NOTE: this is the legacy v1 loop. Reactive, verify-gated markdown reingest
    // (ServiceAction::ReingestContent) is wired into the live v2 loop only
    // (run_with_shutdown_v2). v1 remains ReindexFile-only by design; v1↔v2 parity
    // for markdown reingest is deferred to a separate item (064.004-T / Q2).
    {
        let state_watcher = Arc::clone(&state);
        let ttl_watcher = Arc::clone(&ttl);
        tokio::spawn(async move {
            while let Some(event) = event_rx.recv().await {
                ttl_watcher.reset();
                let mut pending_reindex = matches!(
                    crate::daemon::debounce::adapt_event(&event),
                    crate::daemon::debounce::ServiceAction::ReindexFile { .. }
                );

                // Drain all events within the 2-second debounce window.
                while let Ok(Some(ev)) =
                    tokio::time::timeout(Duration::from_secs(2), event_rx.recv()).await
                {
                    ttl_watcher.reset();
                    if matches!(
                        crate::daemon::debounce::adapt_event(&ev),
                        crate::daemon::debounce::ServiceAction::ReindexFile { .. }
                    ) {
                        pending_reindex = true;
                    }
                }

                if pending_reindex && state_watcher.try_start_indexing() {
                    let should_flush = 'sync: {
                        // 092.003-T: shared daemon (workspace, config) seam.
                        let Some((snapshot, ws_config)) =
                            snapshot_daemon_sync_context(&state_watcher).await
                        else {
                            break 'sync false;
                        };
                        let ws_path = std::path::PathBuf::from(&snapshot.path);
                        match crate::services::code_graph::sync_workspace(
                            &ws_path,
                            &snapshot.data_dir,
                            &snapshot.branch,
                            &ws_config.code_graph,
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
                            Err(e) => {
                                warn!(error = %e, "file-change auto-sync failed");
                                false
                            }
                        }
                    };
                    // finish_indexing MUST come before flush_state.
                    finish_indexing_and_drain_pending_sync(&state_watcher).await;
                    if should_flush {
                        if let Err(e) =
                            crate::tools::write::flush_state(Arc::clone(&state_watcher), None).await
                        {
                            warn!(error = %e, "file-change auto-flush failed");
                        }
                    }
                }
            }
        });
    }

    accept_loop(listener, Arc::clone(&state), ttl, shutdown_tx, shutdown_rx).await;
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

    // ── Hydrate workspace in a background task ────────────────────────────────
    //
    // Spawned before awaiting the file-watcher init timeout so that
    // set_workspace runs concurrently with watcher registration.  On large
    // workspaces the watcher can take up to 5 s; without this reordering the
    // daemon would stay in "starting" for that entire 5 s window, causing the
    // shim's poll_until_ready to time out and report "Daemon failed to reach
    // Ready state" even though startup would have succeeded.
    //
    // Running set_workspace asynchronously also keeps the accept loop responsive
    // so health probes from the shim are answered immediately (with "starting")
    // rather than timing out waiting for CozoDB init + file hydration.
    {
        let state_init = Arc::clone(&state);
        let workspace_str = workspace.to_owned();
        let ttl_init = Arc::clone(&ttl);
        let tx_init = Arc::clone(&shutdown_tx);
        tokio::spawn(async move {
            match crate::tools::lifecycle::set_workspace(Arc::clone(&state_init), workspace_str)
                .await
            {
                Ok(_) => {
                    info!("workspace hydration complete — daemon ready to serve");
                    ttl_init.reset();
                    let ttl_task = Arc::clone(&ttl_init);
                    tokio::spawn(async move {
                        ttl_task.run_until_expired(tx_init).await;
                    });
                    let state_auto = Arc::clone(&state_init);
                    tokio::spawn(async move {
                        if !try_start_startup_sync(&state_auto) {
                            return;
                        }
                        let should_flush = 'sync: {
                            // 092.003-T: shared daemon (workspace, config) seam.
                            let Some((snapshot, ws_config)) =
                                snapshot_daemon_sync_context(&state_auto).await
                            else {
                                break 'sync false;
                            };
                            let ws_path = std::path::PathBuf::from(&snapshot.path);
                            let mut sync_delay = Duration::from_millis(50);
                            let mut synced = false;
                            for attempt in 0..10_u32 {
                                match crate::services::code_graph::sync_workspace(
                                    &ws_path,
                                    &snapshot.data_dir,
                                    &snapshot.branch,
                                    &ws_config.code_graph,
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
                                        synced = true;
                                        break;
                                    }
                                    Err(e) => {
                                        let msg = e.to_string().to_lowercase();
                                        if (msg.contains("locked") || msg.contains("busy"))
                                            && attempt + 1 < 10
                                        {
                                            tokio::time::sleep(sync_delay).await;
                                            sync_delay =
                                                (sync_delay * 2).min(Duration::from_millis(500));
                                            continue;
                                        }
                                        warn!(error = %e, "startup auto-sync failed");
                                        break;
                                    }
                                }
                            }
                            synced
                        };

                        if let Some(snapshot) = state_auto.snapshot_workspace().await {
                            let ws_path = std::path::PathBuf::from(&snapshot.path);
                            match crate::db::connect_db(&snapshot.data_dir, &snapshot.branch).await
                            {
                                Ok(db) => {
                                    let queries = crate::db::queries::CodeGraphQueries::new(db);
                                    let registry_path =
                                        ws_path.join(".engram").join("registry.yaml");
                                    match crate::services::registry::load_registry(&registry_path) {
                                        Ok(Some(mut config)) => {
                                            let _ = crate::services::registry::validate_sources(
                                                &mut config,
                                                &ws_path,
                                            );
                                            match crate::services::ingestion::ingest_all_sources(
                                                &config, &ws_path, &queries,
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
                                                Err(e) => warn!(
                                                    error = %e,
                                                    "startup registry ingestion failed"
                                                ),
                                            }
                                        }
                                        Ok(None) => {
                                            debug!("no registry.yaml — skipping content ingestion");
                                        }
                                        Err(e) => {
                                            warn!(error = %e, "startup registry load failed");
                                        }
                                    }

                                    match backfill_with_scan_progress(&state_auto, &queries).await {
                                        Ok(0) => {}
                                        Ok(n) => {
                                            info!(
                                                updated = n,
                                                "startup content embedding backfill complete"
                                            );
                                        }
                                        Err(e) => warn!(
                                            error = %e,
                                            "startup content embedding backfill failed"
                                        ),
                                    }
                                }
                                Err(e) => {
                                    warn!(
                                        error = %e,
                                        "startup ingestion: failed to connect to database"
                                    );
                                }
                            }
                        }

                        finish_indexing_and_drain_pending_sync(&state_auto).await;
                        if should_flush {
                            if let Err(e) =
                                crate::tools::write::flush_state(Arc::clone(&state_auto), None)
                                    .await
                            {
                                warn!(error = %e, "startup auto-flush failed");
                            }
                        }
                    });
                }
                Err(e) => {
                    error!(error = %e, "workspace hydration failed — initiating shutdown");
                    let _ = tx_init.send(true);
                }
            }
        });
    }

    // ── Start file watcher AFTER IPC bind, using spawn_blocking ──────────────
    //
    // `new_debouncer` + `debouncer.watch(…, RecursiveMode::Recursive)` are
    // synchronous and block until the OS has registered watch handles for every
    // sub-directory.  On a large workspace this can exceed the shim's health-
    // probe timeout.  Using `spawn_blocking` offloads the blocking work to the
    // thread pool so the tokio executor and the already-bound IPC listener remain
    // responsive.  A 5-second timeout lets the daemon continue in degraded mode
    // (no file-change tracking) if the workspace is unusually large.
    //
    // The mpsc channel is created inside the blocking task so that `event_tx` is
    // dropped together with the closure on the timeout/error path.  This prevents
    // the file-change receive loop from waiting indefinitely when the watcher init
    // times out and the blocking thread outlives the timeout window.
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
    // `_watcher_handle` stays in the outer function scope so the watcher remains
    // active for the full lifetime of `run_with_shutdown_v2` and is dropped when
    // the daemon exits, which closes `event_tx` and lets the receive loop end.
    let (_watcher_handle, maybe_event_rx) =
        match tokio::time::timeout(Duration::from_secs(5), watcher_init_handle).await {
            Ok(Ok(Some((handle, rx)))) => (Some(handle), Some(rx)),
            Ok(Ok(None)) => (None, None),
            Ok(Err(join_err)) => {
                error!(
                    error = %join_err,
                    "watcher spawn_blocking panicked; daemon continues degraded"
                );
                (None, None)
            }
            Err(_timeout) => {
                warn!(
                    "file watcher initialisation exceeded 5 s deadline; \
                     daemon continues without file-change tracking"
                );
                (None, None)
            }
        };

    // ── File-change auto-sync loop ────────────────────────────────────────────
    // Only spawned when watcher init succeeded; skipped on degraded-mode paths
    // so the loop does not wait indefinitely for an event_tx that may never send.
    if let Some(mut event_rx) = maybe_event_rx {
        let state_watcher = Arc::clone(&state);
        let ttl_watcher = Arc::clone(&ttl);
        tokio::spawn(async move {
            while let Some(event) = event_rx.recv().await {
                ttl_watcher.reset();
                // Accumulate code-file reindex intent and (verify-gated) markdown
                // reingest intent across the debounce window. `pending_reingest`
                // is deduplicated by path so repeated saves collapse to one gate.
                let mut pending_reingest: std::collections::BTreeSet<std::path::PathBuf> =
                    std::collections::BTreeSet::new();
                let mut pending_reindex = match crate::daemon::debounce::adapt_event(&event) {
                    crate::daemon::debounce::ServiceAction::ReindexFile { .. } => true,
                    crate::daemon::debounce::ServiceAction::ReingestContent { path } => {
                        pending_reingest.insert(path);
                        false
                    }
                    crate::daemon::debounce::ServiceAction::Skip => false,
                };

                while let Ok(Some(ev)) =
                    tokio::time::timeout(Duration::from_secs(2), event_rx.recv()).await
                {
                    ttl_watcher.reset();
                    match crate::daemon::debounce::adapt_event(&ev) {
                        crate::daemon::debounce::ServiceAction::ReindexFile { .. } => {
                            pending_reindex = true;
                        }
                        crate::daemon::debounce::ServiceAction::ReingestContent { path } => {
                            pending_reingest.insert(path);
                        }
                        crate::daemon::debounce::ServiceAction::Skip => {}
                    }
                }

                if (pending_reindex || !pending_reingest.is_empty())
                    && state_watcher.try_start_indexing()
                {
                    // ── Code-file reindex path (unchanged; guarded so it is a
                    //    no-op when only markdown changed) ─────────────────────
                    let should_flush = 'sync: {
                        if !pending_reindex {
                            break 'sync false;
                        }
                        // 092.003-T: shared daemon (workspace, config) seam.
                        let Some((snapshot, ws_config)) =
                            snapshot_daemon_sync_context(&state_watcher).await
                        else {
                            break 'sync false;
                        };
                        let ws_path = std::path::PathBuf::from(&snapshot.path);
                        match crate::services::code_graph::sync_workspace(
                            &ws_path,
                            &snapshot.data_dir,
                            &snapshot.branch,
                            &ws_config.code_graph,
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
                            Err(e) => {
                                warn!(error = %e, "file-change auto-sync failed");
                                false
                            }
                        }
                    };

                    // ── Reactive markdown reingest, verify-gated ──────────────
                    // v2 consumer only: the legacy v1 `run_with_shutdown` loop is
                    // intentionally left ReindexFile-only (v1 parity is a separate
                    // item). Gate/verify/ingest errors log-and-continue inside the
                    // orchestrator, so this never breaks the receive loop.
                    if !pending_reingest.is_empty() {
                        if let Some(snapshot) = state_watcher.snapshot_workspace().await {
                            let ws_path = std::path::PathBuf::from(&snapshot.path);
                            crate::services::reactive_sync::reingest_pending_markdown(
                                &ws_path,
                                &snapshot.data_dir,
                                &snapshot.branch,
                                &pending_reingest,
                            )
                            .await;
                        }
                    }

                    finish_indexing_and_drain_pending_sync(&state_watcher).await;
                    if should_flush {
                        if let Err(e) =
                            crate::tools::write::flush_state(Arc::clone(&state_watcher), None).await
                        {
                            warn!(error = %e, "file-change auto-flush failed");
                        }
                    }
                }
            }
        });
    }

    accept_loop(listener, Arc::clone(&state), ttl, shutdown_tx, shutdown_rx).await;
    crate::services::metrics::shutdown().await?;
    crate::services::dehydration::flush_all_workspaces(&state).await?;
    Ok(())
}

fn try_start_startup_sync(state: &AppState) -> bool {
    if state.try_start_indexing() {
        true
    } else {
        state.set_pending_sync();
        false
    }
}

async fn finish_indexing_and_drain_pending_sync(state: &AppState) {
    state.finish_indexing().await;
    crate::tools::lifecycle::drain_pending_sync(state).await;
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
/// otherwise persist forever, since `finish_indexing` does not touch
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

/// Run the content-embedding backfill while mirroring its progress into
/// [`AppState::scan_progress`].
///
/// The backfill runs after the code-graph scan has already completed, so
/// `scan_status` would otherwise report `running: false` for the entire
/// (potentially long) embedding phase. Relaying [`BackfillProgress`] updates
/// keeps every status surface — `get_workspace_status`, the CLI `index`
/// progress poller, and health — honest about the embedding phase regardless
/// of which path triggered it. Whenever running progress was relayed, a
/// `running: false` snapshot is written on completion — even if nothing was
/// embedded — so status never gets stuck reporting an in-flight backfill.
async fn backfill_with_scan_progress(
    state: &SharedState,
    queries: &crate::db::queries::CodeGraphQueries,
) -> Result<usize, EngramError> {
    let (tx, mut rx) = mpsc::unbounded_channel::<crate::services::ingestion::BackfillProgress>();
    let relay_state = Arc::clone(state);
    let updater = tokio::spawn(async move {
        let mut relayed = false;
        while let Some(p) = rx.recv().await {
            relayed = true;
            relay_state
                .set_scan_progress(Some(backfill_running_progress(p.done, p.total)))
                .await;
        }
        relayed
    });

    let result = crate::services::ingestion::backfill_content_embeddings(queries, Some(&tx)).await;
    drop(tx);
    let relayed_running = updater.await.unwrap_or(false);

    let embedded = *result.as_ref().unwrap_or(&0);
    if let Some(snapshot) =
        backfill_completion_snapshot(relayed_running, embedded, Utc::now().to_rfc3339())
    {
        state.set_scan_progress(Some(snapshot)).await;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::state::AppState;
    #[cfg(unix)]
    use std::path::{Path, PathBuf};

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
            let Some((snap, cfg)) = snapshot_daemon_sync_context(&state).await else {
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

    #[test]
    fn startup_sync_queues_when_indexing_is_already_running() {
        let state = AppState::new(1);
        assert!(state.try_start_indexing(), "should acquire indexing lock");

        assert!(
            !try_start_startup_sync(&state),
            "startup sync must not acquire a second indexing lock"
        );
        assert!(
            state.take_pending_sync(),
            "startup sync must queue a pending sync when hydration already holds the lock"
        );
    }

    #[tokio::test]
    async fn finish_indexing_helper_drains_pending_sync_flag() {
        let state = AppState::new(1);
        assert!(state.try_start_indexing(), "should acquire indexing lock");
        state.set_pending_sync();

        finish_indexing_and_drain_pending_sync(&state).await;

        assert!(
            !state.is_indexing(),
            "finish helper must release the indexing lock"
        );
        assert!(
            !state.take_pending_sync(),
            "finish helper must drain the queued pending sync flag"
        );
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
        // Mirror the relay task used by `backfill_with_scan_progress`: progress
        // sent on the channel must be reflected in `scan_status`.
        let state: SharedState = Arc::new(AppState::new(1));
        let (tx, mut rx) =
            mpsc::unbounded_channel::<crate::services::ingestion::BackfillProgress>();
        let relay_state = Arc::clone(&state);
        let updater = tokio::spawn(async move {
            while let Some(p) = rx.recv().await {
                relay_state
                    .set_scan_progress(Some(backfill_running_progress(p.done, p.total)))
                    .await;
            }
        });

        tx.send(crate::services::ingestion::BackfillProgress {
            done: 256,
            total: 1000,
        })
        .expect("send progress");
        drop(tx);
        updater.await.expect("relay task joins");

        let snapshot = state
            .scan_progress_snapshot()
            .await
            .expect("scan status populated by relay");
        assert!(snapshot.running);
        assert_eq!(snapshot.files_scanned, 256);
        assert_eq!(snapshot.files_total, 1000);
    }
}
