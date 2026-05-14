//! Integration tests — CLI command matrix (Units 3 and 4, 035-S).
//!
//! Exercises the full CLI parity surface by invoking the `engram` binary as a
//! subprocess and asserting exit codes.  A [`DaemonHarness`] pre-spawns the
//! daemon so the CLI finds it running and does not trigger auto-spawn.
//!
//! Scenarios:
//! - `cli_core_lifecycle_bind_status_flush`:
//!   `bind` → `daemon-status` → `workspace-status` → `sync` → `flush` (all exit 0)
//! - `cli_indexed_workflow_sync_and_stats`:
//!   `sync` → `stats` (exit 0)
//! - `cli_workspace_status_fails_for_non_git_directory`:
//!   `workspace-status` on a non-git directory → non-zero exit

#[path = "../helpers/mod.rs"]
mod helpers;

use std::path::Path;
use std::process::Command;
use std::time::Duration;

const READY_TIMEOUT: Duration = Duration::from_secs(20);

/// Run an `engram` CLI subcommand against `workspace` and return the exit code.
///
/// Prepends `--workspace <path> --json` so JSON output is produced and the
/// running daemon for that workspace is targeted.  `ENGRAM_DATA_DIR` is removed
/// to prevent production-database contamination.
fn run_cli(workspace: &Path, subcommand_args: &[&str]) -> i32 {
    let ws = workspace.to_str().expect("workspace path must be UTF-8");
    let mut args: Vec<&str> = vec!["--workspace", ws, "--json"];
    args.extend_from_slice(subcommand_args);

    let output = Command::new(env!("CARGO_BIN_EXE_engram"))
        .args(&args)
        .env_remove("ENGRAM_DATA_DIR")
        .output()
        .expect("engram CLI must execute");
    output.status.code().unwrap_or(-1)
}

/// Unit 3 — core lifecycle CLI subcommands all exit 0 when targeting a live daemon.
///
/// `sync` runs before `flush` to drain any in-progress startup auto-index and
/// prevent the Linux `flush_state`-during-indexing error (7003).
#[tokio::test]
async fn cli_core_lifecycle_bind_status_flush() {
    let harness = helpers::DaemonHarness::spawn(READY_TIMEOUT)
        .await
        .expect("daemon must spawn");

    let ws = harness.workspace.path();

    let exit = run_cli(ws, &["bind"]);
    assert_eq!(exit, 0, "engram bind must exit 0 against a live daemon");

    let exit = run_cli(ws, &["daemon-status"]);
    assert_eq!(
        exit, 0,
        "engram daemon-status must exit 0 against a live daemon"
    );

    let exit = run_cli(ws, &["workspace-status"]);
    assert_eq!(
        exit, 0,
        "engram workspace-status must exit 0 against a live daemon"
    );

    // sync before flush: ensures any startup auto-index completes so flush_state
    // does not race the indexing lock.
    let exit = run_cli(ws, &["sync"]);
    assert_eq!(exit, 0, "engram sync must exit 0 against a live daemon");

    let exit = run_cli(ws, &["flush"]);
    assert_eq!(exit, 0, "engram flush must exit 0 against a live daemon");
}

/// Unit 4 — indexed workflow CLI subcommands (`sync`, `stats`) exit 0 with a live daemon.
#[tokio::test]
async fn cli_indexed_workflow_sync_and_stats() {
    let harness = helpers::DaemonHarness::spawn(READY_TIMEOUT)
        .await
        .expect("daemon must spawn");

    let ws = harness.workspace.path();

    let exit = run_cli(ws, &["sync"]);
    assert_eq!(exit, 0, "engram sync must exit 0 against a live daemon");

    let exit = run_cli(ws, &["stats"]);
    assert_eq!(exit, 0, "engram stats must exit 0 against a live daemon");
}

/// Unit 3 — `workspace-status` against a non-git directory must exit non-zero.
///
/// The daemon refuses to bind to a directory without `.git/HEAD`, so the CLI
/// must return a failure exit code (2 = connection/workspace error).
#[test]
fn cli_workspace_status_fails_for_non_git_directory() {
    let workspace = tempfile::tempdir().expect("tempdir for non-git workspace");
    // Deliberately omit .git/ — this is not a git repository.
    let exit = run_cli(workspace.path(), &["workspace-status"]);
    assert_ne!(
        exit, 0,
        "engram workspace-status must exit non-zero for a non-git directory"
    );
}
