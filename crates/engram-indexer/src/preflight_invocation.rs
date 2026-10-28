//! Supervisor invocation contract and dispatch for the preflight command.
//!
//! Parses the strict `--workspace`, `--timeout-ms` and `--engram` argv, derives
//! the single deadline, and reduces a run to one [`PreflightOutput`].

use std::error::Error;
use std::ffi::OsString;
use std::fmt;
use std::path::PathBuf;
use std::time::Instant;

use crate::preflight::Verifier;

/// A validated supervisor invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
    /// Workspace path, kept exactly as given.
    pub workspace: String,
    /// Total preflight budget in milliseconds.
    pub timeout_ms: u64,
    /// Absolute, canonical path of the engram executable.
    pub engram: PathBuf,
}

/// A malformed supervisor invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsageError {
    /// A required flag is absent.
    MissingFlag,
    /// A flag appears more than once.
    DuplicateFlag,
    /// A flag is not part of the contract.
    UnknownFlag,
    /// A positional argument was supplied.
    UnexpectedPositional,
    /// A flag has no value, or its value is another flag token.
    MissingValue,
    /// `--timeout-ms` is not a valid `u64`.
    InvalidTimeout,
    /// `--workspace` is not valid UTF-8.
    NonUtf8Workspace,
    /// `--workspace` is not an existing directory.
    InvalidWorkspace,
    /// `--engram` is not an absolute existing regular file.
    InvalidEngram,
    /// The timeout does not fit in an `Instant`.
    DeadlineOverflow,
}

impl fmt::Display for UsageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::MissingFlag => "missing required flag",
            Self::DuplicateFlag => "duplicate flag",
            Self::UnknownFlag => "unknown flag",
            Self::UnexpectedPositional => "unexpected positional argument",
            Self::MissingValue => "flag is missing a value",
            Self::InvalidTimeout => "invalid timeout value",
            Self::NonUtf8Workspace => "workspace is not valid UTF-8",
            Self::InvalidWorkspace => "workspace is not an existing directory",
            Self::InvalidEngram => "engram is not an absolute existing file",
            Self::DeadlineOverflow => "timeout overflows the deadline",
        };
        f.write_str(text)
    }
}

impl Error for UsageError {}

/// A failure while preparing the verifier, before any stage runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetupError {
    /// The server layout could not be read from the workspace.
    Layout,
}

impl fmt::Display for SetupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Layout => f.write_str("server layout could not be read"),
        }
    }
}

impl Error for SetupError {}

/// The complete observable result of one supervisor run.
#[derive(Debug, PartialEq, Eq)]
pub struct PreflightOutput {
    /// The verdict JSON line, absent on a usage error.
    pub stdout: Option<&'static str>,
    /// Exactly one structured `engram-preflight ` line.
    pub stderr: String,
    /// The process exit code.
    pub exit_code: u8,
}

/// Parses the supervisor argv strictly.
///
/// # Errors
///
/// Returns a [`UsageError`] for any deviation from the contract.
///
/// # Panics
///
/// Always panics until implemented (RED placeholder).
pub fn parse_invocation(
    _args: impl IntoIterator<Item = OsString>,
) -> Result<Invocation, UsageError> {
    panic!("Worker: 142.070-T")
}

/// Computes the single deadline from `now` plus the timeout.
///
/// # Errors
///
/// Returns [`UsageError::DeadlineOverflow`] when the sum overflows.
///
/// # Panics
///
/// Always panics until implemented (RED placeholder).
pub fn deadline_from(_now: Instant, _timeout_ms: u64) -> Result<Instant, UsageError> {
    panic!("Worker: 142.070-T")
}

/// Parses `args`, builds the verifier with `make`, and runs the preflight.
///
/// # Panics
///
/// Always panics until implemented (RED placeholder).
pub fn main_with<V, F>(_args: impl IntoIterator<Item = OsString>, _make: F) -> PreflightOutput
where
    V: Verifier,
    F: FnOnce(&Invocation, Instant) -> Result<(V, String), SetupError>,
{
    panic!("Worker: 142.070-T")
}
