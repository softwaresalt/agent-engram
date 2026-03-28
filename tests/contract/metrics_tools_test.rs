//! Contract tests for metrics MCP tools (TASK-010.05).
//!
//! Validates `get_branch_metrics`, `get_token_savings_report`, and
//! health report `metrics_summary` extension.

use std::collections::HashMap;
use std::io::Write;
use std::sync::Arc;

use engram::server::state::{AppState, WorkspaceSnapshot};
use engram::tools;
use serde_json::json;

async fn bind_workspace(state: &Arc<AppState>, path: &std::path::Path, branch: &str) {
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

fn write_usage_events(path: &std::path::Path, branch: &str, tool: &str, count: usize) {
    let metrics_dir = path.join(".engram").join("metrics").join(branch);
    std::fs::create_dir_all(&metrics_dir).unwrap_or_else(|e| panic!("create_dir failed: {e}"));
    let mut file = std::fs::File::create(metrics_dir.join("usage.jsonl"))
        .unwrap_or_else(|e| panic!("create file failed: {e}"));
    for index in 0..count {
        let line = serde_json::to_string(&json!({
            "tool_name": tool,
            "timestamp": format!("2026-03-27T12:{index:02}:00Z"),
            "response_bytes": 400_u64,
            "estimated_tokens": 100_u64,
            "symbols_returned": 1_u32,
            "results_returned": 1_u32,
            "branch": branch,
        }))
        .unwrap_or_else(|e| panic!("serialize failed: {e}"));
        writeln!(file, "{line}").unwrap_or_else(|e| panic!("write failed: {e}"));
    }
}

/// AC#1: `get_branch_metrics` returns valid `MetricsSummary` after recording events.
#[tokio::test]
async fn t010_05_get_branch_metrics_returns_summary() {
    // GIVEN a workspace with recorded metrics
    let state = Arc::new(AppState::new(10));
    let workspace = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir failed: {e}"));
    bind_workspace(&state, workspace.path(), "main").await;
    write_usage_events(workspace.path(), "main", "map_code", 3);

    // WHEN dispatching get_branch_metrics
    let result = tools::dispatch(state.clone(), "get_branch_metrics", None).await;

    // THEN the response contains a MetricsSummary structure
    let value = result.unwrap_or_else(|e| panic!("get_branch_metrics should succeed: {e}"));
    assert_eq!(value["branch_name"], "main");
    assert_eq!(value["summary"]["total_tool_calls"], 3);
}

/// AC#2: `get_branch_metrics` with non-existent branch returns error 13002.
#[tokio::test]
async fn t010_05_get_branch_metrics_not_found() {
    // GIVEN a workspace with no metrics for branch "nonexistent__branch"
    let state = Arc::new(AppState::new(10));
    let workspace = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir failed: {e}"));
    bind_workspace(&state, workspace.path(), "main").await;
    write_usage_events(workspace.path(), "main", "map_code", 1);

    // WHEN dispatching get_branch_metrics with a non-existent branch
    let result = tools::dispatch(
        state.clone(),
        "get_branch_metrics",
        Some(json!({ "branch_name": "nonexistent__branch" })),
    )
    .await;

    // THEN the error code is 13002 (METRICS_NOT_FOUND)
    let err = result.expect_err("should fail for non-existent branch");
    let response = err.to_response();
    assert_eq!(
        response.error.code, 13_002,
        "Expected METRICS_NOT_FOUND (13002)"
    );
}

/// AC#3: `get_branch_metrics` without workspace returns error 1001.
#[tokio::test]
async fn t010_05_get_branch_metrics_no_workspace() {
    // GIVEN no workspace bound
    let state = Arc::new(AppState::new(10));

    // WHEN dispatching get_branch_metrics
    let result = tools::dispatch(state.clone(), "get_branch_metrics", None).await;

    // THEN the error code is 1003 (WORKSPACE_NOT_SET)
    let err = result.expect_err("should fail without workspace");
    let response = err.to_response();
    assert_eq!(
        response.error.code, 1003,
        "Expected WORKSPACE_NOT_SET (1003)"
    );
}

/// AC#4: `get_branch_metrics` with `compare_to` returns both summaries and delta.
#[tokio::test]
async fn t010_05_get_branch_metrics_compare() {
    // GIVEN a workspace with metrics on two branches
    let state = Arc::new(AppState::new(10));
    let workspace = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir failed: {e}"));
    bind_workspace(&state, workspace.path(), "main").await;
    write_usage_events(workspace.path(), "main", "map_code", 3);
    write_usage_events(workspace.path(), "feature__auth", "list_symbols", 2);

    // WHEN dispatching get_branch_metrics with compare_to
    let result = tools::dispatch(
        state.clone(),
        "get_branch_metrics",
        Some(json!({
            "branch_name": "main",
            "compare_to": "feature__auth"
        })),
    )
    .await;

    // THEN the response contains both summaries
    let value = result.unwrap_or_else(|e| panic!("compare request should succeed: {e}"));
    assert_eq!(value["summary"]["total_tool_calls"], 3);
    assert_eq!(value["comparison"]["summary"]["total_tool_calls"], 2);
    assert!(
        value.get("delta").is_some(),
        "comparison response should include delta"
    );
}

/// AC#5: `get_token_savings_report` returns formatted text summary.
#[tokio::test]
async fn t010_05_get_token_savings_report() {
    // GIVEN a workspace with recorded metrics
    let state = Arc::new(AppState::new(10));
    let workspace = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir failed: {e}"));
    bind_workspace(&state, workspace.path(), "main").await;
    write_usage_events(workspace.path(), "main", "impact_analysis", 2);

    // WHEN dispatching get_token_savings_report
    let result = tools::dispatch(state.clone(), "get_token_savings_report", None).await;

    // THEN the response contains a formatted text string
    let value = result.unwrap_or_else(|e| panic!("report should succeed: {e}"));
    let report = value["report"]
        .as_str()
        .unwrap_or_else(|| panic!("report field should be a string"));
    assert!(report.contains("On branch main"));
    assert!(report.contains("tool calls"));
}

/// AC#6: `get_health_report` includes `metrics_summary` field.
#[tokio::test]
async fn t010_05_health_report_includes_metrics() {
    // GIVEN a running daemon state
    let state = Arc::new(AppState::new(10));
    let workspace = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir failed: {e}"));
    bind_workspace(&state, workspace.path(), "main").await;
    write_usage_events(workspace.path(), "main", "map_code", 1);

    // WHEN dispatching get_health_report
    let result = tools::dispatch(state.clone(), "get_health_report", None).await;

    // THEN the response contains a "metrics_summary" field
    let value = result.unwrap_or_else(|e| panic!("health report should succeed: {e}"));
    assert!(
        value.get("metrics_summary").is_some(),
        "health report should include metrics_summary field"
    );
    assert_eq!(value["metrics_summary"]["summary"]["total_tool_calls"], 1);
}

/// AC#7: Tool catalog count matches dispatch table.
#[test]
fn t010_05_tool_count_matches_catalog() {
    // GIVEN the tools catalog
    let tools = engram::shim::tools_catalog::all_tools();

    // THEN the tool count matches the declared constant
    // (After adding get_branch_metrics and get_token_savings_report,
    //  TOOL_COUNT should be 16)
    assert_eq!(tools.len(), 16, "Expected 16 tools after metrics additions");
}
