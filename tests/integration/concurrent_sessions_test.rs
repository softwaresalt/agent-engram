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
//! concurrency model (`RwLock` + `AtomicBool` primitives).
//!
//! Out of scope: `active_connections` counter and `RateLimiter` — both are
//! SSE-transport concerns (US5/T091, FR-025/T118) and do not apply to IPC.

use std::sync::Arc;
use std::time::{Duration, Instant};

use engram::daemon::protocol::IpcRequest;
use engram::shim::ipc_client::send_request;
use serde_json::{Value, json};
use tokio::sync::Barrier;

#[path = "../helpers/mod.rs"]
mod helpers;

use helpers::DaemonHarness;

/// S-CS1: Three concurrent `_health` requests succeed without response corruption.
///
/// Three tokio tasks each open an independent IPC connection and issue a
/// `_health` request simultaneously. The daemon's accept loop must handle all
/// three correctly: valid JSON response, `status` field present, response `id`
/// matches request `id`, no cross-contamination between connections.
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
        let req_id = u32::try_from(i + 1).expect("index fits u32");
        let resp = task
            .await
            .expect("concurrent health task must not panic")
            .unwrap_or_else(|e| panic!("concurrent health check {i} must succeed: {e}"));

        assert!(
            resp.error.is_none(),
            "health check {i} must not return an IPC error: {:?}",
            resp.error
        );

        assert_eq!(
            resp.id,
            Value::Number(serde_json::Number::from(req_id)),
            "health response {i} id must match request id {req_id}"
        );

        let body = resp
            .result
            .unwrap_or_else(|| panic!("health check {i} must have a result body"));

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
/// and their `version` fields must be identical — proving no response
/// cross-contamination between the two IPC connections. Response `id` fields
/// must match the originating request `id`.
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
        let req_id = u32::try_from(i + 1).expect("index fits u32");
        let resp = task
            .await
            .expect("concurrent status task must not panic")
            .unwrap_or_else(|e| panic!("get_daemon_status {i} must succeed: {e}"));

        assert!(
            resp.error.is_none(),
            "get_daemon_status {i} must not return an IPC error: {:?}",
            resp.error
        );

        assert_eq!(
            resp.id,
            Value::Number(serde_json::Number::from(req_id)),
            "status response {i} id must match request id {req_id}"
        );

        let body = resp
            .result
            .unwrap_or_else(|| panic!("get_daemon_status {i} must have a result body"));

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
/// `set_workspace` triggers workspace hydration and clears then re-sets
/// `hydration_ready`. Issuing `get_daemon_status` concurrently exercises
/// the shared `AppState` under a write + read race. Both calls must succeed
/// without panics, and each response must be a coherent JSON object.
///
/// A `Barrier(2)` ensures both tasks dispatch their IPC requests from a
/// deterministically synchronised point — both connections are guaranteed to
/// be in-flight simultaneously at the daemon.
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

    let barrier = Arc::new(Barrier::new(2));

    let ep_set = endpoint.clone();
    let ep_status = endpoint.clone();
    let path = workspace_path.clone();
    let b_set = Arc::clone(&barrier);
    let b_status = Arc::clone(&barrier);

    let h_set = tokio::spawn(async move {
        b_set.wait().await;
        let req = IpcRequest {
            jsonrpc: "2.0".to_owned(),
            id: Some(Value::Number(serde_json::Number::from(1))),
            method: "set_workspace".to_owned(),
            params: Some(json!({ "path": path })),
        };
        send_request(&ep_set, &req, Duration::from_secs(30)).await
    });

    let h_status = tokio::spawn(async move {
        b_status.wait().await;
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

    // Each response must be coherent — correct id, no corrupt JSON, required fields present.
    let set_resp = set_result.unwrap();
    assert!(
        set_resp.error.is_none(),
        "set_workspace must not return an IPC error: {:?}",
        set_resp.error
    );
    assert_eq!(
        set_resp.id,
        Value::Number(serde_json::Number::from(1_u32)),
        "set_workspace response id must match request id 1"
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
    assert_eq!(
        status_resp.id,
        Value::Number(serde_json::Number::from(2_u32)),
        "get_daemon_status response id must match request id 2"
    );
    let status_body = status_resp
        .result
        .expect("get_daemon_status must have a result body");
    assert!(
        status_body.get("version").is_some(),
        "get_daemon_status must return a 'version' field: {status_body}"
    );
}

/// S-CS4: Concurrent `index_workspace` calls are serialised by `indexing_in_progress`.
///
/// Two shim connections issue `index_workspace` simultaneously after a prior
/// `set_workspace`. The daemon's `try_start_indexing()` `AtomicBool`
/// compare-exchange ensures only one proceeds; the concurrent caller receives
/// error code 7003 (`IndexInProgress`).
///
/// A `Barrier(2)` ensures both IPC requests depart from a deterministically
/// synchronised point. The workspace is seeded with 20 indexable `.rs` files
/// before the concurrent calls so that indexing reliably takes longer than the
/// IPC round-trip, making the race deterministic rather than timing-dependent.
#[tokio::test]
async fn s_cs4_concurrent_indexing_serialised_by_in_progress_flag() {
    let harness = DaemonHarness::spawn(Duration::from_secs(30))
        .await
        .expect("daemon must start for concurrent indexing test");

    let endpoint = harness.ipc_path().to_str().expect("UTF-8 path").to_owned();
    let workspace_path = harness
        .workspace
        .path()
        .to_str()
        .expect("UTF-8 path")
        .to_owned();

    // Seed the workspace with 20 indexable Rust source files so the indexer has
    // enough work to do that both concurrent index_workspace calls reliably overlap.
    // Each file contains a struct and a function — enough for tree-sitter to parse.
    // Per plan-review advisory S1: deterministic workspace sizing is preferred over
    // timing-based sleeps.
    for i in 0_u32..20 {
        let src = format!(
            "/// Auto-generated stub {i} for concurrent indexing test.\n\
             pub struct Stub{i} {{ pub value: u32 }}\n\
             /// Returns `x + {i}`.\n\
             pub fn stub_fn_{i}(x: u32) -> u32 {{ x.saturating_add({i}) }}\n"
        );
        std::fs::write(
            harness.workspace.path().join(format!("stub_{i:02}.rs")),
            src.as_bytes(),
        )
        .unwrap_or_else(|e| panic!("failed to write seed file {i}: {e}"));
    }

    // Establish workspace before issuing concurrent index calls.
    let set_req = IpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: Some(Value::Number(serde_json::Number::from(0_u32))),
        method: "set_workspace".to_owned(),
        params: Some(json!({ "path": workspace_path })),
    };
    let set_resp = send_request(&endpoint, &set_req, Duration::from_secs(30))
        .await
        .expect("IpcClient transport error on set_workspace");
    assert!(
        set_resp.error.is_none(),
        "set_workspace must succeed before indexing test: {:?}",
        set_resp.error
    );

    // Wait for the set_workspace auto-index to complete before issuing concurrent
    // index_workspace calls.  set_workspace triggers an auto-index that runs in
    // the background; if it still holds the indexing lock when both explicit
    // index_workspace calls arrive, both fail with 7003 instead of exactly one.
    // Poll get_workspace_status until all 20 seeded functions are indexed.
    let startup_deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let status_req = IpcRequest {
            jsonrpc: "2.0".to_owned(),
            id: Some(Value::Number(serde_json::Number::from(99_u32))),
            method: "get_workspace_status".to_owned(),
            params: None,
        };
        let resp = send_request(&endpoint, &status_req, Duration::from_secs(10))
            .await
            .expect("get_workspace_status transport must not fail");
        if resp.error.is_none() {
            let funcs = resp
                .result
                .as_ref()
                .and_then(|r| r["code_graph"]["functions"].as_u64())
                .unwrap_or(0);
            if funcs >= 20 {
                break;
            }
        }
        assert!(
            Instant::now() < startup_deadline,
            "timed out waiting for set_workspace auto-index to finish (20 functions expected)"
        );
        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    let barrier = Arc::new(Barrier::new(2));

    let tasks: Vec<_> = (1_u32..=2)
        .map(|i| {
            let ep = endpoint.clone();
            let b = Arc::clone(&barrier);
            tokio::spawn(async move {
                b.wait().await;
                let req = IpcRequest {
                    jsonrpc: "2.0".to_owned(),
                    id: Some(Value::Number(serde_json::Number::from(i))),
                    method: "index_workspace".to_owned(),
                    params: Some(json!({})),
                };
                send_request(&ep, &req, Duration::from_secs(60)).await
            })
        })
        .collect();

    let mut results = Vec::new();
    for (i, task) in tasks.into_iter().enumerate() {
        let req_id = u32::try_from(i + 1).expect("index fits u32");
        let resp = task
            .await
            .expect("concurrent indexing task must not panic")
            .unwrap_or_else(|e| panic!("index_workspace {i} must complete: {e}"));

        // Response id must echo the request id regardless of success or failure.
        assert_eq!(
            resp.id,
            Value::Number(serde_json::Number::from(req_id)),
            "index_workspace {i} response id must match request id {req_id}"
        );

        results.push(resp);
    }

    let error_count = results.iter().filter(|r| r.error.is_some()).count();

    // With a seeded workspace, indexing reliably takes long enough for the
    // concurrent call to arrive while the first is still in progress.
    // Exactly one call must receive IndexInProgress (code 7003).
    assert_eq!(
        error_count,
        1,
        "exactly one index_workspace call must fail with IndexInProgress; \
         got {error_count} errors out of {} responses: {results:?}",
        results.len()
    );

    // Verify the error carries the correct engram error code 7003.
    let err_resp = results
        .iter()
        .find(|r| r.error.is_some())
        .expect("error response must exist (asserted above)");
    let err = err_resp.error.as_ref().expect("error field must be Some");
    let engram_code = err
        .data
        .as_ref()
        .and_then(|d| d.get("engram_code"))
        .and_then(serde_json::Value::as_u64);
    assert_eq!(
        engram_code,
        Some(7003),
        "concurrent index must fail with IndexInProgress (7003), got: {err:?}"
    );
}
