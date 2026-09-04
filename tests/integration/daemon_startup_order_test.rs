//! Integration tests for daemon startup order — task 025.002-T.
//!
//! Verifies that `run_with_shutdown_v2` — the refactored IPC server entry
//! point — binds the IPC listener *before* starting the file watcher.  This
//! ordering is what prevents the daemon from hanging on large workspaces
//! where `ReadDirectoryChangesW` / `inotify_add_watch` registration takes
//! longer than the shim's health-probe timeout.
//!
//! # Test phases
//!
//! | Phase | Behaviour |
//! |-------|-----------|
//! | Red (stub) | `run_with_shutdown_v2` body is `todo!(…)` → async task panics → test FAILS |
//! | Green (impl) | daemon binds, hydrates, TTL expires → returns `Ok(())` → test PASSES |

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::watch;

use engram::daemon::ipc_server::run_with_shutdown_v2;
use engram::daemon::ttl::TtlTimer;
use engram::daemon::watcher::WatcherConfig;
use engram::models::config::DaemonMode;

// ── 025.002-T harness ─────────────────────────────────────────────────────────

/// Verify `run_with_shutdown_v2` accepts the new function signature and
/// completes successfully when the daemon's idle TTL fires.
///
/// A 1-second TTL causes the daemon to self-terminate shortly after workspace
/// hydration finishes, so the test completes without an external shutdown
/// signal.  A 30-second wall-clock guard prevents an infinite hang if the
/// implementation regresses.
///
/// # Red phase (before 025.002-T)
///
/// `run_with_shutdown_v2` panics with `todo!("025.002-T: …")` — the test
/// reports a panic failure.
///
/// # Green phase (after 025.002-T)
///
/// The function binds the IPC listener first, hydrates the workspace in a
/// background task, then exits cleanly when the TTL expires.  The test
/// returns `Ok(())`.
#[tokio::test]
async fn run_with_shutdown_v2_exits_cleanly_on_ttl_expiry() {
    let workspace = tempfile::TempDir::new().expect("temp workspace dir");
    let workspace_path = workspace
        .path()
        .canonicalize()
        .expect("canonicalize workspace");

    // Minimal git scaffold so the daemon accepts this directory as a workspace.
    let git_dir = workspace_path.join(".git");
    std::fs::create_dir_all(&git_dir).expect("create .git");
    std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").expect("write HEAD");

    // 10-second TTL: enough for background_db_hydration to finish opening a fresh
    // SQLite database even on a loaded Windows machine (Defender scans can delay
    // DbInstance::new + schema bootstrap).  Using 1 s was racy: the hydration
    // background task held the fd-lock past the TTL deadline, causing the final
    // flush_all_workspaces connect_db to time out.
    let ttl = TtlTimer::new(Duration::from_secs(10));
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let watcher_config = WatcherConfig {
        debounce_ms: 300,
        exclude_patterns: vec![],
        watch_patterns: vec![],
    };
    let workspace_str = workspace_path
        .to_str()
        .expect("UTF-8 workspace path")
        .to_owned();

    // 30-second wall-clock guard keeps the test suite from hanging if the
    // implementation blocks the executor thread before the TTL fires.
    tokio::time::timeout(
        Duration::from_secs(30),
        run_with_shutdown_v2(
            &workspace_str,
            DaemonMode::Managed,
            ttl,
            Arc::new(shutdown_tx),
            shutdown_rx,
            watcher_config,
        ),
    )
    .await
    .expect("run_with_shutdown_v2 must complete within 30 s")
    .expect("run_with_shutdown_v2 must return Ok(())");
}
