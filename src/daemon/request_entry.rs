//! Daemon request entry seam: the single admission authority for IPC frames.
//!
//! Every request that reaches daemon dispatch passes through [`admit`] first.
//! The IPC composition root ([`crate::daemon::ipc_server`]) performs framing
//! only — it never decides on its own whether a frame may proceed.

use crate::daemon::protocol::{IpcRequest, IpcResponse};
use crate::server::state::AppState;

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
