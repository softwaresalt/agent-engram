//! Integration tests for graph resolution-recall eval compute (081.005-T).
//!
//! Exercises the four plan scenarios for the graph slice:
//! 1. all-local resolved → `resolution_recall` ≈ 1.0;
//! 2. cross-file drop → `resolution_recall` < 1.0;
//! 3. edge to non-existent def → `false_edge_rate` > 0;
//! 4. empty graph → zeroed report.
//!
//! Plus a denominator sanity check that the parser call-site inventory
//! (`count_call_sites`) matches the expected identifier-call count.

use engram::services::parsing::Language;
use engram::services::retrieval_eval::{compute_graph_metrics, count_call_sites};

#[test]
fn all_local_resolved_gives_full_recall() {
    // Every visible call site resolved to a local def.
    let metrics = compute_graph_metrics(5, 5, 0);
    assert!(
        (metrics.resolution_recall - 1.0).abs() < 1e-9,
        "expected recall 1.0, got {}",
        metrics.resolution_recall
    );
    assert!((metrics.false_edge_rate - 0.0).abs() < 1e-9);
    assert_eq!(metrics.call_sites, 5);
    assert_eq!(metrics.resolved, 5);
}

#[test]
fn cross_file_calls_drop_recall_below_one() {
    // Ten visible call sites, only six resolved within-file.
    let metrics = compute_graph_metrics(10, 6, 0);
    assert!(
        (metrics.resolution_recall - 0.6).abs() < 1e-9,
        "expected recall 0.6, got {}",
        metrics.resolution_recall
    );
    assert!(
        metrics.resolution_recall < 1.0,
        "cross-file drop must lower recall below 1.0"
    );
}

#[test]
fn dangling_callee_raises_false_edge_rate() {
    // Two of eight resolved edges point at a callee with no matching def.
    let metrics = compute_graph_metrics(10, 8, 2);
    assert!(
        (metrics.false_edge_rate - 0.25).abs() < 1e-9,
        "expected false-edge rate 0.25, got {}",
        metrics.false_edge_rate
    );
    assert!(
        metrics.false_edge_rate > 0.0,
        "a dangling callee must yield a positive false-edge rate"
    );
}

#[test]
fn empty_graph_yields_zero_report() {
    let metrics = compute_graph_metrics(0, 0, 0);
    assert!((metrics.resolution_recall - 0.0).abs() < 1e-9);
    assert!((metrics.false_edge_rate - 0.0).abs() < 1e-9);
    assert_eq!(metrics.call_sites, 0);
    assert_eq!(metrics.resolved, 0);
    assert_eq!(metrics.false_edges, 0);
}

#[test]
fn count_call_sites_matches_identifier_calls() {
    // `foo` calls `bar` and `baz`; those two definitions make no calls.
    // Method calls / blocklisted helpers would be excluded by the parser.
    let source = r"
fn foo() {
    bar();
    baz();
}
fn bar() {}
fn baz() {}
";
    assert_eq!(count_call_sites(source, Language::Rust), 2);
}
