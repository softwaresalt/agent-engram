//! Daemon startup activation seam: the initial startup gate and the readiness
//! view published to health probes.
//!
//! This module owns the question "has the daemon finished its initial startup
//! gate, and what readiness does it publish?". The IPC composition root
//! ([`crate::daemon::ipc_server`]) delegates every readiness decision here
//! instead of reading activation state directly.

use crate::server::state::AppState;

/// Outcome of the daemon's initial startup gate.
///
/// The gate is passed once workspace hydration has reached its ready terminal.
/// Until then the daemon is bound and accepting connections but is not yet able
/// to serve real tool calls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupOutcome {
    /// The initial startup gate has not completed; hydration is still running.
    Pending,
    /// The initial startup gate completed; the daemon can serve tool calls.
    Ready,
}

/// Readiness snapshot published by the daemon for health probes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadinessView {
    /// Current outcome of the initial startup gate.
    pub startup: StartupOutcome,
}

impl ReadinessView {
    /// Return `true` when the startup gate has completed.
    #[must_use]
    pub fn is_ready(self) -> bool {
        matches!(self.startup, StartupOutcome::Ready)
    }
}

/// Evaluate the daemon's initial startup gate for `state`.
///
/// The gate reflects exactly the condition the daemon has always used: the
/// retained hydration driver reached its ready terminal.
#[must_use]
pub fn run_initial_gate(state: &AppState) -> StartupOutcome {
    if state.is_hydration_ready() {
        StartupOutcome::Ready
    } else {
        StartupOutcome::Pending
    }
}

/// Build the readiness view published to health probes.
#[must_use]
pub fn readiness(state: &AppState) -> ReadinessView {
    ReadinessView {
        startup: run_initial_gate(state),
    }
}
