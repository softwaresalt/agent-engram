//! Daemon module: persistent workspace MCP server.
//!
//! The daemon manages workspace state, serves MCP tool calls via an IPC server,
//! monitors the workspace for file changes, and self-terminates after a
//! configurable idle timeout. It is spawned automatically by the shim on first
//! use and runs as a background process.

pub mod debounce;
pub mod ipc_server;
pub mod lockfile;
pub mod ttl;
pub mod watcher;

use crate::errors::EngramError;

/// Run the daemon for the given workspace path.
///
/// Lifecycle: hydrate state → acquire lock → bind IPC server → enter Ready state
/// → process tool calls and file events → shutdown on TTL expiry or signal.
///
/// # Errors
///
/// Returns [`EngramError`] if the workspace path is invalid, the lock cannot
/// be acquired, or the IPC server fails to bind.
#[allow(clippy::unused_async)]
pub async fn run(_workspace: &str) -> Result<(), EngramError> {
    // TODO(T018): wire IPC server into daemon startup sequence
    // TODO(T031): implement daemon subcommand full lifecycle
    todo!("Phase 2: daemon startup sequence")
}
