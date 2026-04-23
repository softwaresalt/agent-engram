//! Health diagnostic types for the `engram doctor` subsystem.
//!
//! Provides [`HealthReport`], [`HealthCheck`], and [`HealthStatus`] used by
//! `get_daemon_status` and the `doctor` CLI subcommand to surface structured
//! per-check diagnostics for all eight 029-F failure modes.

use serde::{Deserialize, Serialize};

/// Traffic-light severity for a single diagnostic check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    /// Initial state before checks have run.
    #[default]
    Unknown,
    /// Check passed; no action required.
    Green,
    /// Check raised a warning; degraded but functional.
    Yellow,
    /// Check failed; operator action required.
    Red,
}

impl HealthStatus {
    /// Return the canonical snake_case string for this status.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Green => "green",
            Self::Yellow => "yellow",
            Self::Red => "red",
        }
    }
}

/// A single diagnostic check result within a [`HealthReport`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    /// Stable machine-readable check identifier (e.g. `"binary_version"`).
    pub name: String,
    /// Pass/warn/fail status for this check.
    pub status: HealthStatus,
    /// Human-readable description of the check result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Operator-facing remediation hint when `status` is `Yellow` or `Red`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remediation: Option<String>,
}

/// Structured diagnostic report returned by `get_daemon_status` and
/// the `doctor` CLI subcommand.
///
/// Covers all eight 029-F failure modes:
/// `binary_version`, `pid_liveness`, `workspace_identity`, `pipe_reachability`,
/// `registry_validity`, `offline_scan`, `session_resume`, `telemetry_health`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HealthReport {
    /// Aggregate status: worst-case roll-up of all checks.
    pub overall: HealthStatus,
    /// Per-check results; exactly 8 entries when fully populated.
    pub checks: Vec<HealthCheck>,
}

impl HealthReport {
    /// Return the check with the given name, if present.
    pub fn find_check(&self, name: &str) -> Option<&HealthCheck> {
        self.checks.iter().find(|c| c.name == name)
    }
}

/// Result of a `doctor --smoke` full-handshake round-trip.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmokeResult {
    /// Whether the smoke test passed.
    pub passed: bool,
    /// Human-readable description of the outcome.
    pub message: String,
    /// Round-trip latency in milliseconds, if measured.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
}

/// Progress snapshot for a background offline-change scan (029-F WS-6).
///
/// Returned as `scan_status` in [`WorkspaceStatus`] and serialized as `null`
/// until the first scan is queued.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScanProgress {
    /// Whether a scan is currently running.
    pub running: bool,
    /// Number of files scanned so far in the current run.
    pub files_scanned: u64,
    /// Total files to scan (0 when not yet calculated).
    pub files_total: u64,
    /// ISO 8601 timestamp of the last completed scan, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_completed_at: Option<String>,
}
