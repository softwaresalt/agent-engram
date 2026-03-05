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
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::errors::EngramError;

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
/// Lifecycle: acquire lock → bind IPC server → enter Ready state
/// → process tool calls and file events → shutdown on TTL expiry or signal.
///
/// # Errors
///
/// Returns [`EngramError`] if the workspace path is invalid, the lock cannot
/// be acquired, or the IPC server fails to bind.
pub async fn run(workspace: &str) -> Result<(), EngramError> {
    ipc_server::run(workspace).await
}
