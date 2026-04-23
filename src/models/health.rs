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
