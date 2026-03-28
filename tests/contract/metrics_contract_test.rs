//! Contract tests for dispatch metrics instrumentation (TASK-010.03).

use std::collections::HashMap;
use std::sync::Arc;

use engram::server::state::{AppState, WorkspaceSnapshot};
use engram::tools;
use serde_json::json;

async fn bind_test_workspace(state: &Arc<AppState>, path: &std::path::Path, branch: &str) {
    let snapshot = WorkspaceSnapshot {
        workspace_id: format!("workspace-{branch}"),
        branch: branch.to_owned(),
        data_dir: path.join(".engram"),
        path: path.display().to_string(),
        last_flush: None,
        stale_files: false,
        connection_count: 0,
        file_mtimes: HashMap::new(),
    };
    state
        .set_workspace(snapshot)
        .await
        .expect("workspace should bind");
}

/// AC#1: dispatch records a `UsageEvent` with correct `tool_name` and
/// non-zero `response_bytes` for read tools.
#[tokio::test]
async fn t010_03_dispatch_records_usage_event_for_read_tools() {
    // GIVEN a minimal AppState with a workspace bound
    let state = Arc::new(AppState::new(10));
    let workspace = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir failed: {e}"));
    bind_test_workspace(&state, workspace.path(), "main").await;
    engram::services::metrics::clear_recent_events();

    // WHEN dispatching a read tool (list_symbols)
    let result = tools::dispatch(
        state.clone(),
        "list_symbols",
        Some(json!({ "name_contains": "test" })),
    )
    .await;
    result.unwrap_or_else(|e| panic!("list_symbols should succeed for empty DB: {e}"));

    // THEN a UsageEvent was recorded with tool_name = "list_symbols"
    // and response_bytes > 0
    let recent = engram::services::metrics::recent_events();
    let event = recent
        .last()
        .unwrap_or_else(|| panic!("expected a recorded metrics event"));
    assert_eq!(event.tool_name, "list_symbols");
    assert!(event.response_bytes > 0);
}

/// AC#2: dispatch does NOT record a `UsageEvent` for lifecycle/write tools.
#[tokio::test]
async fn t010_03_dispatch_skips_lifecycle_tools() {
    // GIVEN a minimal AppState
    let state = Arc::new(AppState::new(10));
    engram::services::metrics::clear_recent_events();

    // WHEN dispatching a lifecycle tool (get_daemon_status)
    let result = tools::dispatch(state.clone(), "get_daemon_status", None).await;
    result.unwrap_or_else(|e| panic!("get_daemon_status should succeed: {e}"));

    // THEN no UsageEvent was recorded
    assert!(
        engram::services::metrics::recent_events().is_empty(),
        "lifecycle tools should not record metrics events"
    );
}

/// AC#3: `estimated_tokens` equals `response_bytes` / 4.
#[tokio::test]
async fn t010_03_estimated_tokens_equals_bytes_div_4() {
    // GIVEN a tool response of known byte size
    let state = Arc::new(AppState::new(10));
    let workspace = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir failed: {e}"));
    bind_test_workspace(&state, workspace.path(), "main").await;
    engram::services::metrics::clear_recent_events();

    // WHEN dispatching a read tool
    let result = tools::dispatch(state.clone(), "list_symbols", Some(json!({ "limit": 1 }))).await;
    result.unwrap_or_else(|e| panic!("list_symbols should succeed for empty DB: {e}"));

    // THEN the recorded UsageEvent has estimated_tokens == response_bytes / 4
    let recent = engram::services::metrics::recent_events();
    let event = recent
        .last()
        .unwrap_or_else(|| panic!("expected a recorded metrics event"));
    assert_eq!(event.estimated_tokens, event.response_bytes / 4);
}
