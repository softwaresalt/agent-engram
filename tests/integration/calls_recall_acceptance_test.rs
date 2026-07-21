//! Acceptance verification for rec1 cross-file calls resolution (082.004-T).
//!
//! Indexes a hand-authored multi-file fixture and gates the rec1 change against
//! the 081-F graph-resolution eval subsystem (081.005-T metric) plus a
//! ground-truth expected-edges manifest.
//!
//! The primary gate is target-correctness: every `calls_resolved_singleton`
//! edge produced for the fixture MUST match the hand-authored manifest of
//! ground-truth caller->callee pairs (correct callee id). The aggregate
//! `false_edge_rate` (081-F `count_dangling_calls_edges`) is a conservative
//! LOWER BOUND that detects dangling callees only — NOT mis-resolution to an
//! existing-but-wrong function — so it is a supporting signal, not the sole
//! gate (follow-up D07F0919).
//!
//! Scenarios (4):
//!   1. post-change resolution_recall > pre-change (cross-file post-pass lifts
//!      recall; "pre" = incremental sync which skips the post-pass, "post" =
//!      full index which runs it);
//!   2. aggregate false_edge_rate <= the operator threshold
//!      (RetrievalEvalConfig.thresholds, 081.001-T) — supporting signal only;
//!   3. every calls_resolved_singleton edge matches the expected-edges manifest
//!      AND the produced-edge count equals the manifest size;
//!   4. ambiguous names (defined more than once) contribute no edge — skipped,
//!      never mis-resolved.

#![allow(clippy::needless_raw_string_hashes)]
#![allow(clippy::doc_markdown)]

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use tokio::test;

use engram::db::connect_db;
use engram::db::queries::CodeGraphQueries;
use engram::models::RetrievalEvalConfig;
use engram::models::config::CodeGraphConfig;
use engram::services::code_graph;
use engram::services::parsing::Language;
use engram::services::retrieval_eval::{compute_graph_metrics, count_call_sites};

// ── Fixture ────────────────────────────────────────────────────────────────
//
// Six source files exercising the three resolution outcomes:
//   * in-file direct call:      alpha -> local_a          (same file, a.rs)
//   * unambiguous cross-file:   alpha -> beta (b.rs),
//                               beta  -> gamma (c.rs)      => two singletons
//   * ambiguous cross-file:     caller_dup -> dup         (dup in e.rs AND f.rs)
//                               => skipped, no edge
//
// Parser call-site inventory (the recall denominator) = 4:
//   a.rs: beta(), local_a();  b.rs: gamma();  d.rs: dup().

const FILE_A: &str =
    "pub fn alpha() {\n    beta();\n    local_a();\n}\n\npub fn local_a() {\n    let _ = 1;\n}\n";
const FILE_B: &str = "pub fn beta() {\n    gamma();\n}\n";
const FILE_C: &str = "pub fn gamma() {\n    let _ = 2;\n}\n";
const FILE_D: &str = "pub fn caller_dup() {\n    dup();\n}\n";
const FILE_E: &str = "pub fn dup() {\n    let _ = 3;\n}\n";
const FILE_F: &str = "pub fn dup() {\n    let _ = 4;\n}\n";

/// The fixture's source files, keyed by workspace-relative path.
const FIXTURE: &[(&str, &str)] = &[
    ("src/a.rs", FILE_A),
    ("src/b.rs", FILE_B),
    ("src/c.rs", FILE_C),
    ("src/d.rs", FILE_D),
    ("src/e.rs", FILE_E),
    ("src/f.rs", FILE_F),
];

/// Ground-truth manifest of expected `calls_resolved_singleton` edges, as
/// (caller name, callee name) pairs. This is the primary correctness gate.
const EXPECTED_SINGLETONS: &[(&str, &str)] = &[("alpha", "beta"), ("beta", "gamma")];

fn write_fixture(ws: &Path) {
    for (rel, content) in FIXTURE {
        let full = ws.join(rel);
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent).expect("create dirs");
        }
        fs::write(full, content).expect("write file");
    }
}

/// Total parser call-site inventory over the fixture (the recall denominator).
fn fixture_call_sites() -> usize {
    FIXTURE
        .iter()
        .map(|(_, source)| count_call_sites(source, Language::Rust))
        .sum()
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

/// Index the fixture with the cross-file post-pass (full-index path) and return
/// a queries handle over the resulting graph.
async fn index_fixture() -> (tempfile::TempDir, CodeGraphQueries) {
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
async fn name_to_ids(q: &CodeGraphQueries) -> HashMap<String, Vec<String>> {
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for f in q.all_functions().await.expect("all_functions") {
        map.entry(f.name).or_default().push(f.id);
    }
    map
}

// Scenario 1: the cross-file post-pass lifts resolution_recall above the
// pre-change (sync-only) baseline.
#[test]
async fn post_change_recall_exceeds_pre_change() {
    let call_sites = fixture_call_sites();
    assert_eq!(call_sites, 4, "fixture must expose four visible call sites");

    // Pre-change: first-time sync exercises the incremental path, which does NOT
    // run the cross-file post-pass — only in-file direct edges are resolved.
    let pre_tmp = tempfile::tempdir().expect("tempdir");
    let pre_ws = pre_tmp.path();
    write_fixture(pre_ws);
    let config = CodeGraphConfig::default();
    let (pre_dir, pre_branch) = test_db_params(pre_ws);
    code_graph::sync_workspace(pre_ws, &pre_dir, &pre_branch, &config)
        .await
        .expect("sync");
    let pre_q = queries_for(&pre_dir, &pre_branch).await;
    let pre_resolved = pre_q.count_calls_edges().await.expect("pre count");
    let pre = compute_graph_metrics(call_sites, pre_resolved, 0);

    // Post-change: full index runs the post-pass, adding the cross-file
    // singleton edges on top of the in-file direct edges.
    let (_post_tmp, post_q) = index_fixture().await;
    let post_resolved = post_q.count_calls_edges().await.expect("post count");
    let post_false = post_q
        .count_dangling_calls_edges()
        .await
        .expect("post dangling");
    let post = compute_graph_metrics(call_sites, post_resolved, post_false);

    assert!(
        post.resolution_recall > pre.resolution_recall,
        "post-change recall ({}) must exceed pre-change recall ({})",
        post.resolution_recall,
        pre.resolution_recall
    );
}

// Scenario 2: the aggregate false_edge_rate stays within the operator threshold.
// This is a conservative lower-bound signal (dangling callees only), not the
// sole gate — target-correctness (scenario 3) is the primary gate.
#[test]
async fn false_edge_rate_within_threshold() {
    let (_tmp, q) = index_fixture().await;
    let call_sites = fixture_call_sites();
    let resolved = q.count_calls_edges().await.expect("count");
    let false_edges = q.count_dangling_calls_edges().await.expect("dangling");
    let metrics = compute_graph_metrics(call_sites, resolved, false_edges);

    let threshold = RetrievalEvalConfig::default()
        .thresholds
        .max_false_edge_rate;
    assert!(
        metrics.false_edge_rate <= threshold,
        "false_edge_rate ({}) must be within the operator threshold ({threshold})",
        metrics.false_edge_rate
    );
}

// Scenario 3 (PRIMARY GATE): every calls_resolved_singleton edge matches the
// ground-truth manifest, and the produced-edge count equals the manifest size.
#[test]
async fn singleton_edges_match_expected_manifest() {
    let (_tmp, q) = index_fixture().await;
    let names = name_to_ids(&q).await;

    // Resolve the hand-authored (caller, callee) manifest to concrete id pairs.
    let expected: HashSet<(String, String)> = EXPECTED_SINGLETONS
        .iter()
        .map(|(caller, callee)| {
            let from_ids = names
                .get(*caller)
                .unwrap_or_else(|| panic!("caller {caller} must be indexed"));
            let to_ids = names
                .get(*callee)
                .unwrap_or_else(|| panic!("callee {callee} must be indexed"));
            assert_eq!(from_ids.len(), 1, "caller {caller} must be unambiguous");
            assert_eq!(to_ids.len(), 1, "callee {callee} must be unambiguous");
            (from_ids[0].clone(), to_ids[0].clone())
        })
        .collect();

    let actual: HashSet<(String, String)> = q
        .list_calls_edges_by_resolution("calls_resolved_singleton")
        .await
        .expect("list singletons")
        .into_iter()
        .collect();

    // Count folded into the manifest-match gate (per Copilot #239): the produced
    // edge count must equal the manifest size.
    assert_eq!(
        actual.len(),
        EXPECTED_SINGLETONS.len(),
        "produced singleton count must equal the manifest size; actual: {actual:?}"
    );
    assert_eq!(
        actual, expected,
        "every singleton edge must match the ground-truth manifest (correct callee id)"
    );
}

// Scenario 4: an ambiguous callee name (`dup`, defined in two files) contributes
// no edge — it is skipped, never mis-resolved to either definition.
#[test]
async fn ambiguous_name_contributes_no_edge() {
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

// 094.003-T (U3 Rust-path regression): language-scoping the cross-file singleton
// resolver is a NO-OP for the current Rust-only staged population. Re-running the
// pure-Rust fixture must still produce EXACTLY the ground-truth singleton manifest
// — same-language == Rust, so no candidate is filtered out and none is newly
// admitted. Guards specifically against the language-scope change (join to
// file_node.language) regressing existing Rust singleton resolution.
#[test]
async fn rust_singleton_resolution_unchanged_under_language_scope() {
    let (_tmp, q) = index_fixture().await;
    let names = name_to_ids(&q).await;

    let expected: HashSet<(String, String)> = EXPECTED_SINGLETONS
        .iter()
        .map(|(caller, callee)| {
            let from = names.get(*caller).expect("caller must be indexed")[0].clone();
            let to = names.get(*callee).expect("callee must be indexed")[0].clone();
            (from, to)
        })
        .collect();

    let actual: HashSet<(String, String)> = q
        .list_calls_edges_by_resolution("calls_resolved_singleton")
        .await
        .expect("list singletons")
        .into_iter()
        .collect();

    assert_eq!(
        actual, expected,
        "Rust singleton resolution must be unchanged under same-language scoping"
    );
}
