//! Supervisor preflight verdict runner (stub).

use std::time::Instant;

use crate::preflight::{Failure, Verifier};

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
    /// Returns the JSON line for this verdict.
    #[must_use]
    pub fn json_line(&self) -> &'static str {
        panic!("Worker: 142.069-T")
    }

    /// Returns the process exit code for this verdict.
    #[must_use]
    pub fn exit_code(&self) -> u8 {
        panic!("Worker: 142.069-T")
    }
}

/// Runs all preflight stages and returns the verdict.
#[must_use]
pub fn run<V: Verifier>(deadline: Instant, expected_generation: String, verifier: &mut V) -> Verdict {
    let _ = (deadline, expected_generation, verifier);
    panic!("Worker: 142.069-T")
}

/// Returns the variant name of a failure.
#[must_use]
pub const fn stage_name(failure: Failure) -> &'static str {
    let _ = failure;
    panic!("Worker: 142.069-T")
}
