//! T071: Validate all quickstart.md scenarios against implemented tools.
//! T075: Startup failure smoke test for embedding model load failure (FR-154).
//!
//! This test suite verifies that:
//! - All code graph tools documented in quickstart.md exist in the dispatch table
//! - Each tool returns `WORKSPACE_NOT_SET` (1003) when called without a bound workspace
//! - The `ModelLoadFailed` error (FR-154) produces the correct response shape

use std::sync::Arc;

use serde_json::{Value, json};
use tokio::test;

use engram::errors::codes::{MODEL_LOAD_FAILED, WORKSPACE_NOT_SET};
use engram::errors::{EngramError, SystemError};
use engram::server::state::AppState;
use engram::tools;

// ---------------------------------------------------------------------------
// T071: Quickstart tool existence validation
// ---------------------------------------------------------------------------

/// Helper: dispatch a tool call with no workspace bound and expect
/// error 1003 (`WORKSPACE_NOT_SET`). This confirms the tool name is
/// registered in the dispatch table.
async fn assert_tool_exists(state: Arc<AppState>, method: &str, params: Option<Value>) {
    let result = tools::dispatch(state, method, params).await;
    let err = result.expect_err(&format!(
        "{method} should fail without workspace, proving it exists in dispatch"
    ));
    let resp = err.to_response();
    assert_eq!(
        resp.error.code, WORKSPACE_NOT_SET,
        "{method}: expected WORKSPACE_NOT_SET (1003), got {} ({})",
        resp.error.code, resp.error.name
    );
}

/// All code graph tools from quickstart.md exist and reject calls
/// when no workspace is bound.
#[test]
async fn t071_all_quickstart_tools_exist_in_dispatch() {
    let state = Arc::new(AppState::new(10));

    // US1: Indexing
    assert_tool_exists(
        state.clone(),
        "index_workspace",
        Some(json!({ "force": false })),
    )
    .await;

    // US3: Incremental sync
    assert_tool_exists(state.clone(), "sync_workspace", None).await;

    // US2: Map Code
    assert_tool_exists(
        state.clone(),
        "map_code",
        Some(json!({ "symbol_name": "dispatch", "depth": 2 })),
    )
    .await;

    // US2: List Symbols
    assert_tool_exists(state.clone(), "list_symbols", Some(json!({}))).await;

    // US4: Link Task to Code
    assert_tool_exists(
        state.clone(),
        "link_task_to_code",
        Some(json!({ "task_id": "task:abc", "symbol_name": "dispatch" })),
    )
    .await;

    // US4: Unlink Task from Code
    assert_tool_exists(
        state.clone(),
        "unlink_task_from_code",
        Some(json!({ "task_id": "task:abc", "symbol_name": "dispatch" })),
    )
    .await;

    // US4: Get Active Context
    assert_tool_exists(state.clone(), "get_active_context", None).await;

    // US5: Unified Search
    assert_tool_exists(
        state.clone(),
        "unified_search",
        Some(json!({ "query": "error handling" })),
    )
    .await;

    // US6: Impact Analysis
    assert_tool_exists(
        state.clone(),
        "impact_analysis",
        Some(json!({ "symbol_name": "dispatch" })),
    )
    .await;
}

/// Existing lifecycle and task tools from 001/002 specs also exist.
#[test]
async fn t071_core_tools_also_registered() {
    let state = Arc::new(AppState::new(10));

    // Lifecycle tools (don't need workspace — test differently)
    let daemon_status = tools::dispatch(state.clone(), "get_daemon_status", None)
        .await
        .expect("get_daemon_status should succeed without workspace");
    assert!(
        daemon_status.get("uptime_seconds").is_some(),
        "daemon status should include uptime_seconds"
    );

    // Task tools require workspace
    assert_tool_exists(
        state.clone(),
        "create_task",
        Some(json!({ "title": "test" })),
    )
    .await;
    assert_tool_exists(state.clone(), "flush_state", Some(json!({}))).await;
}

/// A tool name NOT in the dispatch table returns `InvalidParams` (5005).
#[test]
async fn t071_unknown_tool_returns_not_implemented() {
    let state = Arc::new(AppState::new(10));
    let result = tools::dispatch(state, "nonexistent_tool", None).await;
    let err = result.expect_err("unknown tool should error");
    let resp = err.to_response();
    assert_eq!(
        resp.error.code, 5005,
        "unknown tool should return InvalidParams (5005)"
    );
}

// ---------------------------------------------------------------------------
// T075: Startup failure smoke test (FR-154)
// ---------------------------------------------------------------------------

/// Verify that `ModelLoadFailed` produces error code 5006 with
/// `suggestion: "try restarting"` in the details object.
#[test]
async fn t075_model_load_failed_returns_suggestion() {
    let err = EngramError::System(SystemError::ModelLoadFailed {
        reason: "Failed to download bge-small-en-v1.5: connection refused".into(),
    });

    let resp = err.to_response();
    assert_eq!(resp.error.code, MODEL_LOAD_FAILED);
    assert_eq!(resp.error.name, "ModelLoadFailed");
    assert!(
        resp.error
            .message
            .contains("Embedding model failed to load"),
        "message should describe model load failure: {}",
        resp.error.message
    );

    // Verify suggestion field in details
    let details = resp
        .error
        .details
        .expect("ModelLoadFailed should have details");
    let suggestion = details
        .get("suggestion")
        .and_then(Value::as_str)
        .expect("details should contain suggestion");
    assert_eq!(suggestion, "try restarting");

    // Verify reason is forwarded
    let reason = details
        .get("reason")
        .and_then(Value::as_str)
        .expect("details should contain reason");
    assert!(reason.contains("bge-small-en-v1.5"));
}

/// Verify `ModelLoadFailed` JSON shape matches the error taxonomy
/// (code, name, message, details with suggestion).
#[test]
async fn t075_model_load_failed_json_shape() {
    let err = EngramError::System(SystemError::ModelLoadFailed {
        reason: "insufficient memory".into(),
    });

    let resp = err.to_response();
    let json = serde_json::to_value(&resp).expect("serialize ErrorResponse");

    let error_obj = json.get("error").expect("should have 'error' key");
    assert!(error_obj.get("code").and_then(Value::as_u64).is_some());
    assert!(error_obj.get("name").and_then(|v| v.as_str()).is_some());
    assert!(error_obj.get("message").and_then(|v| v.as_str()).is_some());
    assert!(error_obj.get("details").is_some());

    let details = error_obj.get("details").unwrap();
    assert!(
        details.get("suggestion").is_some(),
        "details must include suggestion"
    );
    assert!(
        details.get("reason").is_some(),
        "details must include reason"
    );
}
