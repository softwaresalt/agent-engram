//! Release smoke tests — daemon and CLI (Unit 6, 035-S).
//!
//! Exactly four test scenarios validating production-readiness of a release build:
//!
//! 1. **`smoke_01_daemon_reaches_ready`** — daemon reaches IPC-Ready state.
//! 2. **`smoke_02_core_lifecycle_command_sequence`** — workspace-status, health, shutdown.
//! 3. **`smoke_03_indexed_query_flow`** — `sync_workspace` then `get_workspace_statistics`.
//! 4. **`smoke_04_stale_state_recovery`** — crash daemon, respawn, assert healthy.
//!
//! All scenarios are ignored on Windows because `CozoDB` 0.7.6 panics in a daemon
//! subprocess on that platform (stash `100EACD8`).

#[path = "../helpers/mod.rs"]
mod helpers;

use std::fs;
use std::time::Duration;

use engram::daemon::protocol::IpcRequest;
use engram::shim::ipc_client::send_request;
use engram::shim::lifecycle::check_health;
use serde_json::Value;

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

/// Send an IPC request, assert no error, and return the result value.
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

/// Smoke 1: daemon reaches IPC-Ready state within the ready timeout.
///
/// Validates that the daemon binary starts, binds its IPC socket, and becomes
/// healthy.  Failure here indicates a fundamental startup or IPC binding break.
#[tokio::test]
#[cfg_attr(
    target_os = "windows",
    ignore = "CozoDB 0.7.6 panics in daemon subprocess on Windows (stash 100EACD8)"
)]
async fn smoke_01_daemon_reaches_ready() {
    let harness = helpers::DaemonHarness::spawn(READY_TIMEOUT)
        .await
        .expect("daemon must reach IPC-Ready within timeout");

    assert!(
        check_health(harness.ipc_path().to_str().expect("UTF-8")).await,
        "daemon must be healthy immediately after spawn"
    );
}

/// Smoke 2: core lifecycle command sequence succeeds end-to-end.
///
/// Calls `get_workspace_status`, `get_health_report`, and `_shutdown` in sequence.
/// Validates the core lifecycle IPC surface including graceful shutdown acknowledgment.
#[tokio::test]
#[cfg_attr(
    target_os = "windows",
    ignore = "CozoDB 0.7.6 panics in daemon subprocess on Windows (stash 100EACD8)"
)]
async fn smoke_02_core_lifecycle_command_sequence() {
    let harness = helpers::DaemonHarness::spawn(READY_TIMEOUT)
        .await
        .expect("daemon must spawn");

    let endpoint = harness.ipc_path().to_str().expect("UTF-8").to_owned();

    let status = send_ok(&endpoint, &make_request(1, "get_workspace_status", None)).await;
    assert!(
        status.is_object(),
        "get_workspace_status must return an object, got: {status:?}"
    );

    let health = send_ok(&endpoint, &make_request(2, "get_health_report", None)).await;
    assert!(
        health.is_object(),
        "get_health_report must return an object, got: {health:?}"
    );

    let shutdown = send_ok(&endpoint, &make_request(3, "_shutdown", None)).await;
    assert!(
        shutdown.is_object() || shutdown.is_null(),
        "_shutdown must return an object or null, got: {shutdown:?}"
    );
}

/// Smoke 3: indexed query flow succeeds after incremental sync.
///
/// Calls `sync_workspace` then `get_workspace_statistics` to validate the
/// indexing pipeline and statistics query path from start to finish.
#[tokio::test]
#[cfg_attr(
    target_os = "windows",
    ignore = "CozoDB 0.7.6 panics in daemon subprocess on Windows (stash 100EACD8)"
)]
async fn smoke_03_indexed_query_flow() {
    let harness = helpers::DaemonHarness::spawn(READY_TIMEOUT)
        .await
        .expect("daemon must spawn");

    let endpoint = harness.ipc_path().to_str().expect("UTF-8").to_owned();

    let sync_result = send_ok(&endpoint, &make_request(1, "sync_workspace", None)).await;
    assert!(
        sync_result.is_object() || sync_result.is_null(),
        "sync_workspace must return an object or null, got: {sync_result:?}"
    );

    let stats = send_ok(
        &endpoint,
        &make_request(2, "get_workspace_statistics", None),
    )
    .await;
    assert!(
        stats.is_object(),
        "get_workspace_statistics must return an object after sync, got: {stats:?}"
    );
}

/// Smoke 4: stale-state recovery scenario succeeds.
///
/// Crashes the first daemon (SIGKILL via drop), then starts a second daemon
/// against the same workspace.  Validates that no manual cleanup of stale
/// runtime state (PID file, IPC socket, lock files) is required before restart.
#[tokio::test]
#[cfg_attr(
    target_os = "windows",
    ignore = "CozoDB 0.7.6 panics in daemon subprocess on Windows (stash 100EACD8)"
)]
async fn smoke_04_stale_state_recovery() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    let workspace_path = workspace.path().canonicalize().expect("canonicalize");

    let git_dir = workspace_path.join(".git");
    fs::create_dir_all(&git_dir).expect("create .git");
    fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").expect("write HEAD");

    // First daemon: spawn, verify healthy, then crash.
    let harness1 = helpers::DaemonHarness::spawn_for_workspace(&workspace_path, READY_TIMEOUT)
        .await
        .expect("first daemon must spawn");

    let endpoint1 = harness1.ipc_path().to_str().expect("UTF-8").to_owned();
    assert!(
        check_health(&endpoint1).await,
        "first daemon must be healthy before crash"
    );

    // Crash: kill without graceful shutdown, leaving stale runtime state.
    drop(harness1);
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Recovery: second daemon must start without manual state cleanup.
    let harness2 = helpers::DaemonHarness::spawn_for_workspace(&workspace_path, READY_TIMEOUT)
        .await
        .expect("second daemon must start cleanly after stale-state recovery");

    assert!(
        check_health(harness2.ipc_path().to_str().expect("UTF-8")).await,
        "recovered daemon must be healthy after stale-state recovery"
    );
}
