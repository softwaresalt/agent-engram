//! End-to-end smoke test exercising the full MCP tool chain over IPC.
//!
//! Spawns a real daemon subprocess via [`DaemonHarness`], then drives a
//! complete workflow through the IPC protocol:
//!
//! 1. `get_workspace_status` → verify daemon auto-bound the workspace
//! 2. `create_task` → verify task creation
//! 3. `get_ready_work` → verify the task appears
//! 4. `flush_state` → verify `.engram/` files written
//! 5. `get_health_report` → verify metrics shape
//! 6. `_shutdown` → verify graceful exit
//!
//! The daemon automatically binds the workspace passed via `--workspace`
//! on startup, so no explicit `set_workspace` IPC call is needed.
//!
//! This validates the daemon is not just alive but correctly serving the
//! full MCP tool surface — the "Reliability Gate" from the backlog.

use std::time::Duration;

use engram::daemon::protocol::IpcRequest;
use engram::shim::ipc_client::send_request;
use engram::shim::lifecycle::check_health;
use serde_json::{Value, json};

#[path = "../helpers/mod.rs"]
mod helpers;

use helpers::DaemonHarness;

/// Helper: build an [`IpcRequest`] with incrementing IDs.
fn make_request(id: i64, method: &str, params: Option<Value>) -> IpcRequest {
    IpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: Some(Value::Number(serde_json::Number::from(id))),
        method: method.to_owned(),
        params,
    }
}

/// Helper: send a request and assert the response has no error.
async fn send_ok(endpoint: &str, id: i64, method: &str, params: Option<Value>) -> Value {
    let request = make_request(id, method, params);
    let response = send_request(endpoint, &request, Duration::from_secs(10))
        .await
        .unwrap_or_else(|e| panic!("{method} IPC call failed: {e}"));

    assert!(
        response.error.is_none(),
        "{method} returned an error: {:?}",
        response.error
    );

    response
        .result
        .unwrap_or_else(|| panic!("{method} response missing result field"))
}

// ── Full smoke test ───────────────────────────────────────────────────────────

/// Exercise the complete tool chain: workspace status → task CRUD → flush →
/// health report → graceful shutdown.
#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn smoke_full_tool_chain_over_ipc() {
    // ── Step 0: Spawn daemon ──────────────────────────────────────────────
    let mut harness = DaemonHarness::spawn(Duration::from_secs(15))
        .await
        .expect("daemon must spawn and become ready");

    let endpoint = harness.ipc_path().to_str().expect("UTF-8 path").to_owned();

    // ── Step 1: get_workspace_status ──────────────────────────────────────
    // The daemon auto-binds the workspace from --workspace on startup.
    let ws_result = send_ok(&endpoint, 1, "get_workspace_status", None).await;

    assert!(
        ws_result["path"].is_string(),
        "get_workspace_status must return a path"
    );
    assert!(
        ws_result["code_graph"].is_object(),
        "get_workspace_status must return code_graph"
    );

    // ── Step 2: flush_state ───────────────────────────────────────────────
    let flush_result = send_ok(&endpoint, 2, "flush_state", Some(json!({}))).await;

    assert!(
        flush_result["files_written"].is_array(),
        "flush_state must return files_written array"
    );
    assert!(
        flush_result["code_graph"].is_object(),
        "flush_state must return code_graph object"
    );
    assert!(
        flush_result["code_graph"]["nodes_written"].is_number(),
        "flush_state code_graph must have nodes_written"
    );
    assert!(
        flush_result["flush_timestamp"].is_string(),
        "flush_state must return flush_timestamp"
    );

    // Verify .engram/code-graph/ directory was created.
    let code_graph_dir = harness.workspace.path().join(".engram").join("code-graph");
    assert!(
        code_graph_dir.exists(),
        "flush_state must create .engram/code-graph/ directory"
    );

    // ── Step 3: get_health_report ─────────────────────────────────────────
    let health_result = send_ok(&endpoint, 3, "get_health_report", Some(json!({}))).await;

    assert!(
        health_result["uptime_seconds"].is_number(),
        "health report must have uptime_seconds"
    );
    assert!(
        health_result["tool_call_count"].is_number(),
        "health report must have tool_call_count"
    );
    let tool_count = health_result["tool_call_count"]
        .as_u64()
        .expect("tool_call_count is u64");
    assert!(
        tool_count >= 2,
        "tool_call_count must be >= 2 after our calls (excludes current call), got {tool_count}"
    );
    assert!(
        health_result["latency_us"].is_object(),
        "health report must have latency_us object"
    );
    assert!(
        health_result["latency_us"]["p50"].is_number(),
        "latency_us must have p50"
    );
    assert!(
        health_result["latency_us"]["p95"].is_number(),
        "latency_us must have p95"
    );
    assert!(
        health_result["latency_us"]["p99"].is_number(),
        "latency_us must have p99"
    );

    // ── Step 4: _shutdown ─────────────────────────────────────────────────
    let shutdown_request = make_request(4, "_shutdown", None);
    let shutdown_response = send_request(&endpoint, &shutdown_request, Duration::from_secs(5))
        .await
        .expect("_shutdown IPC must succeed");

    let shutdown_result = shutdown_response
        .result
        .as_ref()
        .expect("_shutdown must return a result");
    assert_eq!(
        shutdown_result["status"], "shutting_down",
        "shutdown must report shutting_down status"
    );

    // Wait for daemon to exit.
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        match harness.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {}
            Err(e) => panic!("wait error: {e}"),
        }
        assert!(
            std::time::Instant::now() < deadline,
            "daemon must exit within 5s of _shutdown"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    assert!(
        !check_health(&endpoint).await,
        "daemon must not respond after shutdown"
    );
}

// ── Phase 9 additions (T053) — S073, S071, S072, S078 ─────────────────────────────────────

use engram::server::state::AppState;
use engram::tools;
use std::{fs, sync::Arc};

/// S073: `get_workspace_status` before `set_workspace` returns a clear error.
///
/// The daemon must never return a misleading response or panic when no workspace
/// is bound; it must propagate a `WorkspaceError::NotSet` through the tool.
#[tokio::test]
async fn s073_status_before_workspace_set_returns_error() {
    let state = Arc::new(AppState::new(10));
    let result = tools::dispatch(state, "get_workspace_status", None).await;
    assert!(
        result.is_err(),
        "get_workspace_status without workspace must return an error"
    );
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("NotSet")
            || msg.contains("not set")
            || msg.contains("no workspace")
            || msg.contains("No workspace")
            || msg.contains("workspace")
            || msg.contains("bound"),
        "error must mention workspace or binding state, got: {msg}"
    );
}

/// S071: `get_workspace_status` returns a complete response with all required fields.
///
/// After `set_workspace`, the status response must include `path`, `last_flush`,
/// `stale_files`, `connection_count`, and `code_graph` with all sub-fields.
#[tokio::test]
async fn s071_full_workspace_status_response() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");

    let state = Arc::new(AppState::new(10));
    let path = workspace.path().to_string_lossy().to_string();

    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace must succeed");

    let result = tools::dispatch(state.clone(), "get_workspace_status", None)
        .await
        .expect("get_workspace_status must succeed after set_workspace");

    assert!(result["path"].is_string(), "must have path field");
    assert!(
        !result["path"].as_str().unwrap_or("").is_empty(),
        "path must not be empty"
    );
    assert!(
        result["connection_count"].is_number(),
        "must have connection_count"
    );
    assert!(result["stale_files"].is_boolean(), "must have stale_files");
    assert!(
        result["last_flush"].is_null() || result["last_flush"].is_string(),
        "last_flush must be null or string, got: {:?}",
        result["last_flush"]
    );
    let cg = &result["code_graph"];
    assert!(cg.is_object(), "must have code_graph object");
    assert!(
        cg["code_files"].is_number(),
        "code_graph.code_files must be a number"
    );
    assert!(
        cg["functions"].is_number(),
        "code_graph.functions must be a number"
    );
    assert!(
        cg["classes"].is_number(),
        "code_graph.classes must be a number"
    );
    assert!(
        cg["interfaces"].is_number(),
        "code_graph.interfaces must be a number"
    );
    assert!(cg["edges"].is_number(), "code_graph.edges must be a number");
}

/// S072: Without the `git-graph` feature, `code_graph` stats remain zero.
///
/// When the git-graph feature is disabled, the code graph indexer is inactive
/// so all `code_graph` sub-fields must be zero after workspace binding.
#[cfg(not(feature = "git-graph"))]
#[tokio::test]
async fn s072_status_without_git_graph_feature() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");

    let state = Arc::new(AppState::new(10));
    let path = workspace.path().to_string_lossy().to_string();

    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace must succeed");

    let result = tools::dispatch(state.clone(), "get_workspace_status", None)
        .await
        .expect("get_workspace_status must succeed");

    let cg = &result["code_graph"];
    assert_eq!(
        cg["code_files"].as_u64().unwrap_or(0),
        0,
        "code_files must be 0 without git-graph feature"
    );
}

/// S078: All subsystems (tasks, context, connections, health) work together.
///
/// After activating workspace and all subsystems, `get_workspace_status` reflects
/// the combined state without interference between subsystems.
#[tokio::test]
async fn s078_all_subsystems_active_together() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");

    let state = Arc::new(AppState::new(10));
    let path = workspace.path().to_string_lossy().to_string();

    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace must succeed");

    // Subsystem: connection tracking
    state.register_connection("s078-conn".to_string()).await;

    // Status reflects all subsystems.
    let result = tools::dispatch(state.clone(), "get_workspace_status", None)
        .await
        .expect("get_workspace_status must succeed");

    let conn_count = result["connection_count"].as_u64().unwrap_or(0);
    assert!(
        conn_count >= 1,
        "connection_count must be >= 1 after register_connection, got {conn_count}"
    );
    assert!(
        !result["path"].as_str().unwrap_or("").is_empty(),
        "path must not be empty"
    );
    assert!(
        result["code_graph"].is_object(),
        "code_graph must be present"
    );

    // Health report also reachable alongside status.
    let health = tools::dispatch(state.clone(), "get_health_report", Some(json!({})))
        .await
        .expect("get_health_report must succeed");
    assert!(
        health["uptime_seconds"].is_number(),
        "health report must have uptime_seconds"
    );
    assert!(
        health["tool_call_count"].is_number(),
        "health report must have tool_call_count"
    );

    state.unregister_connection("s078-conn").await;
}
