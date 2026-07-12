//! Integration tests for the calls staging lifecycle (082.009-T).
//!
//! Covers clear-before-reindex, file-deletion cleanup, and retraction of stale
//! `calls_resolved_singleton` edges so that a changed or removed caller/callee
//! never leaves a dangling cross-file edge or a stale staged row.
//!
//! Scenarios (4):
//!   1. re-indexing a file whose call CHANGED retracts its stale singleton edge
//!      and clears the old staged row
//!   2. DELETING a file retracts its resolved edges and clears its staged rows
//!   3. a deleted CALLEE retracts the inbound singleton edge (caller elsewhere)
//!   4. clear-before-reindex leaves no stale staged row for a forced post-pass

#![allow(clippy::needless_raw_string_hashes)]
#![allow(clippy::doc_markdown)]

use std::fs;
use std::path::Path;

use tokio::test;

use engram::db::connect_db;
use engram::db::queries::CodeGraphQueries;
use engram::models::config::CodeGraphConfig;
use engram::services::code_graph;

fn write_sample_file(dir: &Path, rel_path: &str, content: &str) {
    let full = dir.join(rel_path);
    if let Some(parent) = full.parent() {
        fs::create_dir_all(parent).expect("create dirs");
    }
    fs::write(full, content).expect("write file");
}

fn test_db_params(path: &Path) -> (std::path::PathBuf, String) {
    use sha2::{Digest, Sha256};
    let canon = path
        .canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .to_lowercase();
    let branch = format!("{:x}", Sha256::digest(canon.as_bytes()));
    (std::env::temp_dir().join("engram-test"), branch)
}

async fn queries_for(data_dir: &Path, branch: &str) -> CodeGraphQueries {
    let db = connect_db(data_dir, branch).await.expect("connect_db");
    CodeGraphQueries::new(db)
}

async fn singleton_count(q: &CodeGraphQueries) -> u64 {
    q.count_calls_edges_by_resolution()
        .await
        .expect("count_calls_edges_by_resolution")
        .get("calls_resolved_singleton")
        .copied()
        .unwrap_or(0)
}

async fn staged_callees(q: &CodeGraphQueries) -> Vec<String> {
    q.list_staged_calls()
        .await
        .expect("list_staged_calls")
        .into_iter()
        .map(|s| s.callee_name)
        .collect()
}

// Scenario 1: re-indexing a file whose call changed retracts the stale
// singleton edge and clears the old staged row (no dangling edge).
#[test]
async fn reindex_changed_call_retracts_stale_singleton() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_sample_file(ws, "src/a.rs", "pub fn caller() {\n    helper();\n}\n");
    write_sample_file(ws, "src/b.rs", "pub fn helper() {}\n");

    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);
    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("index 1");
    let q = queries_for(&data_dir, &branch).await;
    assert_eq!(singleton_count(&q).await, 1, "baseline singleton edge");

    // Caller no longer calls helper — force a full reindex.
    write_sample_file(ws, "src/a.rs", "pub fn caller() {}\n");
    code_graph::index_workspace(ws, &data_dir, &branch, &config, true)
        .await
        .expect("index 2");

    let q = queries_for(&data_dir, &branch).await;
    assert_eq!(
        singleton_count(&q).await,
        0,
        "stale singleton edge must be retracted after the call is removed"
    );
    assert!(
        !staged_callees(&q).await.iter().any(|c| c == "helper"),
        "old staged row for the removed call must be cleared"
    );
}

// Scenario 2: deleting a file retracts its resolved edges and clears its
// staged rows.
#[test]
async fn deleting_caller_file_retracts_and_clears() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_sample_file(ws, "src/a.rs", "pub fn caller() {\n    helper();\n}\n");
    write_sample_file(ws, "src/b.rs", "pub fn helper() {}\n");

    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);
    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("index");
    let q = queries_for(&data_dir, &branch).await;
    assert_eq!(singleton_count(&q).await, 1, "baseline singleton edge");

    // Remove the caller's file and sync — triggers handle_deleted_file.
    fs::remove_file(ws.join("src/a.rs")).expect("remove a.rs");
    code_graph::sync_workspace(ws, &data_dir, &branch, &config)
        .await
        .expect("sync after delete");

    let q = queries_for(&data_dir, &branch).await;
    assert_eq!(
        singleton_count(&q).await,
        0,
        "deleting the caller file must retract its singleton edge"
    );
    assert!(
        staged_callees(&q).await.is_empty(),
        "deleting the caller file must clear its staged rows"
    );
}

// Scenario 3: deleting the callee's file retracts the inbound singleton edge
// (the caller lives in another file).
#[test]
async fn deleting_callee_file_retracts_inbound_singleton() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_sample_file(ws, "src/a.rs", "pub fn caller() {\n    shared();\n}\n");
    write_sample_file(ws, "src/b.rs", "pub fn shared() {}\n");

    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);
    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("index");
    let q = queries_for(&data_dir, &branch).await;
    assert_eq!(singleton_count(&q).await, 1, "baseline singleton edge");

    // Remove the callee's file and sync.
    fs::remove_file(ws.join("src/b.rs")).expect("remove b.rs");
    code_graph::sync_workspace(ws, &data_dir, &branch, &config)
        .await
        .expect("sync after delete");

    let q = queries_for(&data_dir, &branch).await;
    assert_eq!(
        singleton_count(&q).await,
        0,
        "deleting the callee file must retract the inbound singleton edge"
    );
}

// Scenario 4: clear-before-reindex leaves no stale staged row that a later
// forced post-pass could resolve into a stale edge.
#[test]
async fn clear_before_reindex_prevents_stale_postpass_edge() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_sample_file(ws, "src/a.rs", "pub fn caller() {\n    helper();\n}\n");
    write_sample_file(ws, "src/b.rs", "pub fn helper() {}\n");

    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);
    // First-time sync stages the call but does not run the post-pass.
    code_graph::sync_workspace(ws, &data_dir, &branch, &config)
        .await
        .expect("sync 1");
    let q = queries_for(&data_dir, &branch).await;
    assert!(
        staged_callees(&q).await.iter().any(|c| c == "helper"),
        "sync must stage the cross-file call"
    );

    // Remove the call and re-sync — clear-before-reindex must drop the stale row.
    write_sample_file(ws, "src/a.rs", "pub fn caller() {}\n");
    code_graph::sync_workspace(ws, &data_dir, &branch, &config)
        .await
        .expect("sync 2");
    let q = queries_for(&data_dir, &branch).await;
    assert!(
        !staged_callees(&q).await.iter().any(|c| c == "helper"),
        "clear-before-reindex must remove the stale staged row"
    );

    // A later forced post-pass must not resolve anything from the cleared row.
    code_graph::index_workspace(ws, &data_dir, &branch, &config, true)
        .await
        .expect("forced index");
    let q = queries_for(&data_dir, &branch).await;
    assert_eq!(
        singleton_count(&q).await,
        0,
        "forced post-pass must not create an edge from a cleared staged row"
    );
}
