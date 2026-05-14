//! Integration tests — named regression workflow tests (Unit 5, 035-S).
//!
//! Each test encodes a previously observed failure mode and asserts the fix
//! still holds, acting as a canary for future regressions.
//!
//! Regressions covered:
//! - **Watcher/startup ordering** — file watcher must not block IPC bind
//!   (source: `docs/compound/bugs/daemon-startup-hang-watcher-blocks-before-ipc-bind-2026-05-02.md`).
//! - **`ENGRAM_DATA_DIR` inheritance** — daemon subprocess must use workspace-local
//!   storage, not an ambient env var
//!   (source: `docs/compound/test-failures/engram-data-dir-inherited-by-test-daemon-spawns-2026-05-08.md`).
//! - **Stale-lock recovery** — daemon must restart cleanly after a crash leaves
//!   stale runtime state
//!   (source: `docs/compound/concurrency-issues/cozodb-sqlite-lock-panic-2026-05-01.md`).

#[path = "../helpers/mod.rs"]
mod helpers;

use std::fs;
use std::time::Duration;

use engram::shim::lifecycle::check_health;

const READY_TIMEOUT: Duration = Duration::from_secs(20);

/// Regression: file watcher initialization must not block IPC bind.
///
/// Prior to the fix, the `notify` watcher was initialized synchronously before
/// the IPC server bound its socket, causing the daemon to hang during startup
/// when source files were present.  The fix defers watcher spawn to after the
/// IPC socket is bound.
///
/// This test pre-populates the workspace with Rust source files to ensure the
/// watcher is engaged during startup, then asserts the daemon reaches Ready.
#[tokio::test]
async fn regression_watcher_startup_ordering() {
    let workspace = tempfile::tempdir().expect("tempdir");
    let workspace_path = workspace.path().canonicalize().expect("canonicalize");

    let git_dir = workspace_path.join(".git");
    fs::create_dir_all(&git_dir).expect("create .git");
    fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").expect("write HEAD");

    // Populate the workspace with source files to engage the file watcher.
    let src_dir = workspace_path.join("src");
    fs::create_dir_all(&src_dir).expect("create src dir");
    for i in 0..10_u8 {
        fs::write(
            src_dir.join(format!("module_{i}.rs")),
            format!("//! Module {i}\n\npub fn func_{i}() {{}}\n"),
        )
        .expect("write source file");
    }

    // Daemon must reach IPC-Ready even with watcher-triggering files present.
    let harness = helpers::DaemonHarness::spawn_for_workspace(&workspace_path, READY_TIMEOUT)
        .await
        .expect("daemon must reach Ready; watcher initialization must not block IPC bind");

    let endpoint = harness.ipc_path().to_str().expect("UTF-8").to_owned();
    assert!(
        check_health(&endpoint).await,
        "daemon must remain healthy after startup with watcher-observed files"
    );
}

/// Regression: daemon subprocess must not inherit ambient `ENGRAM_DATA_DIR`.
///
/// `DaemonHarness` removes `ENGRAM_DATA_DIR` from the subprocess environment
/// (U016-HARNESS-2 fix).  Without that removal, the daemon opens the developer's
/// production `CozoDB`, loads thousands of files, and times out during hydration.
///
/// This test verifies the isolation by asserting the daemon writes its state
/// inside the workspace `.engram/` directory (workspace-local storage), not
/// into any externally-configured path.
#[tokio::test]
async fn regression_engram_data_dir_not_inherited_by_daemon_subprocess() {
    let harness = helpers::DaemonHarness::spawn(READY_TIMEOUT)
        .await
        .expect("daemon must spawn");

    let endpoint = harness.ipc_path().to_str().expect("UTF-8").to_owned();
    assert!(check_health(&endpoint).await, "daemon must be healthy");

    // Daemon must have written its state inside the workspace .engram/ directory.
    let workspace_engram = harness.workspace.path().join(".engram");
    assert!(
        workspace_engram.exists(),
        "daemon must create .engram/ inside the workspace, not in ambient ENGRAM_DATA_DIR"
    );

    // workspace-id file is written at daemon startup; confirms workspace-local data dir.
    assert!(
        workspace_engram.join(".workspace-id").exists(),
        ".workspace-id must exist in workspace .engram/ (confirms workspace-local storage)"
    );
}

/// Regression: daemon must restart cleanly after a crash leaves stale runtime state.
///
/// A prior `SQLITE_BUSY` panic on startup caused the second daemon to fail when
/// the first had crashed with an open lock.  The fix: `WAL`-mode `CozoDB` and
/// graceful lock-release handling on daemon startup.
///
/// This test crashes the first daemon (SIGKILL via drop) and verifies the second
/// daemon starts without manual cleanup of stale lock or socket files.
#[tokio::test]
async fn regression_stale_lock_recovery() {
    let workspace = tempfile::tempdir().expect("tempdir");
    let workspace_path = workspace.path().canonicalize().expect("canonicalize");

    let git_dir = workspace_path.join(".git");
    fs::create_dir_all(&git_dir).expect("create .git");
    fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").expect("write HEAD");

    // First daemon: start, verify healthy, then crash.
    let harness1 = helpers::DaemonHarness::spawn_for_workspace(&workspace_path, READY_TIMEOUT)
        .await
        .expect("first daemon must spawn");

    let endpoint1 = harness1.ipc_path().to_str().expect("UTF-8").to_owned();
    assert!(
        check_health(&endpoint1).await,
        "first daemon must be healthy before crash simulation"
    );

    // Crash simulation: drop kills the process (SIGKILL) without graceful shutdown.
    drop(harness1);
    tokio::time::sleep(Duration::from_millis(300)).await;

    assert!(
        !check_health(&endpoint1).await,
        "crashed daemon endpoint must not respond after kill"
    );

    // Recovery: second daemon must start without manual cleanup of stale state.
    let harness2 = helpers::DaemonHarness::spawn_for_workspace(&workspace_path, READY_TIMEOUT)
        .await
        .expect("second daemon must start after stale-lock recovery (no manual cleanup needed)");

    let endpoint2 = harness2.ipc_path().to_str().expect("UTF-8").to_owned();
    assert!(
        check_health(&endpoint2).await,
        "second daemon must be healthy after stale-lock recovery"
    );
}
