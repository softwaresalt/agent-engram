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

/// Index an arbitrary Python fixture through the full-index call post-pass.
async fn index_python_fixture(files: &[(&str, &str)]) -> (tempfile::TempDir, CodeGraphQueries) {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    for (rel, source) in files {
        let full = ws.join(rel);
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent).expect("create fixture directory");
        }
        fs::write(full, source).expect("write Python fixture");
    }
    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);
    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("index Python fixture");
    let q = queries_for(&data_dir, &branch).await;
    (tmp, q)
}

/// Resolve one indexed function by its simple name and workspace-relative file.
async fn function_id_in(q: &CodeGraphQueries, name: &str, file_path: &str) -> String {
    let matches: Vec<_> = q
        .all_functions()
        .await
        .expect("all_functions")
        .into_iter()
        .filter(|function| function.name == name && function.file_path == file_path)
        .collect();
    assert_eq!(
        matches.len(),
        1,
        "{name} in {file_path} must identify exactly one function; got {matches:?}"
    );
    matches[0].id.clone()
}

/// Return all indexed function IDs carrying `name`.
async fn function_ids_named(q: &CodeGraphQueries, name: &str) -> Vec<String> {
    q.all_functions()
        .await
        .expect("all_functions")
        .into_iter()
        .filter(|function| function.name == name)
        .map(|function| function.id)
        .collect()
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

/// T5b.1 — module-qualified and from-import calls bind to the exact canonical
/// target even when another module defines the same simple function name.
#[test]
async fn python_module_and_from_import_resolve_to_exact_target() {
    let (_tmp, q) = index_python_fixture(&[
        ("bar.py", "def parse():\n    return 1\n"),
        ("baz.py", "def parse():\n    return 2\n"),
        ("caller.py", "import bar\n\ndef run():\n    bar.parse()\n"),
        (
            "user.py",
            "from bar import parse\n\ndef caller():\n    parse()\n",
        ),
    ])
    .await;
    let run_id = function_id_in(&q, "run", "caller.py").await;
    let caller_id = function_id_in(&q, "caller", "user.py").await;
    let bar_parse_id = function_id_in(&q, "parse", "bar.py").await;
    let decoy_parse_id = function_id_in(&q, "parse", "baz.py").await;
    let canonical = q
        .list_calls_edges_by_resolution("calls_resolved_canonical")
        .await
        .expect("list canonical edges");

    assert!(
        canonical.contains(&(run_id.clone(), bar_parse_id.clone())),
        "module-qualified call must resolve to bar.parse; got {canonical:?}"
    );
    assert!(
        canonical.contains(&(caller_id.clone(), bar_parse_id)),
        "from-import call must resolve to bar.parse; got {canonical:?}"
    );
    assert!(
        canonical.iter().all(|(_, to)| to != &decoy_parse_id),
        "no canonical edge may resolve to decoy baz.parse; got {canonical:?}"
    );
}

/// T5b.2 — the latest resolvable module binding wins unless a later opaque
/// rebind poisons it, in which case resolution emits no edge.
#[test]
async fn python_bare_call_site_ordering_resolves_or_fails_closed() {
    let (_tmp, q) = index_python_fixture(&[
        (
            "a.py",
            "def parse():\n    return 0\n\nfrom bar import parse\n\ndef caller():\n    parse()\n",
        ),
        ("bar.py", "def parse():\n    return 9\n"),
    ])
    .await;
    let caller_id = function_id_in(&q, "caller", "a.py").await;
    let bar_parse_id = function_id_in(&q, "parse", "bar.py").await;
    let canonical = q
        .list_calls_edges_by_resolution("calls_resolved_canonical")
        .await
        .expect("list canonical edges");
    assert!(
        canonical.contains(&(caller_id, bar_parse_id)),
        "later from-import must win over the earlier module definition; got {canonical:?}"
    );

    let (_tmp, q) = index_python_fixture(&[(
        "m.py",
        "parse = None\n\ndef parse():\n    return 3\n\ndef caller():\n    parse()\n",
    )])
    .await;
    let caller_id = function_id_in(&q, "caller", "m.py").await;
    let parse_id = function_id_in(&q, "parse", "m.py").await;
    let canonical = q
        .list_calls_edges_by_resolution("calls_resolved_canonical")
        .await
        .expect("list canonical edges");
    assert!(
        canonical.contains(&(caller_id, parse_id)),
        "later definition must overwrite earlier opaque assignment; got {canonical:?}"
    );

    let (_tmp, q) = index_python_fixture(&[
        (
            "n.py",
            "from bar import parse\n\ndef caller():\n    parse()\n\nparse = None\n",
        ),
        ("bar.py", "def parse():\n    return 1\n"),
    ])
    .await;
    let caller_id = function_id_in(&q, "caller", "n.py").await;
    let bar_parse_id = function_id_in(&q, "parse", "bar.py").await;
    let canonical = q
        .list_calls_edges_by_resolution("calls_resolved_canonical")
        .await
        .expect("list canonical edges");
    let singleton = q
        .list_calls_edges_by_resolution("calls_resolved_singleton")
        .await
        .expect("list singleton edges");
    assert!(
        !canonical.contains(&(caller_id.clone(), bar_parse_id.clone()))
            && !singleton.contains(&(caller_id, bar_parse_id)),
        "later assignment must fail closed without canonical or fallback edge"
    );
}

/// T5b.3 — receiver ambiguity, star imports, and duplicate same-name imports
/// never mint a candidate edge.
#[test]
async fn python_fail_closed_vectors_emit_no_edge() {
    let (_tmp, q) = index_python_fixture(&[
        (
            "p.py",
            "from pkg import parse\n\ndef caller():\n    parse.tokenize()\n",
        ),
        ("pkg.py", "def tokenize():\n    return 1\n"),
    ])
    .await;
    let caller_id = function_id_in(&q, "caller", "p.py").await;
    let tokenize_id = function_id_in(&q, "tokenize", "pkg.py").await;
    let canonical = q
        .list_calls_edges_by_resolution("calls_resolved_canonical")
        .await
        .expect("list canonical edges");
    let singleton = q
        .list_calls_edges_by_resolution("calls_resolved_singleton")
        .await
        .expect("list singleton edges");
    assert!(
        !canonical.contains(&(caller_id.clone(), tokenize_id.clone()))
            && !singleton.contains(&(caller_id, tokenize_id)),
        "attribute call on a from-imported symbol must fail closed"
    );

    let (_tmp, q) = index_python_fixture(&[
        ("s.py", "from n import *\n\ndef caller():\n    parse()\n"),
        ("b.py", "def parse():\n    return 1\n"),
        ("c.py", "def parse():\n    return 2\n"),
    ])
    .await;
    let caller_id = function_id_in(&q, "caller", "s.py").await;
    let parse_ids = function_ids_named(&q, "parse").await;
    let canonical = q
        .list_calls_edges_by_resolution("calls_resolved_canonical")
        .await
        .expect("list canonical edges");
    let singleton = q
        .list_calls_edges_by_resolution("calls_resolved_singleton")
        .await
        .expect("list singleton edges");
    assert!(
        parse_ids.iter().all(|parse_id| {
            !canonical.contains(&(caller_id.clone(), parse_id.clone()))
                && !singleton.contains(&(caller_id.clone(), parse_id.clone()))
        }),
        "star-import ambiguity must not resolve to either parse candidate"
    );

    let (_tmp, q) = index_python_fixture(&[
        (
            "d.py",
            "from b import parse\nfrom c import parse\n\ndef caller():\n    parse()\n",
        ),
        ("b.py", "def parse():\n    return 1\n"),
        ("c.py", "def parse():\n    return 2\n"),
    ])
    .await;
    let caller_id = function_id_in(&q, "caller", "d.py").await;
    let parse_ids = function_ids_named(&q, "parse").await;
    let canonical = q
        .list_calls_edges_by_resolution("calls_resolved_canonical")
        .await
        .expect("list canonical edges");
    let singleton = q
        .list_calls_edges_by_resolution("calls_resolved_singleton")
        .await
        .expect("list singleton edges");
    assert!(
        parse_ids.iter().all(|parse_id| {
            !canonical.contains(&(caller_id.clone(), parse_id.clone()))
                && !singleton.contains(&(caller_id.clone(), parse_id.clone()))
        }),
        "duplicate same-name imports must not resolve to either parse candidate"
    );
}

/// T5b.4 — recall-safe no-target reasons retain the legacy unique-name fallback,
/// while a non-unique candidate set still fails closed.
#[test]
async fn python_no_target_falls_back_to_unique_name_only() {
    let (_tmp, q) = index_python_fixture(&[
        ("src/app.py", "def caller():\n    helper()\n"),
        ("src/lib.py", "def helper():\n    return 1\n"),
    ])
    .await;
    let caller_id = function_id_in(&q, "caller", "src/app.py").await;
    let helper_id = function_id_in(&q, "helper", "src/lib.py").await;
    let singleton = q
        .list_calls_edges_by_resolution("calls_resolved_singleton")
        .await
        .expect("list singleton edges");
    assert!(
        singleton.contains(&(caller_id, helper_id)),
        "missing module context must fall back to a unique Python name"
    );

    let (_tmp, q) = index_python_fixture(&[
        (
            "r.py",
            "from .other import parse\n\ndef caller():\n    parse()\n",
        ),
        ("other.py", "def parse():\n    return 1\n"),
    ])
    .await;
    let caller_id = function_id_in(&q, "caller", "r.py").await;
    let parse_id = function_id_in(&q, "parse", "other.py").await;
    let singleton = q
        .list_calls_edges_by_resolution("calls_resolved_singleton")
        .await
        .expect("list singleton edges");
    assert!(
        singleton.contains(&(caller_id, parse_id)),
        "relative import must fall back to a unique Python name"
    );

    let (_tmp, q) = index_python_fixture(&[
        ("src/app2.py", "def caller():\n    helper()\n"),
        ("src/lib1.py", "def helper():\n    return 1\n"),
        ("src/lib2.py", "def helper():\n    return 2\n"),
    ])
    .await;
    let caller_id = function_id_in(&q, "caller", "src/app2.py").await;
    let helper_ids = function_ids_named(&q, "helper").await;
    let canonical = q
        .list_calls_edges_by_resolution("calls_resolved_canonical")
        .await
        .expect("list canonical edges");
    let singleton = q
        .list_calls_edges_by_resolution("calls_resolved_singleton")
        .await
        .expect("list singleton edges");
    assert!(
        helper_ids.iter().all(|helper_id| {
            !canonical.contains(&(caller_id.clone(), helper_id.clone()))
                && !singleton.contains(&(caller_id.clone(), helper_id.clone()))
        }),
        "non-unique fallback candidates must emit no edge"
    );
}

/// T5c.1 — bare-callee winner selection is anchored on the caller's definition
/// position: bindings after the caller cannot be proven effective, and a
/// binding after the winning binding poisons the call (time-axis, C7-1).
#[test]
async fn python_bare_winner_is_anchored_on_caller_position() {
    // Import after an earlier def, caller after both: the later import wins.
    let (_tmp, q) = index_python_fixture(&[
        (
            "w1.py",
            "def parse():\n    return 0\n\nfrom bar import parse\n\ndef caller():\n    parse()\n",
        ),
        ("bar.py", "def parse():\n    return 9\n"),
    ])
    .await;
    let caller_id = function_id_in(&q, "caller", "w1.py").await;
    let bar_parse_id = function_id_in(&q, "parse", "bar.py").await;
    let local_parse_id = function_id_in(&q, "parse", "w1.py").await;
    let canonical = q
        .list_calls_edges_by_resolution("calls_resolved_canonical")
        .await
        .expect("list canonical edges");
    assert!(
        canonical.contains(&(caller_id.clone(), bar_parse_id)),
        "import after the earlier def must win for a later caller; got {canonical:?}"
    );
    assert!(
        canonical.iter().all(|(_, to)| to != &local_parse_id),
        "the shadowed earlier def must not receive an edge; got {canonical:?}"
    );

    // Def after import, caller after both: the later local def wins.
    let (_tmp, q) = index_python_fixture(&[
        (
            "w2.py",
            "from bar import parse\n\ndef parse():\n    return 0\n\ndef caller():\n    parse()\n",
        ),
        ("bar.py", "def parse():\n    return 9\n"),
    ])
    .await;
    let caller_id = function_id_in(&q, "caller", "w2.py").await;
    let local_parse_id = function_id_in(&q, "parse", "w2.py").await;
    let bar_parse_id = function_id_in(&q, "parse", "bar.py").await;
    let canonical = q
        .list_calls_edges_by_resolution("calls_resolved_canonical")
        .await
        .expect("list canonical edges");
    assert!(
        canonical.contains(&(caller_id.clone(), local_parse_id)),
        "def after the import must win for a later caller; got {canonical:?}"
    );
    assert!(
        canonical.iter().all(|(_, to)| to != &bar_parse_id),
        "the shadowed earlier import must not receive an edge; got {canonical:?}"
    );

    // C7-1: an import after the caller poisons the pre-caller def — fail closed.
    let (_tmp, q) = index_python_fixture(&[
        (
            "w3.py",
            "def parse():\n    return 0\n\ndef g():\n    parse()\n\nfrom bar import parse\n",
        ),
        ("bar.py", "def parse():\n    return 9\n"),
    ])
    .await;
    let g_id = function_id_in(&q, "g", "w3.py").await;
    let canonical = q
        .list_calls_edges_by_resolution("calls_resolved_canonical")
        .await
        .expect("list canonical edges");
    let singleton = q
        .list_calls_edges_by_resolution("calls_resolved_singleton")
        .await
        .expect("list singleton edges");
    assert!(
        canonical
            .iter()
            .chain(singleton.iter())
            .all(|(from, _)| from != &g_id),
        "a rebind after the caller must fail closed with no edge; \
         canonical={canonical:?} singleton={singleton:?}"
    );
}

/// T5c.2 — a module receiver reassigned anywhere after its import is no longer
/// provably that module: `receiver.callee()` fails closed for every rebind form.
#[test]
async fn python_module_receiver_rebind_after_import_fails_closed() {
    let rebinds = [
        "bar = factory()",
        "bar += other()",
        "for bar in items():\n    pass",
        "with ctx() as bar:\n    pass",
        "try:\n    pass\nexcept Err() as bar:\n    pass",
        "(bar := factory())",
        "def bar():\n    return 0",
        "class bar:\n    pass",
        "del bar",
        "match value():\n    case bar:\n        pass",
        "from other import *",
    ];
    for rebind in rebinds {
        let source = format!("import bar\n\n{rebind}\n\ndef g():\n    bar.parse()\n");
        let (_tmp, q) = index_python_fixture(&[
            ("m.py", source.as_str()),
            ("bar.py", "def parse():\n    return 1\n"),
        ])
        .await;
        let g_id = function_id_in(&q, "g", "m.py").await;
        let canonical = q
            .list_calls_edges_by_resolution("calls_resolved_canonical")
            .await
            .expect("list canonical edges");
        let singleton = q
            .list_calls_edges_by_resolution("calls_resolved_singleton")
            .await
            .expect("list singleton edges");
        assert!(
            canonical
                .iter()
                .chain(singleton.iter())
                .all(|(from, _)| from != &g_id),
            "receiver rebind `{rebind}` after import must fail closed; \
             canonical={canonical:?} singleton={singleton:?}"
        );
    }

    // A parameter that shadows the receiver name is function-local — fail closed.
    let (_tmp, q) = index_python_fixture(&[
        ("p.py", "import bar\n\ndef g(bar):\n    bar.parse()\n"),
        ("bar.py", "def parse():\n    return 1\n"),
    ])
    .await;
    let g_id = function_id_in(&q, "g", "p.py").await;
    let canonical = q
        .list_calls_edges_by_resolution("calls_resolved_canonical")
        .await
        .expect("list canonical edges");
    let singleton = q
        .list_calls_edges_by_resolution("calls_resolved_singleton")
        .await
        .expect("list singleton edges");
    assert!(
        canonical
            .iter()
            .chain(singleton.iter())
            .all(|(from, _)| from != &g_id),
        "a parameter shadowing the receiver must fail closed; \
         canonical={canonical:?} singleton={singleton:?}"
    );
}

/// T5c.2b (096-F, P0-860) — a module receiver dynamically rebound through a
/// `global` write in a *sibling* function is not provably that module at call
/// time, so `receiver.callee()` must fail closed. The module-scope shadow scan
/// stops at function bodies, so the positioned receiver-rebind check never sees
/// this form; only the dynamic-rebind signal catches it. The module-qualifier
/// guard must consult that signal just as the bare-call path already does.
#[test]
async fn python_module_receiver_dynamic_global_rebind_fails_closed() {
    let (_tmp, q) = index_python_fixture(&[
        (
            "m.py",
            "import bar\n\ndef mutate():\n    global bar\n    bar = factory()\n\ndef g():\n    bar.parse()\n",
        ),
        ("bar.py", "def parse():\n    return 1\n"),
    ])
    .await;
    let g_id = function_id_in(&q, "g", "m.py").await;
    let bar_parse_id = function_id_in(&q, "parse", "bar.py").await;
    let canonical = q
        .list_calls_edges_by_resolution("calls_resolved_canonical")
        .await
        .expect("list canonical edges");
    let singleton = q
        .list_calls_edges_by_resolution("calls_resolved_singleton")
        .await
        .expect("list singleton edges");
    assert!(
        !canonical.contains(&(g_id.clone(), bar_parse_id.clone()))
            && !singleton.contains(&(g_id, bar_parse_id)),
        "a module receiver dynamically rebound via `global` in a sibling must fail closed; \
         canonical={canonical:?} singleton={singleton:?}"
    );
}

/// T5c.3 — a from-imported symbol reassigned at module scope after the import is
/// undecidable at the caller: the bare call fails closed.
#[test]
async fn python_bare_import_assignment_shadow_fails_closed() {
    let (_tmp, q) = index_python_fixture(&[
        (
            "c.py",
            "from bar import parse\n\nparse = factory()\n\ndef caller():\n    parse()\n",
        ),
        ("bar.py", "def parse():\n    return 1\n"),
    ])
    .await;
    let caller_id = function_id_in(&q, "caller", "c.py").await;
    let bar_parse_id = function_id_in(&q, "parse", "bar.py").await;
    let canonical = q
        .list_calls_edges_by_resolution("calls_resolved_canonical")
        .await
        .expect("list canonical edges");
    let singleton = q
        .list_calls_edges_by_resolution("calls_resolved_singleton")
        .await
        .expect("list singleton edges");
    assert!(
        !canonical.contains(&(caller_id.clone(), bar_parse_id.clone()))
            && !singleton.contains(&(caller_id, bar_parse_id)),
        "a module-scope reassignment after the import must fail closed; \
         canonical={canonical:?} singleton={singleton:?}"
    );
}

/// T5c.4 — a bare callee that is bound anywhere in the caller's own body
/// (parameter, in-body assignment, or dynamic global rebind) is function-local
/// and can never resolve to a module-scope import — fail closed.
#[test]
async fn python_bare_callee_function_local_poison_fails_closed() {
    // Parameter shadow.
    let (_tmp, q) = index_python_fixture(&[
        (
            "a.py",
            "from bar import parse\n\ndef g(parse):\n    parse()\n",
        ),
        ("bar.py", "def parse():\n    return 1\n"),
    ])
    .await;
    let g_id = function_id_in(&q, "g", "a.py").await;
    let bar_parse_id = function_id_in(&q, "parse", "bar.py").await;
    let canonical = q
        .list_calls_edges_by_resolution("calls_resolved_canonical")
        .await
        .expect("list canonical edges");
    let singleton = q
        .list_calls_edges_by_resolution("calls_resolved_singleton")
        .await
        .expect("list singleton edges");
    assert!(
        !canonical.contains(&(g_id.clone(), bar_parse_id.clone()))
            && !singleton.contains(&(g_id, bar_parse_id)),
        "a parameter-shadowed callee must fail closed; \
         canonical={canonical:?} singleton={singleton:?}"
    );

    // C12-1: an in-body assignment makes the whole body treat the name as local
    // (UnboundLocalError), so the call before it cannot bind to the import.
    let (_tmp, q) = index_python_fixture(&[
        (
            "b.py",
            "from bar import parse\n\ndef g():\n    parse()\n    parse = factory()\n",
        ),
        ("bar.py", "def parse():\n    return 1\n"),
    ])
    .await;
    let g_id = function_id_in(&q, "g", "b.py").await;
    let bar_parse_id = function_id_in(&q, "parse", "bar.py").await;
    let canonical = q
        .list_calls_edges_by_resolution("calls_resolved_canonical")
        .await
        .expect("list canonical edges");
    let singleton = q
        .list_calls_edges_by_resolution("calls_resolved_singleton")
        .await
        .expect("list singleton edges");
    assert!(
        !canonical.contains(&(g_id.clone(), bar_parse_id.clone()))
            && !singleton.contains(&(g_id, bar_parse_id)),
        "an in-body assignment must poison the callee for the whole body; \
         canonical={canonical:?} singleton={singleton:?}"
    );

    // T-d: a `global` write dynamically rebinds the module name — every caller
    // of the bare name fails closed.
    let (_tmp, q) = index_python_fixture(&[
        (
            "d.py",
            "from bar import f\n\ndef w():\n    global f\n    f = factory()\n    f()\n\ndef sibling():\n    f()\n",
        ),
        ("bar.py", "def f():\n    return 1\n"),
    ])
    .await;
    let sibling_id = function_id_in(&q, "sibling", "d.py").await;
    let bar_f_id = function_id_in(&q, "f", "bar.py").await;
    let canonical = q
        .list_calls_edges_by_resolution("calls_resolved_canonical")
        .await
        .expect("list canonical edges");
    let singleton = q
        .list_calls_edges_by_resolution("calls_resolved_singleton")
        .await
        .expect("list singleton edges");
    assert!(
        !canonical.contains(&(sibling_id.clone(), bar_f_id.clone()))
            && !singleton.contains(&(sibling_id, bar_f_id)),
        "a dynamically rebound module name must fail closed for every caller; \
         canonical={canonical:?} singleton={singleton:?}"
    );
}
