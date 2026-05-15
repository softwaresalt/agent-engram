//! Contract tests for dispatch metrics instrumentation (TASK-010.03).

use std::collections::HashMap;
use std::sync::Arc;

use engram::server::state::{AppState, WorkspaceSnapshot};
use engram::tools;
use serde_json::json;

async fn bind_test_workspace(state: &Arc<AppState>, path: &std::path::Path, branch: &str) {
    let snapshot = WorkspaceSnapshot {
        workspace_id: format!("workspace-{branch}"),
        workspace_uuid: format!("workspace-uuid-{branch}"),
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

/// AC#1: dispatch records the expanded telemetry envelope for read tools.
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

    // THEN a UsageEvent was recorded with the expanded telemetry envelope
    let recent = engram::services::metrics::recent_events();
    let event = recent
        .last()
        .unwrap_or_else(|| panic!("expected a recorded metrics event"));
    assert_eq!(event.tool_name, "list_symbols");
    assert!(event.request_bytes > 0);
    assert!(event.response_bytes > 0);
    assert_eq!(event.estimated_input_tokens, event.request_bytes / 4);
    assert_eq!(event.estimated_output_tokens, event.response_bytes / 4);
    assert_eq!(event.result_count, event.results_returned);
    assert_eq!(
        event.response_shape_counts.get("symbols"),
        Some(&event.symbols_returned)
    );
}

/// AC#2: workspace-oriented tools emit the same telemetry envelope.
#[tokio::test]
async fn t010_03_dispatch_records_usage_event_for_workspace_tools() {
    // GIVEN a minimal AppState
    let state = Arc::new(AppState::new(10));
    let workspace = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir failed: {e}"));
    bind_test_workspace(&state, workspace.path(), "main").await;
    engram::services::metrics::clear_recent_events();

    // WHEN dispatching workspace-oriented tools
    let workspace_result = tools::dispatch(state.clone(), "get_workspace_status", None).await;
    workspace_result.unwrap_or_else(|e| panic!("get_workspace_status should succeed: {e}"));

    let daemon_result = tools::dispatch(state.clone(), "get_daemon_status", None).await;
    daemon_result.unwrap_or_else(|e| panic!("get_daemon_status should succeed: {e}"));

    // THEN a workspace UsageEvent was recorded with workspace-specific shape counts
    let recent = engram::services::metrics::recent_events();
    let workspace_event = recent
        .iter()
        .find(|event| event.tool_name == "get_workspace_status")
        .unwrap_or_else(|| panic!("expected a recorded workspace metrics event"));
    assert_eq!(workspace_event.result_count, 1);
    assert!(workspace_event.response_bytes > 0);
    assert!(
        workspace_event
            .response_shape_counts
            .contains_key("code_files")
    );
    assert!(
        workspace_event
            .response_shape_counts
            .contains_key("functions")
    );
    assert!(
        workspace_event
            .response_shape_counts
            .contains_key("classes")
    );
    assert!(
        workspace_event
            .response_shape_counts
            .contains_key("interfaces")
    );
    assert!(workspace_event.response_shape_counts.contains_key("edges"));
    assert!(
        workspace_event
            .response_shape_counts
            .contains_key("scan_status")
    );
    assert!(!workspace_event.response_shape_counts.contains_key("checks"));

    // AND a daemon UsageEvent still records daemon-specific health counts
    let daemon_event = recent
        .iter()
        .find(|event| event.tool_name == "get_daemon_status")
        .unwrap_or_else(|| panic!("expected a recorded daemon metrics event"));
    assert_eq!(daemon_event.result_count, 1);
    assert!(daemon_event.response_bytes > 0);
    assert!(daemon_event.response_shape_counts.contains_key("checks"));
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

/// AC#4: health report telemetry reflects the actual flat response shape.
#[tokio::test]
async fn t010_03_health_report_records_section_counts() {
    // GIVEN a minimal AppState with a workspace bound
    let state = Arc::new(AppState::new(10));
    let workspace = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir failed: {e}"));
    bind_test_workspace(&state, workspace.path(), "main").await;
    engram::services::metrics::clear_recent_events();

    // WHEN dispatching the health report tool
    let result = tools::dispatch(state.clone(), "get_health_report", None).await;
    result.unwrap_or_else(|e| panic!("get_health_report should succeed: {e}"));

    // THEN the recorded UsageEvent reflects the flat response object, not daemon checks
    let recent = engram::services::metrics::recent_events();
    let event = recent
        .iter()
        .find(|event| event.tool_name == "get_health_report")
        .unwrap_or_else(|| panic!("expected a recorded health report metrics event"));
    assert_eq!(event.result_count, 1);
    assert!(event.response_bytes > 0);
    assert!(event.response_shape_counts.contains_key("sections"));
    assert!(!event.response_shape_counts.contains_key("checks"));
}
