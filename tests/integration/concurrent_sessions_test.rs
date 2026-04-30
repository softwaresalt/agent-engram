//! Integration tests for concurrent IPC sessions (001-F / 001.009-T).
//!
//! Characterizes the daemon's concurrent session handling: multiple shim
//! connections arriving simultaneously must all receive valid responses
//! without response corruption, panics, or data races.
//!
//! The engram daemon uses a tokio async accept loop that spawns a task per
//! IPC connection. Each connection reads one request, dispatches through
//! `tools::dispatch`, and writes the response. These tests verify that the
//! daemon handles simultaneous connections correctly under the `AppState`
//! concurrency model (RwLock + AtomicBool primitives).
//!
//! Out of scope: `active_connections` counter and `RateLimiter` — both are
//! SSE-transport concerns (US5/T091, FR-025/T118) and do not apply to IPC.

use std::time::Duration;

use engram::daemon::protocol::IpcRequest;
use engram::shim::ipc_client::send_request;
use serde_json::{Value, json};

#[path = "../helpers/mod.rs"]
mod helpers;

use helpers::DaemonHarness;

/// S-CS1: Three concurrent `_health` requests succeed without response corruption.
///
/// Three tokio tasks each open an independent IPC connection and issue a
/// `_health` request simultaneously. The daemon's accept loop must handle all
/// three correctly: valid JSON response, `status` field present, no cross-
/// contamination between connections.
///
/// This is the lightest concurrent scenario — `_health` is handled entirely
/// within the IPC server layer without touching `tools::dispatch`.
#[tokio::test]
async fn s_cs1_three_concurrent_health_checks_succeed() {
    let harness = DaemonHarness::spawn(Duration::from_secs(30))
        .await
        .expect("daemon must start for concurrent health test");

    let endpoint = harness.ipc_path().to_str().expect("UTF-8 path").to_owned();

    let tasks: Vec<_> = (1_u32..=3)
        .map(|i| {
            let ep = endpoint.clone();
            tokio::spawn(async move {
                let req = IpcRequest {
                    jsonrpc: "2.0".to_owned(),
                    id: Some(Value::Number(serde_json::Number::from(i))),
                    method: "_health".to_owned(),
                    params: None,
                };
                send_request(&ep, &req, Duration::from_secs(10)).await
            })
        })
        .collect();

    for (i, task) in tasks.into_iter().enumerate() {
        let resp = task
            .await
            .expect("concurrent health task must not panic")
            .unwrap_or_else(|e| panic!("concurrent health check {i} must succeed: {e}"));

        assert!(
            resp.error.is_none(),
            "health check {i} must not return an IPC error: {:?}",
            resp.error
        );

        let body = resp
            .result
            .expect("health check {i} must have a result body");

        assert!(
            body.get("status").is_some(),
            "health response {i} must contain a 'status' field — response corruption detected: {body}"
        );

        assert!(
            body.get("protocol_version").is_some(),
            "health response {i} must contain 'protocol_version': {body}"
        );
    }
}

/// S-CS2: Two concurrent `get_daemon_status` calls return consistent state.
///
/// Two tokio tasks issue `get_daemon_status` simultaneously. Both must succeed
/// and their `protocol_version` fields must be identical — proving no response
/// cross-contamination between the two IPC connections.
///
/// `get_daemon_status` reads `AppState` atomics and the `RwLock`-guarded
/// workspace snapshot. This test verifies that concurrent reads of shared
/// state produce coherent results.
#[tokio::test]
async fn s_cs2_concurrent_get_daemon_status_consistent_state() {
    let harness = DaemonHarness::spawn(Duration::from_secs(30))
        .await
        .expect("daemon must start for concurrent status test");

    let endpoint = harness.ipc_path().to_str().expect("UTF-8 path").to_owned();

    let tasks: Vec<_> = (1_u32..=2)
        .map(|i| {
            let ep = endpoint.clone();
            tokio::spawn(async move {
                let req = IpcRequest {
                    jsonrpc: "2.0".to_owned(),
                    id: Some(Value::Number(serde_json::Number::from(i))),
                    method: "get_daemon_status".to_owned(),
                    params: Some(json!({})),
                };
                send_request(&ep, &req, Duration::from_secs(10)).await
            })
        })
        .collect();

    let mut bodies = Vec::new();
    for (i, task) in tasks.into_iter().enumerate() {
        let resp = task
            .await
            .expect("concurrent status task must not panic")
            .unwrap_or_else(|e| panic!("get_daemon_status {i} must succeed: {e}"));

        assert!(
            resp.error.is_none(),
            "get_daemon_status {i} must not return an IPC error: {:?}",
            resp.error
        );

        let body = resp
            .result
            .expect("get_daemon_status {i} must have a result body");

        assert!(
            body.get("version").is_some(),
            "response {i} must include 'version': {body}"
        );
        assert!(
            body.get("uptime_seconds").is_some(),
            "response {i} must include 'uptime_seconds': {body}"
        );

        bodies.push(body);
    }

    // version must be identical across both responses — response
    // cross-contamination would produce mismatched or corrupt values.
    assert_eq!(
        bodies[0].get("version"),
        bodies[1].get("version"),
        "version must be identical across concurrent get_daemon_status responses"
    );
}

/// S-CS3: Concurrent `set_workspace` + `get_daemon_status` produces coherent state.
///
/// `set_workspace` triggers workspace hydration and may set
/// `indexing_in_progress`. Issuing `get_daemon_status` concurrently exercises
/// the shared `AppState` under a write + read race. Both calls must succeed
/// without panics, and each response must be a coherent JSON object.
///
/// This is the heaviest concurrent scenario and the primary characterization
/// for 001-F: proving the daemon's async concurrency model holds under load.
#[tokio::test]
async fn s_cs3_concurrent_set_workspace_and_status_coherent() {
    let harness = DaemonHarness::spawn(Duration::from_secs(30))
        .await
        .expect("daemon must start for concurrent set+status test");

    let endpoint = harness.ipc_path().to_str().expect("UTF-8 path").to_owned();
    let workspace_path = harness
        .workspace
        .path()
        .to_str()
        .expect("UTF-8 path")
        .to_owned();

    let ep_set = endpoint.clone();
    let ep_status = endpoint.clone();
    let path = workspace_path.clone();

    let h_set = tokio::spawn(async move {
        let req = IpcRequest {
            jsonrpc: "2.0".to_owned(),
            id: Some(Value::Number(serde_json::Number::from(1))),
            method: "set_workspace".to_owned(),
            params: Some(json!({ "path": path })),
        };
        send_request(&ep_set, &req, Duration::from_secs(30)).await
    });

    let h_status = tokio::spawn(async move {
        // Yield once to maximise the chance that both tasks are scheduled
        // before either completes — this is cooperative concurrency within
        // a single tokio thread, not a hard synchronization barrier.
        tokio::task::yield_now().await;
        let req = IpcRequest {
            jsonrpc: "2.0".to_owned(),
            id: Some(Value::Number(serde_json::Number::from(2))),
            method: "get_daemon_status".to_owned(),
            params: Some(json!({})),
        };
        send_request(&ep_status, &req, Duration::from_secs(10)).await
    });

    let set_result = h_set.await.expect("set_workspace task must not panic");
    let status_result = h_status
        .await
        .expect("get_daemon_status task must not panic");

    assert!(
        set_result.is_ok(),
        "set_workspace must succeed under concurrent status call: {set_result:?}"
    );
    assert!(
        status_result.is_ok(),
        "get_daemon_status must succeed under concurrent set_workspace: {status_result:?}"
    );

    // Each response must be a coherent object — no corrupt JSON, no missing fields.
    let set_resp = set_result.unwrap();
    assert!(
        set_resp.error.is_none(),
        "set_workspace must not return an IPC error: {:?}",
        set_resp.error
    );
    let set_body = set_resp
        .result
        .expect("set_workspace must have a result body");
    assert!(
        set_body.is_object(),
        "set_workspace result must be a JSON object: {set_body}"
    );

    let status_resp = status_result.unwrap();
    assert!(
        status_resp.error.is_none(),
        "get_daemon_status must not return an IPC error: {:?}",
        status_resp.error
    );
    let status_body = status_resp
        .result
        .expect("get_daemon_status must have a result body");
    assert!(
        status_body.get("version").is_some(),
        "get_daemon_status must return a 'version' field: {status_body}"
    );
}
