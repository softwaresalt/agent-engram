//! Contract coverage for successful read-response provenance (plan unit F43).
//!
//! Successful read responses must be decorated from the captured
//! [`ReadRequestContext`] rather than later global state so a mid-request
//! activation cannot rewrite which generation a response claims served it.

#![forbid(unsafe_code)]

use std::path::Path;
use std::sync::Arc;

use engram::config::StaleStrategy;
use engram::db::cozo_backend::{ExistingDbLocation, open_existing_generation_via_runtime_copy};
use engram::models::config::DaemonMode;
use engram::server::state::{AppState, ReadRequestContext};
use engram::services::generations::GenerationReadContext;
use engram::tools;
use serde_json::json;
use tempfile::TempDir;

fn create_seeded_db(db_path: &Path) {
    let db = cozo::DbInstance::new("sqlite", db_path.to_str().expect("utf8 path"), "")
        .expect("create db");
    db.run_default(":create probe_row {id => val}")
        .expect("create relation");
}

fn captured_context(generation_id: &str) -> (Arc<ReadRequestContext>, TempDir, TempDir) {
    let published_dir = tempfile::tempdir().expect("published tempdir");
    let runtime_root = tempfile::tempdir().expect("runtime tempdir");
    let published_db_path = published_dir.path().join("engram.db");
    create_seeded_db(&published_db_path);

    let location = ExistingDbLocation::new(published_dir.path(), published_db_path)
        .expect("published database path must validate");
    let opened =
        open_existing_generation_via_runtime_copy(&location, runtime_root.path(), generation_id)
            .expect("open runtime copy");
    let generation = GenerationReadContext::new(opened).expect("valid generation id");

    (
        ReadRequestContext::from_generation(generation, "feature/f43", "workspace-provenance"),
        published_dir,
        runtime_root,
    )
}

#[tokio::test]
async fn successful_read_responses_report_the_captured_generation_not_a_newer_activation() {
    let state = Arc::new(AppState::with_mode(
        DaemonMode::ReadServer,
        1,
        StaleStrategy::Warn,
        10,
        60,
    ));

    let (captured_old, _old_published, _old_runtime) = captured_context("gen-old");
    let (captured_new, _new_published, _new_runtime) = captured_context("gen-new");

    let old_response = tools::dispatch_with_read_context(
        Arc::clone(&state),
        "get_daemon_status",
        None,
        Some(&captured_old),
    )
    .await
    .expect("captured old read must succeed");

    assert_eq!(
        old_response["provenance"],
        json!({
            "workspace_id": "workspace-provenance",
            "branch": "feature/f43",
            "generation_id": "gen-old"
        }),
        "response provenance must come from the pinned captured context even when a newer generation exists"
    );

    let new_response =
        tools::dispatch_with_read_context(state, "get_daemon_status", None, Some(&captured_new))
            .await
            .expect("captured new read must succeed");

    assert_eq!(
        new_response["provenance"]["generation_id"],
        json!("gen-new"),
        "the decorator must follow the supplied captured context, not cached or live global state"
    );
}
