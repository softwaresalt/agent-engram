//! Daemon lifecycle policy seam: start and shutdown lifecycle for the IPC
//! composition root.
//!
//! The composition root ([`crate::daemon::ipc_server`]) owns framing and the
//! accept loop; every lifecycle edge it crosses is delegated here.

use tracing::debug;

use crate::server::state::AppState;

/// Record the daemon lifecycle start edge.
///
/// Called by the composition root once the IPC endpoint is bound and the daemon
/// begins accepting connections.
pub fn on_start(state: &AppState) {
    debug!(
        active_connections = state.active_connections(),
        "daemon lifecycle start"
    );
}

/// Record the daemon lifecycle shutdown edge.
///
/// Called by the composition root after the accept loop has quiesced and before
/// durable shutdown work runs.
pub fn on_shutdown(state: &AppState) {
    debug!(
        uptime_seconds = state.uptime_seconds(),
        "daemon lifecycle shutdown"
    );
}
