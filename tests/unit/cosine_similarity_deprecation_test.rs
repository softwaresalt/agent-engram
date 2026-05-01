//! Unit tests verifying `cosine_similarity()` deprecation and migration (dxo.2.2).
//!
//! Code symbol search must flow through `SurrealDB`'s native KNN operator.
//! Application-level `cosine_similarity()` must no longer be used in the
//! `unified_search` tool handler path.

// ── Source-level migration verification ────────────────────────────────────

/// The `unified_search` tool handler must not import `cosine_similarity` from
/// `services::search` — code similarity scoring is now DB-computed.
#[test]
fn read_rs_does_not_import_cosine_similarity() {
    // GIVEN the unified_search tool handler source
    let src = include_str!("../../src/tools/read.rs");

    // WHEN we scan the import block for cosine_similarity
    // THEN it must not be imported (migration to native KNN complete)
    //
    // The import line `SearchRegion, UnifiedSearchResult, cosine_similarity,`
    // should no longer exist after this task.
    assert!(
        !src.contains("cosine_similarity"),
        "src/tools/read.rs must not reference cosine_similarity after migrating \
         unified_search to vector_search_symbols_native(); found reference in source"
    );
}

/// The public `cosine_similarity()` in `services::search` must carry a
/// `#[deprecated]` attribute to signal that code search callers should use
/// the DB-native KNN path instead.
#[test]
fn cosine_similarity_is_deprecated_in_search_rs() {
    // GIVEN the search service source
    let src = include_str!("../../src/services/search.rs");

    // WHEN we look for the deprecated attribute on cosine_similarity
    // THEN it must be marked as deprecated
    //
    // The function should look like:
    //   #[deprecated(note = "...")]
    //   pub fn cosine_similarity(...) -> f32 { ... }
    let deprecated_pos = src.find("#[deprecated");
    let fn_pos = src.find("pub fn cosine_similarity");

    assert!(
        deprecated_pos.is_some(),
        "services/search.rs must have a #[deprecated] attribute before cosine_similarity()"
    );

    if let (Some(dep), Some(func)) = (deprecated_pos, fn_pos) {
        assert!(
            dep < func,
            "#[deprecated] must appear before `pub fn cosine_similarity` in search.rs"
        );
    }
}

/// `src/db/queries.rs` no longer exists after `SurrealDB` removal (Phase 7).
/// This test confirms the file is gone — which trivially satisfies the invariant
/// that no private `cosine_similarity` helper exists there.
#[test]
fn queries_rs_does_not_have_private_cosine_similarity() {
    assert!(
        !std::path::Path::new("src/db/queries.rs").exists(),
        "src/db/queries.rs should have been deleted in Phase 7 (SurrealDB removal)"
    );
}
