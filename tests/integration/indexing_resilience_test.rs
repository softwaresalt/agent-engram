//! Integration tests for indexing-resilience behaviour (044-F / 049-F).
//!
//! Since 049-F (daemon startup reliability), read-only tool handlers no longer
//! return `IndexInProgress` (7003) — they proceed against the current DB snapshot
//! so that partial results are served instead of blocking callers.
//!
//! `sync_workspace` still queues a deferred sync instead of returning an error
//! when background indexing holds the lock.
//!
//! All tests drive [`AppState`] and the tool dispatch layer directly —
//! no IPC socket or network connection is required.

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

// ── T-IXR-1: get_workspace_statistics allowed while indexing ────────────────

/// `get_workspace_statistics` must NOT return `IndexInProgress` (7003) when
/// background indexing holds the lock — it should proceed with partial data.
#[tokio::test]
async fn t_ixr_01_stats_rejects_while_indexing() {
    let (ws, state) = make_workspace();
    bind_workspace(Arc::clone(&state), ws.path()).await;

    assert!(state.try_start_indexing(), "should acquire indexing lock");

    let result = tools::dispatch(Arc::clone(&state), "get_workspace_statistics", None).await;

    // Tool proceeds during indexing — may succeed (empty stats) or fail with a
    // non-IndexInProgress, non-WorkspaceNotSet error (workspace was bound above).
    if let Err(err) = result {
        let code = err.to_response().error.code;
        assert_ne!(code, INDEX_IN_PROGRESS, "must not return IndexInProgress");
        assert_ne!(code, WORKSPACE_NOT_SET, "must not return WorkspaceNotSet");
    }
}

// ── T-IXR-2: query_memory allowed while indexing ────────────────────────────

/// `query_memory` must NOT return `IndexInProgress` (7003) when indexing is active.
#[tokio::test]
async fn t_ixr_02_query_memory_rejects_while_indexing() {
    let (ws, state) = make_workspace();
    bind_workspace(Arc::clone(&state), ws.path()).await;

    assert!(state.try_start_indexing(), "should acquire indexing lock");

    let result = tools::dispatch(
        Arc::clone(&state),
        "query_memory",
        Some(json!({ "query": "test" })),
    )
    .await;

    if let Err(err) = result {
        let code = err.to_response().error.code;
        assert_ne!(code, INDEX_IN_PROGRESS, "must not return IndexInProgress");
        assert_ne!(code, WORKSPACE_NOT_SET, "must not return WorkspaceNotSet");
    }
}

// ── T-IXR-3: unified_search allowed while indexing ──────────────────────────

/// `unified_search` must NOT return `IndexInProgress` (7003) when indexing is active.
#[tokio::test]
async fn t_ixr_03_unified_search_rejects_while_indexing() {
    let (ws, state) = make_workspace();
    bind_workspace(Arc::clone(&state), ws.path()).await;

    assert!(state.try_start_indexing(), "should acquire indexing lock");

    let result = tools::dispatch(
        Arc::clone(&state),
        "unified_search",
        Some(json!({ "query": "test" })),
    )
    .await;

    if let Err(err) = result {
        let code = err.to_response().error.code;
        assert_ne!(code, INDEX_IN_PROGRESS, "must not return IndexInProgress");
        assert_ne!(code, WORKSPACE_NOT_SET, "must not return WorkspaceNotSet");
    }
}

// ── T-IXR-4: sync_workspace queues deferred sync while indexing ─────────────

/// When `sync_workspace` is called while indexing is active, it must return
/// `{"status": "queued"}` and set the `pending_sync` flag so the sync is
/// executed once indexing finishes.
#[tokio::test]
async fn t_ixr_04_sync_workspace_queues_while_indexing() {
    let (ws, state) = make_workspace();
    bind_workspace(Arc::clone(&state), ws.path()).await;

    assert!(state.try_start_indexing(), "should acquire indexing lock");

    let result = tools::dispatch(Arc::clone(&state), "sync_workspace", Some(json!({})))
        .await
        .expect("sync_workspace should succeed with queued status");

    assert_eq!(result["status"], "queued", "expected status: queued");

    // Flag must be set so background_db_hydration drains it after finish_indexing.
    assert!(
        state.take_pending_sync(),
        "pending_sync must be set after queued sync request"
    );

    // take_pending_sync is idempotent — second call returns false.
    assert!(
        !state.take_pending_sync(),
        "pending_sync flag must be cleared after first drain"
    );
}

// ── T-IXR-5: tools succeed after indexing lock is released ──────────────────

/// After the indexing lock is released via `finish_indexing()`, the previously
/// guarded tools must succeed (or return a non-IndexInProgress error).
#[tokio::test]
async fn t_ixr_05_tools_succeed_after_indexing_finishes() {
    let (ws, state) = make_workspace();
    bind_workspace(Arc::clone(&state), ws.path()).await;

    // Simulate indexing start and finish.
    assert!(state.try_start_indexing(), "should acquire indexing lock");
    state.finish_indexing().await;

    // Stats should no longer return IndexInProgress.
    let result = tools::dispatch(Arc::clone(&state), "get_workspace_statistics", None).await;
    match result {
        Ok(_) => {}
        Err(e) => {
            let code = e.to_response().error.code;
            assert_ne!(
                code, INDEX_IN_PROGRESS,
                "should not return IndexInProgress after indexing finishes"
            );
        }
    }
}
