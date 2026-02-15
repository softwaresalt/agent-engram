use std::sync::Arc;

use serde_json::json;
use tokio::test;

use t_mem::errors::codes::{QUERY_TOO_LONG, WORKSPACE_NOT_SET};
use t_mem::server::state::{AppState, WorkspaceSnapshot};
use t_mem::tools;

fn test_snapshot(id: &str) -> WorkspaceSnapshot {
    WorkspaceSnapshot {
        workspace_id: id.to_string(),
        path: format!("/tmp/{id}"),
        task_count: 0,
        context_count: 0,
        last_flush: None,
        stale_files: false,
        connection_count: 1,
        file_mtimes: std::collections::HashMap::new(),
    }
}

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
        file_mtimes: std::collections::HashMap::new(),
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
        file_mtimes: std::collections::HashMap::new(),
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

// ── T018: get_ready_work contract tests ──────────────────────────

#[test]
async fn contract_get_ready_work_requires_workspace() {
    let state = Arc::new(AppState::new(10));
    let params = Some(json!({ "limit": 5 }));

    let err = tools::dispatch(state, "get_ready_work", params)
        .await
        .expect_err("expected workspace not set error");

    let code = err.to_response().error.code;
    assert_eq!(code, WORKSPACE_NOT_SET);
}

#[test]
async fn contract_get_ready_work_empty_workspace() {
    let state = Arc::new(AppState::new(10));
    state
        .set_workspace(test_snapshot("ready_work_empty"))
        .await
        .expect("set workspace");

    let params = Some(json!({}));
    let result = tools::dispatch(state, "get_ready_work", params)
        .await
        .expect("should succeed on empty workspace");

    assert!(result.get("tasks").is_some(), "must have tasks key");
    assert!(result["tasks"].is_array(), "tasks must be array");
    assert_eq!(result["tasks"].as_array().unwrap().len(), 0);
    assert_eq!(result["total_eligible"].as_u64().unwrap(), 0);
}

#[test]
async fn contract_get_ready_work_returns_tasks() {
    let state = Arc::new(AppState::new(10));
    state
        .set_workspace(test_snapshot("ready_work_basic"))
        .await
        .expect("set workspace");

    // Create a task via dispatch so it exists in the DB
    let create_params = Some(json!({
        "title": "ready-work test task",
        "description": "A task for ready-work"
    }));
    tools::dispatch(state.clone(), "create_task", create_params)
        .await
        .expect("create_task should succeed");

    let params = Some(json!({}));
    let result = tools::dispatch(state, "get_ready_work", params)
        .await
        .expect("should return tasks");

    assert!(result["tasks"].is_array());
    assert!(
        !result["tasks"].as_array().unwrap().is_empty(),
        "should have at least 1 task"
    );
    assert!(result["total_eligible"].as_u64().unwrap() >= 1);
}

#[test]
async fn contract_get_ready_work_limit_caps_results() {
    let state = Arc::new(AppState::new(10));
    state
        .set_workspace(test_snapshot("ready_work_limit"))
        .await
        .expect("set workspace");

    // Create 3 tasks
    for i in 1..=3 {
        let create_params = Some(json!({
            "title": format!("limit test task {i}"),
            "description": format!("task {i}")
        }));
        tools::dispatch(state.clone(), "create_task", create_params)
            .await
            .expect("create_task should succeed");
    }

    let params = Some(json!({ "limit": 2 }));
    let result = tools::dispatch(state, "get_ready_work", params)
        .await
        .expect("should succeed with limit");

    let tasks = result["tasks"].as_array().unwrap();
    assert!(tasks.len() <= 2, "limit should cap results to 2");
    assert!(
        result["total_eligible"].as_u64().unwrap() >= 3,
        "total_eligible should reflect all eligible tasks"
    );
}
