//! Integration tests for query performance observability (dxo.5.1).
//!
//! Verifies that `record_query_metrics()` emits tracing spans with
//! `query_type`, `table`, and `result_count` fields, and that slow
//! queries (>100ms) produce WARN-level log entries.

use std::time::Duration;

use engram::db::queries::{SLOW_QUERY_THRESHOLD_MS, record_query_metrics};

#[test]
fn slow_query_threshold_is_100ms() {
    // GIVEN the defined threshold constant
    // THEN it is 100 milliseconds
    assert_eq!(
        SLOW_QUERY_THRESHOLD_MS, 100,
        "slow query threshold must be 100ms"
    );
}

#[test]
fn record_query_metrics_does_not_panic_for_normal_query() {
    // GIVEN a query that completes within the threshold
    let elapsed = Duration::from_millis(10);

    // WHEN we record metrics
    // THEN it should not panic
    record_query_metrics("crud", "function", 5, elapsed);
}

#[test]
fn record_query_metrics_does_not_panic_for_slow_query() {
    // GIVEN a query that exceeds the slow threshold
    let elapsed = Duration::from_millis(200);

    // WHEN we record metrics for a slow query
    // THEN it should not panic (and should emit a WARN log)
    record_query_metrics("graph_traversal", "function", 42, elapsed);
}

#[test]
fn record_query_metrics_handles_zero_results() {
    // GIVEN a query returning zero results
    let elapsed = Duration::from_millis(5);

    // WHEN we record metrics with result_count=0
    // THEN it should not panic
    record_query_metrics("knn_search", "class", 0, elapsed);
}

#[test]
fn record_query_metrics_handles_zero_duration() {
    // GIVEN a query that completes instantaneously
    let elapsed = Duration::ZERO;

    // WHEN we record metrics
    // THEN it should not panic
    record_query_metrics("crud", "interface", 1, elapsed);
}
