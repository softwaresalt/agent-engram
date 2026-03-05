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

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::watch;
use tracing::{error, info};

use crate::daemon::lockfile::DaemonLock;
use crate::daemon::ttl::TtlTimer;
use crate::daemon::watcher::{WatcherConfig, start_watcher};
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

    // ── 2. Acquire lockfile ───────────────────────────────────────────────────
    let _lock = DaemonLock::acquire(&workspace_path)?;
    info!(workspace = %workspace_path.display(), "daemon lock acquired");

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

    // ── 7. Start file watcher and wire TTL reset ──────────────────────────────
    let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel::<WatcherEvent>();

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

    {
        let ttl_for_watcher = Arc::clone(&ttl);
        tokio::spawn(async move {
            while event_rx.recv().await.is_some() {
                // S047: every file event resets the idle timer.
                ttl_for_watcher.reset();
            }
        });
    }

    // ── 8. Run IPC server ─────────────────────────────────────────────────────
    ipc_server::run_with_shutdown(
        workspace,
        Arc::clone(&ttl),
        Arc::clone(&shutdown_tx),
        shutdown_rx,
    )
    .await?;

    info!("daemon exiting cleanly");
    Ok(())
}
