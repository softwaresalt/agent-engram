//! Eval-gate acceptance for qualification-/method-aware call resolution
//! (088-F / 088.005-T). This is the RELEASE GATE for shipment 081-S: the
//! qualified/method resolver (088.003-T + 088.004-T) may ship only if it lifts
//! call-graph recall WITHOUT regressing precision (the false-edge / dangling
//! rate), using the 081-F retrieval-eval graph metrics.
//!
//! The gate runs the real graph-metric pipeline
//! ([`compute_graph_metrics`] over [`count_call_sites`],
//! `count_calls_edges`, `count_dangling_calls_edges`) on two indexings of the
//! same call shapes:
//!
//! * BASELINE — bare cross-file calls only (models pre-088 behavior, where
//!   path-qualified calls were extracted but dropped): recall = 1/2 = 0.5.
//! * ENHANCED — the same bare calls made from an impl method that also issues a
//!   `Self::assist()` call (the only provably workspace-owned qualified form),
//!   plus a `receiver.method()` call and a crate-root free-fn call
//!   (`crate::do_work()`). The `Self::` call resolves to its exact
//!   `App::assist` index name; the method call and the crate-root free-fn call
//!   stay deferred (excluded from both numerator and denominator), lifting recall
//!   to 2/3 with ZERO false edges — and `crate::do_work()` does NOT mis-resolve
//!   to the unrelated free `do_work` (asserted by identity, since the
//!   dangling-only false_edge_rate cannot see mis-resolution to a real-but-wrong
//!   target).
//!
//! Only `Self::method()` is provably sound: `Self` is always the concrete
//! enclosing impl type, so it rewrites to the exact `Type::method` index name and
//! cannot collide with a re-export, an import alias, or an external same-named
//! symbol. Every other qualified/method form (`crate::free`, bare `Type::method`,
//! `module::free`, `receiver.method()`) can be ambiguous without scope/import
//! analysis (Option C, deferred by 013-D), so it defers — those boundaries are
//! guarded across `calls_qualified_resolution`.
//!
//! ACCEPTANCE EVIDENCE (recorded 2026-07-15):
//!   resolution_recall  0.50 -> 0.667  (UP: +0.167, the sound `Self::` call)
//!   false_edge_rate    0.00 -> 0.00   (NOT down: precision preserved)
//!   dangling edges     0    -> 0      (every resolved edge targets a real def)
//!   identity precision: crate::do_work() creates NO edge to the free `do_work` (0 false edges)

#![allow(clippy::needless_raw_string_hashes)]
#![allow(clippy::doc_markdown)]

use std::fs;
use std::path::Path;

use tokio::test;

use engram::db::connect_db;
use engram::db::queries::CodeGraphQueries;
use engram::models::config::CodeGraphConfig;
use engram::services::code_graph;
use engram::services::parsing::Language;
use engram::services::retrieval_eval::{compute_graph_metrics, count_call_sites};

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

/// Index `files`, then compute the 081-F graph metrics over them exactly as the
/// retrieval-eval pipeline does: the parser call-site inventory as the
/// denominator, the resolved `calls_edge` count as the numerator, and the
/// dangling-edge count as the false-edge signal.
async fn metrics_for(
    files: &[(&str, &str)],
) -> (
    engram::models::retrieval_eval::GraphMetrics,
    CodeGraphQueries,
    tempfile::TempDir,
) {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    for (rel, content) in files {
        write_sample_file(ws, rel, content);
    }
    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);
    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("index");

    let call_sites: usize = files
        .iter()
        .map(|(_, source)| count_call_sites(source, Language::Rust))
        .sum();

    let db = connect_db(&data_dir, &branch).await.expect("connect_db");
    let q = CodeGraphQueries::new(db);
    let resolved = q.count_calls_edges().await.expect("count_calls_edges");
    let false_edges = q
        .count_dangling_calls_edges()
        .await
        .expect("count_dangling_calls_edges");
    let metrics = compute_graph_metrics(call_sites, resolved, false_edges);
    (metrics, q, tmp)
}

// Bare cross-file calls only: one resolvable (`helper`) and one unresolvable
// (`missing_target`). This models pre-088 behavior — path-qualified calls were
// dropped, so they contributed to neither numerator nor denominator.
const BASELINE: &[(&str, &str)] = &[
    (
        "src/a.rs",
        "pub fn caller() {\n    helper();\n    missing_target();\n}\n",
    ),
    ("src/b.rs", "pub fn helper() {}\n"),
];

// The same bare calls, now made from an impl method that also makes a
// `Self::assist()` call — the only provably-workspace-owned qualified form — plus
// a `receiver.method()` call and a crate-root free-fn call `crate::do_work()`
// that must BOTH stay deferred. `Self::assist` resolves; the deferred calls add
// no edge, and `crate::do_work()` does NOT mis-resolve to the unrelated free
// `do_work`.
const ENHANCED: &[(&str, &str)] = &[
    (
        "src/a.rs",
        "pub struct App;\nimpl App {\n    pub fn run() {\n        helper();\n        \
         missing_target();\n        Self::assist();\n        renderer.render();\n        \
         crate::do_work();\n    }\n    pub fn assist() {}\n}\n",
    ),
    ("src/b.rs", "pub fn helper() {}\n"),
    ("src/c.rs", "pub fn do_work() {}\n"),
];

// The release gate: qualified/method-aware resolution lifts call-graph recall
// without regressing the false-edge rate.
#[test]
async fn qualified_resolution_lifts_recall_without_precision_regression() {
    let (baseline, _bq, _bt) = metrics_for(BASELINE).await;
    let (enhanced, eq, _et) = metrics_for(ENHANCED).await;

    // BASELINE: 1 of 2 bare call sites resolved.
    assert_eq!(
        baseline.call_sites, 2,
        "baseline denominator is the two bare calls"
    );
    assert_eq!(baseline.resolved, 1, "only the unique `helper` resolves");
    assert!(
        (baseline.resolution_recall - 0.5).abs() < 1e-9,
        "baseline recall must be 1/2 = 0.5, got {}",
        baseline.resolution_recall
    );

    // ENHANCED: the `Self::assist()` call resolves; the method call and the
    // crate-root free-fn call are deferred (excluded from both counts).
    assert_eq!(
        enhanced.call_sites, 3,
        "denominator counts the two bare + the Self:: call; the method and crate-root free-fn defer"
    );
    assert_eq!(
        enhanced.resolved, 2,
        "helper + Self::assist resolve; missing_target, the method, and crate::do_work do not"
    );
    assert!(
        (enhanced.resolution_recall - 2.0_f64 / 3.0).abs() < 1e-9,
        "enhanced recall must be 2/3, got {}",
        enhanced.resolution_recall
    );

    // RECALL UP.
    assert!(
        enhanced.resolution_recall > baseline.resolution_recall,
        "Self:: resolution must lift recall ({} !> {})",
        enhanced.resolution_recall,
        baseline.resolution_recall
    );

    // PRECISION NOT DOWN: the false-edge rate must not regress, and every resolved
    // edge must target a real definition (zero dangling).
    assert!(
        (baseline.false_edge_rate - 0.0).abs() < 1e-9,
        "baseline false-edge rate must be 0"
    );
    assert!(
        (enhanced.false_edge_rate - 0.0).abs() < 1e-9,
        "enhanced false-edge rate must not regress above 0, got {}",
        enhanced.false_edge_rate
    );
    assert_eq!(
        enhanced.false_edges, 0,
        "no resolved edge may dangle (all target real defs)"
    );

    // The recovered recall: the `Self::assist` singleton (App::run -> App::assist)
    // plus the bare `helper` singleton, by exact identity.
    let singletons = eq
        .list_calls_edges_by_resolution("calls_resolved_singleton")
        .await
        .expect("list singletons");
    let run = eq
        .resolve_reference_target("App::run")
        .await
        .expect("resolve App::run")
        .expect("App::run indexed");
    let assist = eq
        .resolve_reference_target("App::assist")
        .await
        .expect("resolve App::assist")
        .expect("App::assist indexed");
    let helper = eq
        .resolve_reference_target("helper")
        .await
        .expect("resolve helper")
        .expect("helper indexed");
    assert!(
        singletons.contains(&(run.clone(), assist)),
        "recall must include App::run -> App::assist (Self::), got {singletons:?}"
    );
    assert!(
        singletons.contains(&(run, helper)),
        "recall must include App::run -> helper (bare), got {singletons:?}"
    );

    // PRECISION BY IDENTITY: `count_dangling_calls_edges` (the `false_edge_rate`
    // source) is blind to mis-resolution to a REAL but WRONG target, so assert it
    // directly — the deferred crate-root `crate::do_work()` must create NO edge to
    // the unrelated free `do_work`, and exactly the two expected singletons exist.
    let do_work = eq
        .resolve_reference_target("do_work")
        .await
        .expect("resolve do_work")
        .expect("free `do_work` indexed");
    assert!(
        !singletons.iter().any(|(_, to)| to == &do_work),
        "no singleton may target the unrelated free `do_work` (crate::do_work must defer), got {singletons:?}"
    );
    assert_eq!(
        singletons.len(),
        2,
        "exactly the two expected singletons resolve — no false edge, got {singletons:?}"
    );
}
