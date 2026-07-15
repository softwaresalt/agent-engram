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
//! * ENHANCED — the same bare calls plus a crate-rooted `crate::shapes::compute`
//!   call, a `Type::method` call, a `receiver.method()` call, and an EXTERNAL
//!   `mem::swap()` call. The two in-workspace qualified calls resolve to their
//!   correct targets; the method call and the external-module call stay deferred
//!   (excluded from both numerator and denominator), lifting recall to
//!   3/4 = 0.75 with ZERO false edges — and `mem::swap()` does NOT mis-resolve to
//!   the unrelated free `swap` (asserted by identity, since the dangling-only
//!   false_edge_rate cannot see mis-resolution to a real-but-wrong target).
//!
//! ACCEPTANCE EVIDENCE (recorded 2026-07-14):
//!   resolution_recall  0.50 -> 0.75   (UP: +0.25, the two in-workspace qualified calls)
//!   false_edge_rate    0.00 -> 0.00   (NOT down: precision preserved)
//!   dangling edges     0    -> 0      (every resolved edge targets a real def)
//!   identity precision: mem::swap() creates NO edge to the free `swap` (0 false edges)

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

// The same bare calls plus a crate-rooted module call (`crate::shapes::compute`),
// a crate-rooted type-associated call (`crate::widgets::Circle::draw`), a
// method/receiver call (`renderer.render()`), and an EXTERNAL module call
// (`mem::swap()`) that must NOT resolve to the unrelated free `swap`. The two
// crate-rooted qualified calls resolve; the method and external-module calls stay
// deferred (excluded from both numerator and denominator).
const ENHANCED: &[(&str, &str)] = &[
    (
        "src/a.rs",
        "pub fn caller() {\n    helper();\n    missing_target();\n    crate::shapes::compute();\n    \
         crate::widgets::Circle::draw();\n    renderer.render();\n    mem::swap();\n}\n",
    ),
    ("src/b.rs", "pub fn helper() {}\n"),
    ("src/c.rs", "pub fn compute() {}\n"),
    (
        "src/d.rs",
        "pub struct Circle;\nimpl Circle {\n    pub fn draw() {}\n}\n",
    ),
    ("src/e.rs", "pub fn swap() {}\n"),
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

    // ENHANCED: the two qualified calls now resolve; the method call is excluded.
    assert_eq!(
        enhanced.call_sites, 4,
        "denominator counts the two bare + two qualified calls; the method call is excluded"
    );
    assert_eq!(
        enhanced.resolved, 3,
        "helper + module::compute + Circle::draw resolve; missing_target and the method do not"
    );
    assert!(
        (enhanced.resolution_recall - 0.75).abs() < 1e-9,
        "enhanced recall must be 3/4 = 0.75, got {}",
        enhanced.resolution_recall
    );

    // RECALL UP.
    assert!(
        enhanced.resolution_recall > baseline.resolution_recall,
        "qualified resolution must lift recall ({} !> {})",
        enhanced.resolution_recall,
        baseline.resolution_recall
    );

    // PRECISION NOT DOWN: the false-edge rate must not regress, and every
    // resolved edge must target a real definition (zero dangling).
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
        "no resolved qualified edge may dangle (all target real defs)"
    );

    // The recovered recall is the two qualified singletons pointing at their
    // correct targets — recall recovered by exact identity, not by name luck.
    let singletons = eq
        .list_calls_edges_by_resolution("calls_resolved_singleton")
        .await
        .expect("list singletons");
    let caller = eq
        .resolve_reference_target("caller")
        .await
        .expect("resolve caller")
        .expect("caller indexed");
    for target in ["helper", "compute", "Circle::draw"] {
        let target_id = eq
            .resolve_reference_target(target)
            .await
            .expect("resolve target")
            .unwrap_or_else(|| panic!("target `{target}` indexed"));
        assert!(
            singletons.contains(&(caller.clone(), target_id)),
            "recall must include the `caller -> {target}` singleton, got {singletons:?}"
        );
    }

    // PRECISION BY IDENTITY (not just dangling): `count_dangling_calls_edges`
    // used for `false_edge_rate` is blind to mis-resolution to a REAL but WRONG
    // target, so it cannot by itself prove the invariant. Assert it directly: the
    // external `mem::swap()` call must create NO edge to the unrelated free
    // `swap`, and exactly the three expected singletons exist (no false edge).
    let swap_id = eq
        .resolve_reference_target("swap")
        .await
        .expect("resolve swap")
        .expect("free `swap` indexed");
    assert!(
        !singletons.iter().any(|(_, to)| to == &swap_id),
        "no singleton may target the unrelated free `swap` (mem::swap must not mis-resolve), got {singletons:?}"
    );
    assert_eq!(
        singletons.len(),
        3,
        "exactly the three expected singletons resolve — no false edge, got {singletons:?}"
    );
}
