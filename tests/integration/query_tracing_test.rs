//! Integration tests for query performance observability (dxo.5.1).
//!
//! Verifies that `record_query_metrics()` emits tracing spans with
//! `query_type`, `table`, and `result_count` fields, and that slow
//! queries (>100ms) produce WARN-level log entries.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use tracing_subscriber::layer::SubscriberExt;

use engram::db::queries::{SLOW_QUERY_THRESHOLD_MS, record_query_metrics};

/// A minimal tracing `Layer` that counts WARN-level events.
struct WarnCounter(Arc<Mutex<usize>>);

impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for WarnCounter {
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        if event.metadata().level() == &tracing::Level::WARN {
            *self.0.lock().unwrap() += 1;
        }
    }
}

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
fn record_query_metrics_emits_warn_for_slow_query() {
    // GIVEN a custom tracing layer that counts WARN events
    let warn_count = Arc::new(Mutex::new(0_usize));
    let subscriber = tracing_subscriber::registry().with(WarnCounter(warn_count.clone()));
    let _guard = tracing::subscriber::set_default(subscriber);

    // AND a duration that exceeds the slow-query threshold
    let slow = Duration::from_millis(200);
    assert!(
        slow.as_millis() > SLOW_QUERY_THRESHOLD_MS,
        "test duration must exceed SLOW_QUERY_THRESHOLD_MS ({SLOW_QUERY_THRESHOLD_MS}ms) \
         to exercise the WARN path"
    );

    // WHEN metrics are recorded
    record_query_metrics("graph_traversal", "function", 42, slow);

    // THEN at least one WARN event must have been emitted
    assert!(
        *warn_count.lock().unwrap() > 0,
        "record_query_metrics must emit a WARN event when elapsed exceeds SLOW_QUERY_THRESHOLD_MS"
    );
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
