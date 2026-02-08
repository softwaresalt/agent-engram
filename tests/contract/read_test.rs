use std::sync::Arc;

use serde_json::json;
use tokio::test;

use t_mem::errors::codes::WORKSPACE_NOT_SET;
use t_mem::server::state::AppState;
use t_mem::tools;

#[test]
async fn contract_get_task_graph_requires_workspace() {
    let state = Arc::new(AppState::new());
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
    let state = Arc::new(AppState::new());
    let params = Some(json!({
        "work_item_ids": ["AB#123", "AB#456"],
    }));

    let err = tools::dispatch(state, "check_status", params)
        .await
        .expect_err("expected workspace not set error");

    let code = err.to_response().error.code;
    assert_eq!(code, WORKSPACE_NOT_SET);
}
