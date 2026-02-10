use std::fs;
use std::sync::Arc;

use serde_json::json;
use tokio::test;

use t_mem::errors::codes::{INVALID_STATUS, WORKSPACE_NOT_SET};
use t_mem::server::state::AppState;
use t_mem::tools;

#[test]
async fn contract_update_task_requires_workspace() {
    let state = Arc::new(AppState::new(10));
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
async fn contract_update_task_rejects_invalid_transition() {
    // Seed workspace with a completed task
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");

    let tmem_dir = workspace.path().join(".tmem");
    fs::create_dir_all(&tmem_dir).expect("create .tmem");
    fs::write(
        tmem_dir.join("tasks.md"),
        r#"# Tasks

## task:t1

---
id: task:t1
title: Finished task
status: done
created_at: 2026-02-05T10:00:00+00:00
updated_at: 2026-02-05T10:00:00+00:00
---

Task is already complete.
"#,
    )
    .expect("write tasks.md");

    let state = Arc::new(AppState::new(10));
    let path = workspace.path().to_string_lossy().to_string();

    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace should succeed");

    let err = tools::dispatch(
        state.clone(),
        "update_task",
        Some(json!({ "id": "t1", "status": "blocked" })),
    )
    .await
    .expect_err("expected invalid status transition");

    let code = err.to_response().error.code;
    assert_eq!(code, INVALID_STATUS);
}

#[test]
async fn contract_add_blocker_requires_workspace() {
    let state = Arc::new(AppState::new(10));
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
    let state = Arc::new(AppState::new(10));
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

// ─── T057: Contract test for flush_state ────────────────────────────────────

#[test]
async fn contract_flush_state_requires_workspace() {
    let state = Arc::new(AppState::new(10));

    let err = tools::dispatch(state, "flush_state", None)
        .await
        .expect_err("expected workspace not set error");

    let code = err.to_response().error.code;
    assert_eq!(code, WORKSPACE_NOT_SET);
}

#[test]
async fn contract_flush_state_response_shape() {
    // Set up a real workspace with .git/
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    std::fs::create_dir(workspace.path().join(".git")).expect("create .git");

    let state = Arc::new(AppState::new(10));
    let path = workspace.path().to_string_lossy().to_string();

    // Bind workspace
    let bind_result = tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace should succeed");
    assert!(bind_result.get("workspace_id").is_some());

    // Call flush_state
    let result = tools::dispatch(state.clone(), "flush_state", None)
        .await
        .expect("flush_state should succeed");

    // Verify contract response shape
    let files = result.get("files_written").expect("files_written field");
    assert!(files.is_array(), "files_written should be array");
    let files_arr = files.as_array().unwrap();
    assert!(
        files_arr
            .iter()
            .any(|f| f.as_str() == Some(".tmem/tasks.md")),
        "should write tasks.md"
    );
    assert!(
        files_arr
            .iter()
            .any(|f| f.as_str() == Some(".tmem/.lastflush")),
        "should write .lastflush"
    );

    let warnings = result.get("warnings").expect("warnings field");
    assert!(warnings.is_array(), "warnings should be array");

    let ts = result
        .get("flush_timestamp")
        .expect("flush_timestamp field");
    assert!(ts.is_string(), "flush_timestamp should be string");

    // Verify files exist on disk
    let tmem_dir = workspace.path().join(".tmem");
    assert!(tmem_dir.join("tasks.md").exists(), "tasks.md on disk");
    assert!(tmem_dir.join("graph.surql").exists(), "graph.surql on disk");
    assert!(tmem_dir.join(".version").exists(), ".version on disk");
    assert!(tmem_dir.join(".lastflush").exists(), ".lastflush on disk");
}
