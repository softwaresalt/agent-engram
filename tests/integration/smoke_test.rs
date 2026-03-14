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
        ws_result["task_count"].is_number(),
        "get_workspace_status must return task_count"
    );

    // ── Step 2: create_task ───────────────────────────────────────────────
    let task_result = send_ok(
        &endpoint,
        2,
        "create_task",
        Some(json!({
            "title": "smoke test task",
            "status": "todo"
        })),
    )
    .await;

    assert!(
        task_result["task_id"].is_string(),
        "create_task must return a task_id"
    );
    let task_id = task_result["task_id"]
        .as_str()
        .expect("task_id is a string");
    assert!(!task_id.is_empty(), "task_id must not be empty");
    assert_eq!(
        task_result["title"], "smoke test task",
        "task title must echo back"
    );

    // ── Step 3: get_ready_work ────────────────────────────────────────────
    let ready_result = send_ok(&endpoint, 3, "get_ready_work", Some(json!({}))).await;

    assert!(
        ready_result["tasks"].is_array(),
        "get_ready_work must return a tasks array"
    );
    let tasks = ready_result["tasks"].as_array().expect("tasks is array");
    assert!(
        !tasks.is_empty(),
        "get_ready_work must return at least one task"
    );

    let found = tasks
        .iter()
        .any(|t| t["title"].as_str() == Some("smoke test task"));
    assert!(found, "created task must appear in get_ready_work results");

    // ── Step 4: flush_state ───────────────────────────────────────────────
    let flush_result = send_ok(&endpoint, 4, "flush_state", Some(json!({}))).await;

    assert!(
        flush_result["files_written"].is_array(),
        "flush_state must return files_written array"
    );
    let files = flush_result["files_written"]
        .as_array()
        .expect("files_written is array");
    assert!(
        !files.is_empty(),
        "flush_state must write at least one file"
    );

    // Verify .engram/tasks.md was actually written to disk.
    let tasks_md = harness.workspace.path().join(".engram").join("tasks.md");
    assert!(
        tasks_md.exists(),
        "flush_state must create .engram/tasks.md on disk"
    );
    let tasks_md_content = std::fs::read_to_string(&tasks_md).expect("read tasks.md");
    assert!(
        tasks_md_content.contains("smoke test task"),
        "tasks.md must contain the created task title"
    );

    // ── Step 5: get_health_report ─────────────────────────────────────────
    let health_result = send_ok(&endpoint, 5, "get_health_report", Some(json!({}))).await;

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
        tool_count >= 4,
        "tool_call_count must be >= 4 after our calls, got {tool_count}"
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

    // ── Step 6: _shutdown ─────────────────────────────────────────────────
    let shutdown_request = make_request(6, "_shutdown", None);
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
