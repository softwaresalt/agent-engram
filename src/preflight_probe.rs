//! CLI provenance probe facade (plan unit PRE-5, task 142.067-T).
//!
//! Placeholder stubs: the real implementation is written by build-feature
//! after the RED harness in `tests/integration/preflight_probe_test.rs`.

use std::path::Path;
use std::time::Duration;

use thiserror::Error;

/// One read the probe issues through the engram CLI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProbeRead {
    /// CLI arguments appended after `--workspace <W> --json`.
    pub cli_args: &'static [&'static str],
    /// MCP tool the CLI read maps to.
    pub mcp_tool: &'static str,
    /// MCP arguments, as a JSON text.
    pub mcp_arguments_json: &'static str,
}

/// Generation-pinned statistics read.
pub const PREFLIGHT_READ: ProbeRead = ProbeRead {
    cli_args: &["stats"],
    mcp_tool: "get_workspace_statistics",
    mcp_arguments_json: "{}",
};

/// Health read.
pub const HEALTH_READ: ProbeRead = ProbeRead {
    cli_args: &["health"],
    mcp_tool: "get_health_report",
    mcp_arguments_json: "{}",
};

/// Parsed probe result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeReport {
    /// `false` when the daemon answered with a typed JSON refusal.
    pub accepted: bool,
    /// Published generation ID, when the read carries provenance.
    pub generation_id: Option<String>,
    /// `code_files=<n>;functions=<n>;classes=<n>;interfaces=<n>;edges=<n>`.
    pub data_fingerprint: Option<String>,
}

/// Typed probe failure; causes are static text only.
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum ProbeError {
    /// The child could not be spawned.
    #[error("probe spawn failed: {0}")]
    Spawn(&'static str),
    /// The deadline elapsed.
    #[error("probe deadline elapsed")]
    Deadline,
    /// The child exited unsuccessfully without a typed refusal.
    #[error("probe child exit: {0}")]
    Exit(&'static str),
    /// Output exceeded the 1 MiB cap.
    #[error("probe output exceeded the cap")]
    Oversize,
    /// Output was unreadable or not a report.
    #[error("probe output malformed: {0}")]
    Malformed(&'static str),
    /// The observed generation differs from the expected one.
    #[error("probe generation mismatch")]
    Mismatch,
}

/// Run one bounded CLI read and parse its first JSON line.
///
/// # Errors
///
/// Returns a [`ProbeError`] per the typed-result precedence.
///
/// # Panics
///
/// Placeholder: panics until 142.067-T is implemented.
pub fn probe_cli_read(
    engram: &Path,
    workspace: &Path,
    read: &ProbeRead,
    deadline: Duration,
) -> Result<ProbeReport, ProbeError> {
    let _ = (engram, workspace, read, deadline);
    unimplemented!("Worker: 142.067-T probe_cli_read")
}

/// Re-probe until the report's generation equals `expected` or the deadline.
///
/// # Errors
///
/// Returns [`ProbeError::Mismatch`] or [`ProbeError::Deadline`] when the
/// expected generation is not observed in time.
///
/// # Panics
///
/// Placeholder: panics until 142.067-T is implemented.
pub fn wait_for_generation(
    engram: &Path,
    workspace: &Path,
    read: &ProbeRead,
    expected: &str,
    deadline: Duration,
) -> Result<(), ProbeError> {
    let _ = (engram, workspace, read, expected, deadline);
    unimplemented!("Worker: 142.067-T wait_for_generation")
}
