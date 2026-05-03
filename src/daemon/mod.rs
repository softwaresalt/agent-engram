//! Daemon module: persistent workspace MCP server.
//!
//! The daemon manages workspace state, serves MCP tool calls via an IPC server,
//! monitors the workspace for file changes, and self-terminates after a
//! configurable idle timeout. It is spawned automatically by the shim on first
//! use and runs as a background process.

pub mod debounce;
pub mod ipc_server;
pub mod lockfile;
pub mod protocol;
pub mod ttl;
pub mod watcher;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::watch;
use tracing::{error, info};

use crate::daemon::lockfile::DaemonLock;
use crate::daemon::ttl::TtlTimer;
use crate::daemon::watcher::WatcherConfig;
use crate::db::workspace::load_or_create_workspace_id;
use crate::errors::{EngramError, IpcError as DomainIpcError};

/// Operational status of a running daemon instance.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DaemonStatus {
    /// Daemon is initializing: hydrating workspace state and binding the IPC endpoint.
    Starting,
    /// Daemon is fully initialized and accepting tool calls.
    Ready,
    /// Daemon is tearing down: flushing state and closing open connections.
    ShuttingDown,
}

/// Live state snapshot for a running daemon instance.
#[derive(Debug, Clone)]
pub struct DaemonState {
    /// Absolute, canonicalized path to the workspace root.
    pub workspace_path: PathBuf,
    /// SHA-256 hex hash of the canonical workspace path (full 64 chars).
    pub workspace_hash: String,
    /// OS process ID of this daemon instance.
    pub pid: u32,
    /// IPC endpoint address: Unix socket path or Windows named pipe path.
    pub ipc_address: String,
    /// Wall-clock instant when this daemon process started.
    pub started_at: DateTime<Utc>,
    /// Wall-clock instant of the most recent tool call or connection event.
    pub last_activity: DateTime<Utc>,
    /// Maximum idle duration before the daemon self-terminates.
    pub idle_timeout: Duration,
    /// Current operational status.
    pub status: DaemonStatus,
}

/// Run the daemon for the given workspace path.
///
/// Lifecycle:
/// 1. Canonicalize workspace path and create `.engram/run/`.
/// 2. Acquire the daemon lockfile (stale lock → clean up and re-acquire).
/// 3. Parse idle timeout from `ENGRAM_IDLE_TIMEOUT_MS` env var (0 = forever).
/// 4. Create the TTL timer and the shared shutdown channel.
/// 5. Spawn the SIGTERM / Ctrl-C signal handler.
/// 6. Start the file watcher; wire events to TTL reset.
/// 7. Run the IPC accept loop — the TTL task is started inside this step,
///    after the socket is bound, so the idle window begins from "daemon ready".
/// 8. Perform cleanup: flush workspace state, release lock.
///
/// # Errors
///
/// Returns [`EngramError`] if the workspace path is invalid, the lock cannot
/// be acquired, or the IPC server fails to bind.
pub async fn run(workspace: &str) -> Result<(), EngramError> {
    // ── 1. Resolve workspace path ─────────────────────────────────────────────
    let workspace_path = std::fs::canonicalize(workspace).map_err(|e| {
        EngramError::Ipc(DomainIpcError::ConnectionFailed {
            address: workspace.to_owned(),
            reason: format!("cannot canonicalize workspace path: {e}"),
        })
    })?;

    // ── 1.5. Remove stale PID file if the recorded process is dead ───────────
    // Must run before DaemonLock::acquire so an orphaned PID file from a
    // previously-killed daemon does not block the new one from starting.
    let run_dir = workspace_path.join(".engram").join("run");
    std::fs::create_dir_all(&run_dir).map_err(|e| {
        EngramError::Ipc(DomainIpcError::ConnectionFailed {
            address: run_dir.display().to_string(),
            reason: e.to_string(),
        })
    })?;
    // Non-fatal: internal warn! logs are emitted on read/remove errors.
    let _ = remove_stale_pid_if_dead(&run_dir);

    let _ = load_or_create_workspace_id(&workspace_path)?;

    // ── 2. Acquire lockfile ───────────────────────────────────────────────────
    let _lock = DaemonLock::acquire(&workspace_path)?;
    info!(workspace = %workspace_path.display(), "daemon lock acquired");

    // ── T078: Ensure .engram/logs/ directory exists for structured logging ────
    // The file appender itself is deferred to a future phase to avoid installing
    // a second global tracing subscriber (which would panic). For now we create
    // the directory and record its path so operators know where logs will live.
    let log_dir = workspace_path.join(".engram").join("logs");
    if let Err(e) = std::fs::create_dir_all(&log_dir) {
        tracing::warn!(
            error = %e,
            "failed to create .engram/logs/ directory; file logging unavailable"
        );
    } else {
        info!(
            log_dir = %log_dir.display(),
            "structured log directory ready (file appender: daemon.log)"
        );
    }

    // ── 3a. Load plugin config ────────────────────────────────────────────────
    let plugin_config = crate::models::PluginConfig::load(&workspace_path);

    // ── 3b. Resolve idle timeout (env var overrides config for test harness) ──
    let idle_timeout = std::env::var("ENGRAM_IDLE_TIMEOUT_MS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .map(|ms| {
            if ms == 0 {
                Duration::ZERO
            } else {
                Duration::from_millis(ms)
            }
        })
        .unwrap_or_else(|| plugin_config.idle_timeout());

    info!(
        idle_timeout_ms = idle_timeout.as_millis(),
        "idle TTL configured"
    );

    // ── 4. Create TTL timer and shutdown channel ──────────────────────────────
    let ttl = TtlTimer::new(idle_timeout);
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let shutdown_tx = Arc::new(shutdown_tx);

    // ── 5. (TTL task is started inside run_with_shutdown, after the IPC
    //        socket is bound, so the idle window begins from "daemon ready"
    //        rather than "daemon starting".)

    // ── 6. Spawn signal handler ───────────────────────────────────────────────
    {
        let tx = Arc::clone(&shutdown_tx);
        tokio::spawn(async move {
            if let Err(e) = tokio::signal::ctrl_c().await {
                error!(error = %e, "signal handler failed");
            } else {
                info!("Ctrl-C / SIGTERM received — signalling graceful shutdown");
                let _ = tx.send(true);
            }
        });
    }

    // ── 7. Build watcher config ───────────────────────────────────────────────
    //
    // Watcher initialisation is deferred to run_with_shutdown_v2 which starts it
    // AFTER the IPC listener binds (025.002-T fix). We only build the config here.
    let watcher_config = WatcherConfig {
        debounce_ms: plugin_config.debounce_ms,
        exclude_patterns: plugin_config.exclude_patterns.clone(),
        watch_patterns: plugin_config.watch_patterns.clone(),
    };

    // ── T080: Workspace-moved detection — check every 60s that workspace still valid ──
    // If the workspace directory is moved or deleted while the daemon is running,
    // send the shutdown signal so the daemon exits cleanly (S092).
    {
        let ws_path = workspace_path.clone();
        let tx_moved = Arc::clone(&shutdown_tx);
        tokio::spawn(async move {
            let check_interval = std::time::Duration::from_secs(60);
            loop {
                tokio::time::sleep(check_interval).await;
                if !ws_path.exists() {
                    tracing::warn!(
                        path = %ws_path.display(),
                        "workspace path no longer exists — initiating graceful shutdown"
                    );
                    let _ = tx_moved.send(true);
                    break;
                }
            }
        });
    }

    // ── 8. Run IPC server ─────────────────────────────────────────────────────
    info!("startup step 8: entering IPC accept loop (bind happens inside run_with_shutdown_v2)");
    ipc_server::run_with_shutdown_v2(
        workspace,
        Arc::clone(&ttl),
        Arc::clone(&shutdown_tx),
        shutdown_rx,
        watcher_config,
    )
    .await?;

    info!("daemon exiting cleanly");
    Ok(())
}

/// Remove the stale `engram.pid` file from `run_dir` when its recorded process
/// is no longer alive.
///
/// This runs before [`lockfile::DaemonLock::acquire`] so that a PID file left
/// behind by a previously-killed daemon is cleaned up before the new daemon
/// acquires its lock.
///
/// Returns `Ok(Some(dead_pid))` when the stale file is found and successfully
/// deleted, `Ok(None)` when the file is absent, belongs to a live process, or
/// could not be deleted (deletion failures are logged as warnings).
///
/// # Errors
///
/// Returns [`EngramError`] on unexpected I/O failures reading the PID file.
/// A missing PID file is not an error.
pub(crate) fn remove_stale_pid_if_dead(run_dir: &Path) -> Option<u32> {
    use crate::shim::pidfile::PidFile;

    let pid_path = run_dir.join("engram.pid");
    let raw = match std::fs::read_to_string(&pid_path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return None,
        Err(e) => {
            // Read failure is non-fatal: log and treat the file as absent so
            // startup can continue.  Propagating a misleading FlushFailed
            // (write-side) error here would confuse callers and log consumers.
            tracing::warn!(
                path = %pid_path.display(),
                error = %e,
                "failed to read PID file; treating as absent"
            );
            return None;
        }
    };

    // Corrupt or unknown format: treat as absent (non-fatal).
    let pid_file: PidFile = match serde_json::from_str(raw.trim()) {
        Ok(p) => p,
        Err(_) => {
            // Legacy numeric PID file: a bare integer with no JSON wrapper.
            // start_time_unix of 1 matches the UNKNOWN_START_TIME_UNIX sentinel
            // used by PidFile::read(), which treats it as "no start-time check".
            match raw.trim().parse::<u32>() {
                Ok(pid) => PidFile {
                    pid,
                    start_time_unix: 1,
                },
                Err(_) => return None,
            }
        }
    };

    if pid_file.verify_alive().unwrap_or(false) {
        return None; // live process — leave the file untouched
    }

    // Dead process: remove the stale file.
    match std::fs::remove_file(&pid_path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // Already removed by another process between our read and delete —
            // the file is gone either way; treat as absent.
            return None;
        }
        Err(e) => {
            tracing::warn!(
                path = %pid_path.display(),
                error = %e,
                "failed to remove stale PID file; ignoring"
            );
            return None;
        }
    }

    info!(
        pid = pid_file.pid,
        "removed stale PID file for dead process"
    );
    Some(pid_file.pid)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use serde_json::json;
    use tempfile::TempDir;

    use super::remove_stale_pid_if_dead;

    // ── 025.003-T harness ─────────────────────────────────────────────────────
    //
    // Red phase: all tests below panic with `todo!("025.003-T: …")`.
    // Green phase: implementation replaces the stub and each assertion holds.

    /// No `engram.pid` file → `remove_stale_pid_if_dead` must return `None`
    /// without modifying the directory.
    ///
    /// **Red phase**: panics with `todo!("025.003-T: …")` — test FAILS.
    /// **Green phase**: returns `None` — test PASSES.
    #[test]
    fn remove_stale_pid_noop_when_no_pid_file() {
        let dir = TempDir::new().expect("temp run dir");
        let result = remove_stale_pid_if_dead(dir.path());
        assert!(result.is_none(), "must return None when no PID file exists");
    }

    /// A PID file referencing PID 0 (guaranteed dead on every OS) must be
    /// removed and `Some(0)` returned.
    ///
    /// **Red phase**: panics with `todo!("025.003-T: …")` — test FAILS.
    /// **Green phase**: removes `engram.pid`, returns `Some(0)` — test PASSES.
    #[test]
    fn remove_stale_pid_cleans_dead_process_pid_file() {
        let dir = TempDir::new().expect("temp run dir");
        let run_dir = dir.path();

        // PID 0 is unreachable by sysinfo on all platforms (verify_alive returns false).
        let content = json!({"pid": 0u32, "start_time_unix": 1u64}).to_string();
        fs::write(run_dir.join("engram.pid"), content).expect("write stale PID file");

        let dead_pid = remove_stale_pid_if_dead(run_dir);

        assert_eq!(
            dead_pid,
            Some(0),
            "must return Some(dead_pid) after cleanup"
        );
        assert!(
            !run_dir.join("engram.pid").exists(),
            "engram.pid must be removed from disk after stale cleanup"
        );
    }

    /// A PID file referencing the current live process must be left untouched
    /// and `None` returned.
    ///
    /// **Red phase**: panics with `todo!("025.003-T: …")` — test FAILS.
    /// **Green phase**: detects live process, returns `None`, leaves file — test PASSES.
    #[test]
    fn remove_stale_pid_preserves_live_process_pid_file() {
        let dir = TempDir::new().expect("temp run dir");
        let run_dir = dir.path();

        let live_pid = std::process::id();
        // start_time_unix = 1 (UNKNOWN_START_TIME_UNIX) so verify_alive skips
        // start-time comparison and relies on process existence alone.
        let content = json!({"pid": live_pid, "start_time_unix": 1u64}).to_string();
        fs::write(run_dir.join("engram.pid"), content).expect("write live PID file");

        let result = remove_stale_pid_if_dead(run_dir);

        assert!(result.is_none(), "must return None for a live process");
        assert!(
            run_dir.join("engram.pid").exists(),
            "engram.pid must NOT be removed for a live process"
        );
    }

    /// A legacy numeric-only PID file (bare `u32` string, no JSON) with a dead
    /// PID must be removed and `Some(pid)` returned.
    #[test]
    fn remove_stale_pid_cleans_legacy_numeric_pid_file() {
        let dir = TempDir::new().expect("temp run dir");
        let run_dir = dir.path();

        // Legacy format: plain integer, no JSON wrapper.
        // PID 0 is guaranteed dead on every OS.
        fs::write(run_dir.join("engram.pid"), "0").expect("write legacy numeric PID file");

        let dead_pid = remove_stale_pid_if_dead(run_dir);

        assert_eq!(
            dead_pid,
            Some(0),
            "must return Some(0) after cleaning up a legacy numeric PID file"
        );
        assert!(
            !run_dir.join("engram.pid").exists(),
            "engram.pid must be removed after legacy stale PID cleanup"
        );
    }
}
