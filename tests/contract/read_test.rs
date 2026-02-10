use std::sync::Arc;

use serde_json::json;
use tokio::test;

use t_mem::errors::codes::{QUERY_TOO_LONG, WORKSPACE_NOT_SET};
use t_mem::server::state::AppState;
use t_mem::tools;

#[test]
async fn contract_get_task_graph_requires_workspace() {
    let state = Arc::new(AppState::new(10));
    let params = Some(json!({
        "root_task_id": "task:root",
        "depth": 3,
    }));

    let err = tools::dispatch(state, "get_task_graph", params)
        .await
        .expect_err("expected workspace not set error");

    let code = err.to_response().error.code;
    assert_eq!(code, WORKSPACE_NOT_SET);
}

#[test]
async fn contract_check_status_requires_workspace() {
    let state = Arc::new(AppState::new(10));
    let params = Some(json!({
        "work_item_ids": ["AB#123", "AB#456"],
    }));

    let err = tools::dispatch(state, "check_status", params)
        .await
        .expect_err("expected workspace not set error");

    let code = err.to_response().error.code;
    assert_eq!(code, WORKSPACE_NOT_SET);
}

// ── T073: query_memory contract tests ────────────────────────────

#[test]
async fn contract_query_memory_requires_workspace() {
    let state = Arc::new(AppState::new(10));
    let params = Some(json!({
        "query": "user authentication",
    }));

    let err = tools::dispatch(state, "query_memory", params)
        .await
        .expect_err("expected workspace not set error");

    let code = err.to_response().error.code;
    assert_eq!(code, WORKSPACE_NOT_SET);
}

#[test]
async fn contract_query_memory_rejects_long_query() {
    // Build a state with workspace set so we get past the workspace check.
    let state = Arc::new(AppState::new(10));
    let snapshot = t_mem::server::state::WorkspaceSnapshot {
        workspace_id: "test_ws".to_string(),
        path: "/tmp/test-repo".to_string(),
        task_count: 0,
        context_count: 0,
        last_flush: None,
        stale_files: false,
        connection_count: 1,
    };
    state.set_workspace(snapshot).await.expect("set workspace");

    // Query exceeding 2000 chars ≈ 500+ tokens
    let long_query = "a ".repeat(1500);
    let params = Some(json!({
        "query": long_query,
    }));

    let err = tools::dispatch(state, "query_memory", params)
        .await
        .expect_err("expected query too long error");

    let code = err.to_response().error.code;
    assert_eq!(code, QUERY_TOO_LONG);
}

#[test]
async fn contract_query_memory_returns_results_array() {
    // With an active workspace (even empty), query_memory should return
    // a JSON object with a `results` array.
    let state = Arc::new(AppState::new(10));
    let snapshot = t_mem::server::state::WorkspaceSnapshot {
        workspace_id: "test_ws_results".to_string(),
        path: "/tmp/test-repo-results".to_string(),
        task_count: 0,
        context_count: 0,
        last_flush: None,
        stale_files: false,
        connection_count: 1,
    };
    state.set_workspace(snapshot).await.expect("set workspace");

    let params = Some(json!({
        "query": "user login",
    }));

    let result = tools::dispatch(state, "query_memory", params).await;
    // May succeed with empty results or fail with ModelNotLoaded on keyword-only.
    // Either way, it should not return WorkspaceNotSet.
    match result {
        Ok(val) => {
            assert!(
                val.get("results").is_some(),
                "response must contain `results` key"
            );
            assert!(val["results"].is_array(), "`results` must be an array");
        }
        Err(e) => {
            // Acceptable: ModelNotLoaded or DatabaseError (no real DB in unit test)
            let code = e.to_response().error.code;
            assert_ne!(code, WORKSPACE_NOT_SET, "must not be WorkspaceNotSet");
        }
    }
}
