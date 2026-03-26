//! Integration tests for wiring `record_file_hash` into the indexing pipeline
//! (TASK-009.09).
//!
//! Engram analysis findings:
//! * `record_file_hash` exists in `src/services/file_tracker.rs` with the
//!   correct signature but has **zero call sites** in the codebase.
//! * `index_workspace_inner` in `src/tools/write.rs` already receives
//!   `(ws_path, data_dir, branch)` and iterates over files — but never calls
//!   `record_file_hash` after a successful file index.
//! * `detect_offline_changes` is called once in `set_workspace`; it reads
//!   the `file_hash` table which remains empty because no writer stamps hashes.
//!
//! These tests will fail (red phase) until `record_file_hash` is called
//! inside `index_workspace_inner` for every successfully indexed file.
//!
//! Tests: S085–S088.

use std::fs;
use std::sync::Arc;

use serde_json::json;
use tempfile::TempDir;
use tokio::test;

use engram::db::connect_db;
use engram::db::queries::CodeGraphQueries;
use engram::server::state::AppState;
use engram::services::file_tracker::detect_offline_changes;
use engram::tools;

// ── Test helpers ──────────────────────────────────────────────────────────────

/// Create a git workspace containing the given source files, bind it with
/// `set_workspace`, and return the temp dir and shared state.
async fn bind_git_workspace(files: &[(&str, &str)]) -> (TempDir, Arc<AppState>) {
    let ws = TempDir::new().expect("tempdir");
    let git_dir = ws.path().join(".git");
    fs::create_dir_all(&git_dir).expect("create .git");
    fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").expect("write HEAD");

    for (rel, content) in files {
        let abs = ws.path().join(rel);
        if let Some(parent) = abs.parent() {
            fs::create_dir_all(parent).expect("create parent dir");
        }
        fs::write(&abs, content).expect("write source file");
    }

    let state = Arc::new(AppState::new(10));
    let path = ws.path().to_string_lossy().to_string();
    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace must succeed");

    (ws, state)
}

// ── S085: No offline changes reported after index_workspace ──────────────────

/// S085: After `index_workspace` completes, `detect_offline_changes` returns
/// an empty change set for the indexed source files.
///
/// **Red phase** — fails because `record_file_hash` is never called inside
/// `index_workspace_inner`, so `file_hash` table stays empty and every
/// indexed file is incorrectly reported as Added on the next startup.
///
/// Worker instruction: after each successful `queries.upsert_code_file` call
/// in `index_workspace_inner`, call
/// `record_file_hash(&rel_path, file_path, &queries).await?`.
#[test]
async fn s085_no_offline_changes_after_index_workspace() {
    // GIVEN a workspace with Rust source files
    let (ws, state) = bind_git_workspace(&[
        ("src/lib.rs", "pub fn hello() -> &'static str { \"hello\" }"),
        ("src/main.rs", "fn main() { println!(\"hello\"); }"),
    ])
    .await;

    // WHEN index_workspace is called
    tools::dispatch(
        state.clone(),
        "index_workspace",
        Some(json!({ "force": true })),
    )
    .await
    .expect("index_workspace must succeed");

    // THEN detect_offline_changes returns no changes for indexed src/ files
    let snap = state.snapshot_workspace().await.expect("snapshot");
    let db = connect_db(&snap.data_dir, &snap.branch)
        .await
        .expect("connect db");
    let queries = CodeGraphQueries::new(db);

    let changes = detect_offline_changes(ws.path(), &queries)
        .await
        .expect("detect_offline_changes");

    let src_changes: Vec<_> = changes
        .iter()
        .filter(|c| c.path.starts_with("src/"))
        .collect();

    assert!(
        src_changes.is_empty(),
        "after index_workspace all src/ files must have current hashes; \
         got {src_changes:?} \
         (worker: call record_file_hash in index_workspace_inner)"
    );
}

// ── S086: Indexed files have file_hash records ────────────────────────────────

/// S086: After `index_workspace`, each successfully indexed source file has a
/// corresponding record in the `file_hash` table.
///
/// **Red phase** — fails until `record_file_hash` is wired into
/// `index_workspace_inner`.
#[test]
async fn s086_indexed_files_have_file_hash_records() {
    // GIVEN a workspace with a single source file
    let (ws, state) = bind_git_workspace(&[("src/utils.rs", "pub fn helper() {}")]).await;

    // WHEN index_workspace is called
    tools::dispatch(
        state.clone(),
        "index_workspace",
        Some(json!({ "force": true })),
    )
    .await
    .expect("index_workspace must succeed");

    // THEN the file_hash table contains a record for src/utils.rs
    let snap = state.snapshot_workspace().await.expect("snapshot");
    let db = connect_db(&snap.data_dir, &snap.branch)
        .await
        .expect("connect db");
    let queries = CodeGraphQueries::new(db);

    let hashes = queries
        .get_all_file_hashes()
        .await
        .expect("get_all_file_hashes");

    let has_utils = hashes.iter().any(|r| r.file_path == "src/utils.rs");
    assert!(
        has_utils,
        "src/utils.rs must have a file_hash record after indexing; \
         stored: {:?} \
         (worker: call record_file_hash in index_workspace_inner)",
        hashes.iter().map(|r| &r.file_path).collect::<Vec<_>>()
    );

    let _ = ws; // keep temp dir alive
}

// ── S087: Re-indexing unchanged files leaves change set empty ─────────────────

/// S087: Running `index_workspace` twice on the same unchanged files does not
/// cause `detect_offline_changes` to report changes on the second run.
#[test]
async fn s087_reindex_unchanged_files_is_idempotent() {
    // GIVEN a workspace with a stable source file
    let (ws, state) =
        bind_git_workspace(&[("src/stable.rs", "pub fn stable() -> u32 { 42 }")]).await;

    // WHEN index_workspace is called twice with force=true
    tools::dispatch(
        state.clone(),
        "index_workspace",
        Some(json!({ "force": true })),
    )
    .await
    .expect("first index_workspace");
    tools::dispatch(
        state.clone(),
        "index_workspace",
        Some(json!({ "force": true })),
    )
    .await
    .expect("second index_workspace");

    // THEN detect_offline_changes shows no changes for that file
    let snap = state.snapshot_workspace().await.expect("snapshot");
    let db = connect_db(&snap.data_dir, &snap.branch)
        .await
        .expect("connect db");
    let queries = CodeGraphQueries::new(db);

    let changes = detect_offline_changes(ws.path(), &queries)
        .await
        .expect("detect_offline_changes");

    let stable_changes: Vec<_> = changes
        .iter()
        .filter(|c| c.path == "src/stable.rs")
        .collect();

    assert!(
        stable_changes.is_empty(),
        "stable.rs must have no changes after two identical index runs; \
         got: {stable_changes:?}"
    );
}

// ── S088: File modification detected after hash is recorded ───────────────────

/// S088: After `index_workspace` records a hash for a file, a subsequent
/// on-disk modification of that file is correctly detected by
/// `detect_offline_changes` as `Modified`.
///
/// This confirms that recording hashes during indexing enables accurate
/// offline change detection — the pipeline is end-to-end correct.
#[test]
async fn s088_file_modified_after_indexing_is_detected() {
    // GIVEN a workspace with an indexed source file
    let (ws, state) =
        bind_git_workspace(&[("src/mutable.rs", "pub fn version() -> u32 { 1 }")]).await;

    tools::dispatch(
        state.clone(),
        "index_workspace",
        Some(json!({ "force": true })),
    )
    .await
    .expect("index_workspace");

    let snap = state.snapshot_workspace().await.expect("snapshot");
    let db = connect_db(&snap.data_dir, &snap.branch)
        .await
        .expect("connect db");
    let queries = CodeGraphQueries::new(db);

    // WHEN the file is modified after indexing recorded its hash
    fs::write(
        ws.path().join("src/mutable.rs"),
        "pub fn version() -> u32 { 2 }",
    )
    .expect("write updated content");

    // THEN detect_offline_changes reports src/mutable.rs as Modified
    let changes = detect_offline_changes(ws.path(), &queries)
        .await
        .expect("detect_offline_changes");

    let modified: Vec<_> = changes
        .iter()
        .filter(|c| c.path == "src/mutable.rs")
        .collect();

    assert_eq!(
        modified.len(),
        1,
        "modified file must appear in offline change report; \
         changes: {changes:?}"
    );
}
