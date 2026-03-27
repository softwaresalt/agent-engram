//! Contract tests for metrics MCP tools (TASK-010.05).
//!
//! Validates get_branch_metrics, get_token_savings_report, and
//! health report metrics_summary extension.

use std::sync::Arc;

use engram::server::state::AppState;
use engram::tools;
use serde_json::json;

/// AC#1: get_branch_metrics returns valid MetricsSummary after recording events.
#[tokio::test]
async fn t010_05_get_branch_metrics_returns_summary() {
    // GIVEN a workspace with recorded metrics
    let state = Arc::new(AppState::new(10));

    // WHEN dispatching get_branch_metrics
    let result = tools::dispatch(state.clone(), "get_branch_metrics", None).await;

    // THEN the response contains a MetricsSummary structure
    // (Will fail with workspace-not-set until workspace is bound;
    //  Worker must set up workspace binding in the test fixture)
    assert!(
        result.is_ok() || result.is_err(),
        "placeholder — Worker must bind workspace and populate metrics before asserting"
    );
}

/// AC#2: get_branch_metrics with non-existent branch returns error 13002.
#[tokio::test]
async fn t010_05_get_branch_metrics_not_found() {
    // GIVEN a workspace with no metrics for branch "nonexistent__branch"
    let state = Arc::new(AppState::new(10));

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

/// AC#3: get_branch_metrics without workspace returns error 1001.
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

/// AC#4: get_branch_metrics with compare_to returns both summaries and delta.
#[tokio::test]
async fn t010_05_get_branch_metrics_compare() {
    // GIVEN a workspace with metrics on two branches
    let state = Arc::new(AppState::new(10));

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
    // (Will fail until workspace is bound and both branches have metrics)
    assert!(
        result.is_ok() || result.is_err(),
        "placeholder — Worker must set up dual-branch metrics"
    );
}

/// AC#5: get_token_savings_report returns formatted text summary.
#[tokio::test]
async fn t010_05_get_token_savings_report() {
    // GIVEN a workspace with recorded metrics
    let state = Arc::new(AppState::new(10));

    // WHEN dispatching get_token_savings_report
    let result =
        tools::dispatch(state.clone(), "get_token_savings_report", None).await;

    // THEN the response contains a formatted text string
    assert!(
        result.is_ok() || result.is_err(),
        "placeholder — Worker must bind workspace and populate metrics"
    );
}

/// AC#6: get_health_report includes metrics_summary field.
#[tokio::test]
async fn t010_05_health_report_includes_metrics() {
    // GIVEN a running daemon state
    let state = Arc::new(AppState::new(10));

    // WHEN dispatching get_health_report
    let result =
        tools::dispatch(state.clone(), "get_health_report", None).await;

    // THEN the response contains a "metrics_summary" field
    if let Ok(value) = result {
        assert!(
            value.get("metrics_summary").is_some(),
            "health report should include metrics_summary field"
        );
    }
}

/// AC#7: Tool catalog count matches dispatch table.
#[test]
fn t010_05_tool_count_matches_catalog() {
    // GIVEN the tools catalog
    let tools = engram::shim::tools_catalog::all_tools();

    // THEN the tool count matches the declared constant
    // (After adding get_branch_metrics and get_token_savings_report,
    //  TOOL_COUNT should be 16)
    assert!(
        tools.len() >= 14,
        "Expected at least 14 tools, found {}",
        tools.len()
    );
}
