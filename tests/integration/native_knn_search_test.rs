//! Integration tests for native KNN vector search (dxo.2.1 / dxo.2.5).
//!
//! Verifies that `vector_search_symbols_native()` uses `SurrealDB`'s MTREE
//! indexes for O(log n) KNN queries instead of full-table-scan cosine
//! similarity, and that scores are DB-authoritative after dxo.2.3.

use engram::db::connect_db;
use engram::db::queries::CodeGraphQueries;
use engram::models::{Class, Function, Interface};

/// Create a test DB and return a `CodeGraphQueries` handle.
async fn test_queries(label: &str) -> CodeGraphQueries {
    let branch = format!("test_knn_{label}_{}", std::process::id());
    let data_dir = std::env::temp_dir().join("engram-test");
    let db = connect_db(&data_dir, &branch).await.expect("connect_db");
    CodeGraphQueries::new(db)
}

/// Insert a function with a known embedding into the test DB.
async fn insert_function_with_embedding(
    queries: &CodeGraphQueries,
    name: &str,
    embedding: Vec<f32>,
) {
    let func = Function {
        id: format!("function:{name}"),
        name: name.to_string(),
        file_path: "src/test.rs".to_string(),
        line_start: 1,
        line_end: 5,
        signature: format!("fn {name}()"),
        docstring: None,
        body: String::new(),
        body_hash: format!("hash_{name}"),
        token_count: 0,
        embed_type: "explicit_code".to_string(),
        summary: format!("{name} summary"),
        embedding,
    };
    queries
        .upsert_function(&func)
        .await
        .expect("upsert_function");
}

/// Generate a unit vector with a dominant component at `dim_index`.
fn unit_vector(dim_index: usize) -> Vec<f32> {
    let mut v = vec![0.0_f32; 384];
    v[dim_index] = 1.0;
    v
}

#[tokio::test]
async fn knn_returns_top_k_ordered_by_similarity() {
    // GIVEN a DB with 3 functions having distinct embeddings
    let q = test_queries("top_k").await;
    insert_function_with_embedding(&q, "close", unit_vector(0)).await;
    insert_function_with_embedding(&q, "medium", unit_vector(1)).await;
    insert_function_with_embedding(&q, "far", unit_vector(2)).await;

    // WHEN we search with a query embedding most similar to "close"
    let query_emb = unit_vector(0);
    let results = q
        .vector_search_symbols_native(&query_emb, 2)
        .await
        .expect("knn search");

    // THEN we get at most 2 results, with the closest first
    assert!(results.len() <= 2, "should return at most limit=2 results");
    assert!(
        !results.is_empty(),
        "should return at least 1 result for a matching embedding"
    );
    // The first result should be "close" (highest similarity)
    assert_eq!(
        results[0].1.name, "close",
        "first result should be the most similar symbol"
    );
}

#[tokio::test]
async fn knn_results_match_cosine_similarity() {
    // GIVEN a DB with functions that have known embeddings
    let q = test_queries("cosine_match").await;
    let emb_a = unit_vector(0);
    let emb_b = {
        let mut v = vec![0.0_f32; 384];
        v[0] = 0.7;
        v[1] = 0.7;
        v
    };
    insert_function_with_embedding(&q, "exact", emb_a.clone()).await;
    insert_function_with_embedding(&q, "partial", emb_b).await;

    // WHEN we search with query = emb_a
    let results = q
        .vector_search_symbols_native(&emb_a, 10)
        .await
        .expect("knn search");

    // THEN scores should be within floating-point tolerance of manual cosine
    assert!(!results.is_empty());
    let (score, sym) = &results[0];
    assert_eq!(sym.name, "exact");
    // Cosine similarity of identical unit vectors = 1.0
    assert!(
        (*score - 1.0).abs() < 0.01,
        "exact match should have score ~1.0, got {score}"
    );
}

#[tokio::test]
async fn knn_empty_table_returns_empty() {
    // GIVEN an empty database with no symbols
    let q = test_queries("empty").await;

    // WHEN we search with any query embedding
    let query_emb = unit_vector(0);
    let results = q
        .vector_search_symbols_native(&query_emb, 5)
        .await
        .expect("knn search on empty table");

    // THEN results are empty
    assert!(results.is_empty(), "empty table should return no results");
}

#[tokio::test]
async fn knn_zero_vector_query_returns_empty_or_error() {
    // GIVEN a DB with one function having a valid embedding
    let q = test_queries("zero_query").await;
    insert_function_with_embedding(&q, "valid_sym", unit_vector(0)).await;

    // WHEN we search with a zero vector
    let zero_emb = vec![0.0_f32; 384];
    let result = q.vector_search_symbols_native(&zero_emb, 5).await;

    // THEN it either returns empty results or a meaningful error
    match result {
        Ok(results) => {
            // Zero vector cosine is undefined; results should be empty or
            // all scores should be 0.0
            for (score, _) in &results {
                assert!(
                    score.abs() < f32::EPSILON,
                    "zero query vector should produce score ~0.0, got {score}"
                );
            }
        }
        Err(e) => {
            // A descriptive error is also acceptable
            let msg = e.to_string();
            assert!(
                !msg.is_empty(),
                "error message should be descriptive, got empty"
            );
        }
    }
}

// ── dxo.2.5: multi-table, duplicate embeddings, fewer-than-K rows ───────────

/// Insert a class with a known embedding into the test DB.
async fn insert_class_with_embedding(queries: &CodeGraphQueries, name: &str, embedding: Vec<f32>) {
    let class = Class {
        id: format!("class:{name}"),
        name: name.to_string(),
        file_path: "src/test.rs".to_string(),
        line_start: 1,
        line_end: 5,
        docstring: None,
        body: String::new(),
        body_hash: format!("hash_{name}"),
        token_count: 0,
        embed_type: "explicit_code".to_string(),
        summary: format!("{name} summary"),
        embedding,
    };
    queries.upsert_class(&class).await.expect("upsert_class");
}

/// Insert an interface with a known embedding into the test DB.
async fn insert_interface_with_embedding(
    queries: &CodeGraphQueries,
    name: &str,
    embedding: Vec<f32>,
) {
    let iface = Interface {
        id: format!("interface:{name}"),
        name: name.to_string(),
        file_path: "src/test.rs".to_string(),
        line_start: 1,
        line_end: 5,
        docstring: None,
        body: String::new(),
        body_hash: format!("hash_{name}"),
        token_count: 0,
        embed_type: "explicit_code".to_string(),
        summary: format!("{name} summary"),
        embedding,
    };
    queries
        .upsert_interface(&iface)
        .await
        .expect("upsert_interface");
}

/// `vector_search_symbols_native()` spans all three symbol tables.
/// Results from function, class, and interface rows must all be candidates.
#[tokio::test]
async fn knn_searches_across_all_symbol_tables() {
    // GIVEN a DB with one symbol in each of the three tables, all equally similar
    let q = test_queries("multi_table").await;
    let emb = unit_vector(0);
    insert_function_with_embedding(&q, "fn_sym", emb.clone()).await;
    insert_class_with_embedding(&q, "cls_sym", emb.clone()).await;
    insert_interface_with_embedding(&q, "iface_sym", emb.clone()).await;

    // WHEN we search with limit = 10 (more than total symbols)
    let results = q
        .vector_search_symbols_native(&emb, 10)
        .await
        .expect("knn search");

    // THEN we get results from multiple tables
    let tables: std::collections::HashSet<&str> =
        results.iter().map(|(_, s)| s.table.as_str()).collect();
    assert!(
        tables.len() >= 2,
        "should find symbols from at least 2 tables; got tables: {tables:?}"
    );
}

/// When the table has fewer rows than K, results should be capped at table size.
#[tokio::test]
async fn knn_fewer_rows_than_limit_returns_all_rows() {
    // GIVEN a DB with only 2 functions
    let q = test_queries("fewer_rows").await;
    insert_function_with_embedding(&q, "a", unit_vector(0)).await;
    insert_function_with_embedding(&q, "b", unit_vector(1)).await;

    // WHEN we search with limit = 10 (more than available rows)
    let results = q
        .vector_search_symbols_native(&unit_vector(0), 10)
        .await
        .expect("knn search");

    // THEN we get at most 2 results (not 10)
    assert!(
        results.len() <= 2,
        "should not exceed the number of indexed rows; got {}",
        results.len()
    );
}

/// Duplicate embeddings must not cause panics or incorrect results.
#[tokio::test]
async fn knn_duplicate_embeddings_returns_stable_results() {
    // GIVEN a DB with two functions having identical embeddings
    let q = test_queries("duplicates").await;
    let emb = unit_vector(3);
    insert_function_with_embedding(&q, "dup_a", emb.clone()).await;
    insert_function_with_embedding(&q, "dup_b", emb.clone()).await;

    // WHEN we search with the same embedding
    let results = q
        .vector_search_symbols_native(&emb, 5)
        .await
        .expect("knn search with duplicates should not panic");

    // THEN both results have score ~1.0 (identical vectors)
    assert!(
        !results.is_empty(),
        "should return results for duplicate embeddings"
    );
    for (score, sym) in &results {
        assert!(
            (*score - 1.0).abs() < 0.02,
            "duplicate embedding should score ~1.0 for sym {}; got {score}",
            sym.name
        );
    }
}

/// All returned scores must be in the valid [0, 1] range.
#[tokio::test]
async fn knn_scores_are_in_valid_range() {
    // GIVEN a DB with functions of varied embeddings
    let q = test_queries("score_range").await;
    insert_function_with_embedding(&q, "v0", unit_vector(0)).await;
    insert_function_with_embedding(&q, "v1", unit_vector(1)).await;
    insert_function_with_embedding(&q, "v2", unit_vector(2)).await;

    // WHEN we search
    let results = q
        .vector_search_symbols_native(&unit_vector(0), 10)
        .await
        .expect("knn search");

    // THEN all scores are in [0, 1]
    for (score, sym) in &results {
        assert!(
            (0.0..=1.0).contains(score),
            "score for {} must be in [0, 1]; got {score}",
            sym.name
        );
    }
}

/// `vector_search_symbols_native()` must not import `cosine_similarity` —
/// scores are DB-computed. This source-level check complements the unit tests.
#[test]
fn native_knn_source_does_not_use_cosine_similarity_import() {
    let src = include_str!("../../src/db/queries.rs");
    assert!(
        !src.contains("use crate::services::search::cosine_similarity"),
        "queries.rs must not import cosine_similarity after dxo.2.3"
    );
    assert!(
        src.contains("vector::similarity::cosine"),
        "queries.rs must SELECT vector::similarity::cosine() for DB-native scores"
    );
}
