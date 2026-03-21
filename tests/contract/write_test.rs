use std::fs;
use std::sync::Arc;

use serde_json::json;
use tokio::test;

use engram::errors::codes::{INDEX_IN_PROGRESS, WORKSPACE_NOT_SET};
use engram::server::state::AppState;
use engram::services::dehydration::SCHEMA_VERSION;
use engram::tools;

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

    let warnings = result.get("warnings").expect("warnings field");
    assert!(warnings.is_array(), "warnings should be array");

    let ts = result
        .get("flush_timestamp")
        .expect("flush_timestamp field");
    assert!(ts.is_string(), "flush_timestamp should be string");

    // Phase 2: flush_state writes code-graph JSONL files only (tasks.md removed).
    // Verify code_graph summary fields are present.
    let cg = result.get("code_graph").expect("code_graph field");
    assert!(
        cg.get("nodes_written").is_some(),
        "code_graph.nodes_written present"
    );
    assert!(
        cg.get("edges_written").is_some(),
        "code_graph.edges_written present"
    );
}

// ── index_workspace contract tests ──────────────────────────────────

#[test]
async fn contract_index_workspace_requires_workspace() {
    let state = Arc::new(AppState::new(10));
    let params = Some(json!({}));

    let err = tools::dispatch(state, "index_workspace", params)
        .await
        .expect_err("expected workspace not set error");

    assert_eq!(err.to_response().error.code, WORKSPACE_NOT_SET);
}

#[test]
async fn contract_index_workspace_rejects_while_in_progress() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");
    let engram_dir = workspace.path().join(".engram");
    fs::create_dir_all(&engram_dir).expect("create .engram");
    fs::write(engram_dir.join("tasks.md"), "").expect("write tasks.md");
    fs::write(engram_dir.join(".version"), SCHEMA_VERSION).expect("write .version");

    let state = Arc::new(AppState::new(10));
    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": workspace.path().to_str().unwrap() })),
    )
    .await
    .expect("set_workspace should succeed");

    // Simulate an indexing operation in progress.
    assert!(state.try_start_indexing(), "should acquire indexing lock");

    let err = tools::dispatch(state, "index_workspace", Some(json!({})))
        .await
        .expect_err("expected index-in-progress error");

    assert_eq!(err.to_response().error.code, INDEX_IN_PROGRESS);
}

// ── sync_workspace contract tests (T042) ────────────────────────────

#[test]
async fn contract_sync_workspace_requires_workspace() {
    let state = Arc::new(AppState::new(10));
    let params = Some(json!({}));

    let err = tools::dispatch(state, "sync_workspace", params)
        .await
        .expect_err("expected workspace not set error");

    assert_eq!(err.to_response().error.code, WORKSPACE_NOT_SET);
}

#[test]
async fn contract_sync_workspace_rejects_while_in_progress() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");
    let engram_dir = workspace.path().join(".engram");
    fs::create_dir_all(&engram_dir).expect("create .engram");
    fs::write(engram_dir.join("tasks.md"), "").expect("write tasks.md");
    fs::write(engram_dir.join(".version"), SCHEMA_VERSION).expect("write .version");

    let state = Arc::new(AppState::new(10));
    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": workspace.path().to_str().unwrap() })),
    )
    .await
    .expect("set_workspace should succeed");

    // Simulate an indexing operation in progress.
    assert!(state.try_start_indexing(), "should acquire indexing lock");

    let err = tools::dispatch(state, "sync_workspace", Some(json!({})))
        .await
        .expect_err("expected index-in-progress error");

    assert_eq!(err.to_response().error.code, INDEX_IN_PROGRESS);
}

// ── Phase 9: flush_state FR-153 guard ───────────────────────────────

#[test]
async fn contract_flush_state_rejects_while_indexing() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");
    let engram_dir = workspace.path().join(".engram");
    fs::create_dir_all(&engram_dir).expect("create .engram");
    fs::write(engram_dir.join("tasks.md"), "").expect("write tasks.md");
    fs::write(engram_dir.join(".version"), SCHEMA_VERSION).expect("write .version");

    let state = Arc::new(AppState::new(10));
    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": workspace.path().to_str().unwrap() })),
    )
    .await
    .expect("set_workspace should succeed");

    // Simulate an indexing operation in progress
    assert!(state.try_start_indexing(), "should acquire indexing lock");

    let err = tools::dispatch(state, "flush_state", Some(json!({})))
        .await
        .expect_err("expected index-in-progress error");

    assert_eq!(err.to_response().error.code, INDEX_IN_PROGRESS);
}
