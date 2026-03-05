//! Shim module: lightweight stdio MCP proxy.
//!
//! The shim is the MCP client entry point. It connects to (or spawns) the
//! workspace daemon via IPC, then forwards MCP JSON-RPC from stdin to the
//! daemon and returns the response to stdout before exiting.

pub mod ipc_client;
pub mod lifecycle;
pub mod transport;

use std::time::Duration;

use crate::errors::{EngramError, WorkspaceError};

/// Run the shim: connect to or spawn the daemon, then proxy stdio MCP calls.
///
/// Resolves the workspace from the `ENGRAM_WORKSPACE` environment variable,
/// falling back to the current working directory. Ensures the daemon is
/// running before starting the MCP stdio server.
///
/// # Errors
///
/// Returns [`EngramError`] if the daemon cannot be spawned, the IPC connection
/// fails, or the MCP transport encounters a protocol error.
pub async fn run() -> Result<(), EngramError> {
    let workspace = std::env::var("ENGRAM_WORKSPACE").or_else(|_| {
        std::env::current_dir()
            .map(|p| p.display().to_string())
            .map_err(|e| {
                // current_dir() can fail if the working directory has been deleted
                // or permissions have changed. Return a clear diagnostic.
                EngramError::Workspace(WorkspaceError::NotFound {
                    path: format!("<current directory — {e}>"),
                })
            })
    })?;

    let workspace_path = std::fs::canonicalize(&workspace).map_err(|_| {
        EngramError::Workspace(WorkspaceError::NotFound {
            path: workspace.clone(),
        })
    })?;

    lifecycle::ensure_daemon_running(&workspace_path).await?;

    let endpoint = crate::daemon::ipc_server::ipc_endpoint(&workspace_path)?;

    transport::run_shim(endpoint, Duration::from_secs(60)).await
}
