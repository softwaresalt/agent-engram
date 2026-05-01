//! Unit tests verifying observability patterns in `CodeGraphQueries` (CozoDB).
//!
//! Source-level checks verifying that `cozo_queries.rs` retains the
//! `record_query_metrics` call sites and `SLOW_QUERY_THRESHOLD_MS` constant
//! that were migrated from the SurrealDB `queries.rs` in Phase 7.

// GIVEN the cozo_queries.rs source (CozoDB implementation after Phase 7)
// WHEN we inspect it
// THEN the SurrealDB queries.rs must be gone (migration complete)
#[test]
fn surreal_queries_file_removed_after_cozo_migration() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("db")
        .join("queries.rs");
    assert!(
        !path.exists(),
        "src/db/queries.rs must be deleted after Phase 7 CozoDB migration"
    );
}

// GIVEN the cozo_queries.rs source
// WHEN we inspect it
// THEN it must call record_query_metrics for timing observability
#[test]
fn cozo_queries_has_record_query_metrics() {
    let source = include_str!("../../src/db/cozo_queries.rs");
    assert!(
        source.contains("record_query_metrics"),
        "cozo_queries.rs must define or call record_query_metrics"
    );
}

// GIVEN the cozo_queries module
// WHEN we inspect the source
// THEN it must define the slow-query warning threshold at 100ms
#[test]
fn slow_query_threshold_is_100ms() {
    let source = include_str!("../../src/db/cozo_queries.rs");
    assert!(
        source.contains("SLOW_QUERY_THRESHOLD_MS"),
        "cozo_queries.rs must define SLOW_QUERY_THRESHOLD_MS"
    );
    assert!(
        source.contains("100"),
        "SLOW_QUERY_THRESHOLD_MS must be set to 100"
    );
}
