use std::fs;
use std::time::Duration;

use engram::daemon::ipc_server::ipc_endpoint;
use engram::daemon::protocol::IpcRequest;
use engram::shim::ipc_client::send_request;
use engram::shim::lifecycle::{check_health, ensure_daemon_running};
use engram::shim::pidfile::PidFile;
use serde_json::Value;

#[path = "../helpers/mod.rs"]
mod helpers;

#[tokio::test]
async fn shim_recovers_from_stale_pid_file() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    let git_dir = workspace.path().join(".git");
    fs::create_dir(&git_dir).expect("create .git");
    fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").expect("write HEAD");

    let pid_dir = workspace.path().join(".engram").join("run");
    fs::create_dir_all(&pid_dir).expect("create runtime directory");

    let stale_pid = PidFile {
        pid: u32::MAX,
        start_time_unix: 1,
    };
    stale_pid
        .atomic_write(&pid_dir)
        .expect("persist stale pid file");

    tokio::time::timeout(
        Duration::from_secs(15),
        ensure_daemon_running(workspace.path()),
    )
    .await
    .expect("daemon recovery should complete before timeout")
    .expect("daemon should recover from stale pid metadata");

    let pid_file = PidFile::read(workspace.path()).expect("pid file should be rewritten");
    assert_ne!(
        pid_file.pid, stale_pid.pid,
        "recovery should replace stale PID metadata with the live daemon"
    );
    assert!(
        pid_file
            .verify_alive()
            .expect("pid verification should succeed"),
        "rewritten pid file should point at a live daemon"
    );
    assert!(
        workspace
            .path()
            .join(".engram")
            .join(".workspace-id")
            .exists(),
        "daemon startup should persist a workspace identity during recovery"
    );

    let endpoint = ipc_endpoint(workspace.path()).expect("endpoint should resolve");
    assert!(
        check_health(&endpoint).await,
        "recovery path should leave the daemon healthy"
    );

    shutdown_daemon(&endpoint).await;
}

/// Scenario 2: dead-daemon runtime state must not require manual cleanup before restart.
///
/// Simulates a forced daemon crash that leaves stale runtime state (PID file,
/// IPC socket, lock files) in `.engram/run/`. The subsequent call to
/// `ensure_daemon_running` must start a fresh daemon without operator intervention.
#[tokio::test]
async fn shim_recovers_after_daemon_killed_leaves_stale_runtime_state() {
    let workspace = tempfile::tempdir_in(std::env::current_dir().expect("current directory"))
        .expect("workspace tempdir");
    let workspace_path = workspace.path().canonicalize().expect("canonicalize");

    let git_dir = workspace_path.join(".git");
    fs::create_dir(&git_dir).expect("create .git");
    fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").expect("write HEAD");

    // Start first daemon and verify it is healthy.
    let mut harness =
        helpers::DaemonHarness::spawn_for_workspace(&workspace_path, Duration::from_secs(20))
            .await
            .expect("first daemon must spawn");

    let stale_endpoint = harness.ipc_path().to_str().expect("UTF-8").to_owned();
    assert!(
        check_health(&stale_endpoint).await,
        "first daemon must be healthy before crash simulation"
    );

    let stale_pid_file =
        PidFile::read(&workspace_path).expect("first daemon must write structured PID metadata");

    // Crash simulation: kill and reap the daemon without graceful shutdown,
    // leaving stale runtime state in .engram/run/.
    let killed_pid = harness
        .kill_and_wait()
        .expect("first daemon must be killed and reaped");
    assert_eq!(
        stale_pid_file.pid, killed_pid,
        "stale PID metadata must identify the exact child process that was killed"
    );

    // A successful wait proves process exit; its stale endpoint must no longer
    // respond.
    assert!(
        !check_health(&stale_endpoint).await,
        "stale endpoint must not respond after crash"
    );

    // Recovery: ensure_daemon_running must start a fresh daemon without
    // requiring manual cleanup of the stale runtime state.
    tokio::time::timeout(
        Duration::from_secs(25),
        ensure_daemon_running(&workspace_path),
    )
    .await
    .expect("dead-daemon recovery must complete before timeout")
    .expect("daemon must start cleanly from dead runtime state without manual cleanup");

    let recovered_pid_file =
        PidFile::read(&workspace_path).expect("recovery must rewrite structured PID metadata");
    assert_ne!(
        recovered_pid_file.pid, killed_pid,
        "recovery must replace the killed child PID with a different process"
    );
    assert!(
        recovered_pid_file
            .verify_alive()
            .expect("recovered PID verification must succeed"),
        "rewritten PID metadata must identify a live process"
    );

    let endpoint = ipc_endpoint(&workspace_path).expect("endpoint must resolve after recovery");
    assert!(
        check_health(&endpoint).await,
        "recovered daemon must be healthy after dead-runtime-state recovery"
    );

    shutdown_daemon(&endpoint).await;
}

async fn shutdown_daemon(endpoint: &str) {
    let request = IpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: Some(Value::Number(serde_json::Number::from(1))),
        method: "_shutdown".to_owned(),
        params: None,
    };

    let _ = send_request(endpoint, &request, Duration::from_secs(5)).await;

    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while check_health(endpoint).await && tokio::time::Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
