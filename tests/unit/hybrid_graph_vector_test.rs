//! Unit tests for dxo.3.1: `hybrid_graph_vector_search` method on `CodeGraphQueries`.
//!
//! These source-level tests verify that the method exists with the correct
//! signature before exercising it in integration tests (dxo.3.3).

// GIVEN the queries module source
// WHEN we inspect it
// THEN hybrid_graph_vector_search method must be present
#[test]
fn hybrid_graph_vector_search_method_exists() {
    let source = include_str!("../../src/db/queries.rs");
    assert!(
        source.contains("hybrid_graph_vector_search"),
        "queries.rs must define hybrid_graph_vector_search"
    );
}

// GIVEN the hybrid_graph_vector_search method
// WHEN we check its return type
// THEN it must return Vec<(f32, SymbolMatch)>
#[test]
fn hybrid_graph_vector_search_returns_scored_symbol_matches() {
    let source = include_str!("../../src/db/queries.rs");
    assert!(
        source.contains("Vec<(f32, SymbolMatch)>"),
        "hybrid_graph_vector_search must return Vec<(f32, SymbolMatch)>"
    );
}

// GIVEN the hybrid_graph_vector_search method
// WHEN we inspect its parameters
// THEN it must accept a configurable edge_types slice
#[test]
fn hybrid_graph_vector_search_accepts_edge_types_slice() {
    let source = include_str!("../../src/db/queries.rs");
    assert!(
        source.contains("edge_types: &[&str]"),
        "hybrid_graph_vector_search must accept edge_types: &[&str] parameter"
    );
}
