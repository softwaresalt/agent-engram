//! Public indexing-resilience compatibility coverage (044-F / 049-F).
//!
//! Ownership is established only by a real write tool. Private permit identity,
//! stale-terminal, Drop recovery, and timestamp invariants stay co-located with
//! the coordinator implementation in `server::state`.

#[path = "../helpers/mod.rs"]
mod helpers;

use std::sync::Arc;

use engram::errors::codes::INDEX_IN_PROGRESS;
use engram::models::config::WorkspaceConfig;
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

#[tokio::test]
async fn public_owner_preserves_busy_queue_and_release_contracts() {
    let (ws, state) = make_workspace();
    helpers::bind_isolated_workspace(&state, ws.path(), "main", WorkspaceConfig::default()).await;

    tools::dispatch(Arc::clone(&state), "index_workspace", Some(json!({})))
        .await
        .expect("idle index control");
    std::fs::write(
        ws.path().join("src/lib.rs"),
        "pub fn ready() {}\npub fn changed() {}\n",
    )
    .expect("modify fixture before active sync");

    let active_sync = tools::write::sync_workspace(Arc::clone(&state), Some(json!({})));
    let public_contracts = async {
        assert!(state.is_indexing(), "public sync must own admission");

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
    let sync_value = sync_result.expect("owner and one queued successor complete");
    assert!(
        sync_value.get("files_modified").is_some(),
        "control sync must own admission rather than queue behind a background driver"
    );

    if let Err(error) = tools::dispatch(Arc::clone(&state), "get_workspace_statistics", None).await
    {
        assert_ne!(
            error.to_response().error.code,
            INDEX_IN_PROGRESS,
            "normal release leaves no busy owner"
        );
    }
}
