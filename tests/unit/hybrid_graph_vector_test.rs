//! Unit tests for dxo.3.1: `hybrid_graph_vector_search` method on `CodeGraphQueries`.
//!
//! These compile-time tests verify that the method exists with the correct
//! signature. If the method is removed or its signature changes, this file
//! will not compile, surfacing the contract breakage at build time rather
//! than at runtime.

use engram::db::queries::{CodeGraphQueries, SymbolMatch};
use engram::errors::EngramError;

/// Compile-time contract check: `hybrid_graph_vector_search` must exist on
/// `CodeGraphQueries`, accept the specified parameter types, and return
/// `Result<Vec<(f32, SymbolMatch)>, EngramError>`.
///
/// This function is never called; its existence enforces the API at compile time.
#[allow(dead_code)]
fn _assert_hybrid_graph_vector_search_signature(
    q: &CodeGraphQueries,
) -> impl std::future::Future<Output = Result<Vec<(f32, SymbolMatch)>, EngramError>> + '_ {
    // root_id: &str, max_depth: usize, query_embedding: &[f32], limit: usize, edge_types: &[&str]
    q.hybrid_graph_vector_search("root", 2, &[0.0f32; 4], 5, &["calls", "imports"])
}

// GIVEN the hybrid_graph_vector_search API contract
// WHEN this file compiles
// THEN the method exists with the expected signature, parameter types, and return type

#[test]
fn hybrid_graph_vector_search_method_exists() {
    // Compilation of `_assert_hybrid_graph_vector_search_signature` above IS
    // the test. Source scanning is replaced by a compile-time type check that
    // the Rust type checker enforces on every build.
}

#[test]
fn hybrid_graph_vector_search_returns_scored_symbol_matches() {
    // Verified at compile time: the return type `Result<Vec<(f32, SymbolMatch)>, _>`
    // is the declared return of `_assert_hybrid_graph_vector_search_signature`.
}

#[test]
fn hybrid_graph_vector_search_accepts_edge_types_slice() {
    // Verified at compile time: `_assert_hybrid_graph_vector_search_signature`
    // passes `&["calls", "imports"]` for the `edge_types: &[&str]` parameter.
}
