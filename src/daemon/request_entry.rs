//! Daemon request entry seam: the single admission authority for IPC frames.
//!
//! Every request that reaches daemon dispatch passes through [`admit`] first.
//! The IPC composition root ([`crate::daemon::ipc_server`]) performs framing
//! only — it never decides on its own whether a frame may proceed.

use std::sync::Arc;

use serde_json::{Value, json};
use tracing::{info, warn};

use crate::daemon::protocol::{HealthCheckResult, IpcRequest, IpcResponse};
use crate::daemon::startup_activation::ReadServerStartupGate;
use crate::daemon::{error_transport, startup_activation};
use crate::errors::{ActivationError, EngramError, ReadServerRefusalError};
use crate::server::state::{AppState, ReadRequestContext, SharedState};
use crate::shim::version::{ENGRAM_BUILD_HASH, ENGRAM_PROTOCOL_VERSION};
use crate::tools;
use crate::tools::capabilities::{self, CapabilityClass, InputOwnership};

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

// ── Read-server admission (F20) ──────────────────────────────────────────────

/// Admission decision for a `ReadServer`-mode frame.
///
/// Distinct from [`Admission`] because a read-server admission carries the
/// captured context forward: this is the *sole* site at which a request
/// acquires its [`ReadRequestContext`]. Dispatch (F21) only re-checks that a
/// context was supplied; it never captures one of its own, so there is exactly
/// one place where "which generation does this request read?" is decided.
#[derive(Debug)]
pub enum ReadAdmission {
    /// The frame may proceed, reading through the captured context.
    ///
    /// Held as an `Arc` so the generation stays open for the whole request
    /// even if a later reconciliation swaps the daemon's active generation.
    Admitted(Arc<ReadRequestContext>),
    /// The frame is refused; the boxed response is returned to the client.
    Refused(Box<IpcResponse>),
}

impl ReadAdmission {
    /// The captured context, when the frame was admitted.
    #[must_use]
    pub fn context(&self) -> Option<&Arc<ReadRequestContext>> {
        match self {
            ReadAdmission::Admitted(context) => Some(context),
            ReadAdmission::Refused(_) => None,
        }
    }

    /// Return `true` when the frame was admitted.
    #[must_use]
    pub fn is_admitted(&self) -> bool {
        matches!(self, ReadAdmission::Admitted(_))
    }
}

/// Admit a `ReadServer`-mode frame, authorizing it before any activation work.
///
/// Order matters and is the whole point of F20:
///
/// 1. validate the frame envelope (a malformed frame must not trigger work);
/// 2. refuse outright while the startup gate withholds dispatch;
/// 3. resolve the method's canonical descriptor and refuse unauthorized
///    methods -- unknown methods, non-`Read` methods, methods unavailable in
///    read-server mode, and methods not dispatched through
///    [`tools::dispatch`] (`_health`, `_shutdown`) -- **before** triggering
///    any reconciliation or activation work;
/// 4. trigger reconciliation as a bounded background task rather than
///    awaiting it inline, so an authorized request's admission latency never
///    includes manifest I/O or a generation open;
/// 5. only then capture the context the request will read through.
///
/// `_health` and `_shutdown` are answered by [`process_request`] before
/// dispatch is ever considered and therefore never reach this function in
/// production, but this function must independently refuse them (and any
/// other non-generation-backed method) so it is correct on its own terms, not
/// only correct given today's call sites.
///
/// A failed or still-pending reconciliation is never fatal to an authorized
/// request: the previously activated generation is still open and still
/// correct, so the request is admitted against it rather than refused.
/// Refusing here would turn a transient publisher problem into a read
/// outage.
pub async fn admit_read(gate: &Arc<ReadServerStartupGate>, frame: &Frame) -> ReadAdmission {
    if let Err(response) = frame.validate() {
        return ReadAdmission::Refused(Box::new(response));
    }

    if !gate.readiness().await.admits_dispatch() {
        return ReadAdmission::Refused(Box::new(refusal(
            frame,
            EngramError::Activation(ActivationError::GenerationNotYetActivated {
                generation_id: "<pending initial activation>".to_owned(),
            }),
        )));
    }

    if !is_generation_backed_read(&frame.method) {
        return ReadAdmission::Refused(Box::new(refusal(
            frame,
            EngramError::ReadServerRefusal(ReadServerRefusalError::WriteControlRefused {
                operation: frame.method.clone(),
            }),
        )));
    }

    spawn_background_reconciliation(gate);

    match gate.admitted_context().await {
        Some(context) => ReadAdmission::Admitted(context),
        None => ReadAdmission::Refused(Box::new(refusal(
            frame,
            EngramError::Activation(ActivationError::GenerationNotYetActivated {
                generation_id: "<no active generation>".to_owned(),
            }),
        ))),
    }
}

/// Whether `method` is a `Read` tool dispatched through [`tools::dispatch`]
/// and therefore actually reads through a captured generation.
///
/// This is deliberately narrower than "refuse `Write`/`Control`"
/// ([`tools::enforce_read_server_dispatch`]'s check): `_health` and
/// `_shutdown` are declared `Read`- and `Control`-capability respectively but
/// carry [`InputOwnership::IpcServer`] because [`process_request`] answers
/// them directly and never routes them through [`tools::dispatch`]. Neither
/// one reads a generation, so neither one has any business triggering
/// reconciliation.
fn is_generation_backed_read(method: &str) -> bool {
    capabilities::descriptor(method).is_some_and(|descriptor| {
        descriptor.capability == CapabilityClass::Read
            && descriptor.read_server_available
            && descriptor.input_ownership == InputOwnership::DaemonHandler
    })
}

/// RAII guard releasing the reconciliation in-flight slot on drop.
///
/// Guarantees the slot clears on every exit path of the spawned task,
/// including a panic, matching the codebase's existing pattern of recovering
/// from stuck state rather than leaving a permanent lock.
struct ReconciliationInFlightGuard {
    gate: Arc<ReadServerStartupGate>,
}

impl Drop for ReconciliationInFlightGuard {
    fn drop(&mut self) {
        self.gate.release_reconciliation();
    }
}

/// Trigger durable-manifest reconciliation in the background and return
/// immediately.
///
/// Awaiting [`crate::services::generations::GenerationActivator::maybe_activate_newer`]
/// inline would make an admitted request's latency include manifest I/O and,
/// whenever a newer generation needs activating, whole-inventory digest
/// hashing and a database open. Spawning it instead means this request always
/// captures whatever context is *already* active; a later request observes
/// the swap once the spawned task installs it. Concurrent spawns never
/// duplicate work: `maybe_activate_newer`'s single-flight gate coalesces them,
/// so a spawned task that finds an activation already in flight (or already
/// rejected) returns almost immediately.
///
/// The reconciliation slot on `gate` is claimed before `tokio::spawn` is ever
/// called: if a reconciliation task is already in flight, this function
/// returns without spawning a second one. This bounds the number of
/// concurrently spawned reconciliation tasks to one, regardless of how many
/// admitted reads arrive while an activation is slow.
///
/// Errors are deliberately swallowed into a log line: no caller's correctness
/// depends on this task's outcome, only on the active generation staying
/// open and correct, which a failed reconciliation never changes.
fn spawn_background_reconciliation(gate: &Arc<ReadServerStartupGate>) {
    if !gate.try_claim_reconciliation() {
        return;
    }

    let gate = Arc::clone(gate);
    tokio::spawn(async move {
        let _guard = ReconciliationInFlightGuard {
            gate: Arc::clone(&gate),
        };
        match gate.activator().maybe_activate_newer().await {
            Ok(None) => {}
            Ok(Some(generation)) => {
                let (branch, workspace_id) = gate.identity();
                let context = ReadRequestContext::from_generation(generation, branch, workspace_id);
                gate.install_context(context).await;
            }
            Err(error) => {
                warn!(
                    error = %error,
                    "generation reconciliation failed; continuing to serve the active generation"
                );
            }
        }
    });
}

/// Build the refusal response for a typed availability error.
fn refusal(frame: &Frame, error: EngramError) -> IpcResponse {
    IpcResponse::error(
        frame.id.clone().unwrap_or(Value::Null),
        error_transport::to_ipc_error(error),
    )
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
