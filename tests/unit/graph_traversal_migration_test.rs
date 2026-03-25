//! Unit tests verifying `map_code` and `impact_analysis` use native graph traversal (dxo.1.2).
//!
//! After dxo.1.1 added `graph_neighborhood()` (SurrealQL-native, single batched
//! query per hop), `map_code` and `impact_analysis` must delegate to it instead
//! of the manual `bfs_neighborhood()` BFS loop.

// ── Source-level migration verification ────────────────────────────────────

/// `src/tools/read.rs` must not call `bfs_neighborhood` — both `map_code` and
/// `impact_analysis` must use `graph_neighborhood` for native graph traversal.
#[test]
fn read_rs_does_not_call_bfs_neighborhood() {
    // GIVEN the tool handler source
    let src = include_str!("../../src/tools/read.rs");

    // WHEN we scan for bfs_neighborhood calls
    // THEN neither map_code nor impact_analysis may call it
    assert!(
        !src.contains("bfs_neighborhood"),
        "src/tools/read.rs must not call bfs_neighborhood() after dxo.1.2; \
         use graph_neighborhood() (SurrealQL-native) instead"
    );
}

/// `src/tools/read.rs` must call `graph_neighborhood` for graph traversal.
#[test]
fn read_rs_calls_graph_neighborhood() {
    // GIVEN the tool handler source
    let src = include_str!("../../src/tools/read.rs");

    // WHEN we scan for graph_neighborhood calls
    // THEN it must be present (both map_code and impact_analysis must use it)
    assert!(
        src.contains("graph_neighborhood"),
        "src/tools/read.rs must call graph_neighborhood() after dxo.1.2 migration"
    );
}

/// The `impact_analysis` doc comment must reflect the `SurrealQL` graph traversal,
/// not the old BFS loop description.
#[test]
fn impact_analysis_docs_reference_graph_traversal() {
    // GIVEN the tool handler source
    let src = include_str!("../../src/tools/read.rs");

    // WHEN we check the impact_analysis function comment
    // THEN it should not reference "BFS" as the traversal mechanism
    //
    // The old comment said "BFS traverse code graph" — after migration it must
    // say "graph traversal" or "graph_neighborhood" to reflect the new approach.
    assert!(
        !src.contains("BFS traverse code graph"),
        "impact_analysis doc comment still says 'BFS traverse code graph'; \
         update to reflect native graph traversal after dxo.1.2"
    );
}
