//! Public indexing-resilience compatibility coverage (044-F / 049-F).
//!
//! Ownership is established only by a real write tool. Private permit identity,
//! stale-terminal, Drop recovery, and timestamp invariants stay co-located with
//! the coordinator implementation in `server::state`.

use std::sync::Arc;

use engram::errors::codes::{INDEX_IN_PROGRESS, WORKSPACE_NOT_SET};
use engram::server::state::AppState;
use engram::services::dehydration::SCHEMA_VERSION;
use engram::tools;
use serde_json::json;
use tempfile::tempdir;

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Build a minimal workspace for state tests (git root + .engram dir).
fn make_workspace() -> (tempfile::TempDir, Arc<AppState>) {
    let dir = tempdir().expect("tempdir");
    std::fs::create_dir(dir.path().join(".git")).expect(".git");
    std::fs::create_dir(dir.path().join("src")).expect("src");
    std::fs::write(dir.path().join("src/lib.rs"), "pub fn ready() {}\n").expect("fixture");
    let engram_dir = dir.path().join(".engram");
    std::fs::create_dir_all(&engram_dir).expect(".engram");
    std::fs::write(engram_dir.join(".version"), SCHEMA_VERSION).expect(".version");
    let state = Arc::new(AppState::new(10));
    (dir, state)
}

async fn bind_workspace(state: Arc<AppState>, path: &std::path::Path) {
    tools::dispatch(
        Arc::clone(&state),
        "set_workspace",
        Some(json!({ "path": path.to_str().unwrap() })),
    )
    .await
    .expect("set_workspace");
}

#[tokio::test]
async fn public_owner_preserves_busy_read_queue_and_release_contracts() {
    let (ws, state) = make_workspace();
    bind_workspace(Arc::clone(&state), ws.path()).await;

    tools::dispatch(Arc::clone(&state), "index_workspace", Some(json!({})))
        .await
        .expect("idle index control");

    let active_sync = tools::write::sync_workspace(Arc::clone(&state), Some(json!({})));
    let public_contracts = async {
        assert!(state.is_indexing(), "public sync must own admission");

        for (method, params) in [
            ("get_workspace_statistics", None),
            ("query_memory", Some(json!({ "query": "test" }))),
            ("unified_search", Some(json!({ "query": "test" }))),
        ] {
            if let Err(error) = tools::dispatch(Arc::clone(&state), method, params).await {
                let code = error.to_response().error.code;
                assert_ne!(code, INDEX_IN_PROGRESS, "{method} remains readable");
                assert_ne!(code, WORKSPACE_NOT_SET, "{method} keeps its binding");
            }
        }

        let busy_index = tools::dispatch(Arc::clone(&state), "index_workspace", Some(json!({})))
            .await
            .expect_err("busy index must reject");
        assert_eq!(busy_index.to_response().error.code, INDEX_IN_PROGRESS);

        let queued = tools::dispatch(Arc::clone(&state), "sync_workspace", Some(json!({})))
            .await
            .expect("busy sync must queue");
        assert_eq!(
            queued,
            json!({
                "status": "queued",
                "message": "Sync queued; will run after current indexing completes"
            })
        );
    };

    let (sync_result, ()) = tokio::join!(biased; active_sync, public_contracts);
    sync_result.expect("owner and one queued successor complete");

    if let Err(error) = tools::dispatch(Arc::clone(&state), "get_workspace_statistics", None).await
    {
        assert_ne!(
            error.to_response().error.code,
            INDEX_IN_PROGRESS,
            "normal release leaves no busy owner"
        );
    }
}
