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

// ─── T127: Contract test for work_item_id assignment and retrieval (FR-017) ──

#[test]
async fn contract_work_item_id_roundtrip_via_update_and_graph() {
    let workspace = tempfile::tempdir().expect("workspace");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");

    let tmem_dir = workspace.path().join(".tmem");
    fs::create_dir_all(&tmem_dir).expect("create .tmem");
    fs::write(
        tmem_dir.join("tasks.md"),
        r#"# Tasks

## task:wi1

---
id: task:wi1
title: Linked task
status: todo
work_item_id: AB#12345
created_at: 2026-02-05T10:00:00+00:00
updated_at: 2026-02-05T10:00:00+00:00
---

Task linked to external work item.
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
    .expect("set_workspace");

    // Update the task status — work_item_id should be preserved
    tools::dispatch(
        state.clone(),
        "update_task",
        Some(json!({ "id": "wi1", "status": "in_progress" })),
    )
    .await
    .expect("update_task should succeed");

    // Retrieve via get_task_graph — verify work_item_id is present
    let graph = tools::dispatch(
        state.clone(),
        "get_task_graph",
        Some(json!({ "root_task_id": "wi1" })),
    )
    .await
    .expect("get_task_graph should succeed");

    let root = graph.get("root").expect("root node");
    assert_eq!(
        root.get("status").and_then(|s| s.as_str()),
        Some("in_progress")
    );

    // Also verify via check_status with the work_item_id
    let status_result = tools::dispatch(
        state.clone(),
        "check_status",
        Some(json!({ "work_item_ids": ["AB#12345"] })),
    )
    .await
    .expect("check_status should succeed");

    let statuses = status_result.get("statuses").expect("statuses map");
    let entry = statuses.get("AB#12345").expect("AB#12345 entry");
    assert_eq!(
        entry.get("status").and_then(|s| s.as_str()),
        Some("in_progress")
    );
}

// ─── T129: Contract test for create_task WorkspaceNotSet (1003) ─────────────

#[test]
async fn contract_create_task_requires_workspace() {
    let state = Arc::new(AppState::new(10));
    let params = Some(json!({
        "title": "New task",
    }));

    let err = tools::dispatch(state, "create_task", params)
        .await
        .expect_err("expected workspace not set error");

    let code = err.to_response().error.code;
    assert_eq!(code, WORKSPACE_NOT_SET);
}

// ─── T130: Contract test for create_task empty title TaskTitleEmpty (3005) ───

#[test]
async fn contract_create_task_rejects_empty_title() {
    let workspace = tempfile::tempdir().expect("workspace");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");

    let state = Arc::new(AppState::new(10));
    let path = workspace.path().to_string_lossy().to_string();

    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace should succeed");

    // Empty title
    let err = tools::dispatch(state.clone(), "create_task", Some(json!({ "title": "" })))
        .await
        .expect_err("expected TaskTitleEmpty error");

    let code = err.to_response().error.code;
    assert_eq!(code, t_mem::errors::codes::TASK_TITLE_EMPTY);
}

#[test]
async fn contract_create_task_rejects_oversized_title() {
    let workspace = tempfile::tempdir().expect("workspace");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");

    let state = Arc::new(AppState::new(10));
    let path = workspace.path().to_string_lossy().to_string();

    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace should succeed");

    // Title > 200 chars
    let long_title = "a".repeat(201);
    let err = tools::dispatch(
        state.clone(),
        "create_task",
        Some(json!({ "title": long_title })),
    )
    .await
    .expect_err("expected TaskTitleEmpty error for oversized title");

    let code = err.to_response().error.code;
    assert_eq!(code, t_mem::errors::codes::TASK_TITLE_EMPTY);
}
