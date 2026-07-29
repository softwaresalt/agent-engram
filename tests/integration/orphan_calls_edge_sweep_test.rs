//! U1 (103.001-T / 685FAA80): failing orphan `calls_edge` sweep harness (RED).
//!
//! Proves the forced-index / `--revalidate-code-graph` certify path leaves
//! orphaned `calls_edge` rows (endpoints with no live `function_meta`)
//! un-swept while it advances the durable `code_graph_extraction_generation`
//! marker. Such rows are non-traversable (queries join through
//! `function_meta`, so no wrong answer is returned) but they violate the H4
//! raw-row-hygiene invariant of the 101-F hardening and inflate `calls_edge`
//! cardinality. There is NO existing global orphan GC for `calls_edge`
//! (`rm_orphan_edges` is lineage-only; `count_dangling_calls_edges` only
//! counts).
//!
//! RED in U1: no sweep runs before the marker advances, so an injected orphan
//! survives a forced/revalidate run — `count_dangling_calls_edges() > 0` and a
//! dangling `(from, to)` pair remains. U1 GREEN wires
//! `retract_dangling_calls_edges` into BOTH certify blocks (before
//! `set_code_graph_extraction_generation`), retraction-only and keyed on
//! `function_meta` liveness so a live edge is never removed (A2 — no recall
//! loss).

#![allow(clippy::doc_markdown)]

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use sha2::{Digest, Sha256};
use tokio::test;

use engram::db::connect_db;
use engram::db::queries::CodeGraphQueries;
use engram::models::config::CodeGraphConfig;
use engram::services::code_graph;

// ── Corpus (reused from the 100-F / 101-F fixtures) ──────────────────────────

/// Rust: `plat` defined twice under mutually-exclusive `cfg` gates, called bare
/// by `describe` (a real same-file duplicate-name shape).
const RUST_DUP: &str = "\
#[cfg(unix)]
pub fn plat() -> u8 { 1 }

#[cfg(windows)]
pub fn plat() -> u8 { 2 }

pub fn describe() {
    let _ = plat();
}
";

/// Rust: unique-name same-file control — the `caller_unique -> helper` direct
/// edge is LIVE (both endpoints resolve) and must survive the sweep.
const RUST_UNIQUE: &str = "\
pub fn helper() -> u8 { 7 }

pub fn caller_unique() {
    let _ = helper();
}
";

const CARGO_TOML: &str = "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2024\"\n";

// Fully-synthetic endpoint ids that never correspond to an extracted function,
// so they remain dangling across any re-extraction and can only be removed by
// the orphan sweep.
const GHOST_CALLER: &str = "function:orphan_ghost_caller";
const GHOST_CALLEE: &str = "function:orphan_ghost_callee";
const GHOST_CALLER2: &str = "function:orphan_ghost_caller_2";

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
    write_one(ws, "src/dup.rs", RUST_DUP);
    write_one(ws, "src/unique.rs", RUST_UNIQUE);
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

/// The set of live function ids (every id with a `function_meta` row).
async fn live_ids(q: &CodeGraphQueries) -> HashSet<String> {
    q.all_functions()
        .await
        .expect("all_functions")
        .into_iter()
        .map(|f| f.id)
        .collect()
}

/// The first live function id carrying `name`.
async fn id_named(q: &CodeGraphQueries, name: &str) -> String {
    q.all_functions()
        .await
        .expect("all_functions")
        .into_iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| panic!("no live function named {name}"))
        .id
}

/// Count `direct` `calls_edge` rows whose `from` OR `to` has no live
/// `function_meta` row — the orphan population the sweep must reduce to zero.
async fn dangling_direct_pairs(q: &CodeGraphQueries) -> Vec<(String, String)> {
    let live = live_ids(q).await;
    q.list_calls_edges_by_resolution("direct")
        .await
        .expect("list direct edges")
        .into_iter()
        .filter(|(from, to)| !live.contains(from) || !live.contains(to))
        .collect()
}

/// The live `(caller_unique -> helper)` direct edge, by name, must be present.
async fn has_live_unique_edge(q: &CodeGraphQueries) -> bool {
    let caller = id_named(q, "caller_unique").await;
    let helper = id_named(q, "helper").await;
    q.list_calls_edges_by_resolution("direct")
        .await
        .expect("list direct edges")
        .contains(&(caller, helper))
}

// ── Scenario 1: `--revalidate-code-graph` sweeps orphans ─────────────────────

/// A gated revalidation run must sweep orphaned `calls_edge` rows BEFORE it
/// certifies the generation, so zero orphans remain and the live unique-name
/// edge is preserved. RED in U1 (no sweep) — the ghost orphans survive.
#[test]
async fn revalidation_sweeps_orphan_calls_edges() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_fixture(ws);
    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);

    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("index should succeed");
    let db = connect_db(&data_dir, &branch).await.expect("db connect");
    let q = CodeGraphQueries::new(db);

    // Inject orphan edges: one with a dangling callee, one with a dangling
    // caller (covers the `not has_def[from] OR not has_def[to]` predicate). The
    // live `caller_unique -> helper` edge already exists from indexing.
    let helper = id_named(&q, "helper").await;
    q.create_calls_edge(GHOST_CALLER, GHOST_CALLEE)
        .await
        .expect("inject fully-dangling orphan");
    q.create_calls_edge(GHOST_CALLER2, &helper)
        .await
        .expect("inject dangling-caller orphan");
    // Stale generation so the gated revalidation actually runs.
    q.set_code_graph_extraction_generation("0")
        .expect("stale generation marker");

    assert!(
        !dangling_direct_pairs(&q).await.is_empty(),
        "precondition: injected orphan edges are present"
    );

    let sync_res = code_graph::sync_workspace_with_progress(
        ws, &data_dir, &branch, &config, false, true, None,
    )
    .await
    .expect("gated revalidation sync should succeed");
    // A6 observability: the SyncResult reports the swept count (both injected
    // orphan rows), not just a silently-cleaned database.
    assert_eq!(
        sync_res.dangling_edges_swept, 2,
        "SyncResult.dangling_edges_swept must report the two swept orphan rows (A6)"
    );

    let db2 = connect_db(&data_dir, &branch).await.expect("db reconnect");
    let q2 = CodeGraphQueries::new(db2);

    // The sweep must have removed every orphan row (both endpoint-dangling
    // classes), so the raw-row-hygiene count is zero.
    assert_eq!(
        q2.count_dangling_calls_edges()
            .await
            .expect("count dangling"),
        0,
        "revalidation must sweep dangling-callee orphan rows (H4 raw-row hygiene)"
    );
    let remaining = dangling_direct_pairs(&q2).await;
    assert!(
        remaining.is_empty(),
        "revalidation must sweep ALL orphan direct edges (either endpoint dangling); remaining: {remaining:?}"
    );

    // Recall preserved: the live unique-name edge survives the sweep (A2).
    assert!(
        has_live_unique_edge(&q2).await,
        "orphan sweep must not remove the live caller_unique -> helper edge (no recall loss)"
    );

    // Marker advanced only after a clean sweep.
    assert_eq!(
        q2.code_graph_extraction_generation()
            .expect("read generation marker"),
        Some("1".to_owned()),
        "a successful sweep + revalidation advances the generation marker"
    );
}

// ── Scenario 2: forced index sweeps orphans ──────────────────────────────────

/// A forced full index (`force = true`) also sweeps orphaned rows before the
/// marker advances. RED in U1 — the ghost orphan survives a force re-index.
#[test]
async fn force_index_sweeps_orphan_calls_edges() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_fixture(ws);
    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);

    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("initial index should succeed");
    let db = connect_db(&data_dir, &branch).await.expect("db connect");
    let q = CodeGraphQueries::new(db);

    q.create_calls_edge(GHOST_CALLER, GHOST_CALLEE)
        .await
        .expect("inject fully-dangling orphan");

    assert!(
        q.count_dangling_calls_edges()
            .await
            .expect("count dangling")
            > 0,
        "precondition: injected orphan is present before the forced index"
    );

    let idx_res = code_graph::index_workspace(ws, &data_dir, &branch, &config, true)
        .await
        .expect("forced index should succeed");
    // A6 observability: the IndexResult reports the single swept orphan row.
    assert_eq!(
        idx_res.dangling_edges_swept, 1,
        "IndexResult.dangling_edges_swept must report the swept orphan row (A6)"
    );

    let db2 = connect_db(&data_dir, &branch).await.expect("db reconnect");
    let q2 = CodeGraphQueries::new(db2);

    assert_eq!(
        q2.count_dangling_calls_edges()
            .await
            .expect("count dangling"),
        0,
        "a forced index must sweep dangling orphan rows before certifying the generation"
    );
    assert!(
        has_live_unique_edge(&q2).await,
        "forced-index sweep must preserve the live caller_unique -> helper edge"
    );
}

// ── Scenario 3: primitive removes exactly the orphans, idempotently ──────────

/// The `retract_dangling_calls_edges` primitive removes exactly the orphan rows
/// (both endpoint-dangling classes), preserves the live edge (A2), and is
/// idempotent — a second immediate call sweeps zero (A5).
#[test]
async fn retract_dangling_primitive_exact_and_idempotent() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_fixture(ws);
    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);

    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("index should succeed");
    let db = connect_db(&data_dir, &branch).await.expect("db connect");
    let q = CodeGraphQueries::new(db);

    // Baseline: the fresh index leaves no orphans of either class.
    assert!(
        dangling_direct_pairs(&q).await.is_empty(),
        "a fresh index leaves no orphan calls_edge rows"
    );

    // Inject exactly two orphan rows: fully-dangling + dangling-caller.
    let helper = id_named(&q, "helper").await;
    q.create_calls_edge(GHOST_CALLER, GHOST_CALLEE)
        .await
        .expect("inject fully-dangling orphan");
    q.create_calls_edge(GHOST_CALLER2, &helper)
        .await
        .expect("inject dangling-caller orphan");

    let swept = q
        .retract_dangling_calls_edges()
        .await
        .expect("primitive sweep");
    assert_eq!(
        swept, 2,
        "the sweep must remove exactly the two injected orphan rows (A2 — nothing else)"
    );
    assert!(
        dangling_direct_pairs(&q).await.is_empty(),
        "no orphan (either endpoint dangling) remains after the sweep"
    );
    assert!(
        has_live_unique_edge(&q).await,
        "the live caller_unique -> helper edge is preserved (A2 — no recall loss)"
    );

    // A5 idempotence: a second immediate sweep removes nothing.
    let second = q
        .retract_dangling_calls_edges()
        .await
        .expect("second sweep");
    assert_eq!(
        second, 0,
        "a second immediate sweep is a strict no-op (A5 idempotence)"
    );
    assert!(
        has_live_unique_edge(&q).await,
        "the live edge survives the idempotent re-sweep"
    );
}
