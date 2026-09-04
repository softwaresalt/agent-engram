//! Daemon IPC composition root: newline-delimited JSON-RPC over a local socket.
//!
//! This module owns framing and the accept loop only. It listens on the
//! workspace-scoped IPC endpoint (Unix domain socket on Linux/macOS, Windows
//! named pipe on Windows), reads exactly one request line per connection, and
//! writes the response frame. Every decision beyond framing is delegated to a
//! named seam:
//!
//! | Seam | Module |
//! |------|--------|
//! | Startup gate and readiness | [`crate::daemon::startup_activation`] |
//! | Request admission and entry | [`crate::daemon::request_entry`] |
//! | Domain-error to wire conversion | [`crate::daemon::error_transport`] |
//! | Start and shutdown lifecycle | [`crate::daemon::lifecycle_policy`] |
//!
//! # Endpoint naming
//!
//! | Platform | Format |
//! |----------|--------|
//! | Unix     | `{workspace}/.engram/run/engram.sock` |
//! | Windows  | `\\.\pipe\engram-{workspace_key}` |

use std::path::Path;
use std::sync::Arc;

use interprocess::local_socket::{
    ListenerOptions,
    tokio::{Listener, Stream, prelude::*},
};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, watch};
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

use crate::config::StaleStrategy;
use crate::daemon::protocol::IpcResponse;
use crate::daemon::ttl::TtlTimer;
use crate::daemon::watcher::{WatcherConfig, start_watcher};
use crate::daemon::{lifecycle_policy, request_entry, startup_activation};
use crate::db::workspace::daemon_key_for_workspace;
use crate::errors::{ConfigError, EngramError, IpcError as DomainIpcError};
use crate::models::WatcherEvent;
use crate::models::config::{DaemonMode, PluginConfig};
use crate::server::state::{AppState, SharedState};

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
        Ok(_) => request_entry::process_request(&line, &state).await,
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

// ── Daemon entry point ───────────────────────────────────────────────────────

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
/// `mode` is the already-resolved [`DaemonMode`] for this daemon process. It is
/// supplied by the caller — which owns config resolution via
/// [`DaemonMode::resolve`] — and is handed straight to
/// [`AppState::with_mode`]. There is no default applied here.
///
/// # Errors
///
/// Returns [`EngramError`] if path validation, lock acquisition, or listener
/// binding fails.
pub async fn run_with_shutdown(
    workspace: &str,
    mode: DaemonMode,
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

    let state: SharedState = Arc::new(AppState::with_mode(mode, 1, StaleStrategy::Warn, 20, 60));

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
    lifecycle_policy::on_start(&state);

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
    let startup_driver = lifecycle_policy::spawn_daemon_driver(async move {
        tokio::select! {
            () = startup_activation::run_startup_driver(
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
    let watcher_driver = lifecycle_policy::spawn_daemon_driver(async move {
        tokio::select! {
            () = lifecycle_policy::run_watcher_driver(watcher_state, watcher_ttl, event_rx, false) => {}
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
    lifecycle_policy::on_shutdown(&state);
    if let Err(error) = startup_driver.join().await {
        warn!(%error, "startup driver join failed");
    }
    if let Err(error) = watcher_driver.join().await {
        warn!(%error, "legacy watcher driver join failed");
    }
    lifecycle_policy::join_retained_hydration(&state).await?;
    lifecycle_policy::shutdown_services(&state).await
}

/// Run the daemon accept loop for the given workspace path (legacy API).
///
/// Delegates to [`run_with_shutdown`] with a no-op TTL and a one-time
/// Ctrl-C shutdown. New code should call [`run_with_shutdown`] directly.
///
/// This entry point takes no `mode` argument because it has no caller-supplied
/// mode context. Rather than defaulting silently, it resolves the mode from the
/// workspace's own `.engram/config.toml` through [`PluginConfig::load`] and
/// [`DaemonMode::resolve`] — the single shared mode resolver — exactly as
/// [`crate::daemon::run`] does. A present-but-unrecognized `mode` setting is a
/// hard error here, never a fallback to managed.
///
/// # Errors
///
/// Returns [`EngramError`] if path validation, mode resolution, lock
/// acquisition, or listener binding fails.
pub async fn run(workspace: &str) -> Result<(), EngramError> {
    let workspace_path = std::fs::canonicalize(workspace).map_err(|e| {
        EngramError::Ipc(DomainIpcError::ConnectionFailed {
            address: workspace.to_owned(),
            reason: format!("cannot canonicalize workspace path: {e}"),
        })
    })?;
    let mode = resolve_daemon_mode(&workspace_path)?;

    let ttl = TtlTimer::new(std::time::Duration::ZERO); // no auto-shutdown
    let (tx, rx) = watch::channel(false);
    // Legacy callers have no file watcher; pass a channel that is immediately
    // closed so the auto-sync loop in run_with_shutdown exits cleanly.
    let (event_tx, event_rx) = mpsc::unbounded_channel::<WatcherEvent>();
    drop(event_tx);
    run_with_shutdown(workspace, mode, ttl, Arc::new(tx), rx, event_rx).await
}

/// Resolve the daemon mode for `workspace_path` from its persisted config.
///
/// Loads `.engram/config.toml` via [`PluginConfig::load`] and hands the raw
/// `mode` setting to [`DaemonMode::resolve`]. An absent setting resolves to
/// [`DaemonMode::Managed`]; a present-but-unrecognized value is a hard
/// [`ConfigError::InvalidValue`].
///
/// [`PluginConfig::load`] is deliberately lenient for every *other* config
/// field: a TOML parse failure anywhere in the file falls back to
/// [`PluginConfig::default`] so unrelated daemon settings degrade gracefully.
/// Mode selection is a safety boundary rather than a convenience setting, so
/// that same fallback must not silently discard an explicitly configured
/// `mode = "read_server"` merely because some unrelated field in the same
/// file is malformed. This function therefore independently re-parses the
/// raw file first and fails closed on any parse error, before ever consulting
/// the (possibly-defaulted) [`PluginConfig::load`] result.
///
/// # Errors
///
/// Returns [`EngramError::Config`] when `config.toml` exists but fails to
/// parse, or when the configured `mode` value is present but unrecognized.
pub fn resolve_daemon_mode(workspace_path: &Path) -> Result<DaemonMode, EngramError> {
    let config_path = workspace_path.join(".engram").join("config.toml");
    if let Ok(content) = std::fs::read_to_string(&config_path) {
        if let Err(parse_error) = toml::from_str::<PluginConfig>(&content) {
            return Err(EngramError::Config(ConfigError::InvalidValue {
                key: "config.toml".to_owned(),
                reason: format!(
                    "{path} failed to parse: {parse_error}",
                    path = config_path.display()
                ),
            }));
        }
    }

    let plugin_config = PluginConfig::load(workspace_path);
    DaemonMode::resolve(plugin_config.mode.as_deref()).map_err(|e| {
        EngramError::Config(ConfigError::InvalidValue {
            key: "mode".to_owned(),
            reason: e.to_string(),
        })
    })
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
/// `mode` is the already-resolved [`DaemonMode`] for this daemon process,
/// supplied by [`crate::daemon::run`] from `.engram/config.toml` via
/// [`DaemonMode::resolve`]. It is handed straight to [`AppState::with_mode`];
/// no default is applied here.
///
/// # Errors
///
/// Returns [`EngramError`] if path validation, run-directory creation, or
/// listener binding fails.
pub async fn run_with_shutdown_v2(
    workspace: &str,
    mode: DaemonMode,
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

    let state: SharedState = Arc::new(AppState::with_mode(mode, 1, StaleStrategy::Warn, 20, 60));

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
    lifecycle_policy::on_start(&state);

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
    let startup_driver = lifecycle_policy::spawn_daemon_driver(async move {
        tokio::select! {
            () = startup_activation::run_startup_driver(
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
    let watcher_driver = lifecycle_policy::spawn_daemon_driver(async move {
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
                () = lifecycle_policy::run_watcher_driver(watcher_state, watcher_ttl, event_rx, true) => {}
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
    lifecycle_policy::on_shutdown(&state);
    if let Err(error) = startup_driver.join().await {
        warn!(%error, "startup driver join failed");
    }
    if let Err(error) = watcher_driver.join().await {
        warn!(%error, "v2 watcher driver join failed");
    }
    lifecycle_policy::join_retained_hydration(&state).await?;
    lifecycle_policy::shutdown_services(&state).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::state::AppState;
    use serde_json::json;
    #[cfg(unix)]
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;
    use std::time::Duration;

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
        let state = Arc::new(AppState::with_mode(
            DaemonMode::Managed,
            1,
            StaleStrategy::Warn,
            20,
            60,
        ));
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
        let state = Arc::new(AppState::with_mode(
            DaemonMode::Managed,
            1,
            StaleStrategy::Warn,
            20,
            60,
        ));
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
}
