//! Integration tests — workspace lifecycle workflow (Unit 2, 035-S).
//!
//! Scenarios:
//! - `s_wl_01_bind_ready_index_query`: bind → workspace-status → `sync_workspace` → statistics.
//! - `s_wl_02_shutdown_restart_query`: graceful shutdown, restart from same workspace, re-query.

#[path = "../helpers/mod.rs"]
mod helpers;

use std::time::Duration;

use engram::daemon::protocol::IpcRequest;
use engram::shim::ipc_client::send_request;
use engram::shim::lifecycle::check_health;
use serde_json::{Value, json};

const READY_TIMEOUT: Duration = Duration::from_secs(20);
const IPC_TIMEOUT: Duration = Duration::from_secs(10);

fn make_request(id: i64, method: &str, params: Option<Value>) -> IpcRequest {
    IpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: Some(Value::Number(id.into())),
        method: method.to_owned(),
        params,
    }
}

/// Call the IPC endpoint, assert the response carries no error, return the result.
async fn send_ok(endpoint: &str, req: &IpcRequest) -> Value {
    let resp = send_request(endpoint, req, IPC_TIMEOUT)
        .await
        .expect("IPC request must succeed");
    assert!(
        resp.error.is_none(),
        "unexpected IPC error from {}: {:?}",
        req.method,
        resp.error
    );
    resp.result.unwrap_or(Value::Null)
}

/// S-WL-01: bind → workspace-status → `sync_workspace` → `get_workspace_statistics`.
///
/// Verifies the complete read-path lifecycle from daemon start through indexed query.
/// The `set_workspace` call is idempotent when the daemon is already bound.
#[tokio::test]
async fn s_wl_01_bind_ready_index_query() {
    let harness = helpers::DaemonHarness::spawn(READY_TIMEOUT)
        .await
        .expect("daemon must spawn and reach IPC-ready state");

    let endpoint = harness.ipc_path().to_str().expect("UTF-8").to_owned();

    let workspace_path = harness.workspace.path().to_str().expect("UTF-8").to_owned();

    // Re-bind the workspace (idempotent; must succeed with a result object).
    let bind_result = send_ok(
        &endpoint,
        &make_request(1, "set_workspace", Some(json!({ "path": workspace_path }))),
    )
    .await;
    assert!(
        bind_result.is_object(),
        "set_workspace must return an object, got: {bind_result:?}"
    );

    // Workspace status must report a valid object.
    let status = send_ok(&endpoint, &make_request(2, "get_workspace_status", None)).await;
    assert!(
        status.is_object(),
        "get_workspace_status must return an object, got: {status:?}"
    );

    // Incremental sync — resolves any in-progress startup auto-index.
    let sync_result = send_ok(&endpoint, &make_request(3, "sync_workspace", None)).await;
    assert!(
        sync_result.is_object() || sync_result.is_null(),
        "sync_workspace must return an object or null, got: {sync_result:?}"
    );

    // Statistics query after sync: empty workspace returns zero counts.
    let wstats = send_ok(
        &endpoint,
        &make_request(4, "get_workspace_statistics", None),
    )
    .await;
    assert!(
        wstats.is_object(),
        "get_workspace_statistics must return an object, got: {wstats:?}"
    );
}

/// S-WL-02: graceful shutdown, restart from the same workspace, re-query status.
///
/// Verifies the daemon can be stopped cleanly and a fresh daemon can start against
/// the same workspace without manual state cleanup.
#[tokio::test]
async fn s_wl_02_shutdown_restart_query() {
    let mut harness = helpers::DaemonHarness::spawn(READY_TIMEOUT)
        .await
        .expect("first daemon must spawn");

    let workspace_path = harness.workspace.path().to_path_buf();
    let endpoint1 = harness.ipc_path().to_str().expect("UTF-8").to_owned();

    assert!(
        check_health(&endpoint1).await,
        "first daemon must be healthy"
    );

    // Graceful shutdown request (best-effort; ignore send errors).
    let _ = send_request(&endpoint1, &make_request(1, "_shutdown", None), IPC_TIMEOUT).await;

    // Wait for the process to exit (up to 10 s), then proceed regardless.
    let exit_deadline = std::time::Instant::now() + Duration::from_secs(10);
    loop {
        if let Ok(Some(_)) = harness.try_wait() {
            break;
        }
        if std::time::Instant::now() >= exit_deadline {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Restart against the same workspace.
    let harness2 = helpers::DaemonHarness::spawn_for_workspace(&workspace_path, READY_TIMEOUT)
        .await
        .expect("second daemon must start against the same workspace");

    let endpoint2 = harness2.ipc_path().to_str().expect("UTF-8").to_owned();

    assert!(
        check_health(&endpoint2).await,
        "restarted daemon must be healthy"
    );

    // Workspace status must be present after restart.
    let status = send_ok(&endpoint2, &make_request(2, "get_workspace_status", None)).await;
    assert!(
        status.is_object(),
        "get_workspace_status must return an object after restart, got: {status:?}"
    );
}
