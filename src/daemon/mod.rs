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
use crate::daemon::watcher::{WatcherConfig, start_watcher};
use crate::db::workspace::load_or_create_workspace_id;
use crate::errors::{EngramError, IpcError as DomainIpcError};
use crate::models::WatcherEvent;

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

    let run_dir = workspace_path.join(".engram").join("run");
    std::fs::create_dir_all(&run_dir).map_err(|e| {
        EngramError::Ipc(DomainIpcError::ConnectionFailed {
            address: run_dir.display().to_string(),
            reason: e.to_string(),
        })
    })?;
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

    // ── 7. Start file watcher ─────────────────────────────────────────────────
    let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel::<WatcherEvent>();

    let watcher_config = WatcherConfig {
        debounce_ms: plugin_config.debounce_ms,
        exclude_patterns: plugin_config.exclude_patterns.clone(),
        watch_patterns: plugin_config.watch_patterns.clone(),
    };
    let _watcher_handle =
        start_watcher(&workspace_path, watcher_config, event_tx).unwrap_or_else(|e| {
            error!(error = %e, "file watcher failed to start; daemon continues degraded");
            None
        });

    // event_rx is forwarded to run_with_shutdown which wires up TTL resets,
    // debounced auto-sync, and auto-flush using the shared AppState.

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
    ipc_server::run_with_shutdown(
        workspace,
        Arc::clone(&ttl),
        Arc::clone(&shutdown_tx),
        shutdown_rx,
        event_rx,
    )
    .await?;

    info!("daemon exiting cleanly");
    Ok(())
}

/// Check whether `.engram/run/engram.pid` in `run_dir` references a dead process
/// and, if so, remove the stale file before the daemon acquires its lock.
///
/// This runs before [`lockfile::DaemonLock::acquire`] so that a PID file left
/// behind by a cleanly-exited daemon is cleaned up with a visible log message
/// rather than silently overwritten on the next successful lock acquisition.
///
/// Returns `Some(dead_pid)` when a stale file is detected and removed, `None`
/// when the PID file does not exist or the recorded process is still alive.
///
/// # Errors
///
/// Propagates unexpected I/O failures encountered while reading or removing the
/// file.  A missing file is not an error.
#[allow(dead_code)]
pub(crate) fn remove_stale_pid_if_dead(_run_dir: &Path) -> Result<Option<u32>, EngramError> {
    todo!(
        "025.003-T: \
         (1) read run_dir/engram.pid via serde_json; \
         (2) if file missing return Ok(None); \
         (3) parse into PidFile; \
         (4) call PidFile::verify_alive(); if alive return Ok(None); \
         (5) std::fs::remove_file(pid_path) — ignore NotFound; \
         (6) info!(pid, \"removed stale PID file for dead process\"); \
         (7) return Ok(Some(dead_pid)); \
         see docs/exec-plans/2026-05-02-engram-server-reliability-plan.md Unit 2B"
    )
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

    /// No `engram.pid` file → `remove_stale_pid_if_dead` must return `Ok(None)`
    /// without modifying the directory.
    ///
    /// **Red phase**: panics with `todo!("025.003-T: …")` — test FAILS.
    /// **Green phase**: returns `Ok(None)` — test PASSES.
    #[test]
    fn remove_stale_pid_noop_when_no_pid_file() {
        let dir = TempDir::new().expect("temp run dir");
        let result = remove_stale_pid_if_dead(dir.path())
            .expect("absent PID file must not produce an error");
        assert!(result.is_none(), "must return None when no PID file exists");
    }

    /// A PID file referencing PID 0 (guaranteed dead on every OS) must be
    /// removed and `Ok(Some(0))` returned.
    ///
    /// **Red phase**: panics with `todo!("025.003-T: …")` — test FAILS.
    /// **Green phase**: removes `engram.pid`, returns `Ok(Some(0))` — test PASSES.
    #[test]
    fn remove_stale_pid_cleans_dead_process_pid_file() {
        let dir = TempDir::new().expect("temp run dir");
        let run_dir = dir.path();

        // PID 0 is unreachable by sysinfo on all platforms (verify_alive returns false).
        let content = json!({"pid": 0u32, "start_time_unix": 1u64}).to_string();
        fs::write(run_dir.join("engram.pid"), content).expect("write stale PID file");

        let dead_pid = remove_stale_pid_if_dead(run_dir)
            .expect("dead PID cleanup must not error");

        assert_eq!(dead_pid, Some(0), "must return Some(dead_pid) after cleanup");
        assert!(
            !run_dir.join("engram.pid").exists(),
            "engram.pid must be removed from disk after stale cleanup"
        );
    }

    /// A PID file referencing the current live process must be left untouched
    /// and `Ok(None)` returned.
    ///
    /// **Red phase**: panics with `todo!("025.003-T: …")` — test FAILS.
    /// **Green phase**: detects live process, returns `Ok(None)`, leaves file — test PASSES.
    #[test]
    fn remove_stale_pid_preserves_live_process_pid_file() {
        let dir = TempDir::new().expect("temp run dir");
        let run_dir = dir.path();

        let live_pid = std::process::id();
        // start_time_unix = 1 (UNKNOWN_START_TIME_UNIX) so verify_alive skips
        // start-time comparison and relies on process existence alone.
        let content = json!({"pid": live_pid, "start_time_unix": 1u64}).to_string();
        fs::write(run_dir.join("engram.pid"), content).expect("write live PID file");

        let result = remove_stale_pid_if_dead(run_dir)
            .expect("live PID check must not error");

        assert!(result.is_none(), "must return None for a live process");
        assert!(
            run_dir.join("engram.pid").exists(),
            "engram.pid must NOT be removed for a live process"
        );
    }
}
