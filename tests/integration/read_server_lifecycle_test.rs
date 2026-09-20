//! Integration coverage for the read-server lifecycle policy (plan unit F44,
//! 142.052-T).
//!
//! Verifies the daemon lifecycle seam directly: `ReadServer` mode must skip
//! managed-mode hydration, startup scanning, watcher registration, implicit
//! sync, and shutdown graph flush, while `Managed` mode remains unchanged.

#![forbid(unsafe_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use engram::daemon::ipc_server::run_with_shutdown_v2;
use engram::daemon::lifecycle_policy::{
    LifecycleActivityCounters, lifecycle_activity_counters, reset_lifecycle_activity_counters,
};
use engram::daemon::ttl::TtlTimer;
use engram::daemon::watcher::WatcherConfig;
use engram::models::config::DaemonMode;
use tempfile::TempDir;
use tokio::sync::watch;

fn create_workspace(mode: DaemonMode) -> TempDir {
    let tempdir = tempfile::tempdir().expect("workspace tempdir");
    let root = tempdir.path();
    fs::create_dir_all(root.join(".git")).expect("create .git");
    fs::write(root.join(".git").join("HEAD"), "ref: refs/heads/main\n").expect("write git HEAD");
    fs::create_dir_all(root.join(".engram")).expect("create .engram");
    fs::write(
        root.join(".engram").join("config.toml"),
        format!("mode = \"{}\"\n", mode.as_str()),
    )
    .expect("write config.toml");
    fs::write(root.join("lib.rs"), "pub fn indexed() {}\n").expect("write source file");
    tempdir
}

async fn run_lifecycle(workspace: &Path, mode: DaemonMode) -> LifecycleActivityCounters {
    reset_lifecycle_activity_counters();
    let ttl = TtlTimer::new(Duration::from_secs(1));
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let workspace_str = workspace
        .to_str()
        .expect("workspace path must be valid UTF-8")
        .to_owned();

    tokio::time::timeout(
        Duration::from_secs(30),
        run_with_shutdown_v2(
            &workspace_str,
            mode,
            ttl,
            std::sync::Arc::new(shutdown_tx),
            shutdown_rx,
            WatcherConfig {
                daemon_mode: mode,
                ..WatcherConfig::default()
            },
        ),
    )
    .await
    .expect("daemon lifecycle must complete within 30 s")
    .expect("daemon lifecycle must exit cleanly");

    lifecycle_activity_counters()
}

fn metrics_dir(workspace: &Path) -> PathBuf {
    workspace.join(".engram").join("metrics").join("main")
}

fn code_graph_dir(workspace: &Path) -> PathBuf {
    workspace.join(".engram").join("code-graph").join("main")
}

#[tokio::test]
async fn read_server_lifecycle_skips_managed_startup_and_shutdown_work() {
    let read_server_workspace = create_workspace(DaemonMode::ReadServer);
    let read_server_counts =
        run_lifecycle(read_server_workspace.path(), DaemonMode::ReadServer).await;

    assert_eq!(read_server_counts.startup_hydration_calls, 0);
    assert_eq!(read_server_counts.startup_source_scan_calls, 0);
    assert_eq!(read_server_counts.offline_source_scan_calls, 0);
    assert_eq!(read_server_counts.watcher_registration_calls, 0);
    assert_eq!(read_server_counts.implicit_sync_calls, 0);
    assert_eq!(read_server_counts.shutdown_flush_calls, 0);
    assert!(
        !metrics_dir(read_server_workspace.path()).exists(),
        "read-server startup must not initialize metrics state"
    );
    assert!(
        !code_graph_dir(read_server_workspace.path()).exists(),
        "read-server lifecycle must not write code-graph snapshots"
    );

    let managed_workspace = create_workspace(DaemonMode::Managed);
    let managed_counts = run_lifecycle(managed_workspace.path(), DaemonMode::Managed).await;

    assert_eq!(managed_counts.startup_hydration_calls, 1);
    assert_eq!(managed_counts.startup_source_scan_calls, 1);
    assert_eq!(managed_counts.offline_source_scan_calls, 1);
    assert_eq!(managed_counts.watcher_registration_calls, 1);
    assert!(managed_counts.implicit_sync_calls >= 1);
    assert_eq!(managed_counts.shutdown_flush_calls, 1);
}
