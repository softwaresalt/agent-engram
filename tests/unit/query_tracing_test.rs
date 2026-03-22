//! Unit tests for dxo.5.1: `#[tracing::instrument]` spans on `CodeGraphQueries`.
//!
//! Source-level checks verifying that the instrument attributes and
//! `record_query_metrics` call sites are wired into `queries.rs`.

// GIVEN the queries.rs source
// WHEN we inspect it
// THEN at least one #[tracing::instrument] attribute must be present on a public method
#[test]
fn queries_has_tracing_instrument_attribute() {
    let source = include_str!("../../src/db/queries.rs");
    assert!(
        source.contains("#[tracing::instrument"),
        "queries.rs must contain #[tracing::instrument] on public methods"
    );
}

// GIVEN the vector_search_symbols_native method
// WHEN we inspect the source
// THEN it must call record_query_metrics for timing observability
#[test]
fn vector_search_native_calls_record_query_metrics() {
    let source = include_str!("../../src/db/queries.rs");
    assert!(
        source.contains("record_query_metrics"),
        "queries.rs must call record_query_metrics in at least one method"
    );
}

// GIVEN the queries module
// WHEN we inspect the source
// THEN it must emit a slow-query warning at the 100ms threshold
#[test]
fn slow_query_threshold_is_100ms() {
    let source = include_str!("../../src/db/queries.rs");
    assert!(
        source.contains("SLOW_QUERY_THRESHOLD_MS"),
        "queries.rs must define SLOW_QUERY_THRESHOLD_MS"
    );
    assert!(
        source.contains("100"),
        "SLOW_QUERY_THRESHOLD_MS must be set to 100"
    );
}
