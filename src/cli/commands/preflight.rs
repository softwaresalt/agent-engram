//! Root preflight relay (plan unit NEW-4, task 142.072-T); Step 2 stub.
#![allow(missing_docs)]

use std::path::{Path, PathBuf};

pub const RELAY_GRACE_MS: u64 = 2000;
pub const RELAY_STAGES: [&str; 7] = [""; 7];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[rustfmt::skip]
pub enum RelayCause { Ok, MissingSibling, RelayDeadline, RelayOversize }

impl RelayCause {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        stub()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelayOutcome {
    pub line: &'static str,
    pub exit: i32,
    pub cause: RelayCause,
    pub elapsed_ms: u64,
}

impl RelayOutcome {
    #[must_use]
    pub fn stderr_line(&self) -> String {
        stub()
    }
}

fn stub() -> ! {
    unimplemented!("Worker: 142.072-T")
}

/// Resolves the sibling `engram-indexer` next to `engram_exe`.
///
/// # Errors
///
/// A `Failed`/`Build` outcome when the sibling is absent.
pub fn resolve_supervisor(engram_exe: &Path) -> Result<PathBuf, RelayOutcome> {
    let _ = engram_exe;
    stub()
}

#[must_use]
pub fn run_relay(sup: &Path, workspace: &Path, timeout_ms: u64, engram: &Path) -> RelayOutcome {
    let _ = (sup, workspace, timeout_ms, engram);
    stub()
}

#[must_use]
pub fn normalize_verdict(stdout: &[u8], exit_code: Option<i32>) -> (&'static str, i32) {
    let _ = (stdout, exit_code);
    stub()
}

#[must_use]
pub fn run_preflight(workspace: &Path, timeout_ms: u64) -> i32 {
    let _ = (workspace, timeout_ms);
    stub()
}
