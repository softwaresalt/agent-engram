#[path = "../helpers/mod.rs"]
mod helpers;

use std::fs;
use std::sync::Arc;

use serde_json::json;
use tokio::test;

use engram::db::connect_db;
use engram::db::queries::CodeGraphQueries;
use engram::errors::codes::{INDEX_IN_PROGRESS, WORKSPACE_NOT_SET};
use engram::models::config::WorkspaceConfig;
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
#[serial_test::serial(metrics_writer)]
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
    engram::services::metrics::shutdown()
        .await
        .expect("shutdown metrics writer");

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
#[serial_test::serial(metrics_writer)]
async fn contract_busy_writes_preserve_public_responses_and_complete_mask() {
    engram::services::metrics::shutdown()
        .await
        .expect("reset metrics writer");
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");
    fs::write(
        workspace.path().join("app.py"),
        "def run():\n    return 1\n",
    )
    .expect("write Python fixture");
    let engram_dir = workspace.path().join(".engram");
    fs::create_dir_all(&engram_dir).expect("create .engram");
    fs::write(engram_dir.join("tasks.md"), "").expect("write tasks.md");
    fs::write(engram_dir.join(".version"), SCHEMA_VERSION).expect("write .version");

    let state = Arc::new(AppState::new(10));
    helpers::bind_isolated_workspace(&state, workspace.path(), "main", WorkspaceConfig::default())
        .await;

    // Idle control: the public Index tool acquires and completes normally.
    let idle = tools::dispatch(Arc::clone(&state), "index_workspace", Some(json!({})))
        .await
        .expect("idle index_workspace should succeed");
    assert!(
        idle.get("files_parsed").is_some(),
        "idle response keeps the index result schema"
    );

    let snapshot = state
        .snapshot_workspace()
        .await
        .expect("bound workspace snapshot");
    let db = connect_db(&snapshot.data_dir, &snapshot.branch)
        .await
        .expect("connect indexed database");
    let queries = CodeGraphQueries::new(db);
    queries
        .set_python_extraction_version("0")
        .await
        .expect("set stale Python marker");
    queries
        .set_code_graph_extraction_generation("0")
        .await
        .expect("set stale code-graph marker");
    drop(queries);

    // Poll the routine Sync only until it yields while holding admission, then
    // stop polling it until every competing call has observed that owner.
    let mut active_sync = std::pin::pin!(tools::write::sync_workspace(
        Arc::clone(&state),
        Some(json!({}))
    ));
    let completed_before_admission =
        std::future::poll_fn(|cx| match active_sync.as_mut().poll(cx) {
            std::task::Poll::Pending if state.is_indexing() => std::task::Poll::Ready(None),
            std::task::Poll::Pending => std::task::Poll::Pending,
            std::task::Poll::Ready(result) => std::task::Poll::Ready(Some(result)),
        })
        .await;
    assert!(
        completed_before_admission.is_none(),
        "public sync must yield while owning coordinator admission"
    );
    assert!(
        state.is_indexing(),
        "acquisition barrier must retain the routine sync owner"
    );

    let index_error = tools::dispatch(Arc::clone(&state), "index_workspace", Some(json!({})))
        .await
        .expect_err("busy index_workspace must reject");
    assert_eq!(index_error.to_response().error.code, INDEX_IN_PROGRESS);

    let flush_error = tools::dispatch(Arc::clone(&state), "flush_state", Some(json!({})))
        .await
        .expect_err("busy flush_state must reject");
    assert_eq!(flush_error.to_response().error.code, INDEX_IN_PROGRESS);

    let queued = tools::dispatch(
        Arc::clone(&state),
        "sync_workspace",
        Some(json!({
            "backfill_python_canonical": true,
            "revalidate_code_graph": true
        })),
    )
    .await
    .expect("busy sync_workspace must return queued status");
    assert_eq!(
        queued,
        json!({
            "status": "queued",
            "message": "Sync queued; will run after current indexing completes"
        }),
        "queued MCP response must remain exact"
    );

    let sync_value = active_sync
        .await
        .expect("routine owner and transferred sync should complete");
    assert!(
        sync_value.get("files_modified").is_some(),
        "control sync must own admission rather than queue behind a background driver"
    );

    let db = connect_db(&snapshot.data_dir, &snapshot.branch)
        .await
        .expect("reconnect indexed database");
    let queries = CodeGraphQueries::new(db);
    assert_eq!(
        queries
            .python_extraction_version()
            .expect("read Python marker")
            .as_deref(),
        Some("1"),
        "transferred full mask must execute the Python backfill bit"
    );
    assert_eq!(
        queries
            .code_graph_extraction_generation()
            .expect("read code-graph marker")
            .as_deref(),
        Some("1"),
        "transferred full mask must execute the revalidation bit"
    );
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
