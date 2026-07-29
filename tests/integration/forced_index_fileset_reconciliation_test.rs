//! U2 (103.002-T / 92EE75BB): forced-index file-set reconciliation harness.
//!
//! Proves the forced-index route (`index_workspace`) advances the durable
//! `code_graph_extraction_generation` marker while visiting ONLY currently
//! discovered files — so a file that was previously indexed but is now
//! **excluded** (still on disk, so it never re-appears in a future incremental
//! sync's deletion phase) keeps its stale same-file `direct` edge and stale
//! `function_meta` while the marker falsely certifies its generation.
//!
//! The incremental sync path already reconciles the prior indexed set against
//! the current set (Phase 1 deletion sweep, `handle_deleted_file`); the forced
//! route must gain the equivalent — evict `indexed − discovered` BEFORE the
//! marker advances, gated identically to the marker-advance condition
//! (`force || !any_hash_skipped`) so a partial hash-skip index never evicts.
//!
//! RED in U2: the forced route does no reconciliation, so the excluded file's
//! stale edge survives. U2 GREEN evicts it before certifying.

#![allow(clippy::doc_markdown)]

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use sha2::{Digest, Sha256};
use tokio::test;

use engram::db::connect_db;
use engram::db::queries::CodeGraphQueries;
use engram::models::config::CodeGraphConfig;
use engram::services::code_graph;

// ── Corpus: two files, each with a unique-name same-file direct call ─────────

const RUST_A: &str = "\
pub fn a_helper() -> u8 { 1 }

pub fn a_caller() {
    let _ = a_helper();
}
";

const RUST_B: &str = "\
pub fn b_helper() -> u8 { 2 }

pub fn b_caller() {
    let _ = b_helper();
}
";

const CARGO_TOML: &str = "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2024\"\n";

// ── Harness helpers ──────────────────────────────────────────────────────────

fn write_one(ws: &Path, rel: &str, content: &str) {
    let full = ws.join(rel);
    if let Some(parent) = full.parent() {
        fs::create_dir_all(parent).expect("create dirs");
    }
    fs::write(full, content).expect("write file");
}

fn write_fixture(ws: &Path) {
    write_one(ws, "Cargo.toml", CARGO_TOML);
    write_one(ws, "src/a.rs", RUST_A);
    write_one(ws, "src/b.rs", RUST_B);
}

fn test_db_params(path: &Path) -> (std::path::PathBuf, String) {
    let canon = path
        .canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .to_lowercase();
    let branch = format!("{:x}", Sha256::digest(canon.as_bytes()));
    (std::env::temp_dir().join("engram-test"), branch)
}

/// A `CodeGraphConfig` that excludes `src/a.rs` from discovery (still on disk).
fn config_excluding_a() -> CodeGraphConfig {
    CodeGraphConfig {
        exclude_patterns: vec!["src/a.rs".to_owned()],
        ..CodeGraphConfig::default()
    }
}

/// `id -> name` over every indexed function.
async fn id_to_name(q: &CodeGraphQueries) -> HashMap<String, String> {
    q.all_functions()
        .await
        .expect("all_functions")
        .into_iter()
        .map(|f| (f.id, f.name))
        .collect()
}

/// Every `direct` `calls_edge` mapped to `(from_name, to_name)`. A missing
/// endpoint id surfaces as `<dangling:...>` rather than a silent `""`.
async fn direct_edge_names(q: &CodeGraphQueries) -> Vec<(String, String)> {
    let names = id_to_name(q).await;
    q.list_calls_edges_by_resolution("direct")
        .await
        .expect("list direct edges")
        .into_iter()
        .map(|(from, to)| {
            let f = names
                .get(&from)
                .cloned()
                .unwrap_or_else(|| format!("<dangling:{from}>"));
            let t = names
                .get(&to)
                .cloned()
                .unwrap_or_else(|| format!("<dangling:{to}>"));
            (f, t)
        })
        .collect()
}

// ── Scenario 1 (H5 positive): excluded-still-on-disk file is evicted ─────────

/// A previously-indexed file that is now excluded (still on disk) has its stale
/// same-file direct edge AND `function_meta`/`code_file` evicted before the
/// forced index certifies the generation. RED in U2 — the stale edge survives.
#[test]
async fn forced_index_evicts_excluded_still_on_disk_file() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_fixture(ws);
    let (data_dir, branch) = test_db_params(ws);

    // Initial index: both files discovered and indexed.
    code_graph::index_workspace(ws, &data_dir, &branch, &CodeGraphConfig::default(), false)
        .await
        .expect("initial index should succeed");

    let db = connect_db(&data_dir, &branch).await.expect("db connect");
    let q = CodeGraphQueries::new(db);
    let before = direct_edge_names(&q).await;
    assert!(
        before.contains(&("a_caller".to_owned(), "a_helper".to_owned())),
        "precondition: a.rs same-file edge indexed; got {before:?}"
    );
    assert!(
        before.contains(&("b_caller".to_owned(), "b_helper".to_owned())),
        "precondition: b.rs same-file edge indexed; got {before:?}"
    );

    // Forced re-index with a.rs excluded (still on disk, no longer discovered).
    code_graph::index_workspace(ws, &data_dir, &branch, &config_excluding_a(), true)
        .await
        .expect("forced re-index should succeed");

    let db2 = connect_db(&data_dir, &branch).await.expect("db reconnect");
    let q2 = CodeGraphQueries::new(db2);
    let after = direct_edge_names(&q2).await;

    // The excluded file's stale same-file edge must be evicted.
    assert!(
        !after.iter().any(|(f, t)| f == "a_caller" || t == "a_helper"),
        "forced-index reconciliation must evict the excluded file's stale edge; remaining: {after:?}"
    );
    // Its code_file / function_meta must be gone too (full eviction).
    assert!(
        q2.get_code_file_by_path("src/a.rs")
            .await
            .expect("query a.rs")
            .is_none(),
        "excluded file's code_file record must be removed by reconciliation"
    );

    // The still-discovered file's edge is preserved (no collateral eviction).
    assert!(
        after.contains(&("b_caller".to_owned(), "b_helper".to_owned())),
        "the still-discovered b.rs edge must survive; got {after:?}"
    );

    // Marker certifies only AFTER reconciliation.
    assert_eq!(
        q2.code_graph_extraction_generation()
            .expect("read generation marker"),
        Some("1".to_owned()),
        "the generation marker advances after a clean forced-index reconciliation"
    );
}

// ── Scenario 2 (H5 negative): discovered files are never evicted ─────────────

/// A forced re-index that discovers every file evicts nothing — both same-file
/// edges survive. Guards against over-eager reconciliation.
#[test]
async fn forced_index_keeps_all_discovered_files() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_fixture(ws);
    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);

    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("initial index should succeed");
    // Forced re-index, nothing excluded.
    code_graph::index_workspace(ws, &data_dir, &branch, &config, true)
        .await
        .expect("forced re-index should succeed");

    let db = connect_db(&data_dir, &branch).await.expect("db connect");
    let q = CodeGraphQueries::new(db);
    let after = direct_edge_names(&q).await;
    assert!(
        after.contains(&("a_caller".to_owned(), "a_helper".to_owned())),
        "a.rs edge must survive when a.rs is still discovered; got {after:?}"
    );
    assert!(
        after.contains(&("b_caller".to_owned(), "b_helper".to_owned())),
        "b.rs edge must survive; got {after:?}"
    );
}

// ── Scenario 3 (H4 idempotence): second forced index reconciles zero ─────────

/// After the excluded file is evicted, a SECOND forced index (still excluded)
/// reconciles nothing new and re-certifies the same marker value.
#[test]
async fn second_forced_index_reconciles_zero() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_fixture(ws);
    let (data_dir, branch) = test_db_params(ws);

    code_graph::index_workspace(ws, &data_dir, &branch, &CodeGraphConfig::default(), false)
        .await
        .expect("initial index should succeed");
    // First forced index with a.rs excluded — evicts a.rs.
    code_graph::index_workspace(ws, &data_dir, &branch, &config_excluding_a(), true)
        .await
        .expect("first forced re-index should succeed");
    // Second forced index — a.rs already gone, nothing to reconcile.
    code_graph::index_workspace(ws, &data_dir, &branch, &config_excluding_a(), true)
        .await
        .expect("second forced re-index should succeed");

    let db = connect_db(&data_dir, &branch).await.expect("db connect");
    let q = CodeGraphQueries::new(db);
    let after = direct_edge_names(&q).await;
    assert!(
        !after.iter().any(|(f, t)| f == "a_caller" || t == "a_helper"),
        "a.rs stays evicted across repeated forced indexes; remaining: {after:?}"
    );
    assert!(
        after.contains(&("b_caller".to_owned(), "b_helper".to_owned())),
        "b.rs edge remains; got {after:?}"
    );
    assert_eq!(
        q.code_graph_extraction_generation()
            .expect("read generation marker"),
        Some("1".to_owned()),
        "the marker re-certifies the same value idempotently (A5/H4)"
    );
}
