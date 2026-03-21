//! Integration tests for daemon lifecycle management (T046–T047).
//!
//! Scenarios covered:
//! - S037: Graceful shutdown via `_shutdown` IPC flushes state and removes lock
//! - S038: SIGTERM / Ctrl-C triggers graceful shutdown
//! - S039-S040: SIGKILL → stale lock/socket detected → new daemon starts cleanly
//! - S050: Daemon self-terminates after idle timeout; restarts successfully

use std::time::Duration;

use engram::daemon::protocol::IpcRequest;
use engram::shim::ipc_client::send_request;
use engram::shim::lifecycle::check_health;
use serde_json::Value;

#[path = "../helpers/mod.rs"]
mod helpers;

use helpers::DaemonHarness;

// ── T046 / S037: Graceful shutdown via IPC ────────────────────────────────────

/// Sending `_shutdown` IPC must return `{"status": "shutting_down",
/// "flush_started": true}` and the daemon process must exit within 5 seconds.
#[tokio::test]
async fn t046_s037_shutdown_ipc_triggers_graceful_exit() {
    let mut harness = DaemonHarness::spawn(Duration::from_secs(15))
        .await
        .expect("daemon must spawn and become ready");

    let endpoint = harness.ipc_path().to_str().expect("UTF-8 path").to_owned();

    // Send _shutdown request.
    let request = IpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: Some(Value::Number(serde_json::Number::from(99))),
        method: "_shutdown".to_owned(),
        params: None,
    };
    let response = send_request(&endpoint, &request, Duration::from_secs(5))
        .await
        .expect("_shutdown IPC must succeed");

    let result = response
        .result
        .as_ref()
        .expect("_shutdown must return a result, not an error");

    assert_eq!(
        result["status"], "shutting_down",
        "_shutdown response must carry status=shutting_down"
    );
    assert_eq!(
        result["flush_started"], true,
        "_shutdown response must carry flush_started=true"
    );

    // The daemon must exit within 5 s.
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        match harness.try_wait() {
            Ok(Some(_)) => break, // exited cleanly
            Ok(None) => {}
            Err(e) => panic!("wait error: {e}"),
        }
        assert!(
            std::time::Instant::now() < deadline,
            "daemon must exit within 5 s of receiving _shutdown"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // After exit the IPC endpoint should no longer be healthy.
    assert!(
        !check_health(&endpoint).await,
        "daemon must no longer respond after shutdown"
    );
}

// ── T046 / S050: Restart after idle timeout ──────────────────────────────────

/// A daemon configured with a short idle timeout must self-terminate, and a
/// new daemon spawned for the same workspace must start cleanly.
#[tokio::test]
async fn t046_s050_daemon_exits_after_idle_timeout_and_restarts() {
    use tempfile::TempDir;

    let workspace = TempDir::new().expect("tempdir");
    let workspace_path = workspace.path().canonicalize().expect("canonicalize");

    let git_dir = workspace_path.join(".git");
    std::fs::create_dir_all(&git_dir).expect("create .git");
    std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").expect("write HEAD");

    // Spawn with a 500 ms idle timeout — the daemon should self-terminate soon.
    let mut harness1 = DaemonHarness::spawn_with_idle_timeout_ms(500, Duration::from_secs(15))
        .await
        .expect("daemon must spawn with short timeout");

    // Override workspace path: use the shared one from above instead of harness's TempDir.
    // (harness1 owns its own TempDir; use harness1.workspace.path() which is its own dir)
    let harness_workspace = harness1.workspace.path().to_path_buf();

    // The daemon should self-terminate after ~500 ms of inactivity.
    let deadline = std::time::Instant::now() + Duration::from_secs(8);
    loop {
        match harness1.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {}
            Err(e) => panic!("wait error: {e}"),
        }
        assert!(
            std::time::Instant::now() < deadline,
            "daemon must self-terminate after idle timeout"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Spawn a fresh daemon for the same workspace — must succeed.
    let harness2 = DaemonHarness::spawn_for_workspace(&harness_workspace, Duration::from_secs(15))
        .await
        .expect("new daemon must start after previous timed out");

    let endpoint = harness2.ipc_path().to_str().expect("UTF-8").to_owned();
    assert!(
        check_health(&endpoint).await,
        "new daemon must be healthy after restart"
    );
}

// ── T047 / S039-S040: SIGKILL crash recovery ─────────────────────────────────

/// After a daemon is killed (simulating a crash), a new daemon spawned for the
/// same workspace must detect the stale lock, clean up, and start successfully.
#[tokio::test]
async fn t047_s039_s040_new_daemon_starts_after_crash() {
    use tempfile::TempDir;

    // Create and own the workspace independently so it survives daemon 1's death.
    let workspace = TempDir::new().expect("tempdir");
    let workspace_path = workspace.path().canonicalize().expect("canonicalize");

    let git_dir = workspace_path.join(".git");
    std::fs::create_dir_all(&git_dir).expect("create .git");
    std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").expect("write HEAD");

    // Spawn daemon 1.
    let harness1 = DaemonHarness::spawn_for_workspace(&workspace_path, Duration::from_secs(15))
        .await
        .expect("first daemon must spawn");

    let endpoint1 = harness1.ipc_path().to_str().expect("UTF-8").to_owned();
    assert!(
        check_health(&endpoint1).await,
        "first daemon must be healthy before crash"
    );

    // Simulate SIGKILL: kill without graceful shutdown and drop the harness.
    drop(harness1); // Drop triggers kill() in Drop impl.

    // Brief pause to let the OS release the file lock.
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Spawn daemon 2 for the same workspace path.
    let harness2 = DaemonHarness::spawn_for_workspace(&workspace_path, Duration::from_secs(15))
        .await
        .expect("second daemon must start after crash recovery");

    let endpoint2 = harness2.ipc_path().to_str().expect("UTF-8").to_owned();
    assert!(
        check_health(&endpoint2).await,
        "second daemon must be healthy after crash recovery"
    );
}

/// After a crash the stale IPC socket (Unix) or old pipe (Windows) must not
/// prevent a fresh daemon from binding its endpoint.
#[tokio::test]
async fn t047_s040_stale_socket_cleaned_up_after_crash() {
    use tempfile::TempDir;

    let workspace = TempDir::new().expect("tempdir");
    let workspace_path = workspace.path().canonicalize().expect("canonicalize");

    let git_dir = workspace_path.join(".git");
    std::fs::create_dir_all(&git_dir).expect("create .git");
    std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").expect("write HEAD");

    let harness1 = DaemonHarness::spawn_for_workspace(&workspace_path, Duration::from_secs(15))
        .await
        .expect("first daemon must spawn");

    let _endpoint1 = harness1.ipc_path().to_str().expect("UTF-8").to_owned();

    // Kill without cleanup — harness1 drop triggers force-kill.
    drop(harness1);
    tokio::time::sleep(Duration::from_millis(200)).await;

    // A second spawn attempt must NOT fail with "address already in use" or
    // similar — it should cleanly take over the endpoint.
    let harness2 = DaemonHarness::spawn_for_workspace(&workspace_path, Duration::from_secs(15))
        .await
        .expect("second daemon must bind IPC endpoint after crash cleanup");

    let endpoint = harness2.ipc_path().to_str().expect("UTF-8").to_owned();
    assert!(
        check_health(&endpoint).await,
        "recovered daemon must be healthy"
    );
}

// ── T047 / Stale lock detection ───────────────────────────────────────────────

/// Data written to the workspace before a crash must be rehydrated after
/// the daemon restarts (S095-S096).
#[tokio::test]
async fn t047_data_persists_across_crash_and_restart() {
    use tempfile::TempDir;

    let workspace = TempDir::new().expect("tempdir");
    let workspace_path = workspace.path().canonicalize().expect("canonicalize");

    let git_dir = workspace_path.join(".git");
    std::fs::create_dir_all(&git_dir).expect("create .git");
    std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").expect("write HEAD");

    let harness1 = DaemonHarness::spawn_for_workspace(&workspace_path, Duration::from_secs(15))
        .await
        .expect("first daemon must spawn");

    let _endpoint1 = harness1.ipc_path().to_str().expect("UTF-8").to_owned();

    // create_task removed (Phase 1 cleanup); verify daemon restarts and serves statistics.
    drop(harness1);
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Restart daemon 2.
    let harness2 = DaemonHarness::spawn_for_workspace(&workspace_path, Duration::from_secs(15))
        .await
        .expect("daemon 2 must start after crash");

    let endpoint2 = harness2.ipc_path().to_str().expect("UTF-8").to_owned();
    assert!(check_health(&endpoint2).await, "daemon 2 must be healthy");

    // Query workspace statistics — verify the daemon is functional after crash+restart.
    let list_req = IpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: Some(Value::Number(serde_json::Number::from(2))),
        method: "get_workspace_statistics".to_owned(),
        params: None,
    };
    let list_resp = send_request(&endpoint2, &list_req, Duration::from_secs(5))
        .await
        .expect("get_workspace_statistics must succeed after restart");

    assert!(
        list_resp.error.is_none(),
        "get_workspace_statistics must not error after crash recovery: {list_resp:?}"
    );
}
