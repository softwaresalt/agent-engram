//! Supervisor preflight verdict runner.
//!
//! Drives the seven preflight stages in order and reduces the outcome to a
//! single [`Verdict`] with a stable JSON line and process exit code.

use std::time::Instant;

use crate::preflight::{Failure, Preflight, Verifier};

/// Exit code for usage errors.
pub const EXIT_USAGE: u8 = 2;

/// Final preflight verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// All stages passed.
    Succeeded,
    /// A stage failed.
    Failed(Failure),
}

impl Verdict {
    /// Returns the single-line JSON representation of this verdict.
    #[must_use]
    pub const fn json_line(&self) -> &'static str {
        match self {
            Self::Succeeded => r#"{"state":"Succeeded"}"#,
            Self::Failed(Failure::Build) => r#"{"state":"Failed","stage":"Build"}"#,
            Self::Failed(Failure::Seal) => r#"{"state":"Failed","stage":"Seal"}"#,
            Self::Failed(Failure::Publish) => r#"{"state":"Failed","stage":"Publish"}"#,
            Self::Failed(Failure::DaemonVerified) => {
                r#"{"state":"Failed","stage":"DaemonVerified"}"#
            }
            Self::Failed(Failure::HealthVerified) => {
                r#"{"state":"Failed","stage":"HealthVerified"}"#
            }
            Self::Failed(Failure::CliProbeVerified) => {
                r#"{"state":"Failed","stage":"CliProbeVerified"}"#
            }
            Self::Failed(Failure::McpProbeVerified) => {
                r#"{"state":"Failed","stage":"McpProbeVerified"}"#
            }
        }
    }

    /// Returns the process exit code: 0 on success, 1 on failure.
    #[must_use]
    pub const fn exit_code(&self) -> u8 {
        match self {
            Self::Succeeded => 0,
            Self::Failed(_) => 1,
        }
    }
}

/// Runs all seven preflight stages and returns the resulting verdict.
#[must_use]
pub fn run<V: Verifier>(
    deadline: Instant,
    expected_generation: String,
    verifier: &mut V,
) -> Verdict {
    match run_stages(deadline, expected_generation, verifier) {
        Ok(()) => Verdict::Succeeded,
        Err(failure) => Verdict::Failed(failure),
    }
}

fn run_stages<V: Verifier>(
    deadline: Instant,
    expected_generation: String,
    verifier: &mut V,
) -> Result<(), Failure> {
    let _succeeded = Preflight::start(deadline, expected_generation)
        .verify(verifier)?
        .verify(verifier)?
        .verify(verifier)?
        .verify(verifier)?
        .verify(verifier)?
        .verify(verifier)?
        .verify(verifier)?;
    Ok(())
}

/// Returns the variant name of a failure.
#[must_use]
pub const fn stage_name(failure: Failure) -> &'static str {
    match failure {
        Failure::Build => "Build",
        Failure::Seal => "Seal",
        Failure::Publish => "Publish",
        Failure::DaemonVerified => "DaemonVerified",
        Failure::HealthVerified => "HealthVerified",
        Failure::CliProbeVerified => "CliProbeVerified",
        Failure::McpProbeVerified => "McpProbeVerified",
    }
}
