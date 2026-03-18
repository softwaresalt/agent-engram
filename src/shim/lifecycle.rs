//! Shim lifecycle: daemon health-check, spawn, and wait-for-ready logic.
//!
//! Before forwarding the first request the shim checks whether a daemon is
//! already running by sending an `_health` IPC message. If the check fails
//! (no daemon running), the shim spawns a new daemon process via
//! `tokio::process::Command` and waits with exponential backoff until the
//! daemon reports `Ready`.

use std::path::Path;
use std::time::Duration;

use serde_json::Value;
use tracing::{debug, info, instrument};

use crate::daemon::ipc_server::ipc_endpoint;
use crate::daemon::protocol::IpcRequest;
use crate::errors::{DaemonError, EngramError};

// ── Backoff constants ─────────────────────────────────────────────────────────

/// Maximum number of health-check poll attempts after spawning the daemon.
const MAX_BACKOFF_ATTEMPTS: u32 = 30;
/// Initial delay before the first poll (milliseconds).
const INITIAL_BACKOFF_MS: u64 = 10;
/// Maximum delay cap per backoff step (milliseconds).
const MAX_BACKOFF_MS: u64 = 500;
/// Default total wall-clock budget allowed for the ready-wait loop (milliseconds).
const DEFAULT_READY_TIMEOUT_MS: u64 = 30_000;

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
    let request = IpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: Some(Value::Number(serde_json::Number::from(0))),
        method: "_health".to_owned(),
        params: None,
    };

    match crate::shim::ipc_client::send_request(endpoint, &request, Duration::from_millis(500))
        .await
    {
        Ok(response) => {
            let is_ready = response
                .result
                .as_ref()
                .and_then(|v| v.get("status"))
                .and_then(|s| s.as_str())
                == Some("ready");
            debug!(ready = is_ready, "health check returned");
            is_ready
        }
        Err(e) => {
            debug!(error = %e, "health check failed");
            false
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
    let endpoint = ipc_endpoint(workspace)?;

    if check_health(&endpoint).await {
        info!("daemon already running and healthy");
        return Ok(());
    }

    spawn_daemon(workspace)?;

    poll_until_ready(&endpoint).await
}

/// Spawn the daemon as a detached child process for the given workspace.
fn spawn_daemon(workspace: &Path) -> Result<(), EngramError> {
    let workspace_str = workspace.to_str().ok_or_else(|| {
        EngramError::Daemon(DaemonError::SpawnFailed {
            reason: "workspace path contains non-UTF-8 characters".to_owned(),
        })
    })?;

    let current_exe = std::env::current_exe().map_err(|e| {
        EngramError::Daemon(DaemonError::SpawnFailed {
            reason: format!("cannot locate current executable: {e}"),
        })
    })?;

    info!(
        exe = %current_exe.display(),
        workspace = %workspace_str,
        "spawning daemon process"
    );

    // Spawn detached: all stdio handles closed, no process group membership.
    tokio::process::Command::new(&current_exe)
        .args(["daemon", "--workspace", workspace_str])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| {
            EngramError::Daemon(DaemonError::SpawnFailed {
                reason: format!("failed to spawn daemon: {e}"),
            })
        })?;

    Ok(())
}

/// Poll the health endpoint with exponential backoff until the daemon is ready.
///
/// If the daemon does not become healthy within the budget (wall-clock
/// [`ready_timeout_ms()`] ms or [`MAX_BACKOFF_ATTEMPTS`] polls), one final
/// check is made to handle the race where a concurrent shim spawned the
/// daemon just ahead of us.
async fn poll_until_ready(endpoint: &str) -> Result<(), EngramError> {
    let timeout_ms = ready_timeout_ms();
    let deadline = tokio::time::Instant::now() + Duration::from_millis(timeout_ms);
    let mut delay_ms = INITIAL_BACKOFF_MS;

    for attempt in 0..MAX_BACKOFF_ATTEMPTS {
        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        delay_ms = (delay_ms * 2).min(MAX_BACKOFF_MS);

        if check_health(endpoint).await {
            info!(attempt, "daemon reached ready state");
            return Ok(());
        }

        if tokio::time::Instant::now() >= deadline {
            debug!(attempt, "ready-wait deadline exceeded");
            break;
        }
    }

    // Final check: a concurrent shim may have raced and won the spawn.
    if check_health(endpoint).await {
        info!("daemon ready (concurrent shim won the spawn race)");
        return Ok(());
    }

    Err(EngramError::Daemon(DaemonError::NotReady { timeout_ms }))
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

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
}
