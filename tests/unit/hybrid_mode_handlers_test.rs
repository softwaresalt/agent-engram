//! Unit tests for dxo.3.2: hybrid graph+vector mode wired into tool handlers.
//!
//! Source-level checks verifying that the optional parameters and the
//! `hybrid_graph_vector_search` call site exist in `read.rs`.

// GIVEN the unified_search params struct
// WHEN we inspect the source
// THEN it must accept an optional scope_to_symbol parameter
#[test]
fn unified_search_params_has_scope_to_symbol() {
    let source = include_str!("../../src/tools/read.rs");
    assert!(
        source.contains("scope_to_symbol"),
        "UnifiedSearchParams must have scope_to_symbol field"
    );
}

// GIVEN the impact_analysis params struct
// WHEN we inspect the source
// THEN it must accept an optional concept parameter
#[test]
fn impact_analysis_params_has_concept() {
    let source = include_str!("../../src/tools/read.rs");
    assert!(
        source.contains("concept: Option<String>"),
        "ImpactAnalysisParams must have concept: Option<String> field"
    );
}

// GIVEN the impact_analysis handler
// WHEN we inspect the source
// THEN it must call hybrid_graph_vector_search when concept is provided
#[test]
fn impact_analysis_calls_hybrid_graph_vector_search() {
    let source = include_str!("../../src/tools/read.rs");
    assert!(
        source.contains("hybrid_graph_vector_search"),
        "impact_analysis handler must call hybrid_graph_vector_search"
    );
}
