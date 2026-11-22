//! Shim lifecycle: daemon health-check, spawn, and wait-for-ready logic.
//!
//! Before forwarding the first request the shim checks whether a daemon is
//! already running by sending an `_health` IPC message. If the check fails
//! (no daemon running), the shim spawns a new daemon process via
//! `tokio::process::Command` and waits with exponential backoff until the
//! daemon reports `Ready`.

use std::path::Path;
use std::time::Duration;
use std::{future::Future, process::ExitStatus};

use serde_json::Value;
use tracing::{debug, info, instrument, warn};

use crate::daemon::ipc_server::ipc_endpoint;
use crate::daemon::protocol::{HealthCheckResult, IpcRequest};
use crate::db::workspace::{canonicalize_workspace, normalize_canonical};
use crate::errors::{DaemonError, EngramError, IpcError};
use crate::shim::pidfile::PidFile;
use crate::shim::version::{ENGRAM_PROTOCOL_VERSION, ensure_protocol_compatible};

// ── Backoff constants ─────────────────────────────────────────────────────────

/// Initial delay before the first poll (milliseconds).
const INITIAL_BACKOFF_MS: u64 = 10;
/// Maximum delay cap per backoff step (milliseconds).
const MAX_BACKOFF_MS: u64 = 500;
/// Default total wall-clock budget allowed for the ready-wait loop (milliseconds).
const DEFAULT_READY_TIMEOUT_MS: u64 = 30_000;
/// Brief budget for a concurrent winner to publish readiness or PID metadata.
const WINNER_PUBLICATION_GRACE_MS: u64 = 750;
/// Poll interval used only after this shim's exact child exits.
const WINNER_PUBLICATION_POLL_MS: u64 = 25;
/// Maximum number of daemon respawns per shim invocation.
pub(crate) const MAX_RESPAWN_ATTEMPTS: u8 = 1;
/// Maximum duration for a live-daemon pipe reachability probe.
pub(crate) const PIPE_PROBE_TIMEOUT_MS: u64 = 100;
const SHUTDOWN_WAIT_TIMEOUT_MS: u64 = 2_000;
const SHUTDOWN_POLL_MS: u64 = 100;

/// Parse a ready-timeout value from an optional raw string.
///
/// Returns the parsed `u64` milliseconds if `raw` is `Some` and parses
/// successfully, otherwise falls back to [`DEFAULT_READY_TIMEOUT_MS`].
fn parse_timeout_ms(raw: Option<&str>) -> u64 {
    raw.and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(DEFAULT_READY_TIMEOUT_MS)
}

/// Return the ready-wait timeout in milliseconds.
///
/// Reads `ENGRAM_READY_TIMEOUT_MS` from the environment. Falls back to
/// [`DEFAULT_READY_TIMEOUT_MS`] (10 s) if the variable is absent or cannot
/// be parsed as a `u64`.
fn ready_timeout_ms() -> u64 {
    parse_timeout_ms(std::env::var("ENGRAM_READY_TIMEOUT_MS").ok().as_deref())
}

// ── Health check ─────────────────────────────────────────────────────────────

/// Check whether a daemon is healthy at `endpoint`.
///
/// Sends an `_health` JSON-RPC request with a short timeout. Returns `true`
/// if the response contains `"status": "ready"`, `false` on any error or
/// unexpected payload.
#[instrument(fields(endpoint = %endpoint))]
pub async fn check_health(endpoint: &str) -> bool {
    matches!(probe_health(endpoint).await, HealthOutcome::Ready)
}

// ── Result-preserving health probe ────────────────────────────────────────────

/// Terminal classification kind for a daemon `_health` probe failure that is
/// provably permanent (the daemon replied, but the reply demonstrates
/// protocol or contract incompatibility).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TerminalKind {
    /// Daemon reports a protocol version this shim does not support.
    VersionMismatch { expected: u32, actual: u32 },
    /// Daemon returned JSON-RPC error `-32601` (Method Not Found) for `_health`.
    MethodNotFound,
    /// Daemon response contained no `result` field.
    MissingResult,
    /// Daemon response `result` could not be decoded as `HealthCheckResult`.
    UndecodablePayload,
}

impl TerminalKind {
    /// Fixed, client-safe message for this terminal outcome.
    ///
    /// Used by both the request-triggered single-flight probe
    /// (`transport::ShimHandler::forwarding_endpoint`) and the independent
    /// late-readiness monitor (`spawn_late_readiness_monitor`) as their sole
    /// source of terminal-outcome wording, so the two independent publishers
    /// can never report divergent text for the same classified cause.
    /// Contains no daemon-supplied free-form text, path, or environment
    /// value — only fixed, variable-free strings with `VersionMismatch`'s
    /// own typed `expected`/`actual` fields interpolated.
    #[must_use]
    pub(crate) fn client_message(&self) -> String {
        match self {
            TerminalKind::VersionMismatch { expected, actual } => {
                format!("daemon protocol version {actual} is incompatible (expected {expected})")
            }
            TerminalKind::MethodNotFound => {
                "daemon does not implement the _health method".to_owned()
            }
            TerminalKind::MissingResult => {
                "daemon _health response omitted the result payload".to_owned()
            }
            TerminalKind::UndecodablePayload => {
                "daemon _health result could not be decoded".to_owned()
            }
        }
    }
}

/// Outcome of a single `_health` probe, preserving terminal vs transient
/// classification at construction time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum HealthOutcome {
    /// Daemon is reachable, protocol-compatible, and reports `status: "ready"`.
    Ready,
    /// Probe failed but the failure is transient (transport error, daemon not
    /// ready yet, non-fatal JSON-RPC error codes).
    Transient,
    /// Probe succeeded at the transport layer but revealed a permanent
    /// incompatibility.
    Terminal(TerminalKind),
}

/// Result-preserving health probe that classifies the daemon response into
/// [`HealthOutcome`] without collapsing failure modes.
///
/// # Classification rule (conservative — terminal only on proof)
///
/// * Any `Err` from `ipc_client::send_request` → [`HealthOutcome::Transient`]
/// * JSON-RPC error `-32601` → [`HealthOutcome::Terminal(MethodNotFound)`]
/// * Any other JSON-RPC error code → [`HealthOutcome::Transient`]
/// * Missing `result` → [`HealthOutcome::Terminal(MissingResult)`]
/// * Undecodable `result` → [`HealthOutcome::Terminal(UndecodablePayload)`]
/// * Version mismatch → [`HealthOutcome::Terminal(VersionMismatch{..})`]
/// * Version-compatible, status ≠ "ready" → [`HealthOutcome::Transient`]
#[instrument(fields(endpoint = %endpoint))]
pub(crate) async fn probe_health(endpoint: &str) -> HealthOutcome {
    let request = IpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: Some(Value::Number(serde_json::Number::from(0))),
        method: "_health".to_owned(),
        params: None,
    };

    let response =
        match crate::shim::ipc_client::send_request(endpoint, &request, Duration::from_millis(500))
            .await
        {
            Ok(r) => r,
            Err(e) => {
                debug!(error = %e, "health probe transport error (transient)");
                return HealthOutcome::Transient;
            }
        };

    // JSON-RPC error classification.
    if let Some(error) = response.error {
        if error.code == -32601 {
            warn!(
                endpoint = %endpoint,
                terminal_kind = "MethodNotFound",
                "daemon does not recognize _health method"
            );
            return HealthOutcome::Terminal(TerminalKind::MethodNotFound);
        }
        debug!(
            code = error.code,
            "daemon returned non-fatal JSON-RPC error (transient)"
        );
        return HealthOutcome::Transient;
    }

    // Missing result.
    let Some(result_value) = response.result else {
        warn!(
            endpoint = %endpoint,
            terminal_kind = "MissingResult",
            "daemon omitted _health result payload"
        );
        return HealthOutcome::Terminal(TerminalKind::MissingResult);
    };

    // Undecodable payload.
    let Ok(health) = serde_json::from_value::<HealthCheckResult>(result_value) else {
        warn!(
            endpoint = %endpoint,
            terminal_kind = "UndecodablePayload",
            "daemon _health result cannot be decoded"
        );
        return HealthOutcome::Terminal(TerminalKind::UndecodablePayload);
    };

    // Version compatibility.
    if health.protocol_version != ENGRAM_PROTOCOL_VERSION {
        warn!(
            endpoint = %endpoint,
            terminal_kind = "VersionMismatch",
            expected = ENGRAM_PROTOCOL_VERSION,
            actual = health.protocol_version,
            "daemon protocol version incompatible"
        );
        return HealthOutcome::Terminal(TerminalKind::VersionMismatch {
            expected: ENGRAM_PROTOCOL_VERSION,
            actual: health.protocol_version,
        });
    }

    // Version-compatible but not ready.
    if health.status != "ready" {
        debug!(
            status = %health.status,
            protocol_version = health.protocol_version,
            build_hash = %health.build_hash,
            "daemon not ready yet (transient)"
        );
        return HealthOutcome::Transient;
    }

    debug!(
        ready = true,
        protocol_version = health.protocol_version,
        build_hash = %health.build_hash,
        "health check returned"
    );
    HealthOutcome::Ready
}

async fn fetch_health(endpoint: &str) -> Result<HealthCheckResult, EngramError> {
    let request = IpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: Some(Value::Number(serde_json::Number::from(0))),
        method: "_health".to_owned(),
        params: None,
    };

    let response =
        crate::shim::ipc_client::send_request(endpoint, &request, Duration::from_millis(500))
            .await?;

    if let Some(error) = response.error {
        return Err(EngramError::Ipc(IpcError::ReceiveFailed {
            reason: format!(
                "daemon returned _health error {}: {}",
                error.code, error.message
            ),
        }));
    }

    let health: HealthCheckResult = serde_json::from_value(response.result.ok_or_else(|| {
        EngramError::Ipc(IpcError::ReceiveFailed {
            reason: "daemon omitted _health result payload".to_owned(),
        })
    })?)
    .map_err(|e| {
        EngramError::Ipc(IpcError::ReceiveFailed {
            reason: format!("invalid _health payload: {e}"),
        })
    })?;

    ensure_protocol_compatible(health.protocol_version)?;
    Ok(health)
}

async fn daemon_ready(endpoint: &str) -> Result<bool, EngramError> {
    match fetch_health(endpoint).await {
        Ok(health) => Ok(health.status == "ready"),
        Err(e @ EngramError::Ipc(IpcError::VersionMismatch { .. })) => Err(e),
        Err(e) => {
            debug!(error = %e, "health probe did not find a ready daemon");
            Ok(false)
        }
    }
}

// ── Daemon lifecycle ──────────────────────────────────────────────────────────

/// Whether a lifecycle call started the ready daemon or reused an existing one.
#[derive(Debug)]
pub(crate) enum DaemonOwnership {
    /// The daemon was already running before this lifecycle call.
    Reused,
    /// This lifecycle call spawned or respawned the ready daemon.
    Started {
        /// Exact child handle retained for bounded doctor cleanup.
        child: Box<tokio::process::Child>,
        /// Process identity published by the exact spawned daemon.
        pid_file: PidFile,
    },
}

impl DaemonOwnership {
    #[must_use]
    pub(crate) const fn is_started(&self) -> bool {
        matches!(self, Self::Started { .. })
    }
}

/// Controls whether lifecycle validation may replace an existing daemon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExistingDaemonPolicy {
    /// Preserve the normal shim behavior of respawning incompatible daemons.
    Respawn,
    /// Refuse to replace an existing daemon after a failed health contract.
    Preserve,
}

/// Ensure the daemon is running for `workspace`.
///
/// Steps:
/// 1. Compute the IPC endpoint for the workspace.
/// 2. Perform a health check — if the daemon is already ready, return `Ok(())`.
/// 3. Spawn a new daemon process (detached).
/// 4. Poll `check_health` with exponential backoff until the daemon is ready
///    or the time budget is exhausted.
///
/// # Errors
///
/// Returns [`EngramError::Daemon`] if:
/// - The daemon binary cannot be located or spawned.
/// - The daemon does not become healthy within the configured timeout ms.
#[instrument(fields(workspace = %workspace.display()))]
pub async fn ensure_daemon_running(workspace: &Path) -> Result<(), EngramError> {
    ensure_daemon_running_with_ownership(workspace)
        .await
        .map(|_| ())
}

/// Ensure the daemon is running and report whether this invocation owns it.
#[doc(hidden)]
pub(crate) async fn ensure_daemon_running_with_ownership(
    workspace: &Path,
) -> Result<DaemonOwnership, EngramError> {
    let canonical = normalize_canonical(canonicalize_workspace(&workspace.to_string_lossy())?);
    let endpoint = ipc_endpoint(&canonical)?;

    ensure_daemon_running_inner(
        &canonical,
        endpoint,
        MAX_RESPAWN_ATTEMPTS,
        ExistingDaemonPolicy::Respawn,
    )
    .await
}

/// Ensure the daemon is running without replacing an existing published daemon.
///
/// Doctor smoke may start a daemon only when no live or reachable daemon is
/// already published for the workspace. A failed health contract on an
/// existing daemon is returned to the caller without sending shutdown.
pub(crate) async fn ensure_daemon_running_for_doctor(
    workspace: &Path,
) -> Result<DaemonOwnership, EngramError> {
    let canonical = normalize_canonical(canonicalize_workspace(&workspace.to_string_lossy())?);
    let endpoint = ipc_endpoint(&canonical)?;

    ensure_daemon_running_inner(&canonical, endpoint, 0, ExistingDaemonPolicy::Preserve).await
}

/// Ensure the daemon is running for `workspace`, starting from a specific
/// discovery endpoint.
///
/// This is primarily used by integration harnesses that need to simulate a
/// stale daemon endpoint while still exercising the real respawn path.
///
/// # Errors
///
/// Returns the same errors as [`ensure_daemon_running`].
#[doc(hidden)]
pub async fn ensure_daemon_running_with_endpoint(
    workspace: &Path,
    endpoint: String,
) -> Result<(), EngramError> {
    let canonical = normalize_canonical(canonicalize_workspace(&workspace.to_string_lossy())?);
    ensure_daemon_running_inner(
        &canonical,
        endpoint,
        MAX_RESPAWN_ATTEMPTS,
        ExistingDaemonPolicy::Respawn,
    )
    .await
    .map(|_| ())
}

async fn ensure_daemon_running_inner(
    workspace: &Path,
    endpoint: String,
    respawns_remaining: u8,
    existing_policy: ExistingDaemonPolicy,
) -> Result<DaemonOwnership, EngramError> {
    match daemon_ready(&endpoint).await {
        Ok(true) => {
            info!("daemon already running and healthy");
            return Ok(DaemonOwnership::Reused);
        }
        Ok(false) => {}
        Err(e @ EngramError::Ipc(IpcError::VersionMismatch { .. })) => {
            if existing_policy == ExistingDaemonPolicy::Preserve {
                return Err(e);
            }
            if respawns_remaining == 0 {
                return Err(e);
            }

            let pid_hint = live_daemon_pid(workspace)?;
            info!(
                pid = pid_hint.as_ref().map_or(0, |pid_file| pid_file.pid),
                error = %e,
                "version mismatch detected — respawning daemon"
            );
            return respawn_daemon(workspace, &endpoint, pid_hint).await;
        }
        Err(e) => {
            if respawns_remaining > 0 {
                if let Some(pid_hint) = live_daemon_pid(workspace)? {
                    info!(
                        pid = pid_hint.pid,
                        error = %e,
                        "health probe failed against live daemon — respawning"
                    );
                    return respawn_daemon(workspace, &endpoint, Some(pid_hint)).await;
                }
            }

            debug!(error = %e, "health probe did not find a reusable daemon");
        }
    }

    if let Some(pid_file) = PidFile::read(workspace) {
        if pid_file.verify_alive()? {
            match crate::shim::ipc_client::probe(
                &endpoint,
                Duration::from_millis(PIPE_PROBE_TIMEOUT_MS),
            )
            .await
            {
                Ok(()) => {
                    if existing_policy == ExistingDaemonPolicy::Preserve {
                        info!(
                            pid = pid_file.pid,
                            "waiting for reachable existing daemon to become ready"
                        );
                        return poll_until_ready(workspace, &endpoint, None).await;
                    }
                    info!(
                        pid = pid_file.pid,
                        "reusing reachable daemon from PID metadata"
                    );
                    return poll_until_ready(workspace, &endpoint, None).await;
                }
                Err(e) if existing_policy == ExistingDaemonPolicy::Preserve => {
                    return Err(existing_daemon_unavailable(
                        &endpoint,
                        &format!(
                            "live daemon PID {} has an unreachable IPC endpoint: {e}",
                            pid_file.pid
                        ),
                    ));
                }
                Err(e) if respawns_remaining > 0 => {
                    info!(
                        pid = pid_file.pid,
                        error = %e,
                        "live daemon PID has unreachable pipe — respawning"
                    );
                    return respawn_daemon(workspace, &endpoint, Some(pid_file)).await;
                }
                Err(e) => {
                    debug!(
                        pid = pid_file.pid,
                        error = %e,
                        "pipe probe failed without respawn budget"
                    );
                }
            }
        }
    }

    if existing_policy == ExistingDaemonPolicy::Preserve
        && crate::shim::ipc_client::probe(&endpoint, Duration::from_millis(PIPE_PROBE_TIMEOUT_MS))
            .await
            .is_ok()
    {
        info!("waiting for reachable existing daemon endpoint to become ready");
        return poll_until_ready(workspace, &endpoint, None).await;
    }

    let child = spawn_daemon(workspace)?;
    let endpoint = ipc_endpoint(workspace)?;
    poll_until_ready(workspace, &endpoint, Some(child)).await
}

fn existing_daemon_unavailable(endpoint: &str, reason: &str) -> EngramError {
    EngramError::Ipc(IpcError::ConnectionFailed {
        address: endpoint.to_owned(),
        reason: format!("doctor refused to replace an existing daemon: {reason}"),
    })
}

/// Build a daemon-spawn error for the debug-only capture seam.
#[cfg(any(debug_assertions, test))]
fn test_capture_spawn_error(reason: impl Into<String>) -> EngramError {
    EngramError::Daemon(DaemonError::SpawnFailed {
        reason: reason.into(),
    })
}

#[cfg(any(debug_assertions, test))]
fn validate_test_trace_dir(workspace: &Path, trace_dir: &Path) -> Result<(), EngramError> {
    let expected = workspace.join(".engram");
    if trace_dir != expected {
        return Err(test_capture_spawn_error(format!(
            "test daemon trace directory {} is not the fixed owned directory {}",
            trace_dir.display(),
            expected.display()
        )));
    }
    Ok(())
}

#[cfg(any(debug_assertions, test))]
fn create_new_test_trace_file(path: &Path) -> Result<std::fs::File, EngramError> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => {
            return Err(test_capture_spawn_error(format!(
                "refusing to replace existing test daemon trace {}",
                path.display()
            )));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(test_capture_spawn_error(format!(
                "failed to inspect test daemon trace {}: {error}",
                path.display()
            )));
        }
    }

    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| {
            test_capture_spawn_error(format!(
                "failed to exclusively create test daemon trace {}: {error}",
                path.display()
            ))
        })
}

#[cfg(any(debug_assertions, test))]
fn open_test_autospawn_trace_files(
    workspace: &Path,
) -> Result<(std::fs::File, std::fs::File), EngramError> {
    let canonical_workspace = workspace.canonicalize().map_err(|error| {
        test_capture_spawn_error(format!(
            "failed to canonicalize test daemon workspace {}: {error}",
            workspace.display()
        ))
    })?;
    let requested_trace_dir = canonical_workspace.join(".engram");
    let metadata = std::fs::symlink_metadata(&requested_trace_dir).map_err(|error| {
        test_capture_spawn_error(format!(
            "failed to inspect fixed test daemon trace directory {}: {error}",
            requested_trace_dir.display()
        ))
    })?;
    if metadata.file_type().is_symlink() {
        return Err(test_capture_spawn_error(format!(
            "refusing symlinked test daemon trace directory {}",
            requested_trace_dir.display()
        )));
    }
    let canonical_trace_dir = requested_trace_dir.canonicalize().map_err(|error| {
        test_capture_spawn_error(format!(
            "failed to canonicalize fixed test daemon trace directory {}: {error}",
            requested_trace_dir.display()
        ))
    })?;
    validate_test_trace_dir(&canonical_workspace, &canonical_trace_dir)?;

    let stdout =
        create_new_test_trace_file(&canonical_trace_dir.join("test-autospawn.stdout.log"))?;
    let stderr =
        create_new_test_trace_file(&canonical_trace_dir.join("test-autospawn.stderr.log"))?;
    Ok((stdout, stderr))
}

/// Spawn the daemon as a detached child process for the given workspace.
fn spawn_daemon(workspace: &Path) -> Result<tokio::process::Child, EngramError> {
    let workspace_str = workspace.to_str().ok_or_else(|| {
        EngramError::Daemon(DaemonError::SpawnFailed {
            reason: "workspace path contains non-UTF-8 characters".to_owned(),
        })
    })?;

    let current_exe = daemon_executable()?;

    info!(
        exe = %current_exe.display(),
        workspace = %workspace_str,
        "spawning daemon process"
    );

    // Spawn detached: all stdio handles closed, no process group membership.
    // Clear ENGRAM_DATA_DIR so the daemon computes its own data directory from
    // the workspace path rather than inheriting a developer-level override that
    // may point to a different workspace's data (e.g. the engram project itself
    // when ENGRAM_DATA_DIR is set in the shell).  Passing an absolute data-dir
    // override through the shim causes every spawned daemon — regardless of
    // workspace — to share the same CozoDB, which is incorrect.  Users who need
    // a non-default data location should configure it via the daemon's own
    // environment (service manager unit, wrapper script, etc.).
    let mut command = tokio::process::Command::new(&current_exe);
    command
        .args(["daemon", "--workspace", workspace_str])
        .stdin(std::process::Stdio::null())
        .env_remove("ENGRAM_DATA_DIR");

    #[cfg(debug_assertions)]
    if std::env::var_os("ENGRAM_TEST_CAPTURE_AUTOSPAWN_TRACE").is_some_and(|value| value == "1") {
        let (stdout, stderr) = open_test_autospawn_trace_files(workspace)?;
        command.stdout(stdout).stderr(stderr);
    } else {
        command
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
    }

    #[cfg(not(debug_assertions))]
    command
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());

    command.spawn().map_err(|e| {
        EngramError::Daemon(DaemonError::SpawnFailed {
            reason: format!("failed to spawn daemon: {e}"),
        })
    })
}

async fn respawn_daemon(
    workspace: &Path,
    endpoint: &str,
    pid_hint: Option<PidFile>,
) -> Result<DaemonOwnership, EngramError> {
    request_shutdown(endpoint).await;
    wait_for_daemon_exit(endpoint, pid_hint).await?;
    let child = spawn_daemon(workspace)?;
    let endpoint = ipc_endpoint(workspace)?;
    poll_until_ready(workspace, &endpoint, Some(child)).await
}

fn daemon_executable() -> Result<std::path::PathBuf, EngramError> {
    if let Some(path) = std::env::var_os("CARGO_BIN_EXE_engram") {
        return Ok(path.into());
    }

    let current_exe = std::env::current_exe().map_err(|e| {
        EngramError::Daemon(DaemonError::SpawnFailed {
            reason: format!("cannot locate current executable: {e}"),
        })
    })?;

    if current_exe
        .file_stem()
        .is_some_and(|stem| stem == std::ffi::OsStr::new("engram"))
    {
        return Ok(current_exe);
    }

    let extension = current_exe
        .extension()
        .map(|ext| std::ffi::OsString::from(format!(".{}", ext.to_string_lossy())))
        .unwrap_or_default();
    let binary_name = std::ffi::OsString::from(format!("engram{}", extension.to_string_lossy()));

    let mut candidates = Vec::new();
    if let Some(parent) = current_exe.parent() {
        candidates.push(parent.join(&binary_name));
        if let Some(grandparent) = parent.parent() {
            candidates.push(grandparent.join(&binary_name));
        }
    }

    for candidate in candidates {
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    Ok(current_exe)
}

async fn request_shutdown(endpoint: &str) {
    let request = IpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: Some(Value::Number(serde_json::Number::from(1))),
        method: "_shutdown".to_owned(),
        params: None,
    };

    if let Err(e) =
        crate::shim::ipc_client::send_request(endpoint, &request, Duration::from_secs(2)).await
    {
        debug!(error = %e, "daemon shutdown request failed");
    }
}

async fn wait_for_daemon_exit(
    endpoint: &str,
    pid_hint: Option<PidFile>,
) -> Result<(), EngramError> {
    let deadline = tokio::time::Instant::now() + Duration::from_millis(SHUTDOWN_WAIT_TIMEOUT_MS);

    loop {
        let endpoint_reachable =
            crate::shim::ipc_client::probe(endpoint, Duration::from_millis(PIPE_PROBE_TIMEOUT_MS))
                .await
                .is_ok();
        let pid_alive = if let Some(pid_file) = pid_hint.as_ref() {
            pid_file.verify_alive()?
        } else {
            false
        };

        if !endpoint_reachable && !pid_alive {
            return Ok(());
        }

        if tokio::time::Instant::now() >= deadline {
            return Err(EngramError::Daemon(DaemonError::ShutdownTimeout {
                timeout_ms: SHUTDOWN_WAIT_TIMEOUT_MS,
            }));
        }

        tokio::time::sleep(Duration::from_millis(SHUTDOWN_POLL_MS)).await;
    }
}

fn live_daemon_pid(workspace: &Path) -> Result<Option<PidFile>, EngramError> {
    let Some(pid_file) = PidFile::read(workspace) else {
        return Ok(None);
    };

    if pid_file.verify_alive()? {
        Ok(Some(pid_file))
    } else {
        Ok(None)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConcurrentWinner {
    EndpointReady,
    PidPublished,
}

async fn wait_for_concurrent_winner<Ready, ReadyFuture, LivePid>(
    mut endpoint_ready: Ready,
    mut independently_owned_pid_is_live: LivePid,
    timeout: Duration,
    poll_interval: Duration,
) -> Result<Option<ConcurrentWinner>, EngramError>
where
    Ready: FnMut() -> ReadyFuture,
    ReadyFuture: Future<Output = Result<bool, EngramError>>,
    LivePid: FnMut() -> Result<bool, EngramError>,
{
    let deadline = tokio::time::Instant::now() + timeout;

    loop {
        if endpoint_ready().await? {
            return Ok(Some(ConcurrentWinner::EndpointReady));
        }
        if independently_owned_pid_is_live()? {
            return Ok(Some(ConcurrentWinner::PidPublished));
        }

        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Ok(None);
        }
        tokio::time::sleep(poll_interval.min(remaining)).await;
    }
}

async fn resolve_spawned_child_exit(
    workspace: &Path,
    endpoint: &str,
    spawned_pid: Option<u32>,
    status: ExitStatus,
) -> Result<ConcurrentWinner, EngramError> {
    let winner = wait_for_concurrent_winner(
        || daemon_ready(endpoint),
        || {
            live_daemon_pid(workspace)
                .map(|pid_file| pid_file.is_some_and(|pid_file| Some(pid_file.pid) != spawned_pid))
        },
        Duration::from_millis(WINNER_PUBLICATION_GRACE_MS),
        Duration::from_millis(WINNER_PUBLICATION_POLL_MS),
    )
    .await?;

    winner.ok_or_else(|| {
        EngramError::Daemon(DaemonError::SpawnFailed {
            reason: format!(
                "spawned daemon exited before readiness with status {status}; \
                 no concurrent winner published within {WINNER_PUBLICATION_GRACE_MS} ms"
            ),
        })
    })
}

/// Poll the health endpoint with exponential backoff until the daemon is ready.
///
/// Polls with exponential backoff (capped at [`MAX_BACKOFF_MS`]) until the
/// wall-clock deadline computed from [`ready_timeout_ms()`] is exceeded. One
/// final probe is made after the deadline to handle the race where a
/// concurrent shim spawned the daemon just ahead of us.
async fn poll_until_ready(
    workspace: &Path,
    endpoint: &str,
    mut spawned_child: Option<tokio::process::Child>,
) -> Result<DaemonOwnership, EngramError> {
    let outcome = poll_until_ready_inner(workspace, endpoint, spawned_child.as_mut()).await;

    match outcome {
        Ok(PollOutcome::Started(pid_file)) => {
            let child = spawned_child.ok_or_else(|| {
                EngramError::Daemon(DaemonError::SpawnFailed {
                    reason: "ready ownership omitted the spawned child handle".to_owned(),
                })
            })?;
            Ok(DaemonOwnership::Started {
                child: Box::new(child),
                pid_file,
            })
        }
        Ok(PollOutcome::Reused) => {
            if let Some(child) = spawned_child.as_mut() {
                cleanup_spawned_child(child).await?;
            }
            Ok(DaemonOwnership::Reused)
        }
        Err(error) => {
            if let Some(child) = spawned_child.as_mut() {
                if let Err(cleanup_error) = cleanup_spawned_child(child).await {
                    return Err(combine_cleanup_error(error, cleanup_error));
                }
            }
            Err(error)
        }
    }
}

fn cleanup_error(pid: Option<u32>, reason: impl Into<String>) -> EngramError {
    EngramError::Daemon(DaemonError::CleanupFailed {
        pid,
        reason: reason.into(),
    })
}

pub(crate) fn combine_cleanup_error(
    original_error: EngramError,
    cleanup_failure: EngramError,
) -> EngramError {
    let pid = match &cleanup_failure {
        EngramError::Daemon(DaemonError::CleanupFailed { pid, .. }) => *pid,
        _ => None,
    };
    cleanup_error_with_original(original_error, cleanup_failure, pid)
}

fn cleanup_error_with_original(
    original_error: EngramError,
    cleanup_failure: EngramError,
    pid: Option<u32>,
) -> EngramError {
    cleanup_error(
        pid,
        format!("original failure: {original_error}; cleanup failure: {cleanup_failure}"),
    )
}

async fn cleanup_spawned_child(child: &mut tokio::process::Child) -> Result<(), EngramError> {
    let pid = child.id();
    match child.try_wait() {
        Ok(Some(_)) => return Ok(()),
        Ok(None) => {}
        Err(error) => {
            return Err(cleanup_error(
                pid,
                format!("failed to inspect exact child before kill: {error}"),
            ));
        }
    }

    if let Err(error) = child.start_kill() {
        return Err(cleanup_error(
            pid,
            format!("failed to kill exact child: {error}"),
        ));
    }

    match tokio::time::timeout(
        Duration::from_millis(SHUTDOWN_WAIT_TIMEOUT_MS),
        child.wait(),
    )
    .await
    {
        Ok(Ok(_)) => Ok(()),
        Ok(Err(error)) => Err(cleanup_error(
            pid,
            format!("failed to reap exact child: {error}"),
        )),
        Err(_) => Err(cleanup_error(
            pid,
            format!("timed out reaping exact child after {SHUTDOWN_WAIT_TIMEOUT_MS}ms"),
        )),
    }
}

pub(crate) async fn cleanup_owned_daemon(
    workspace: &Path,
    endpoint: &str,
    pid_file: &PidFile,
    child: &mut tokio::process::Child,
) -> Result<(), EngramError> {
    let pid = child.id();
    let pid_is_still_owned = PidFile::read(workspace).is_some_and(|published| {
        published == *pid_file
            && Some(pid_file.pid) == child.id()
            && pid_file.verify_alive().is_ok_and(|alive| alive)
    });

    if pid_is_still_owned {
        request_shutdown(endpoint).await;
    }

    match tokio::time::timeout(
        Duration::from_millis(SHUTDOWN_WAIT_TIMEOUT_MS),
        child.wait(),
    )
    .await
    {
        Ok(Ok(_)) => Ok(()),
        Ok(Err(error)) => Err(cleanup_error(
            pid,
            format!("failed to wait for exact child after graceful shutdown: {error}"),
        )),
        Err(_) => cleanup_spawned_child(child).await,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PollOutcome {
    Reused,
    Started(PidFile),
}

async fn poll_until_ready_inner(
    workspace: &Path,
    endpoint: &str,
    mut spawned_child: Option<&mut tokio::process::Child>,
) -> Result<PollOutcome, EngramError> {
    let timeout_ms = ready_timeout_ms();
    let deadline = tokio::time::Instant::now() + Duration::from_millis(timeout_ms);
    let mut delay_ms = INITIAL_BACKOFF_MS;
    let mut attempt: u32 = 0;

    loop {
        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        delay_ms = (delay_ms * 2).min(MAX_BACKOFF_MS);
        attempt += 1;

        if daemon_ready(endpoint).await? {
            info!(attempt, "daemon reached ready state");
            let Some(child) = spawned_child.as_deref_mut() else {
                return Ok(PollOutcome::Reused);
            };
            let spawned_pid = child.id();
            let exit_status = child.try_wait().map_err(|error| {
                EngramError::Daemon(DaemonError::SpawnFailed {
                    reason: format!("failed to inspect spawned daemon status: {error}"),
                })
            })?;
            if let Some(status) = exit_status {
                resolve_spawned_child_exit(workspace, endpoint, spawned_pid, status).await?;
                return Ok(PollOutcome::Reused);
            }
            return match live_daemon_pid(workspace)? {
                Some(pid_file) if Some(pid_file.pid) == spawned_pid => {
                    Ok(PollOutcome::Started(pid_file))
                }
                _ => Ok(PollOutcome::Reused),
            };
        }

        if let Some(child) = spawned_child.as_deref_mut() {
            let spawned_pid = child.id();
            let exit_status = child.try_wait().map_err(|error| {
                EngramError::Daemon(DaemonError::SpawnFailed {
                    reason: format!("failed to inspect spawned daemon status: {error}"),
                })
            })?;
            if let Some(status) = exit_status {
                // A concurrent shim can win the workspace lock while our exact
                // child exits. Its ready endpoint or independently-owned PID
                // metadata may be published just after the child exit becomes
                // visible, so allow only a brief grace window for that proof.
                match resolve_spawned_child_exit(workspace, endpoint, spawned_pid, status).await? {
                    ConcurrentWinner::EndpointReady => {
                        info!(
                            %status,
                            "spawned daemon exited after a concurrent daemon became ready"
                        );
                        return Ok(PollOutcome::Reused);
                    }
                    ConcurrentWinner::PidPublished => {
                        info!(
                            %status,
                            "spawned daemon exited after a concurrent daemon published PID metadata"
                        );
                        spawned_child = None;
                    }
                }
            }
        }

        if tokio::time::Instant::now() >= deadline {
            debug!(attempt, "ready-wait deadline exceeded");
            break;
        }
    }

    // Final check: a concurrent shim may have raced and won the spawn.
    if daemon_ready(endpoint).await? {
        info!("daemon ready (concurrent shim won the spawn race)");
        if let Some(child) = spawned_child {
            let still_running = child
                .try_wait()
                .map_err(|error| {
                    EngramError::Daemon(DaemonError::SpawnFailed {
                        reason: format!("failed to inspect spawned daemon status: {error}"),
                    })
                })?
                .is_none();
            if still_running {
                let spawned_pid = child.id();
                return match live_daemon_pid(workspace)? {
                    Some(pid_file) if Some(pid_file.pid) == spawned_pid => {
                        Ok(PollOutcome::Started(pid_file))
                    }
                    _ => Ok(PollOutcome::Reused),
                };
            }
        }
        return Ok(PollOutcome::Reused);
    }

    Err(EngramError::Daemon(DaemonError::NotReady { timeout_ms }))
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    const CLEANUP_CHILD_FIXTURE_ENV: &str = "ENGRAM_LIFECYCLE_CLEANUP_CHILD_FIXTURE";

    fn create_capture_workspace() -> tempfile::TempDir {
        let workspace = tempfile::tempdir().expect("workspace tempdir");
        std::fs::create_dir(workspace.path().join(".git")).expect("create git metadata");
        std::fs::create_dir(workspace.path().join(".engram")).expect("create .engram");
        workspace
    }

    #[tokio::test]
    async fn cleanup_owned_daemon_reaps_exact_started_child_after_failure() {
        if std::env::var_os(CLEANUP_CHILD_FIXTURE_ENV).is_some() {
            tokio::time::sleep(Duration::from_secs(30)).await;
            return;
        }

        let workspace = create_capture_workspace();
        let current_exe = std::env::current_exe().expect("test executable path");
        let mut child = tokio::process::Command::new(current_exe)
            .args([
                "--exact",
                "shim::lifecycle::tests::cleanup_owned_daemon_reaps_exact_started_child_after_failure",
                "--nocapture",
            ])
            .env(CLEANUP_CHILD_FIXTURE_ENV, "1")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("spawn cleanup fixture child");
        let pid = child.id().expect("cleanup fixture child PID");
        let pid_file = PidFile {
            pid,
            start_time_unix: 1,
        };
        pid_file
            .atomic_write(&workspace.path().join(".engram").join("run"))
            .expect("publish cleanup fixture PID");
        let endpoint = ipc_endpoint(workspace.path()).expect("test IPC endpoint");

        cleanup_owned_daemon(workspace.path(), &endpoint, &pid_file, &mut child)
            .await
            .expect("owned child cleanup should succeed");

        assert!(
            !pid_file
                .verify_alive()
                .expect("verify cleanup fixture liveness"),
            "cleanup left owned child {pid} alive"
        );
        assert!(
            child.try_wait().expect("inspect cleanup fixture").is_some(),
            "cleanup killed but did not reap owned child {pid}"
        );
    }

    #[test]
    fn cleanup_failure_combines_original_startup_error() {
        let original = EngramError::Daemon(DaemonError::NotReady { timeout_ms: 750 });
        let cleanup = cleanup_error(Some(42), "timed out reaping exact child after 5000ms");

        let combined = combine_cleanup_error(original, cleanup);

        let EngramError::Daemon(DaemonError::CleanupFailed { pid, reason }) = combined else {
            panic!("combined cleanup failure must retain a structured daemon error");
        };
        assert_eq!(pid, Some(42));
        assert!(reason.contains("original failure"));
        assert!(reason.contains("Daemon failed to reach Ready state"));
        assert!(reason.contains("timed out reaping exact child"));
    }

    /// Default timeout is 30 000 ms when no env var value is provided.
    #[test]
    fn ready_timeout_default_is_30_seconds() {
        assert_eq!(parse_timeout_ms(None), DEFAULT_READY_TIMEOUT_MS);
        assert_eq!(parse_timeout_ms(None), 30_000);
    }

    /// A valid numeric string overrides the default.
    #[test]
    fn ready_timeout_env_var_overrides_default() {
        assert_eq!(parse_timeout_ms(Some("5000")), 5_000);
    }

    /// An invalid (non-numeric) string falls back to the default.
    #[test]
    fn ready_timeout_invalid_env_var_falls_back_to_default() {
        assert_eq!(
            parse_timeout_ms(Some("not_a_number")),
            DEFAULT_READY_TIMEOUT_MS
        );
    }

    #[tokio::test]
    async fn concurrent_winner_publication_grace_observes_delayed_independent_pid() {
        let mut pid_checks = 0_u8;
        let winner = wait_for_concurrent_winner(
            || async { Ok(false) },
            || {
                pid_checks += 1;
                Ok(pid_checks == 3)
            },
            Duration::from_secs(1),
            Duration::from_millis(1),
        )
        .await
        .expect("winner publication helper");

        assert_eq!(winner, Some(ConcurrentWinner::PidPublished));
        assert_eq!(pid_checks, 3);
    }

    #[test]
    fn capture_parent_must_be_exact_canonical_owned_engram() {
        let workspace = std::path::PathBuf::from("owned-workspace");
        assert!(validate_test_trace_dir(&workspace, &workspace.join(".engram")).is_ok());
        assert!(validate_test_trace_dir(&workspace, &workspace.join("redirected")).is_err());
    }

    #[test]
    fn fixed_capture_creation_never_truncates_existing_leaf() {
        let workspace = create_capture_workspace();

        let (mut stdout, stderr) =
            open_test_autospawn_trace_files(workspace.path()).expect("create fixed capture files");
        stdout.write_all(b"sentinel\n").expect("write sentinel");
        drop(stdout);
        drop(stderr);

        assert!(
            open_test_autospawn_trace_files(workspace.path()).is_err(),
            "a concurrent capture creator must not reopen or truncate fixed files"
        );
        assert_eq!(
            std::fs::read(workspace.path().join(".engram/test-autospawn.stdout.log"))
                .expect("read existing capture"),
            b"sentinel\n"
        );
    }

    #[cfg(windows)]
    #[test]
    fn capture_rejects_symlinked_parent_and_leaf() {
        use std::os::windows::fs::{symlink_dir, symlink_file};

        let workspace = tempfile::tempdir().expect("workspace tempdir");
        std::fs::create_dir(workspace.path().join(".git")).expect("create git metadata");
        let outside = tempfile::tempdir().expect("outside tempdir");
        symlink_dir(outside.path(), workspace.path().join(".engram"))
            .expect("create parent symlink");
        assert!(
            open_test_autospawn_trace_files(workspace.path()).is_err(),
            "a symlinked .engram parent must be rejected"
        );

        let workspace = create_capture_workspace();
        let outside_leaf = workspace.path().join("outside.log");
        std::fs::write(&outside_leaf, b"sentinel\n").expect("write outside leaf");
        symlink_file(
            &outside_leaf,
            workspace.path().join(".engram/test-autospawn.stdout.log"),
        )
        .expect("create leaf symlink");
        assert!(
            open_test_autospawn_trace_files(workspace.path()).is_err(),
            "a symlinked fixed leaf must be rejected"
        );
        assert_eq!(
            std::fs::read(outside_leaf).expect("read outside leaf"),
            b"sentinel\n"
        );
    }
}
