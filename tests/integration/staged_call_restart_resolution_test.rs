//! End-to-end durability proof for staged cross-file call resolution across a
//! simulated daemon restart (089.003-T).
//!
//! This is the load-bearing test for feature 089-F. It proves that a cross-file
//! call which is STAGED but UNRESOLVED before a restart still resolves after a
//! restart, purely from the rehydrated `staged_call` rows — matching a full
//! re-index and creating NO false edges.
//!
//! Flow:
//!   1. `sync_workspace` (incremental) stages `caller -> helper` but does NOT run
//!      the post-pass, so it is staged-but-unresolved (singleton_count == 0).
//!   2. `dehydrate_code_graph` writes nodes/edges/staged_calls.jsonl.
//!   3. Simulate restart: a fresh data dir (empty DB) receives a copy of the
//!      code-graph JSONL subtree; `hydrate_code_graph` restores nodes, edges,
//!      and staged rows.
//!   4. The post-pass `reresolve_calls_edges` resolves the rehydrated staged
//!      call into exactly one `calls_resolved_singleton` edge.
//!   5. The result matches a full re-index oracle, with no extra/false edges.
//!
//! Without staged durability (pre-089-F) the staged rows would be lost on
//! dehydration and the call could NEVER resolve after restart.

#![allow(clippy::needless_raw_string_hashes)]
#![allow(clippy::doc_markdown)]

use std::fs;
use std::path::Path;

use tokio::test;

use engram::db::connect_db;
use engram::db::queries::CodeGraphQueries;
use engram::models::config::CodeGraphConfig;
use engram::services::code_graph;
use engram::services::dehydration::dehydrate_code_graph;
use engram::services::hydration::hydrate_code_graph;

fn write_sample_file(dir: &Path, rel_path: &str, content: &str) {
    let full = dir.join(rel_path);
    if let Some(parent) = full.parent() {
        fs::create_dir_all(parent).expect("create dirs");
    }
    fs::write(full, content).expect("write file");
}

fn branch_for(path: &Path) -> String {
    use sha2::{Digest, Sha256};
    let canon = path
        .canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .to_lowercase();
    format!("{:x}", Sha256::digest(canon.as_bytes()))
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

/// Recursively copy a directory subtree (used to move the JSONL snapshot from
/// the "pre-restart" data dir into a fresh "post-restart" data dir).
fn copy_dir_all(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).expect("create dst dir");
    for entry in fs::read_dir(src).expect("read_dir") {
        let entry = entry.expect("dir entry");
        let ty = entry.file_type().expect("file type");
        let target = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).expect("copy file");
        }
    }
}

#[test]
async fn staged_call_resolves_after_simulated_restart_via_rehydration() {
    let ws_tmp = tempfile::tempdir().expect("ws tempdir");
    let ws = ws_tmp.path();
    write_sample_file(ws, "src/a.rs", "pub fn caller() {\n    helper();\n}\n");
    write_sample_file(ws, "src/b.rs", "pub fn helper() {}\n");
    let branch = branch_for(ws);
    let config = CodeGraphConfig::default();

    // --- Pre-restart daemon: incremental sync stages the call, no post-pass. ---
    let data_orig_tmp = tempfile::tempdir().expect("data_orig tempdir");
    let data_orig = data_orig_tmp.path();
    code_graph::sync_workspace(ws, data_orig, &branch, &config)
        .await
        .expect("sync");
    let q_orig = queries_for(data_orig, &branch).await;
    assert_eq!(
        singleton_count(&q_orig).await,
        0,
        "sync must not resolve the cross-file call (staged-but-unresolved)"
    );
    let staged_pre = q_orig.list_staged_calls().await.expect("list_staged_calls");
    assert!(
        staged_pre.iter().any(|s| s.callee_name == "helper"),
        "the cross-file call must be staged pre-restart, got {staged_pre:?}"
    );

    // Dehydrate the graph (including staged rows) to JSONL.
    dehydrate_code_graph(&q_orig, data_orig, &branch)
        .await
        .expect("dehydrate");
    let staged_jsonl = data_orig
        .join("code-graph")
        .join(&branch)
        .join("staged_calls.jsonl");
    assert!(
        staged_jsonl.exists(),
        "dehydration must persist staged_calls.jsonl"
    );

    // --- Simulate restart: fresh empty DB + copied JSONL snapshot. ---
    let data_restart_tmp = tempfile::tempdir().expect("data_restart tempdir");
    let data_restart = data_restart_tmp.path();
    copy_dir_all(
        &data_orig.join("code-graph").join(&branch),
        &data_restart.join("code-graph").join(&branch),
    );
    let q_restart = queries_for(data_restart, &branch).await;
    hydrate_code_graph(ws, data_restart, &branch, &q_restart)
        .await
        .expect("hydrate");

    // Staged rows must be restored, and still unresolved until the post-pass runs.
    let staged_post = q_restart
        .list_staged_calls()
        .await
        .expect("list_staged_calls post-hydrate");
    assert!(
        staged_post.iter().any(|s| s.callee_name == "helper"),
        "staged rows must survive rehydration, got {staged_post:?}"
    );
    assert_eq!(
        singleton_count(&q_restart).await,
        0,
        "the call must still be unresolved immediately after rehydration"
    );

    // --- Post-pass resolves the rehydrated staged call. ---
    q_restart
        .reresolve_calls_edges()
        .await
        .expect("reresolve after restart");
    assert_eq!(
        singleton_count(&q_restart).await,
        1,
        "the post-pass must resolve the rehydrated staged call into one edge"
    );

    // --- Oracle: a full re-index yields the same singleton edge. ---
    let data_full_tmp = tempfile::tempdir().expect("data_full tempdir");
    let data_full = data_full_tmp.path();
    code_graph::index_workspace(ws, data_full, &branch, &config, false)
        .await
        .expect("full index");
    let q_full = queries_for(data_full, &branch).await;
    assert_eq!(
        singleton_count(&q_full).await,
        1,
        "full re-index oracle must produce one singleton edge"
    );

    // No false edges: the rehydrated path must not create MORE singleton edges
    // than the full-index oracle.
    let restart_singletons = q_restart
        .list_calls_edges_by_resolution("calls_resolved_singleton")
        .await
        .expect("list restart singletons");
    let full_singletons = q_full
        .list_calls_edges_by_resolution("calls_resolved_singleton")
        .await
        .expect("list full singletons");
    assert_eq!(
        restart_singletons.len(),
        full_singletons.len(),
        "rehydrated resolution must match the full re-index exactly (no false edges)"
    );
}
