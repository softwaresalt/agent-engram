//! Integration tests for multi-session workspace resume (029-F WS-6 + lifecycle).
//!
//! Verifies that:
//! - Workspace identity (`workspace_id`) persists across daemon restarts.
//! - A second daemon session can bind the same workspace.
//! - `get_workspace_status` succeeds after a daemon restart.

use std::time::Duration;

use engram::daemon::protocol::IpcRequest;
use engram::shim::ipc_client::send_request;
use engram::shim::lifecycle::check_health;
use serde_json::{Value, json};
use tempfile::TempDir;

#[path = "../helpers/mod.rs"]
mod helpers;

use helpers::DaemonHarness;

/// Workspace ID must be stable across daemon restarts (derived from path +
/// branch so independent of session).
///
/// Scenario:
/// 1. Create a persistent workspace dir (kept alive for both sessions).
/// 2. Spawn daemon 1 → bind workspace → capture `workspace_id` → shut down.
/// 3. Spawn daemon 2 for the same workspace → bind → verify `workspace_id` matches.
#[tokio::test]
async fn workspace_id_stable_across_daemon_restart() {
    // Keep TempDir alive across both daemon sessions.
    let workspace = TempDir::new().expect("workspace tempdir");
    let workspace_path = workspace.path().canonicalize().expect("canonicalize");

    // Create minimal .git so the daemon accepts this as a valid workspace.
    let git_dir = workspace_path.join(".git");
    std::fs::create_dir_all(&git_dir).expect("create .git");
    std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").expect("write HEAD");

    // ── Session 1 ──────────────────────────────────────────────────────────────
    let harness1 = DaemonHarness::spawn_for_workspace(&workspace_path, Duration::from_secs(30))
        .await
        .expect("first daemon must start");

    let endpoint1 = harness1.ipc_path().to_str().expect("UTF-8 path").to_owned();
    let path_str = workspace_path.to_str().expect("UTF-8 path").to_owned();

    let bind_req = IpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: Some(Value::Number(serde_json::Number::from(1))),
        method: "set_workspace".to_owned(),
        params: Some(json!({ "path": path_str })),
    };

    let bind_resp1 = send_request(&endpoint1, &bind_req, Duration::from_secs(10))
        .await
        .expect("set_workspace must succeed on first daemon");

    let workspace_id1 = bind_resp1
        .result
        .as_ref()
        .and_then(|r| r.get("workspace_id"))
        .and_then(Value::as_str)
        .expect("workspace_id must be present in set_workspace response")
        .to_owned();

    drop(harness1);
    tokio::time::sleep(Duration::from_millis(300)).await;

    // ── Session 2 ──────────────────────────────────────────────────────────────
    let harness2 = DaemonHarness::spawn_for_workspace(&workspace_path, Duration::from_secs(30))
        .await
        .expect("second daemon must start");

    let endpoint2 = harness2.ipc_path().to_str().expect("UTF-8 path").to_owned();

    let bind_resp2 = send_request(&endpoint2, &bind_req, Duration::from_secs(10))
        .await
        .expect("set_workspace must succeed on second daemon");

    let workspace_id2 = bind_resp2
        .result
        .as_ref()
        .and_then(|r| r.get("workspace_id"))
        .and_then(Value::as_str)
        .expect("workspace_id must be present in second session")
        .to_owned();

    assert_eq!(
        workspace_id1, workspace_id2,
        "workspace_id must be stable across daemon restarts (derived from path, not session)"
    );
}

/// `get_workspace_status` after a restart must return a valid status for the
/// same workspace that was previously bound.
#[tokio::test]
async fn get_workspace_status_valid_after_restart() {
    let workspace = TempDir::new().expect("workspace tempdir");
    let workspace_path = workspace.path().canonicalize().expect("canonicalize");

    let git_dir = workspace_path.join(".git");
    std::fs::create_dir_all(&git_dir).expect("create .git");
    std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").expect("write HEAD");

    let path_str = workspace_path.to_str().expect("UTF-8 path").to_owned();

    // ── Session 1: bind and shut down ─────────────────────────────────────────
    let harness1 = DaemonHarness::spawn_for_workspace(&workspace_path, Duration::from_secs(30))
        .await
        .expect("first daemon must start");

    let endpoint1 = harness1.ipc_path().to_str().expect("UTF-8 path").to_owned();
    assert!(check_health(&endpoint1).await, "daemon 1 must be healthy");

    let bind_req = IpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: Some(Value::Number(serde_json::Number::from(1))),
        method: "set_workspace".to_owned(),
        params: Some(json!({ "path": path_str })),
    };
    let bind_resp = send_request(&endpoint1, &bind_req, Duration::from_secs(10))
        .await
        .expect("set_workspace must succeed");
    assert!(bind_resp.error.is_none(), "set_workspace must not error: {:?}", bind_resp.error);

    drop(harness1);
    tokio::time::sleep(Duration::from_millis(300)).await;

    // ── Session 2: restart and query status ───────────────────────────────────
    let harness2 = DaemonHarness::spawn_for_workspace(&workspace_path, Duration::from_secs(30))
        .await
        .expect("second daemon must start");

    let endpoint2 = harness2.ipc_path().to_str().expect("UTF-8 path").to_owned();
    assert!(check_health(&endpoint2).await, "daemon 2 must be healthy");

    // Re-bind so the daemon has workspace context.
    let _ = send_request(&endpoint2, &bind_req, Duration::from_secs(10))
        .await
        .expect("re-bind must succeed on second daemon");

    let status_req = IpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: Some(Value::Number(serde_json::Number::from(2))),
        method: "get_workspace_status".to_owned(),
        params: Some(json!({ "path": path_str })),
    };

    let status_resp = send_request(&endpoint2, &status_req, Duration::from_secs(10))
        .await
        .expect("get_workspace_status must succeed after restart");

    assert!(
        status_resp.error.is_none(),
        "get_workspace_status must not error after restart: {:?}",
        status_resp.error
    );

    let result = status_resp.result.expect("status result must be present");
    assert!(
        result.get("branch").is_some(),
        "get_workspace_status must return a branch field after restart: {result:?}"
    );
}
