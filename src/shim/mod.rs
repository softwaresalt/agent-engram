//! Shim module: lightweight stdio MCP proxy.
//!
//! The shim is the MCP client entry point. It connects to (or spawns) the
//! workspace daemon via IPC, then forwards MCP JSON-RPC from stdin to the
//! daemon and returns the response to stdout before exiting.

pub mod ipc_client;
pub mod lifecycle;
pub mod transport;

use crate::errors::EngramError;

/// Run the shim: connect to or spawn the daemon, then proxy stdio MCP calls.
///
/// # Errors
///
/// Returns [`EngramError`] if the daemon cannot be spawned, the IPC connection
/// fails, or the MCP transport encounters a protocol error.
#[allow(clippy::unused_async)]
pub async fn run() -> Result<(), EngramError> {
    // TODO(T029): implement rmcp StdioTransport + ServerHandler
    // TODO(T027): delegate to lifecycle::ensure_daemon_running
    todo!("Phase 3: shim stdio transport")
}
