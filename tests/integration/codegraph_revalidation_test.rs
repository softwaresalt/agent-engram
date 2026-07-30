//! U1 (101.001-T): failing generation-gated revalidation harness (RED).
//!
//! Proves the 100-F freshness gap: the fail-closed same-file guard only
//! withholds the ambiguous first-match edge for FRESHLY extracted files, so a
//! WRONG same-file direct edge persisted BEFORE the fix survives an
//! unchanged-bytes hash-skip on a routine sync. A durable
//! `code_graph_extraction_generation` marker plus an opt-in
//! `--revalidate-code-graph` gate must force re-extraction of the affected
//! files on a generation bump so the guard re-runs and drops the stale edge.
//!
//! Test shape mirrors the 096-F T7 `python_extraction_version` backfill suite
//! (`code_graph_test.rs`) and reuses the 100-F Rust `cfg`-gated duplicate-name
//! corpus (`same_file_shadowing_acceptance_test.rs`).
//!
//! RED in U1: the marker seam is threaded but inert, so (a) a fresh index does
//! not advance the generation marker and (b) the gated revalidation does not
//! force re-extraction — the injected wrong edge survives. U2 (101.002-T) wires
//! the gating + force re-extraction + marker advance to turn these GREEN.

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

// ── Corpus (reused from the 100-F acceptance fixture) ────────────────────────

/// Rust: `plat` defined twice under mutually-exclusive `cfg` gates (a real,
/// valid same-file duplicate-name shape), called bare by `describe`. The 100-F
/// guard fails closed: a fresh index mints NO `describe -> plat` edge.
const RUST_DUP: &str = "\
#[cfg(unix)]
pub fn plat() -> u8 { 1 }

#[cfg(windows)]
pub fn plat() -> u8 { 2 }

pub fn describe() {
    let _ = plat();
}
";

/// Rust: unique-name same-file control — recall must be preserved across a
/// revalidation (`caller_unique -> helper` stays resolved).
const RUST_UNIQUE: &str = "\
pub fn helper() -> u8 { 7 }

pub fn caller_unique() {
    let _ = helper();
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

fn sha256_hex(s: &str) -> String {
    format!("{:x}", Sha256::digest(s.as_bytes()))
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

/// All function ids carrying `name`, in extraction order.
async fn ids_named(q: &CodeGraphQueries, name: &str) -> Vec<String> {
    q.all_functions()
        .await
        .expect("all_functions")
        .into_iter()
        .filter(|f| f.name == name)
        .map(|f| f.id)
        .collect()
}

/// Sentinel prefix for a `calls_edge` endpoint whose function id has NO live
/// `function_meta` row — a DANGLING edge left by a re-extraction that re-minted
/// ids. `edge_name_pairs` surfaces these (instead of `unwrap_or_default()`'s
/// silent `""`) so a stale row is DETECTED rather than masked as `("","")`
/// (Copilot review, 101.002-T).
const DANGLING_PREFIX: &str = "<dangling:";

/// Map an endpoint id to its function name, or to a visible dangling sentinel
/// when the id has no live `function_meta` row.
fn resolve_endpoint(names: &HashMap<String, String>, id: &str) -> String {
    names
        .get(id)
        .cloned()
        .unwrap_or_else(|| format!("{DANGLING_PREFIX}{id}>"))
}

/// Assert no edge pair references a dangling (retired) function id — i.e. no
/// stale `calls_edge` row survived a revalidation re-extraction (plan H4: zero
/// stale wrong same-file edges remain).
fn assert_no_dangling_edges(pairs: &[(String, String)]) {
    let dangling: Vec<&(String, String)> = pairs
        .iter()
        .filter(|(from, to)| from.starts_with(DANGLING_PREFIX) || to.starts_with(DANGLING_PREFIX))
        .collect();
    assert!(
        dangling.is_empty(),
        "H4 violated: dangling calls_edge rows remain after revalidation (endpoints have no live function_meta): {dangling:?}"
    );
}

/// Every `calls` edge across all resolution classes, mapped to `(from, to)`
/// function-name pairs (ids are re-minted by a force re-extraction). A missing
/// endpoint id surfaces as a `<dangling:...>` sentinel, never a silent `""`.
async fn edge_name_pairs(q: &CodeGraphQueries) -> Vec<(String, String)> {
    let names = id_to_name(q).await;
    let mut pairs = Vec::new();
    for resolution in [
        "direct",
        "calls_resolved_singleton",
        "calls_resolved_canonical",
    ] {
        for (from, to) in q
            .list_calls_edges_by_resolution(resolution)
            .await
            .expect("list edges")
        {
            pairs.push((
                resolve_endpoint(&names, &from),
                resolve_endpoint(&names, &to),
            ));
        }
    }
    pairs
}

// ── U1 scenarios ─────────────────────────────────────────────────────────────

/// Scenario 1 (stale-generation correction) — a WRONG same-file direct edge
/// persisted under an OLD generation is corrected/dropped (target-identity) by a
/// generation-gated revalidation run, cross-file recall is preserved, and the
/// marker advances. RED in U1 (revalidation inert).
#[test]
async fn stale_generation_revalidation_drops_wrong_same_file_edge() {
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

    // Simulate a database written BEFORE the 100-F fix: a stale generation plus a
    // persisted WRONG same-file direct edge (`describe -> plat`, targeting the
    // shadowed first `cfg`-gated def) that a fresh extraction now withholds.
    let describe_id = ids_named(&q, "describe").await.remove(0);
    let plat_ids = ids_named(&q, "plat").await;
    assert_eq!(
        plat_ids.len(),
        2,
        "cfg-gated `plat` must be extracted twice to be ambiguous"
    );
    q.create_calls_edge(&describe_id, &plat_ids[0])
        .await
        .expect("inject wrong same-file edge");
    q.set_code_graph_extraction_generation("0")
        .await
        .expect("stale generation marker");

    // Precondition: the stale wrong edge is persisted as a direct edge.
    let before = q
        .list_calls_edges_by_resolution("direct")
        .await
        .expect("direct edges");
    assert!(
        before.contains(&(describe_id.clone(), plat_ids[0].clone())),
        "precondition: injected wrong same-file edge is persisted; got {before:?}"
    );

    // Gated revalidation sync over unchanged bytes: force re-extract, re-run the
    // 100-F guard, advance the generation marker.
    code_graph::sync_workspace_with_progress(ws, &data_dir, &branch, &config, false, true, None)
        .await
        .expect("gated revalidation sync should succeed");

    let db2 = connect_db(&data_dir, &branch).await.expect("db reconnect");
    let q2 = CodeGraphQueries::new(db2);

    // Target-identity: NO calls edge, in any resolution class, may target a
    // same-file duplicate-name def after revalidation.
    let pairs = edge_name_pairs(&q2).await;
    // H4 (raw-row hygiene): the revalidation must actually REMOVE the stale
    // direct edge, not leave it dangling under a retired function id. Without
    // this the `to == "plat"` filter below passes even when the raw stale row
    // survives (it maps to a `<dangling:...>` endpoint) — the masking Copilot
    // flagged (101.002-T).
    assert_no_dangling_edges(&pairs);
    let wrong: Vec<&(String, String)> = pairs.iter().filter(|(_, to)| to == "plat").collect();
    assert!(
        wrong.is_empty(),
        "generation-gated revalidation must drop the stale wrong same-file edge; offending: {wrong:?}"
    );

    // Recall preserved: the unique-name same-file control still resolves.
    assert!(
        pairs.contains(&("caller_unique".to_owned(), "helper".to_owned())),
        "revalidation must preserve unique-name same-file recall; pairs: {pairs:?}"
    );

    // Marker advanced to the current generation.
    assert_eq!(
        q2.code_graph_extraction_generation()
            .expect("read generation marker"),
        Some("1".to_owned()),
        "a successful revalidation advances the generation marker"
    );
}

/// Scenario 2 (no-op on match, A5/C12-5) — a run whose stored generation equals
/// the current one performs NO re-extraction and leaves edges untouched: the
/// injected edge (which a re-extraction would drop) survives, ids unchanged.
#[test]
async fn matching_generation_revalidation_is_noop() {
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

    let describe_id = ids_named(&q, "describe").await.remove(0);
    let plat_ids = ids_named(&q, "plat").await;
    q.create_calls_edge(&describe_id, &plat_ids[0])
        .await
        .expect("inject wrong same-file edge");
    // Matching (current) generation: revalidation must short-circuit.
    q.set_code_graph_extraction_generation("1")
        .await
        .expect("current generation marker");

    code_graph::sync_workspace_with_progress(ws, &data_dir, &branch, &config, false, true, None)
        .await
        .expect("matching-generation sync should succeed");

    let db2 = connect_db(&data_dir, &branch).await.expect("db reconnect");
    let q2 = CodeGraphQueries::new(db2);

    // Strict no-op: no re-extraction happened, so the injected edge survives with
    // its original ids (a re-extraction would have re-minted node ids and the
    // 100-F guard would have withheld the edge).
    let after = q2
        .list_calls_edges_by_resolution("direct")
        .await
        .expect("direct edges");
    assert!(
        after.contains(&(describe_id, plat_ids[0].clone())),
        "matching generation must be a strict no-op — injected edge must survive; got {after:?}"
    );
    assert_eq!(
        q2.code_graph_extraction_generation()
            .expect("read generation marker"),
        Some("1".to_owned()),
        "matching generation leaves the marker untouched"
    );
}

/// Scenario 3 (content_hash preserved, A4) — the generation marker is a separate
/// `schema_meta` record: setting/advancing it never mutates
/// `file_node.content_hash`, which stays the raw source SHA.
#[test]
async fn generation_marker_does_not_affect_content_hash() {
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

    let before = q
        .get_code_file_by_path("src/dup.rs")
        .await
        .expect("query code file")
        .expect("dup.rs indexed");
    assert_eq!(
        before.content_hash,
        sha256_hex(RUST_DUP),
        "content_hash is the raw source SHA"
    );

    // Round-trip the marker through the getter/setter.
    q.set_code_graph_extraction_generation("0")
        .await
        .expect("set stale generation");
    assert_eq!(
        q.code_graph_extraction_generation().expect("read marker"),
        Some("0".to_owned()),
    );
    q.set_code_graph_extraction_generation("1")
        .await
        .expect("advance generation");
    assert_eq!(
        q.code_graph_extraction_generation().expect("read marker"),
        Some("1".to_owned()),
    );

    // content_hash is unchanged by the marker writes.
    let after = q
        .get_code_file_by_path("src/dup.rs")
        .await
        .expect("query code file")
        .expect("dup.rs indexed");
    assert_eq!(
        after.content_hash, before.content_hash,
        "the generation marker must not perturb file_node.content_hash (A4)"
    );
}
