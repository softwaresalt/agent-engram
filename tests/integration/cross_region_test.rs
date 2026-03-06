//! Integration tests for cross-region task-to-code linking (Phase 6).
//!
//! Creates a temporary workspace, indexes sample code, creates a task,
//! links it to symbols via `link_task_to_code`, verifies `get_active_context`,
//! then unlinks and confirms removal.

use std::fs;
use std::path::Path;
use std::sync::Arc;

use serde_json::json;
use tokio::test;

use engram::db::connect_db;
use engram::db::queries::CodeGraphQueries;
use engram::models::config::CodeGraphConfig;
use engram::server::state::AppState;
use engram::services::code_graph;
use engram::services::dehydration::SCHEMA_VERSION;
use engram::tools;

/// Helper: write a sample Rust file into the workspace.
fn write_sample_file(dir: &Path, rel_path: &str, content: &str) {
    let full = dir.join(rel_path);
    if let Some(parent) = full.parent() {
        fs::create_dir_all(parent).expect("create dirs");
    }
    fs::write(full, content).expect("write file");
}

/// Full lifecycle: create task → index → link → `get_active_context` → unlink.
#[test]
#[allow(clippy::too_many_lines)]
async fn cross_region_link_lifecycle() {
    // 1. Set up workspace with sample code.
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();

    fs::create_dir(ws.join(".git")).expect("create .git");
    let engram_dir = ws.join(".engram");
    fs::create_dir_all(&engram_dir).expect("create .engram");
    fs::write(engram_dir.join("tasks.md"), "").expect("write tasks.md");
    fs::write(engram_dir.join(".version"), SCHEMA_VERSION).expect("write .version");

    write_sample_file(
        ws,
        "src/lib.rs",
        r"
/// A helper function for testing cross-region links.
pub fn cross_region_helper(x: u32) -> u32 {
    x * 2
}

/// Another function.
pub fn another_function() -> bool {
    true
}
",
    );

    let state = Arc::new(AppState::new(10));

    // 2. Bind workspace.
    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": ws.to_str().unwrap() })),
    )
    .await
    .expect("set_workspace should succeed");

    // 3. Create a task in Region B, then update to in_progress.
    let create_result = tools::dispatch(
        state.clone(),
        "create_task",
        Some(json!({
            "title": "Implement cross-region helper",
        })),
    )
    .await
    .expect("create_task should succeed");

    let task_id = create_result["task_id"]
        .as_str()
        .expect("task should have a task_id")
        .to_string();

    // Transition to in_progress for get_active_context.
    tools::dispatch(
        state.clone(),
        "update_task",
        Some(json!({ "id": task_id, "status": "in_progress" })),
    )
    .await
    .expect("update_task to in_progress should succeed");

    // 4. Index workspace using the service directly (bypasses tool handler).
    //    Use the workspace_id from the snapshot so it matches what set_workspace
    //    stored.
    let snapshot = state
        .snapshot_workspace()
        .await
        .expect("should have workspace snapshot");
    let ws_id = snapshot.workspace_id.clone();
    let canonical = std::path::PathBuf::from(&snapshot.path);
    let config = CodeGraphConfig::default();
    let idx_result = code_graph::index_workspace(&canonical, &ws_id, &config, false)
        .await
        .expect("indexing should succeed");

    assert!(
        idx_result.functions_indexed >= 2,
        "should index at least 2 functions"
    );

    // Verify function is actually in DB.
    let db = connect_db(&ws_id).await.expect("connect_db");
    let cg_q = CodeGraphQueries::new(db);

    let func = cg_q
        .get_function_by_name("cross_region_helper")
        .await
        .expect("direct function lookup");
    assert!(
        func.is_some(),
        "cross_region_helper should be in the DB after indexing"
    );

    // 5. Link task to a symbol by name.
    let link_result = tools::dispatch(
        state.clone(),
        "link_task_to_code",
        Some(json!({
            "task_id": task_id,
            "symbol_name": "cross_region_helper",
        })),
    )
    .await
    .expect("link_task_to_code should succeed");

    assert!(
        link_result["links_created"].as_u64().unwrap_or(0) >= 1,
        "should create at least one concerns edge"
    );

    // 6. Verify get_active_context returns the linked symbol.
    let ctx = tools::dispatch(state.clone(), "get_active_context", Some(json!({})))
        .await
        .expect("get_active_context should succeed");

    let primary = &ctx["primary_task"];
    assert!(!primary.is_null(), "should have a primary in-progress task");

    let linked_symbols = primary["linked_symbols"]
        .as_array()
        .expect("linked_symbols should be an array");
    assert!(
        !linked_symbols.is_empty(),
        "primary task should have linked symbols"
    );

    // 7. Unlink the task from the symbol.
    let unlink_result = tools::dispatch(
        state.clone(),
        "unlink_task_from_code",
        Some(json!({
            "task_id": task_id,
            "symbol_name": "cross_region_helper",
        })),
    )
    .await
    .expect("unlink_task_from_code should succeed");

    assert!(
        unlink_result["links_removed"].as_u64().unwrap_or(0) >= 1,
        "should remove at least one concerns edge"
    );

    // 8. Verify get_active_context no longer has the symbol linked.
    let ctx_after = tools::dispatch(state.clone(), "get_active_context", Some(json!({})))
        .await
        .expect("get_active_context should succeed after unlink");

    let primary_after = &ctx_after["primary_task"];
    assert!(
        !primary_after.is_null(),
        "should still have a primary task (it is in_progress)"
    );

    let linked_after = primary_after["linked_symbols"]
        .as_array()
        .expect("linked_symbols should still be an array");
    assert!(
        linked_after.is_empty(),
        "linked_symbols should be empty after unlink"
    );
}
