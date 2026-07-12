//! Integration tests for cross-file call staging capture (082.002-T).
//!
//! Verifies that when a callee cannot be resolved within the caller's own
//! file, the call site is recorded in the `staged_call` relation rather than
//! silently dropped, on BOTH the full-index and incremental-sync paths, while
//! in-file calls still create a direct `calls_edge` and blocklisted names are
//! never staged.
//!
//! Scenarios (4):
//!   1. cross-file callee is recorded in `staged_call` (not dropped)
//!   2. in-file callee still yields a direct `calls_edge`
//!   3. sync path parity with index path (identical staged rows)
//!   4. blocklisted names are never staged

#![allow(clippy::needless_raw_string_hashes)]
#![allow(clippy::doc_markdown)]

use std::collections::HashSet;
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

/// (callee_name, source_file) pairs of all staged calls — caller_id is a
/// per-index UUID so it is excluded to allow cross-workspace comparison.
async fn staged_pairs(q: &CodeGraphQueries) -> HashSet<(String, String)> {
    q.list_staged_calls()
        .await
        .expect("list_staged_calls")
        .into_iter()
        .map(|s| (s.callee_name, s.source_file))
        .collect()
}

#[test]
async fn cross_file_callee_is_recorded_not_dropped() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    // `caller` (a.rs) calls `helper` which is defined in b.rs — cross-file.
    write_sample_file(ws, "src/a.rs", "pub fn caller() {\n    helper();\n}\n");
    write_sample_file(ws, "src/b.rs", "pub fn helper() {}\n");

    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);
    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("index");

    let q = queries_for(&data_dir, &branch).await;
    let staged = q.list_staged_calls().await.expect("list_staged_calls");
    assert!(
        staged
            .iter()
            .any(|s| s.callee_name == "helper" && s.source_file == "src/a.rs"),
        "cross-file call `helper` from src/a.rs must be staged, got {staged:?}"
    );
}

#[test]
async fn in_file_callee_yields_direct_edge_not_staged() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    // Both functions live in the same file — the call resolves locally.
    write_sample_file(
        ws,
        "src/c.rs",
        "pub fn c1() {\n    c2();\n}\npub fn c2() {}\n",
    );

    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);
    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("index");

    let q = queries_for(&data_dir, &branch).await;
    let calls = q.count_calls_edges().await.expect("count_calls_edges");
    assert!(
        calls >= 1,
        "in-file c1 -> c2 should create a direct calls_edge"
    );
    let staged = q.list_staged_calls().await.expect("list_staged_calls");
    assert!(
        !staged.iter().any(|s| s.callee_name == "c2"),
        "in-file callee c2 must NOT be staged, got {staged:?}"
    );
}

#[test]
async fn sync_path_parity_with_index_path() {
    let config = CodeGraphConfig::default();

    // Workspace A: full index path.
    let tmp_a = tempfile::tempdir().expect("tempdir a");
    let ws_a = tmp_a.path();
    write_sample_file(ws_a, "src/a.rs", "pub fn caller() {\n    helper();\n}\n");
    write_sample_file(ws_a, "src/b.rs", "pub fn helper() {}\n");
    let (dir_a, branch_a) = test_db_params(ws_a);
    code_graph::index_workspace(ws_a, &dir_a, &branch_a, &config, false)
        .await
        .expect("index a");
    let staged_a = staged_pairs(&queries_for(&dir_a, &branch_a).await).await;

    // Workspace B: identical content, first-time sync path (all files "added").
    let tmp_b = tempfile::tempdir().expect("tempdir b");
    let ws_b = tmp_b.path();
    write_sample_file(ws_b, "src/a.rs", "pub fn caller() {\n    helper();\n}\n");
    write_sample_file(ws_b, "src/b.rs", "pub fn helper() {}\n");
    let (dir_b, branch_b) = test_db_params(ws_b);
    code_graph::sync_workspace(ws_b, &dir_b, &branch_b, &config)
        .await
        .expect("sync b");
    let staged_b = staged_pairs(&queries_for(&dir_b, &branch_b).await).await;

    assert!(
        staged_a.contains(&("helper".to_owned(), "src/a.rs".to_owned())),
        "index path must stage helper, got {staged_a:?}"
    );
    assert_eq!(
        staged_a, staged_b,
        "sync path must stage the same (callee, source_file) pairs as index path"
    );
}

#[test]
async fn blocklisted_names_are_never_staged() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    // `clone` is blocklisted (never extracted); `realcall` is a genuine
    // cross-file call that must be staged.
    write_sample_file(
        ws,
        "src/d.rs",
        "pub fn f() {\n    let x = 1;\n    x.clone();\n    realcall();\n}\n",
    );

    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);
    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("index");

    let q = queries_for(&data_dir, &branch).await;
    let staged = q.list_staged_calls().await.expect("list_staged_calls");
    assert!(
        staged.iter().any(|s| s.callee_name == "realcall"),
        "genuine cross-file call `realcall` must be staged, got {staged:?}"
    );
    assert!(
        !staged.iter().any(|s| s.callee_name == "clone"),
        "blocklisted `clone` must NEVER be staged, got {staged:?}"
    );
}
