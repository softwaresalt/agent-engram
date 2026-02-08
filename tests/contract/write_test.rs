use std::sync::Arc;

use serde_json::json;
use tokio::test;

use t_mem::errors::codes::WORKSPACE_NOT_SET;
use t_mem::server::state::AppState;
use t_mem::tools;

#[test]
async fn contract_update_task_requires_workspace() {
    let state = Arc::new(AppState::new());
    let params = Some(json!({
        "id": "task:abc123",
        "status": "in_progress",
    }));

    let err = tools::dispatch(state, "update_task", params)
        .await
        .expect_err("expected workspace not set error");

    let code = err.to_response().error.code;
    assert_eq!(code, WORKSPACE_NOT_SET);
}

#[test]
async fn contract_add_blocker_requires_workspace() {
    let state = Arc::new(AppState::new());
    let params = Some(json!({
        "task_id": "task:abc123",
        "reason": "waiting on review",
    }));

    let err = tools::dispatch(state, "add_blocker", params)
        .await
        .expect_err("expected workspace not set error");

    let code = err.to_response().error.code;
    assert_eq!(code, WORKSPACE_NOT_SET);
}

#[test]
async fn contract_register_decision_requires_workspace() {
    let state = Arc::new(AppState::new());
    let params = Some(json!({
        "topic": "database backend",
        "decision": "use surrealdb",
    }));

    let err = tools::dispatch(state, "register_decision", params)
        .await
        .expect_err("expected workspace not set error");

    let code = err.to_response().error.code;
    assert_eq!(code, WORKSPACE_NOT_SET);
}
