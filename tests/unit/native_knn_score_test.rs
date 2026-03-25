//! Unit tests verifying `vector_search_symbols_native()` uses DB-returned scores (dxo.2.3).
//!
//! After dxo.2.2 migrated `unified_search` to `vector_search_symbols_native()`, the
//! native function itself still computed scores app-side. This task moves score
//! computation to `SurrealDB`'s `vector::similarity::cosine()` function so scores
//! are DB-authoritative.

// ── Source-level migration verification ────────────────────────────────────

/// `vector_search_symbols_native()` must NOT carry `#[allow(deprecated)]` —
/// that suppression was needed only while it still called `cosine_similarity`.
/// Once scores come from the DB the suppression must be removed.
#[test]
fn vector_search_symbols_native_does_not_allow_deprecated() {
    // GIVEN the queries module source
    let src = include_str!("../../src/db/queries.rs");

    // WHEN we look for the allow(deprecated) annotation on the native function
    // THEN it must be absent (the function no longer calls cosine_similarity)
    //
    // The interim comment:
    //   // cosine_similarity: interim use until dxo.2.3 migrates to DB-native SELECT score
    //   #[allow(deprecated)]
    //   pub async fn vector_search_symbols_native(
    // must be gone after this task.
    assert!(
        !src.contains("interim use until dxo.2.3"),
        "The dxo.2.3 interim suppression comment must be removed from queries.rs once \
         vector_search_symbols_native() uses DB-native scores."
    );
}

/// `queries.rs` must not import `cosine_similarity` from `services::search` at all —
/// that import was transitional for dxo.2.1/2.2 while app-level scoring was still used.
/// After dxo.2.3 only the DB computes scores for code symbols.
#[test]
fn queries_rs_does_not_import_cosine_similarity() {
    // GIVEN the queries module source
    let src = include_str!("../../src/db/queries.rs");

    // WHEN we look for a cosine_similarity import
    // THEN it must be absent
    assert!(
        !src.contains("use crate::services::search::cosine_similarity"),
        "src/db/queries.rs must not import cosine_similarity after dxo.2.3; \
         score computation belongs to SurrealDB's vector::similarity::cosine()."
    );
}

/// `vector_search_symbols_native()` SQL must request the similarity score
/// from `SurrealDB` by including `vector::similarity::cosine` in the SELECT.
#[test]
fn vector_search_symbols_native_selects_db_score() {
    // GIVEN the queries module source
    let src = include_str!("../../src/db/queries.rs");

    // WHEN we look for the DB score selection expression
    // THEN it must be present in the native function's SQL
    assert!(
        src.contains("vector::similarity::cosine"),
        "vector_search_symbols_native() must SELECT vector::similarity::cosine() \
         from SurrealDB so scores are DB-authoritative, not app-computed."
    );
}
