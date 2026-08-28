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
            "daemon not ready yet (transient)"
        );
        return HealthOutcome::Transient;
    }

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
    let canonical = normalize_canonical(canonicalize_workspace(&workspace.to_string_lossy())?);
    let endpoint = ipc_endpoint(&canonical)?;

    ensure_daemon_running_inner(&canonical, endpoint, MAX_RESPAWN_ATTEMPTS).await
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
    ensure_daemon_running_inner(&canonical, endpoint, MAX_RESPAWN_ATTEMPTS).await
}

async fn ensure_daemon_running_inner(
    workspace: &Path,
    endpoint: String,
    respawns_remaining: u8,
) -> Result<(), EngramError> {
    match daemon_ready(&endpoint).await {
        Ok(true) => {
            info!("daemon already running and healthy");
            return Ok(());
        }
        Ok(false) => {}
        Err(e @ EngramError::Ipc(IpcError::VersionMismatch { .. })) => {
            if respawns_remaining == 0 {
                return Err(e);
            }

            let pid_hint = live_daemon_pid(workspace)?;
            info!(
                pid = pid_hint.as_ref().map_or(0, |pid_file| pid_file.pid),
                error = %e,
                "version mismatch detected — respawning daemon"
            );
            let next_endpoint = respawn_daemon(workspace, &endpoint, pid_hint).await?;
            return Box::pin(ensure_daemon_running_inner(
                workspace,
                next_endpoint,
                respawns_remaining - 1,
            ))
            .await;
        }
        Err(e) => {
            if respawns_remaining > 0 {
                if let Some(pid_hint) = live_daemon_pid(workspace)? {
                    info!(
                        pid = pid_hint.pid,
                        error = %e,
                        "health probe failed against live daemon — respawning"
                    );
                    let next_endpoint =
                        respawn_daemon(workspace, &endpoint, Some(pid_hint)).await?;
                    return Box::pin(ensure_daemon_running_inner(
                        workspace,
                        next_endpoint,
                        respawns_remaining - 1,
                    ))
                    .await;
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
                    info!(
                        pid = pid_file.pid,
                        "reusing reachable daemon from PID metadata"
                    );
                    return poll_until_ready(workspace, &endpoint, None).await;
                }
                Err(e) if respawns_remaining > 0 => {
                    info!(
                        pid = pid_file.pid,
                        error = %e,
                        "live daemon PID has unreachable pipe — respawning"
                    );
                    let next_endpoint =
                        respawn_daemon(workspace, &endpoint, Some(pid_file)).await?;
                    return Box::pin(ensure_daemon_running_inner(
                        workspace,
                        next_endpoint,
                        respawns_remaining - 1,
                    ))
                    .await;
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

    let mut child = spawn_daemon(workspace)?;
    let endpoint = ipc_endpoint(workspace)?;
    poll_until_ready(workspace, &endpoint, Some(&mut child)).await
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
) -> Result<String, EngramError> {
    request_shutdown(endpoint).await;
    wait_for_daemon_exit(endpoint, pid_hint).await?;
    let mut child = spawn_daemon(workspace)?;
    let endpoint = ipc_endpoint(workspace)?;
    poll_until_ready(workspace, &endpoint, Some(&mut child)).await?;
    Ok(endpoint)
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
    mut spawned_child: Option<&mut tokio::process::Child>,
) -> Result<(), EngramError> {
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
            return Ok(());
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
                        return Ok(());
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
        return Ok(());
    }

    Err(EngramError::Daemon(DaemonError::NotReady { timeout_ms }))
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    fn create_capture_workspace() -> tempfile::TempDir {
        let workspace = tempfile::tempdir().expect("workspace tempdir");
        std::fs::create_dir(workspace.path().join(".git")).expect("create git metadata");
        std::fs::create_dir(workspace.path().join(".engram")).expect("create .engram");
        workspace
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
