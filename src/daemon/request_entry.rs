//! Daemon request entry seam: the single admission authority for IPC frames.
//!
//! Every request that reaches daemon dispatch passes through [`admit`] first.
//! The IPC composition root ([`crate::daemon::ipc_server`]) performs framing
//! only — it never decides on its own whether a frame may proceed.

use std::sync::Arc;

use serde_json::{Value, json};
use tracing::info;

use crate::daemon::protocol::{HealthCheckResult, IpcRequest, IpcResponse};
use crate::daemon::{error_transport, startup_activation};
use crate::server::state::{AppState, SharedState};
use crate::shim::version::{ENGRAM_BUILD_HASH, ENGRAM_PROTOCOL_VERSION};
use crate::tools;

/// A decoded IPC request frame awaiting admission.
pub type Frame = IpcRequest;

/// Admission decision for a decoded [`Frame`].
#[derive(Debug)]
pub enum Admission {
    /// The frame may proceed to dispatch.
    Admitted,
    /// The frame is refused; the boxed response is returned to the client.
    Refused(Box<IpcResponse>),
}

impl Admission {
    /// Return `true` when the frame was admitted.
    #[must_use]
    pub fn is_admitted(&self) -> bool {
        matches!(self, Admission::Admitted)
    }
}

/// Decide whether `frame` may proceed to daemon dispatch.
///
/// Today's admission rule is exactly the protocol-level frame validation the
/// daemon has always applied: a frame with a missing or malformed envelope is
/// refused with the protocol error response, everything else is admitted.
///
/// `state` is part of the seam signature so state-dependent admission (for
/// example read-server refusal) can be added without changing every caller.
#[must_use]
pub fn admit(_state: &AppState, frame: &Frame) -> Admission {
    match frame.validate() {
        Ok(()) => Admission::Admitted,
        Err(response) => Admission::Refused(Box::new(response)),
    }
}

/// Deserialize and dispatch a single raw request line, returning an [`IpcResponse`].
///
/// This is the daemon's only request-entry path: the frame is decoded, passed
/// through [`admit`], and only then dispatched. The IPC composition root
/// ([`crate::daemon::ipc_server`]) never dispatches a frame on its own.
///
/// The returned flag is `true` when the request asked the daemon to shut down;
/// the caller signals shutdown only after the response has been written.
pub async fn process_request(line: &str, state: &SharedState) -> (IpcResponse, bool) {
    let request = match IpcRequest::from_line(line.trim()) {
        Ok(r) => r,
        Err(err_response) => return (err_response, false),
    };

    if let Admission::Refused(err_response) = admit(state, &request) {
        return (*err_response, false);
    }

    // Safe to unwrap: validate() ensures id is Some.
    let id = request.id.clone().unwrap_or(Value::Null);

    match request.method.as_str() {
        "_health" => {
            // Return "starting" while workspace hydration is in progress so
            // the shim keeps polling rather than treating the daemon as healthy
            // before it can serve real tool calls.
            let snapshot = state.snapshot_workspace().await;
            let status = if snapshot.is_some() && startup_activation::readiness(state).is_ready() {
                "ready"
            } else {
                "starting"
            };
            (
                IpcResponse::success(
                    id,
                    json!(HealthCheckResult {
                        status: status.to_owned(),
                        uptime_seconds: state.uptime_seconds(),
                        workspace: snapshot.map(|s| s.path),
                        active_connections: state.active_connections(),
                        protocol_version: ENGRAM_PROTOCOL_VERSION,
                        build_hash: ENGRAM_BUILD_HASH.to_owned(),
                    }),
                ),
                false,
            )
        }
        // T052: `_shutdown` is signalled by the connection handler only after
        // this response has been written and flushed (S022, S037).
        "_shutdown" => {
            info!("daemon received _shutdown IPC request — initiating graceful shutdown");
            (
                IpcResponse::success(
                    id,
                    json!({ "status": "shutting_down", "flush_started": true }),
                ),
                true,
            )
        }
        method => (
            match tools::dispatch(Arc::clone(state), method, request.params).await {
                Ok(result) => IpcResponse::success(id, result),
                Err(e) => IpcResponse::error(id, error_transport::to_ipc_error(e)),
            },
            false,
        ),
    }
}
