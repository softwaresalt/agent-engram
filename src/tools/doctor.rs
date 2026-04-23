//! Doctor diagnostic tool: health report and smoke-test functionality.
//!
//! Provides [`get_health_report_for_daemon`] (returns a structured
//! [`HealthReport`] covering all eight 029-F failure modes) and
//! [`run_smoke_test`] (runs a full shim→daemon handshake round-trip
//! for the `doctor --smoke` CLI subcommand).

use std::path::Path;

use crate::errors::EngramError;
use crate::models::health::{HealthReport, SmokeResult};
use crate::server::state::AppState;

/// Build a structured health report for the daemon covering all eight
/// 029-F failure modes.
///
/// Returns a [`HealthReport`] with one [`crate::models::health::HealthCheck`]
/// per failure mode: `binary_version`, `pid_liveness`, `workspace_identity`,
/// `pipe_reachability`, `registry_validity`, `offline_scan`, `session_resume`,
/// `telemetry_health`.
pub async fn get_health_report_for_daemon(
    _state: &AppState,
) -> Result<HealthReport, EngramError> {
    todo!("Worker: implement doctor health report covering all 8 failure modes (029.004.002-T)")
}

/// Run a full shim→daemon handshake round-trip for the `doctor --smoke` CLI flag.
///
/// Spawns the daemon if not running, connects as a shim, exchanges the version
/// handshake, calls `set_workspace`, then shuts down. Returns a [`SmokeResult`]
/// indicating pass or fail with latency measurement.
pub async fn run_smoke_test(_workspace: &Path) -> Result<SmokeResult, EngramError> {
    todo!("Worker: implement doctor --smoke full handshake (029.004.003-T)")
}
