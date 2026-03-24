//! Integration tests for native KNN vector search on `content_record` (TASK-008.06).
//!
//! Verifies that `vector_search_content_native()` uses SurrealDB's MTREE index
//! on the `content_record` table to rank docs, specs, backlog entries, and other
//! content by cosine similarity, matching the behaviour of `vector_search_symbols_native`.

use engram::db::connect_db;
use engram::db::queries::CodeGraphQueries;
use engram::models::ContentRecord;

/// Create an isolated test DB and return a `CodeGraphQueries` handle.
async fn test_queries(label: &str) -> CodeGraphQueries {
    let branch = format!("test_content_knn_{label}_{}", std::process::id());
    let data_dir = std::env::temp_dir().join("engram-test");
    let db = connect_db(&data_dir, &branch).await.expect("connect_db");
    CodeGraphQueries::new(db)
}

/// Build a `ContentRecord` with a known 384-dim embedding and upsert it.
async fn insert_content_record_with_embedding(
    queries: &CodeGraphQueries,
    id: &str,
    content_type: &str,
    content: &str,
    embedding: Vec<f32>,
) {
    let record = ContentRecord {
        id: id.to_string(),
        content_type: content_type.to_string(),
        file_path: format!("docs/{id}.md"),
        content_hash: format!("hash_{id}"),
        content: content.to_string(),
        embedding: Some(embedding),
        source_path: "docs".to_string(),
        file_size_bytes: content.len() as u64,
        ingested_at: chrono::Utc::now(),
    };
    queries
        .upsert_content_record(&record)
        .await
        .expect("upsert_content_record");
}

/// Generate a 384-dim unit vector with a dominant `1.0` at `dim_index`.
fn unit_vector(dim_index: usize) -> Vec<f32> {
    let mut v = vec![0.0_f32; 384];
    v[dim_index] = 1.0;
    v
}

#[tokio::test]
async fn content_knn_returns_top_k_ordered_by_similarity() {
    // GIVEN a DB with 3 content records having distinct embeddings
    let q = test_queries("top_k").await;
    insert_content_record_with_embedding(&q, "close", "spec", "spec content close", unit_vector(0))
        .await;
    insert_content_record_with_embedding(
        &q,
        "medium",
        "spec",
        "spec content medium",
        unit_vector(1),
    )
    .await;
    insert_content_record_with_embedding(&q, "far", "spec", "spec content far", unit_vector(2))
        .await;

    // WHEN we search with a query embedding most similar to "close"
    let query_emb = unit_vector(0);
    let results = q
        .vector_search_content_native(&query_emb, 2, None)
        .await
        .expect("content knn search");

    // THEN we get at most 2 results, with the closest first
    assert!(
        results.len() <= 2,
        "should return at most limit=2, got {}",
        results.len()
    );
    assert!(
        !results.is_empty(),
        "should return at least 1 result for a matching embedding"
    );
    assert_eq!(
        results[0].1.id, "close",
        "first result should be the most similar record, got '{}'",
        results[0].1.id
    );
}

#[tokio::test]
async fn content_knn_score_matches_cosine_similarity() {
    // GIVEN two records: one whose embedding exactly matches the query
    let q = test_queries("cosine").await;
    let emb_exact = unit_vector(0);
    let emb_partial = {
        let mut v = vec![0.0_f32; 384];
        v[0] = 0.7;
        v[1] = 0.7;
        v
    };
    insert_content_record_with_embedding(&q, "exact", "docs", "exact match", emb_exact.clone())
        .await;
    insert_content_record_with_embedding(&q, "partial", "docs", "partial match", emb_partial)
        .await;

    // WHEN we search with query = emb_exact
    let results = q
        .vector_search_content_native(&emb_exact, 10, None)
        .await
        .expect("content knn search");

    // THEN the exact-match record leads with score ~1.0
    assert!(!results.is_empty());
    let (score, rec) = &results[0];
    assert_eq!(rec.id, "exact");
    assert!(
        (*score - 1.0).abs() < 0.01,
        "exact match should have score ~1.0, got {score}"
    );
}

#[tokio::test]
async fn content_knn_empty_table_returns_empty() {
    // GIVEN an empty database
    let q = test_queries("empty").await;

    // WHEN we search for any embedding
    let results = q
        .vector_search_content_native(&unit_vector(0), 5, None)
        .await
        .expect("content knn on empty table");

    // THEN no results are returned
    assert!(results.is_empty(), "empty table should return no results");
}

#[tokio::test]
async fn content_knn_content_type_filter_excludes_other_types() {
    // GIVEN records of two different content_types
    let q = test_queries("filter").await;
    insert_content_record_with_embedding(&q, "spec_a", "spec", "spec a", unit_vector(0)).await;
    insert_content_record_with_embedding(
        &q,
        "backlog_a",
        "backlog",
        "backlog a",
        unit_vector(0),
    )
    .await;

    // WHEN we search restricted to "spec"
    let results = q
        .vector_search_content_native(&unit_vector(0), 10, Some("spec"))
        .await
        .expect("filtered content knn");

    // THEN only spec records are returned
    assert!(
        !results.is_empty(),
        "should return the spec record"
    );
    for (_, rec) in &results {
        assert_eq!(
            rec.content_type, "spec",
            "filter should exclude non-spec records, got type '{}'",
            rec.content_type
        );
    }
}

#[tokio::test]
async fn content_knn_respects_limit() {
    // GIVEN 5 records all with the same embedding
    let q = test_queries("limit").await;
    for i in 0..5 {
        insert_content_record_with_embedding(
            &q,
            &format!("rec{i}"),
            "memory",
            &format!("record {i}"),
            unit_vector(0),
        )
        .await;
    }

    // WHEN we search with limit=3
    let results = q
        .vector_search_content_native(&unit_vector(0), 3, None)
        .await
        .expect("limited content knn");

    // THEN at most 3 results are returned
    assert!(
        results.len() <= 3,
        "result count {} exceeds limit 3",
        results.len()
    );
}
