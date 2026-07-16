//! Regression tier for the retrieval-eval subsystem (081.007-T / 084.012-T).
//!
//! The tier IS the deliverable: it runs the eval over a fixture corpus and
//! asserts the metrics clear the committed `docs/eval/baseline.json` thresholds.
//! The graph metrics are computed from the **real** indexed path — index a
//! fixture workspace, then reproduce `run_retrieval_eval`'s exact graph pipeline
//! (parser call-site denominator + real `calls` edges + real dangling count) —
//! not from injected constants (F137D72E). `false_edge_rate` remains a
//! dangling-only lower bound, so target-correctness is asserted directly against
//! a ground-truth expected-edges manifest (the 084.004 correctness nuance).
//!
//! Scenarios:
//! 1. fixture semantic evaluation meets the semantic baseline;
//! 2. real-path graph metrics match the fixture's expected recall and clear the
//!    graph baseline;
//! 3. real-path target-correctness: singleton edges match the manifest exactly;
//! 4. a seeded regression in the count path fails the tier (the tier has teeth);
//! 5. the baseline JSON round-trips through serde.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use serde_json::json;

use engram::db::connect_db;
use engram::db::queries::CodeGraphQueries;
use engram::models::Function;
use engram::models::config::{CodeGraphConfig, WorkspaceConfig};
use engram::models::retrieval_eval::{
    GraphMetrics, RetrievalEvalConfig, RetrievalEvalReport, RetrievalEvalThresholds,
};
use engram::server::state::AppState;
use engram::services::code_graph;
use engram::services::retrieval_eval::{
    check_thresholds, compute_graph_metrics, evaluate_semantic, evaluate_target_correctness,
    scan_call_site_inventory,
};
use engram::tools;

/// The committed graduated baseline thresholds.
const BASELINE_JSON: &str = include_str!("../../docs/eval/baseline.json");

/// Build a synthetic function record with an empty embedding (keyword-only).
fn make_fn(id: &str, name: &str, signature: &str, docstring: &str, file_path: &str) -> Function {
    Function {
        id: id.to_owned(),
        name: name.to_owned(),
        file_path: file_path.to_owned(),
        line_start: 1,
        line_end: 2,
        signature: signature.to_owned(),
        docstring: Some(docstring.to_owned()),
        body: String::new(),
        body_hash: String::new(),
        token_count: 0,
        embed_type: "explicit_code".to_owned(),
        embedding: Vec::new(),
        summary: String::new(),
    }
}

/// A deterministic fixture corpus where each symbol's docstring is unique.
fn fixture_corpus() -> Vec<Function> {
    vec![
        make_fn(
            "function:alpha",
            "alpha",
            "fn alpha()",
            "alpha unique widget assembly",
            "src/alpha.rs",
        ),
        make_fn(
            "function:bravo",
            "bravo",
            "fn bravo()",
            "bravo distinct gadget compilation",
            "src/bravo.rs",
        ),
        make_fn(
            "function:charlie",
            "charlie",
            "fn charlie()",
            "charlie separate sprocket rendering",
            "src/charlie.rs",
        ),
    ]
}

/// Assemble the semantic portion of a fixture report from the **real**
/// `evaluate_semantic` over a deterministic synthetic corpus (distinct
/// docstrings ⇒ near-perfect self-retrieval). The graph portion is filled from
/// the real indexed path by the graph scenarios below.
fn semantic_fixture_report(graph: GraphMetrics) -> RetrievalEvalReport {
    let config = RetrievalEvalConfig {
        enabled: true,
        languages: vec!["rust".to_owned()],
        k: 5,
        sample_size: 200,
        ..RetrievalEvalConfig::default()
    };
    let semantic = evaluate_semantic(&fixture_corpus(), &config).expect("semantic eval");

    RetrievalEvalReport {
        enabled: true,
        branch: "main".to_owned(),
        evaluated_at: "2026-07-10T00:00:00Z".to_owned(),
        k: config.k,
        sample_size: config.sample_size,
        languages: config.languages,
        semantic,
        graph,
        thresholds_breached: false,
        threshold_breaches: Vec::new(),
    }
}

fn baseline() -> RetrievalEvalThresholds {
    serde_json::from_str(BASELINE_JSON).expect("baseline.json parses")
}

// ── Real graph fixture (F137D72E) ────────────────────────────────────────────
//
// A hand-authored multi-file Rust fixture with a known ground truth, indexed via
// the real `code_graph::index_workspace` so the graph metrics come from the same
// pipeline `run_retrieval_eval` uses — never injected constants.
//
// Parser call-site inventory (recall denominator) = 4:
//   a.rs: beta(), local_a();  b.rs: gamma();  d.rs: dup().
// Resolved `calls` edges (numerator) = 3:
//   in-file direct: alpha -> local_a;
//   cross-file singletons: alpha -> beta, beta -> gamma.
// The ambiguous `dup` (defined in e.rs AND f.rs) is skipped, never mis-resolved,
// so it contributes no edge. No resolved edge is dangling ⇒ false_edge_rate = 0.
//   expected resolution_recall = 3/4 = 0.75  (clears baseline floor 0.5)
//   expected false_edge_rate   = 0.0         (within baseline ceiling 0.2)

const FILE_A: &str =
    "pub fn alpha() {\n    beta();\n    local_a();\n}\n\npub fn local_a() {\n    let _ = 1;\n}\n";
const FILE_B: &str = "pub fn beta() {\n    gamma();\n}\n";
const FILE_C: &str = "pub fn gamma() {\n    let _ = 2;\n}\n";
const FILE_D: &str = "pub fn caller_dup() {\n    dup();\n}\n";
const FILE_E: &str = "pub fn dup() {\n    let _ = 3;\n}\n";
const FILE_F: &str = "pub fn dup() {\n    let _ = 4;\n}\n";

const FIXTURE: &[(&str, &str)] = &[
    ("src/a.rs", FILE_A),
    ("src/b.rs", FILE_B),
    ("src/c.rs", FILE_C),
    ("src/d.rs", FILE_D),
    ("src/e.rs", FILE_E),
    ("src/f.rs", FILE_F),
];

/// Ground-truth manifest of expected `calls_resolved_singleton` edges as
/// (caller, callee) name pairs — the primary target-correctness gate.
const EXPECTED_SINGLETONS: &[(&str, &str)] = &[("alpha", "beta"), ("beta", "gamma")];

const FIXTURE_CALL_SITES: usize = 4;
const FIXTURE_RESOLVED: u64 = 3;
const FIXTURE_FALSE_EDGES: u64 = 0;

fn write_fixture(ws: &Path) {
    for (rel, content) in FIXTURE {
        let full = ws.join(rel);
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent).expect("create dirs");
        }
        fs::write(full, content).expect("write file");
    }
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

/// Index the fixture with the cross-file post-pass (full-index path) and return
/// a queries handle plus the indexed data-dir/branch for a subsequent eval read.
async fn index_fixture() -> (
    tempfile::TempDir,
    CodeGraphQueries,
    std::path::PathBuf,
    String,
) {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_fixture(ws);
    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);
    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("index");
    let db = connect_db(&data_dir, &branch).await.expect("connect_db");
    let queries = CodeGraphQueries::new(db);
    (tmp, queries, data_dir, branch)
}

/// Reproduce `run_retrieval_eval`'s exact graph pipeline over the indexed fixture:
/// the parser call-site inventory (denominator), the real gated `calls` edge
/// count (numerator), and the real dangling count (false edges). This is the
/// REAL path under test — no injected metrics.
async fn real_graph_metrics(ws: &Path, queries: &CodeGraphQueries) -> GraphMetrics {
    let languages = vec!["rust".to_owned()];
    let files = queries.list_code_files().await.expect("list_code_files");
    let inventory = scan_call_site_inventory(ws, &files, &languages)
        .await
        .expect("scan_call_site_inventory");
    let resolved = queries
        .count_calls_edges_in_languages(&languages)
        .await
        .expect("count_calls_edges_in_languages");
    // Language-gated dangling count — identical scoping to the resolved
    // numerator, matching the corrected `run_retrieval_eval` handler (084.008-T
    // / Thread-8): `false_edge_rate` is a ratio of commensurable units.
    let false_edges = queries
        .count_dangling_calls_edges_in_languages(&languages)
        .await
        .expect("count_dangling_calls_edges_in_languages");
    let mut graph = compute_graph_metrics(inventory.call_sites, resolved, false_edges);
    graph.index_stale = inventory.index_stale;
    graph.unreadable_files = inventory.unreadable_files;
    graph
}

/// Build a `name -> [function_id]` map from the indexed function corpus.
async fn name_to_ids(q: &CodeGraphQueries) -> HashMap<String, Vec<String>> {
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for f in q.all_functions().await.expect("all_functions") {
        map.entry(f.name).or_default().push(f.id);
    }
    map
}

#[test]
fn fixture_meets_semantic_baseline() {
    // Semantic side uses the real evaluate_semantic; graph side is irrelevant to
    // this assertion, so a passing placeholder graph keeps the report well-formed.
    let report = semantic_fixture_report(compute_graph_metrics(
        FIXTURE_CALL_SITES,
        FIXTURE_RESOLVED,
        FIXTURE_FALSE_EDGES,
    ));
    let thresholds = baseline();
    let check = check_thresholds(&report, &thresholds);

    assert!(
        check.passed,
        "fixture must clear the semantic baseline; breaches: {:?}",
        check.breaches
    );
    // Sanity: the semantic self-retrieval on a distinct corpus is near-perfect.
    assert!(report.semantic.recall_at_k >= thresholds.min_recall_at_k);
    assert!(report.semantic.mrr >= thresholds.min_mrr);
}

// Scenario 2: the REAL indexed-path graph metrics match the fixture's expected
// recall exactly and clear the committed graph baseline.
#[tokio::test]
async fn real_path_graph_metrics_match_expected_and_clear_baseline() {
    let (tmp, queries, _dir, _branch) = index_fixture().await;
    let graph = real_graph_metrics(tmp.path(), &queries).await;

    assert_eq!(
        graph.call_sites, FIXTURE_CALL_SITES,
        "real parser inventory must be the fixture denominator"
    );
    assert_eq!(
        graph.resolved, FIXTURE_RESOLVED,
        "real resolved-edge count must match the fixture ground truth"
    );
    assert_eq!(
        graph.false_edges, FIXTURE_FALSE_EDGES,
        "no resolved edge is dangling in the fixture"
    );
    assert!(
        (graph.resolution_recall - 0.75).abs() < 1e-9,
        "real-path resolution_recall must be 3/4 = 0.75, got {}",
        graph.resolution_recall
    );
    assert!(
        !graph.index_stale,
        "an unmodified freshly-indexed tree must not be flagged stale"
    );

    let report = semantic_fixture_report(graph);
    let thresholds = baseline();
    let check = check_thresholds(&report, &thresholds);
    assert!(
        check.passed,
        "the real-path fixture must clear the full baseline; breaches: {:?}",
        check.breaches
    );
}

// Scenario 3 (PRIMARY GATE): real-path target-correctness. Every produced
// `calls_resolved_singleton` edge matches the ground-truth manifest (correct
// callee id) and the produced count equals the manifest size. This is asserted
// directly because `false_edge_rate` is only a dangling-only lower bound and
// cannot, on its own, prove a resolved edge points at the RIGHT callee.
#[tokio::test]
async fn real_path_singleton_edges_match_manifest() {
    let (_tmp, queries, _dir, _branch) = index_fixture().await;
    let names = name_to_ids(&queries).await;

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

    let actual: HashSet<(String, String)> = queries
        .list_calls_edges_by_resolution("calls_resolved_singleton")
        .await
        .expect("list singletons")
        .into_iter()
        .collect();

    assert_eq!(
        actual.len(),
        EXPECTED_SINGLETONS.len(),
        "produced singleton count must equal the manifest size; actual: {actual:?}"
    );
    assert_eq!(
        actual, expected,
        "every singleton edge must match the ground-truth manifest (correct callee id)"
    );

    // The ambiguous `dup` name must never be a resolved singleton target.
    let dup_ids: HashSet<&String> = names.get("dup").expect("dup indexed").iter().collect();
    assert_eq!(dup_ids.len(), 2, "dup must have two ambiguous definitions");
    assert!(
        actual.iter().all(|(_, to)| !dup_ids.contains(to)),
        "no singleton edge may resolve to an ambiguous `dup` definition"
    );
}

// Scenario 4: a seeded regression in the count path fails the tier. The correct
// real-path report clears the baseline; a report built from a regressed count
// (the cross-file post-pass silently dropping resolved edges) must FAIL the SAME
// baseline — proving the tier catches a genuine count-path regression rather than
// passing vacuously.
#[tokio::test]
async fn seeded_count_regression_fails_the_tier() {
    let (tmp, queries, _dir, _branch) = index_fixture().await;
    let real = real_graph_metrics(tmp.path(), &queries).await;
    let thresholds = baseline();

    // The real path passes and is non-vacuous (it actually measured something).
    assert!(
        real.call_sites > 0 && real.resolved > 0,
        "real path must measure edges"
    );
    assert!(
        check_thresholds(&semantic_fixture_report(real.clone()), &thresholds).passed,
        "sanity: the correct real path clears the baseline"
    );

    // Seed a regression: only the in-file direct edge survives (the cross-file
    // post-pass regressed), dropping recall to 1/4 = 0.25, below the 0.5 floor.
    let regressed = compute_graph_metrics(real.call_sites, 1, real.false_edges);
    let check = check_thresholds(&semantic_fixture_report(regressed), &thresholds);
    assert!(
        !check.passed,
        "a regressed resolved-edge count must fail the graph baseline"
    );
    assert!(
        check
            .breaches
            .iter()
            .any(|b| b.contains("resolution_recall")),
        "the breach list must name resolution_recall; got {:?}",
        check.breaches
    );
}

#[test]
fn baseline_json_round_trips() {
    let original = baseline();
    let serialized = serde_json::to_string(&original).expect("serialize thresholds");
    let reparsed: RetrievalEvalThresholds =
        serde_json::from_str(&serialized).expect("re-parse thresholds");
    assert_eq!(original, reparsed, "baseline thresholds must round-trip");
}

// ── Thread-2: exercise the REAL `run_retrieval_eval` dispatch ─────────────────
//
// The scenarios above reproduce the graph pipeline directly; the plan
// (084.012-T, "assert graph metrics from the real run_retrieval_eval path")
// also requires driving the actual handler so a wiring regression — e.g. the
// resolved/dangling language-scope mismatch corrected in 084.008-T — cannot
// stay green. Bind the indexed fixture workspace, dispatch the tool, and assert
// the returned `graph` report against the ground truth.

/// Dispatch a tool, retrying the known transient reopen lock / in-progress
/// index the daemon's background bind-scan can surface on rapid setup.
async fn dispatch_retry(
    state: Arc<AppState>,
    tool: &str,
    args: Option<serde_json::Value>,
) -> serde_json::Value {
    let mut last_err = String::new();
    for _ in 0u32..12 {
        match tools::dispatch(state.clone(), tool, args.clone()).await {
            Ok(value) => return value,
            Err(err) => {
                let lowered = err.to_string().to_ascii_lowercase();
                if lowered.contains("database is locked")
                    || lowered.contains("sqlite_busy")
                    || lowered.contains("in progress")
                    || lowered.contains("indexinprogress")
                {
                    last_err = err.to_string();
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    continue;
                }
                panic!("dispatch {tool} failed: {err}");
            }
        }
    }
    panic!("dispatch {tool} failed after retries on transient error: {last_err}");
}

#[tokio::test]
async fn real_path_via_run_retrieval_eval_dispatch_matches_ground_truth() {
    let workspace = tempfile::tempdir().expect("tempdir");
    let git_dir = workspace.path().join(".git");
    fs::create_dir_all(&git_dir).expect("create .git");
    fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").expect("write HEAD");
    write_fixture(workspace.path());

    let state = Arc::new(AppState::new(10));
    let path = workspace.path().to_string_lossy().to_string();
    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace must succeed");
    let config = WorkspaceConfig {
        retrieval_eval: RetrievalEvalConfig {
            enabled: true,
            languages: vec!["rust".to_owned()],
            k: 5,
            sample_size: 200,
            ..RetrievalEvalConfig::default()
        },
        ..WorkspaceConfig::default()
    };
    state.set_workspace_config(Some(config)).await;

    dispatch_retry(state.clone(), "index_workspace", Some(json!({}))).await;
    let report = dispatch_retry(state.clone(), "run_retrieval_eval", Some(json!({}))).await;

    let graph = &report["graph"];
    assert_eq!(
        graph["call_sites"],
        json!(FIXTURE_CALL_SITES),
        "real handler parser inventory must be the fixture denominator; report: {report}"
    );
    assert_eq!(
        graph["resolved"],
        json!(FIXTURE_RESOLVED),
        "real handler resolved-edge count must match the ground truth; report: {report}"
    );
    assert_eq!(
        graph["false_edges"],
        json!(FIXTURE_FALSE_EDGES),
        "real handler must report no dangling edge in the fixture; report: {report}"
    );
    let recall = graph["resolution_recall"]
        .as_f64()
        .expect("resolution_recall must be a number");
    assert!(
        (recall - 0.75).abs() < 1e-9,
        "real-handler resolution_recall must be 3/4 = 0.75, got {recall}; report: {report}"
    );
}

// ── Thread-3: the target-correctness report fields are populated + asserted ───
//
// `GraphMetrics::target_correct` / `target_mismatch` are the plan's model
// surface for manifest-backed target-correctness (084.001-T / 084.004-T). They
// are 0 in production (no manifest); this fixture/regression path populates them
// from `evaluate_target_correctness` and asserts the SERIALIZED report carries
// them, so the documented contract is exercised rather than left as
// indistinguishable zero defaults.
#[tokio::test]
async fn real_path_report_populates_target_correctness_fields() {
    let (tmp, queries, _dir, _branch) = index_fixture().await;
    let names = name_to_ids(&queries).await;

    // Ground-truth expected-target manifest: (caller_id, callee_id) by identity.
    let expected: HashSet<(String, String)> = EXPECTED_SINGLETONS
        .iter()
        .map(|(caller, callee)| {
            let from = names.get(*caller).expect("caller indexed")[0].clone();
            let to = names.get(*callee).expect("callee indexed")[0].clone();
            (from, to)
        })
        .collect();

    let produced: Vec<(String, String)> = queries
        .list_calls_edges_by_resolution("calls_resolved_singleton")
        .await
        .expect("list singletons");

    let tc = evaluate_target_correctness(&produced, &expected);

    // Wire the manifest result into the report's graph fields (the fixture /
    // regression path the model doc names as the populating caller).
    let mut graph = real_graph_metrics(tmp.path(), &queries).await;
    graph.target_correct = tc.target_correct;
    graph.target_mismatch = tc.target_mismatch;

    let expected_count = u64::try_from(EXPECTED_SINGLETONS.len()).expect("fits u64");
    assert_eq!(
        graph.target_correct, expected_count,
        "every produced singleton matches the manifest by identity"
    );
    assert_eq!(
        graph.target_mismatch, 0,
        "no wrong-but-existing target in the fixture"
    );

    // The SERIALIZED report must carry the populated counters, not zero defaults.
    let report = semantic_fixture_report(graph);
    let value = serde_json::to_value(&report).expect("serialize report");
    assert_eq!(
        value["graph"]["target_correct"],
        json!(expected_count),
        "serialized report must expose the populated target_correct; report: {value}"
    );
    assert_eq!(
        value["graph"]["target_mismatch"],
        json!(0u64),
        "serialized report must expose target_mismatch=0; report: {value}"
    );
}

#[tokio::test]
async fn seeded_misresolution_to_real_target_fails_target_correctness_gate() {
    let (tmp, queries, _dir, _branch) = index_fixture().await;
    let names = name_to_ids(&queries).await;

    let expected: HashSet<(String, String)> = EXPECTED_SINGLETONS
        .iter()
        .map(|(caller, callee)| {
            let from = names.get(*caller).expect("caller indexed")[0].clone();
            let to = names.get(*callee).expect("callee indexed")[0].clone();
            (from, to)
        })
        .collect();

    let alpha = names.get("alpha").expect("alpha indexed")[0].clone();
    let local_a = names.get("local_a").expect("local_a indexed")[0].clone();
    let wrong_but_real = vec![(alpha, local_a)];
    let tc = evaluate_target_correctness(&wrong_but_real, &expected);

    let mut graph = real_graph_metrics(tmp.path(), &queries).await;
    graph.target_correct = tc.target_correct;
    graph.target_mismatch = tc.target_mismatch;
    assert_eq!(
        graph.target_mismatch, 1,
        "seed must be a real target mismatch"
    );

    let check = check_thresholds(&semantic_fixture_report(graph), &baseline());
    assert!(
        !check.passed,
        "target-correctness gate must fail a wrong-but-existing callee"
    );
    assert!(
        check
            .breaches
            .iter()
            .any(|b| b.contains("target_correctness")),
        "breach list must name target_correctness; got {:?}",
        check.breaches
    );
}
