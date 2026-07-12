//! Integration tests for the cross-file calls resolution post-pass (082.008-T).
//!
//! The post-pass resolves staged cross-file calls (082.002-T) against a
//! workspace-global `name -> [function_id]` index and creates a `calls_edge`
//! ONLY for unambiguous (exactly-one-definition) names, tagged with the
//! canonical provenance `calls_resolved_singleton`. It runs in the full /
//! `--force` index path only — never on incremental sync.
//!
//! Scenarios (6):
//!   1. unique cross-file name -> one edge tagged `calls_resolved_singleton`
//!   2. ambiguous name (2 defs) -> skipped (no edge)
//!   3. name with no matching def -> no edge (no false edge)
//!   4. incremental sync path -> post-pass NOT invoked (no singleton edges)
//!   5. a singleton that became ambiguous (2nd same-named def added) is
//!      retracted by re-resolution even when the caller file is unchanged
//!   6. with EMPTY staging (post-rehydration / fresh upgrade) the post-pass
//!      PRESERVES existing singleton edges instead of destroying them

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

// Scenario 1: an unambiguous cross-file callee yields exactly one
// calls_resolved_singleton edge.
#[test]
async fn unique_cross_file_name_resolves_to_singleton_edge() {
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
    assert_eq!(
        singleton_count(&q).await,
        1,
        "unambiguous cross-file call must create one singleton edge"
    );
    let singletons = q
        .list_calls_edges_by_resolution("calls_resolved_singleton")
        .await
        .expect("list singletons");
    assert_eq!(
        singletons.len(),
        1,
        "exactly one singleton edge, {singletons:?}"
    );
}

// Scenario 2: an ambiguous name (two definitions) is skipped — no edge.
#[test]
async fn ambiguous_name_is_skipped() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_sample_file(ws, "src/a.rs", "pub fn caller() {\n    dup();\n}\n");
    write_sample_file(ws, "src/b.rs", "pub fn dup() {}\n");
    write_sample_file(ws, "src/c.rs", "pub fn dup() {}\n");

    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);
    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("index");

    let q = queries_for(&data_dir, &branch).await;
    assert_eq!(
        singleton_count(&q).await,
        0,
        "ambiguous name (2 defs) must not create a singleton edge"
    );
}

// Scenario 3: a call to a name with no definition creates no false edge.
#[test]
async fn unmatched_name_creates_no_false_edge() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_sample_file(
        ws,
        "src/a.rs",
        "pub fn caller() {\n    absent_target_fn();\n}\n",
    );

    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);
    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("index");

    let q = queries_for(&data_dir, &branch).await;
    assert_eq!(
        singleton_count(&q).await,
        0,
        "unmatched callee name must not create a false singleton edge"
    );
}

// Scenario 4: the incremental sync path does NOT invoke the post-pass.
#[test]
async fn sync_path_does_not_invoke_postpass() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_sample_file(ws, "src/a.rs", "pub fn caller() {\n    helper();\n}\n");
    write_sample_file(ws, "src/b.rs", "pub fn helper() {}\n");

    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);
    // First-time sync exercises the incremental path (all files "added").
    code_graph::sync_workspace(ws, &data_dir, &branch, &config)
        .await
        .expect("sync");

    let q = queries_for(&data_dir, &branch).await;
    // The call is staged (082.002-T) but never promoted to a singleton edge,
    // because the post-pass runs on full index only.
    assert_eq!(
        singleton_count(&q).await,
        0,
        "sync path must not create singleton edges"
    );
    let staged = q.list_staged_calls().await.expect("list_staged_calls");
    assert!(
        staged.iter().any(|s| s.callee_name == "helper"),
        "sync path must still stage the unresolved cross-file call, got {staged:?}"
    );
}

// Scenario 5: a singleton resolved on a prior index must be RETRACTED by
// re-resolution once its callee name becomes ambiguous (a second same-named
// definition is added). The caller file is unchanged, so a non-forced full
// index skips it; the post-pass alone must revalidate and drop the now-invalid
// singleton rather than leaving it stale.
#[test]
async fn reresolution_retracts_now_ambiguous_singleton() {
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

    // Add a SECOND `helper` definition -> the name is now ambiguous. The caller
    // file is unchanged, so a non-forced index skips it and the post-pass must
    // retract the stale singleton on its own.
    write_sample_file(ws, "src/c.rs", "pub fn helper() {}\n");
    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("index 2");

    let q = queries_for(&data_dir, &branch).await;
    assert_eq!(
        singleton_count(&q).await,
        0,
        "a singleton that became ambiguous must be retracted by re-resolution"
    );
}

// Scenario 6: the post-pass must PRESERVE existing singleton edges when the
// staging relation is empty. This models JSONL rehydration and a fresh upgrade,
// where singleton edges are restored but `staged_call` rows are not — a
// destructive global retract-then-rebuild would wrongly delete every singleton.
#[test]
async fn postpass_preserves_singletons_when_staging_empty() {
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

    // Model rehydration: the singleton edge exists but staging is empty.
    q.clear_staged_calls_for_file("src/a.rs")
        .await
        .expect("clear staging");
    assert!(
        q.list_staged_calls()
            .await
            .expect("list_staged_calls")
            .is_empty(),
        "staging must be empty to model the rehydration case"
    );

    // Re-running the post-pass must NOT destroy the existing singleton.
    q.reresolve_calls_edges().await.expect("reresolve");
    assert_eq!(
        singleton_count(&q).await,
        1,
        "post-pass must preserve singletons when staging is empty"
    );
}
