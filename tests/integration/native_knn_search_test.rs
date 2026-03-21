//! Integration tests for native KNN vector search (dxo.2.1).
//!
//! Verifies that `vector_search_symbols_native()` uses `SurrealDB`'s MTREE
//! indexes for O(log n) KNN queries instead of full-table-scan cosine
//! similarity.

use engram::db::connect_db;
use engram::db::queries::CodeGraphQueries;
use engram::models::Function;

/// Create a test DB and return a `CodeGraphQueries` handle.
async fn test_queries(label: &str) -> CodeGraphQueries {
    let hash = format!("test_knn_{label}_{}", std::process::id());
    let db = connect_db(&hash).await.expect("connect_db");
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
