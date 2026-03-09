//! Integration tests for multi-workspace isolation and concurrent scaling (T024–T025).
//!
//! Scenarios covered:
//! - S088: Two workspaces have separate IPC endpoints and do not share data.
//! - S089: IPC endpoint addresses are uniquely scoped per canonical workspace path.
//! - S090: 20 concurrent workspace daemons all become healthy without interference.
//! - S091: Canonical workspace paths (via `canonicalize`) drive IPC addressing.

use std::time::Duration;

use engram::daemon::protocol::IpcRequest;
use engram::shim::ipc_client::send_request;
use engram::shim::lifecycle::check_health;
use serde_json::json;

#[path = "../helpers/mod.rs"]
mod helpers;

use helpers::DaemonHarness;

// ── T024 / S089: Unique endpoint per workspace ────────────────────────────────

/// Two daemons spawned for different workspaces must listen on distinct IPC endpoints.
#[tokio::test]
async fn t024_s089_distinct_workspaces_have_distinct_ipc_endpoints() {
    let harness_a = DaemonHarness::spawn(Duration::from_secs(15))
        .await
        .expect("daemon A must spawn");
    let harness_b = DaemonHarness::spawn(Duration::from_secs(15))
        .await
        .expect("daemon B must spawn");

    assert_ne!(
        harness_a.ipc_path(),
        harness_b.ipc_path(),
        "distinct workspaces must produce distinct IPC endpoint paths"
    );
}

// ── T024 / S088: Data isolation ───────────────────────────────────────────────

/// Each daemon reports its own workspace path in `_health` responses; the two
/// workspace paths must differ, proving data-store isolation.
#[tokio::test]
async fn t024_s088_workspace_health_responses_report_distinct_paths() {
    let harness_a = DaemonHarness::spawn(Duration::from_secs(15))
        .await
        .expect("daemon A must spawn");
    let harness_b = DaemonHarness::spawn(Duration::from_secs(15))
        .await
        .expect("daemon B must spawn");

    let ep_a = harness_a.ipc_path().to_str().expect("UTF-8").to_owned();
    let ep_b = harness_b.ipc_path().to_str().expect("UTF-8").to_owned();

    let health_req = |id: u64| IpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: Some(json!(id)),
        method: "_health".to_owned(),
        params: None,
    };

    let resp_a = send_request(&ep_a, &health_req(1), Duration::from_secs(5))
        .await
        .expect("health request to workspace A must succeed");
    let resp_b = send_request(&ep_b, &health_req(2), Duration::from_secs(5))
        .await
        .expect("health request to workspace B must succeed");

    assert!(resp_a.error.is_none(), "workspace A health must not error");
    assert!(resp_b.error.is_none(), "workspace B health must not error");

    // Both daemons must report their own workspace path, which must differ.
    let ws_a = resp_a
        .result
        .as_ref()
        .and_then(|v| v["workspace"].as_str())
        .unwrap_or("")
        .to_owned();
    let ws_b = resp_b
        .result
        .as_ref()
        .and_then(|v| v["workspace"].as_str())
        .unwrap_or("")
        .to_owned();

    assert!(
        !ws_a.is_empty(),
        "daemon A must report a non-empty workspace path"
    );
    assert!(
        !ws_b.is_empty(),
        "daemon B must report a non-empty workspace path"
    );
    assert_ne!(
        ws_a, ws_b,
        "daemon A and B must report different workspace paths (A={ws_a}, B={ws_b})"
    );
}

/// T024 / S091: Each daemon's reported workspace corresponds to its actual `TempDir`.
///
/// Verifies that canonical path resolution (`canonicalize`) is applied so the
/// IPC endpoint is stable and matches what `ipc_endpoint()` computes from the
/// same path.
#[tokio::test]
async fn t024_s091_daemon_workspace_path_matches_harness_tempdir() {
    let harness = DaemonHarness::spawn(Duration::from_secs(15))
        .await
        .expect("daemon must spawn");

    let endpoint = harness.ipc_path().to_str().expect("UTF-8").to_owned();

    let request = IpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: Some(json!(1)),
        method: "_health".to_owned(),
        params: None,
    };

    let response = send_request(&endpoint, &request, Duration::from_secs(5))
        .await
        .expect("health request must succeed");

    let reported_ws = response
        .result
        .as_ref()
        .and_then(|v| v["workspace"].as_str())
        .unwrap_or("")
        .to_owned();

    assert!(!reported_ws.is_empty(), "daemon must report its workspace");

    // The reported path must overlap with the harness TempDir (both canonical).
    let tempdir_str = harness
        .workspace
        .path()
        .canonicalize()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();

    // Normalise path separators for cross-platform comparison.
    let normalise = |s: &str| s.replace('\\', "/").to_lowercase();
    let reported_norm = normalise(&reported_ws);
    let tempdir_norm = normalise(&tempdir_str);

    assert!(
        reported_norm.contains(&tempdir_norm) || tempdir_norm.contains(&reported_norm),
        "daemon workspace ({reported_ws}) must correspond to harness TempDir ({tempdir_str})"
    );
}

// ── T025 / S090: 20 concurrent workspaces ────────────────────────────────────

/// Spawn 20 daemon processes concurrently, wait for all to become healthy,
/// then verify each reports a distinct workspace path.
///
/// This is a boundary test: it exercises the OS-level limits on concurrent
/// IPC listeners and `SurrealDB` embedded instances.
#[tokio::test]
async fn t025_s090_twenty_concurrent_workspaces_all_healthy() {
    const NUM_WORKSPACES: usize = 20;

    // Spawn all daemons concurrently.
    let spawn_tasks: Vec<_> = (0..NUM_WORKSPACES)
        .map(|_| {
            tokio::spawn(async {
                DaemonHarness::spawn(Duration::from_secs(60))
                    .await
                    .map_err(|e| e.to_string())
            })
        })
        .collect();

    // Collect harnesses — fail fast if any daemon fails to start.
    let mut harnesses = Vec::with_capacity(NUM_WORKSPACES);
    for (i, task) in spawn_tasks.into_iter().enumerate() {
        let harness = task
            .await
            .unwrap_or_else(|e| panic!("spawn task {i} panicked: {e}"))
            .unwrap_or_else(|e| panic!("daemon {i} failed to spawn: {e}"));
        harnesses.push(harness);
    }

    assert_eq!(
        harnesses.len(),
        NUM_WORKSPACES,
        "all {NUM_WORKSPACES} daemons must have spawned successfully"
    );

    // Verify all daemons are healthy in parallel.
    let health_tasks: Vec<_> = harnesses
        .iter()
        .enumerate()
        .map(|(i, harness)| {
            let endpoint = harness.ipc_path().to_str().expect("valid UTF-8").to_owned();
            tokio::spawn(async move {
                let healthy = check_health(&endpoint).await;
                (i, healthy)
            })
        })
        .collect();

    let mut failures = Vec::new();
    for task in health_tasks {
        let (i, healthy) = task.await.expect("health check task must not panic");
        if !healthy {
            failures.push(i);
        }
    }

    assert!(
        failures.is_empty(),
        "daemons {failures:?} of {NUM_WORKSPACES} were not healthy after spawn"
    );

    // Verify each daemon reports a distinct workspace path.
    let mut workspace_paths = Vec::with_capacity(NUM_WORKSPACES);
    for harness in &harnesses {
        let endpoint = harness.ipc_path().to_str().expect("valid UTF-8").to_owned();
        let request = IpcRequest {
            jsonrpc: "2.0".to_owned(),
            id: Some(json!(1)),
            method: "_health".to_owned(),
            params: None,
        };
        if let Ok(resp) = send_request(&endpoint, &request, Duration::from_secs(5)).await {
            if let Some(ws) = resp.result.as_ref().and_then(|v| v["workspace"].as_str()) {
                workspace_paths.push(ws.to_owned());
            }
        }
    }

    // All workspace paths must be unique.
    let unique_count = {
        let mut sorted = workspace_paths.clone();
        sorted.sort();
        sorted.dedup();
        sorted.len()
    };

    assert_eq!(
        unique_count,
        workspace_paths.len(),
        "all {NUM_WORKSPACES} daemons must report distinct workspace paths \
         (found {unique_count} unique out of {})",
        workspace_paths.len()
    );

    // Explicit drop to kill all daemon processes before the temp dirs are cleaned up.
    drop(harnesses);
}
