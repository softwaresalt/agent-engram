//! Regression tier for the retrieval-eval subsystem (081.007-T).
//!
//! The tier IS the deliverable: it runs the eval over a fixture corpus and
//! asserts the metrics clear the committed `docs/eval/baseline.json` thresholds.
//!
//! Scenarios:
//! 1. fixture evaluation meets the semantic baseline;
//! 2. fixture evaluation meets the graph baseline;
//! 3. a stricter threshold breach fails the tier;
//! 4. the baseline JSON round-trips through serde.

use engram::models::Function;
use engram::models::retrieval_eval::{
    GraphMetrics, RetrievalEvalConfig, RetrievalEvalReport, RetrievalEvalThresholds,
};
use engram::services::retrieval_eval::{
    check_thresholds, compute_graph_metrics, evaluate_semantic,
};

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

/// Assemble a fixture report: real semantic eval + representative graph counts.
fn fixture_report() -> RetrievalEvalReport {
    let config = RetrievalEvalConfig {
        enabled: true,
        languages: vec!["rust".to_owned()],
        k: 5,
        sample_size: 200,
        ..RetrievalEvalConfig::default()
    };
    let semantic = evaluate_semantic(&fixture_corpus(), &config).expect("semantic eval");
    // Nine of ten visible call sites resolved, no dangling edges.
    let graph: GraphMetrics = compute_graph_metrics(10, 9, 0);

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

#[test]
fn fixture_meets_semantic_baseline() {
    let report = fixture_report();
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

#[test]
fn fixture_meets_graph_baseline() {
    let report = fixture_report();
    let thresholds = baseline();

    assert!(
        report.graph.resolution_recall >= thresholds.min_resolution_recall,
        "resolution_recall {} below floor {}",
        report.graph.resolution_recall,
        thresholds.min_resolution_recall
    );
    assert!(
        report.graph.false_edge_rate <= thresholds.max_false_edge_rate,
        "false_edge_rate {} above ceiling {}",
        report.graph.false_edge_rate,
        thresholds.max_false_edge_rate
    );
}

#[test]
fn threshold_breach_fails_tier() {
    let report = fixture_report();
    // Demand a resolution recall the fixture (0.9) cannot meet.
    let strict = RetrievalEvalThresholds {
        min_resolution_recall: 0.99,
        ..baseline()
    };
    let check = check_thresholds(&report, &strict);

    assert!(!check.passed, "an unmet threshold must fail the tier");
    assert!(
        check
            .breaches
            .iter()
            .any(|b| b.contains("resolution_recall")),
        "breach list must name the offending metric; got {:?}",
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
