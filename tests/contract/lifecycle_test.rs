use std::fs;
use std::sync::Arc;

use serde_json::{Value, json};
use tokio::test;

use t_mem::errors::codes::WORKSPACE_LIMIT_REACHED;
use t_mem::server::state::AppState;
use t_mem::tools;

#[test]
async fn contract_set_workspace_returns_hydrated_flag() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");

    let state = Arc::new(AppState::new(10));
    let path = workspace.path().to_string_lossy().to_string();

    let result = tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace should succeed");

    assert_eq!(result.get("hydrated").and_then(Value::as_bool), Some(true));
    let workspace_id = result
        .get("workspace_id")
        .and_then(Value::as_str)
        .expect("workspace_id present");
    assert!(!workspace_id.is_empty(), "workspace_id must not be empty");
}

#[test]
async fn contract_get_daemon_status_reports_counts() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
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

    let status = tools::dispatch(state.clone(), "get_daemon_status", None)
        .await
        .expect("get_daemon_status should succeed");

    let active = status
        .get("active_workspaces")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    assert_eq!(active, 1);
    assert!(
        status
            .get("active_connections")
            .and_then(Value::as_u64)
            .is_some(),
        "active_connections present"
    );
}

#[test]
async fn contract_get_workspace_status_reports_state() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");
    let state = Arc::new(AppState::new(10));
    let path = workspace.path().to_string_lossy().to_string();
    let canonical = fs::canonicalize(workspace.path()).expect("canonicalize workspace");
    let canonical_path = canonical.display().to_string();

    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace should succeed");

    let status = tools::dispatch(state.clone(), "get_workspace_status", None)
        .await
        .expect("get_workspace_status should succeed");

    assert_eq!(
        status.get("path").and_then(Value::as_str),
        Some(canonical_path.as_str())
    );
    assert_eq!(status.get("task_count").and_then(Value::as_u64), Some(0));
    assert_eq!(status.get("context_count").and_then(Value::as_u64), Some(0));
    assert_eq!(
        status.get("stale_files").and_then(Value::as_bool),
        Some(false)
    );
}

// ─── T112: Workspace limit (FR-009a) ─────────────────────────────────────────

#[test]
async fn contract_set_workspace_enforces_limit() {
    let first = tempfile::tempdir().expect("first workspace");
    let second = tempfile::tempdir().expect("second workspace");
    fs::create_dir(first.path().join(".git")).expect("create first .git");
    fs::create_dir(second.path().join(".git")).expect("create second .git");

    let state = Arc::new(AppState::new(1));
    let first_path = first.path().to_string_lossy().to_string();
    let second_path = second.path().to_string_lossy().to_string();

    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": first_path })),
    )
    .await
    .expect("first workspace should bind");

    let err = tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": second_path })),
    )
    .await
    .expect_err("second workspace should exceed limit");

    let payload = err.to_response();
    assert_eq!(payload.error.code, WORKSPACE_LIMIT_REACHED);
    let limit = payload
        .error
        .details
        .as_ref()
        .and_then(|d| d.get("limit"))
        .and_then(Value::as_u64)
        .expect("limit detail present");
    assert_eq!(limit, 1);
}
