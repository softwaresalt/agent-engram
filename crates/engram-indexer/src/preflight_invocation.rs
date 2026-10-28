//! Supervisor invocation contract and dispatch for the preflight command.
//!
//! Parses the strict `--workspace`, `--timeout-ms` and `--engram` argv, derives
//! the single deadline, and reduces a run to one [`PreflightOutput`].

use std::error::Error;
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::preflight::{Failure, Verifier};
use crate::preflight_verdict::{EXIT_USAGE, Verdict, run, stage_name};

/// A validated supervisor invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
    /// Workspace path, kept exactly as given.
    pub workspace: String,
    /// Total preflight budget in milliseconds.
    pub timeout_ms: u64,
    /// Absolute, canonical path of the engram executable. On Windows,
    /// canonicalization yields a verbatim `\\?\` path.
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

#[derive(Default)]
struct Parsed {
    workspace: Option<OsString>,
    timeout: Option<OsString>,
    engram: Option<OsString>,
}

fn is_flag_token(value: &OsStr) -> bool {
    value.to_str().is_some_and(|text| text.starts_with("--"))
        || value.as_encoded_bytes().starts_with(b"--")
}

/// Parses the supervisor argv strictly.
///
/// # Errors
///
/// Returns a [`UsageError`] for any deviation from the contract.
pub fn parse_invocation(
    args: impl IntoIterator<Item = OsString>,
) -> Result<Invocation, UsageError> {
    let mut parsed = Parsed::default();
    let mut iter = args.into_iter();
    while let Some(token) = iter.next() {
        let slot = match token.to_str() {
            Some("--workspace") => &mut parsed.workspace,
            Some("--timeout-ms") => &mut parsed.timeout,
            Some("--engram") => &mut parsed.engram,
            _ if is_flag_token(&token) => return Err(UsageError::UnknownFlag),
            _ => return Err(UsageError::UnexpectedPositional),
        };
        if slot.is_some() {
            return Err(UsageError::DuplicateFlag);
        }
        let value = iter.next().ok_or(UsageError::MissingValue)?;
        if is_flag_token(&value) {
            return Err(UsageError::MissingValue);
        }
        *slot = Some(value);
    }
    let workspace = parse_workspace(parsed.workspace.ok_or(UsageError::MissingFlag)?)?;
    let timeout = parsed.timeout.ok_or(UsageError::MissingFlag)?;
    let engram = parsed.engram.ok_or(UsageError::MissingFlag)?;
    let timeout_ms = timeout
        .to_str()
        .and_then(|text| text.parse::<u64>().ok())
        .ok_or(UsageError::InvalidTimeout)?;
    Ok(Invocation {
        workspace,
        timeout_ms,
        engram: parse_engram(engram)?,
    })
}

fn parse_workspace(value: OsString) -> Result<String, UsageError> {
    let workspace = value
        .into_string()
        .map_err(|_| UsageError::NonUtf8Workspace)?;
    if std::path::Path::new(&workspace).is_dir() {
        Ok(workspace)
    } else {
        Err(UsageError::InvalidWorkspace)
    }
}

fn parse_engram(value: OsString) -> Result<PathBuf, UsageError> {
    let path = PathBuf::from(value);
    if !path.is_absolute() {
        return Err(UsageError::InvalidEngram);
    }
    let canonical = path.canonicalize().map_err(|_| UsageError::InvalidEngram)?;
    if canonical.is_file() {
        Ok(canonical)
    } else {
        Err(UsageError::InvalidEngram)
    }
}

/// Largest accepted timeout (`u32::MAX` seconds).
///
/// `Instant` arithmetic does not reliably overflow on all platforms (some represent it as an
/// unbounded `Duration`), so `checked_add` alone would not reject absurd values; this cap does.
const MAX_TIMEOUT_MS: u64 = 1_000 * 4_294_967_295;

/// Computes the single deadline from `now` plus the timeout.
///
/// # Errors
///
/// Returns [`UsageError::DeadlineOverflow`] when the sum overflows.
pub fn deadline_from(now: Instant, timeout_ms: u64) -> Result<Instant, UsageError> {
    if timeout_ms > MAX_TIMEOUT_MS {
        return Err(UsageError::DeadlineOverflow);
    }
    now.checked_add(Duration::from_millis(timeout_ms))
        .ok_or(UsageError::DeadlineOverflow)
}

fn stderr_line(stage: &str, cause: &str, started: Instant) -> String {
    let elapsed_ms = started.elapsed().as_millis();
    format!("engram-preflight stage={stage} cause={cause} elapsed_ms={elapsed_ms}\n")
}

fn usage_output(started: Instant) -> PreflightOutput {
    PreflightOutput {
        stdout: None,
        stderr: stderr_line("none", "usage", started),
        exit_code: EXIT_USAGE,
    }
}

/// Parses `args`, builds the verifier with `make`, and runs the preflight.
///
/// `make` runs only after the deadline is computed, receives that deadline, and must not mutate
/// the filesystem. A usage error (including a deadline overflow) never calls `make`.
///
/// The stderr cause is `deadline` when the verdict is failed and `Instant::now() >= deadline` at
/// the time the run returns. This is a heuristic: a verifier failure that happens to land after
/// the deadline is reported as `deadline`.
pub fn main_with<V, F>(args: impl IntoIterator<Item = OsString>, make: F) -> PreflightOutput
where
    V: Verifier,
    F: FnOnce(&Invocation, Instant) -> Result<(V, String), SetupError>,
{
    let started = Instant::now();
    let Ok(invocation) = parse_invocation(args) else {
        return usage_output(started);
    };
    let Ok(deadline) = deadline_from(started, invocation.timeout_ms) else {
        return usage_output(started);
    };
    let (mut verifier, generation) = match make(&invocation, deadline) {
        Ok(parts) => parts,
        Err(SetupError::Layout) => {
            let verdict = Verdict::Failed(Failure::Build);
            return PreflightOutput {
                stdout: Some(verdict.json_line()),
                stderr: stderr_line(stage_name(Failure::Build), "setup", started),
                exit_code: verdict.exit_code(),
            };
        }
    };
    let verdict = run(deadline, generation, &mut verifier);
    let (stage, cause) = match verdict {
        Verdict::Succeeded => ("none", "ok"),
        Verdict::Failed(failure) if Instant::now() >= deadline => (stage_name(failure), "deadline"),
        Verdict::Failed(failure) => (stage_name(failure), "verifier"),
    };
    PreflightOutput {
        stdout: Some(verdict.json_line()),
        stderr: stderr_line(stage, cause, started),
        exit_code: verdict.exit_code(),
    }
}
