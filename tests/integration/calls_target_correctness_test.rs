//! Target-correctness verification for the graph resolution eval (084.004-T).
//!
//! `false_edge_rate` (081-F `count_dangling_calls_edges`) is a conservative
//! DANGLING-ONLY lower bound: it detects edges whose callee matches no known
//! definition, but is blind to mis-resolution to an existing-but-WRONG function
//! (2026-07-08-callgraph-cross-file-resolution-deliberation.md:25-33). True
//! target-correctness requires asserting each produced
//! `calls_resolved_singleton` edge against a hand-authored expected-target
//! MANIFEST by EXACT identity.
//!
//! This file (084.004.001-ST) builds that fixture workspace and the ground-truth
//! manifest, and characterizes that the real indexer produces exactly the
//! manifest's singleton edges — so 084.004.002-ST can assert target-correctness
//! (`target_correct` / `target_mismatch`) against a trustworthy ground truth.
//!
//! Fixture (mirrors `calls_recall_acceptance_test`'s proven shape):
//!   * in-file direct call:      alpha -> local_a          (same file, a.rs)
//!   * unambiguous cross-file:   alpha -> beta (b.rs),
//!                               beta  -> gamma (c.rs)      => two singletons
//!   * ambiguous cross-file:     caller_dup -> dup         (dup in e.rs AND f.rs)
//!                               => skipped, never mis-resolved (no edge)

#![allow(clippy::needless_raw_string_hashes)]
#![allow(clippy::doc_markdown)]

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use tokio::test;

use engram::db::connect_db;
use engram::db::queries::CodeGraphQueries;
use engram::models::config::CodeGraphConfig;
use engram::services::code_graph;

// ── Fixture source files ─────────────────────────────────────────────────────

const FILE_A: &str =
    "pub fn alpha() {\n    beta();\n    local_a();\n}\n\npub fn local_a() {\n    let _ = 1;\n}\n";
const FILE_B: &str = "pub fn beta() {\n    gamma();\n}\n";
const FILE_C: &str = "pub fn gamma() {\n    let _ = 2;\n}\n";
const FILE_D: &str = "pub fn caller_dup() {\n    dup();\n}\n";
const FILE_E: &str = "pub fn dup() {\n    let _ = 3;\n}\n";
const FILE_F: &str = "pub fn dup() {\n    let _ = 4;\n}\n";

/// The fixture's source files, keyed by workspace-relative path.
pub const FIXTURE: &[(&str, &str)] = &[
    ("src/a.rs", FILE_A),
    ("src/b.rs", FILE_B),
    ("src/c.rs", FILE_C),
    ("src/d.rs", FILE_D),
    ("src/e.rs", FILE_E),
    ("src/f.rs", FILE_F),
];

/// Ground-truth manifest of expected `calls_resolved_singleton` edges, as
/// (caller name, callee name) pairs. This is the correctness oracle: every
/// produced singleton edge MUST match one of these by exact identity, and no
/// other singleton may be produced.
pub const EXPECTED_SINGLETONS: &[(&str, &str)] = &[("alpha", "beta"), ("beta", "gamma")];

// ── Fixture harness ──────────────────────────────────────────────────────────

fn write_fixture(ws: &Path) {
    for (rel, content) in FIXTURE {
        let full = ws.join(rel);
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent).expect("create dirs");
        }
        fs::write(full, content).expect("write file");
    }
}

/// Explicit data-dir + per-workspace branch, mirroring
/// `calls_recall_acceptance_test`. The data-dir is passed to `index_workspace`
/// and `connect_db` DIRECTLY, so this path never consults `ENGRAM_DATA_DIR` and
/// stays isolated from the developer's real repo DB.
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

/// Index the fixture with the cross-file post-pass (full-index path) and return
/// a queries handle over the resulting graph.
pub async fn index_fixture() -> (tempfile::TempDir, CodeGraphQueries) {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_fixture(ws);
    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);
    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("index");
    let q = queries_for(&data_dir, &branch).await;
    (tmp, q)
}

/// Build a `name -> [function_id]` map from the indexed function corpus.
pub async fn name_to_ids(q: &CodeGraphQueries) -> HashMap<String, Vec<String>> {
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for f in q.all_functions().await.expect("all_functions") {
        map.entry(f.name).or_default().push(f.id);
    }
    map
}

/// Resolve the hand-authored (caller-name, callee-name) manifest to the set of
/// concrete `(from_id, to_id)` edges the graph is expected to contain. Panics if
/// any manifest endpoint is missing or ambiguous — the fixture guarantees each
/// manifest endpoint is a single, unambiguous definition.
pub async fn expected_edge_ids(q: &CodeGraphQueries) -> HashSet<(String, String)> {
    let names = name_to_ids(q).await;
    EXPECTED_SINGLETONS
        .iter()
        .map(|(caller, callee)| {
            let from = names
                .get(*caller)
                .unwrap_or_else(|| panic!("caller {caller} must be indexed"));
            let to = names
                .get(*callee)
                .unwrap_or_else(|| panic!("callee {callee} must be indexed"));
            assert_eq!(from.len(), 1, "caller {caller} must be unambiguous");
            assert_eq!(to.len(), 1, "callee {callee} must be unambiguous");
            (from[0].clone(), to[0].clone())
        })
        .collect()
}

// ── 084.004.001-ST: fixture / manifest ground-truth characterization ─────────

/// The real indexer produces EXACTLY the manifest's singleton edges over the
/// fixture — no more, no fewer — establishing the manifest as trustworthy ground
/// truth for the 084.004.002-ST target-correctness assertions.
#[test]
async fn fixture_produces_exactly_the_ground_truth_singletons() {
    let (_tmp, q) = index_fixture().await;

    let produced: HashSet<(String, String)> = q
        .list_calls_edges_by_resolution("calls_resolved_singleton")
        .await
        .expect("list singletons")
        .into_iter()
        .collect();

    let expected = expected_edge_ids(&q).await;

    assert_eq!(
        produced.len(),
        EXPECTED_SINGLETONS.len(),
        "produced singleton count must equal the manifest size; produced: {produced:?}"
    );
    assert_eq!(
        produced, expected,
        "every produced singleton must match the ground-truth manifest by exact identity"
    );
}

/// The ambiguous name `dup` (defined twice) contributes no singleton edge — it
/// is skipped, never mis-resolved to either definition, so it can never be a
/// false ground-truth positive.
#[test]
async fn ambiguous_name_contributes_no_singleton() {
    let (_tmp, q) = index_fixture().await;
    let names = name_to_ids(&q).await;

    let dup_ids: HashSet<&String> = names
        .get("dup")
        .expect("dup must be indexed")
        .iter()
        .collect();
    assert_eq!(dup_ids.len(), 2, "dup must have two ambiguous definitions");

    let singletons = q
        .list_calls_edges_by_resolution("calls_resolved_singleton")
        .await
        .expect("list singletons");

    assert!(
        singletons.iter().all(|(_, to)| !dup_ids.contains(to)),
        "no singleton edge may resolve to an ambiguous `dup` definition; got {singletons:?}"
    );
}
