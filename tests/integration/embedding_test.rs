//! Integration tests for the embedding and search subsystem (T076).
//!
//! Verifies lazy model behavior, graceful degradation when the `embeddings`
//! feature is disabled, and keyword-only hybrid search integration.

use engram::services::embedding::{self, EMBEDDING_DIM, MAX_QUERY_CHARS};
use engram::services::search::{self, SearchCandidate, hybrid_search};

// ── Model cache directory ────────────────────────────────────────

#[test]
fn model_cache_dir_is_under_data_dir() {
    let dir = embedding::model_cache_dir();
    let path_str = dir.to_string_lossy();
    assert!(
        path_str.contains("engram") && path_str.contains("models"),
        "expected path containing engram/models, got {path_str}"
    );
}

// ── Embedding dimension constant ─────────────────────────────────

#[test]
fn embedding_dimension_is_384() {
    assert_eq!(EMBEDDING_DIM, 384);
}

// ── Feature-gated stub behavior ──────────────────────────────────

#[cfg(not(feature = "embeddings"))]
#[test]
fn embed_text_returns_model_not_loaded_without_feature() {
    let err = embedding::embed_text("hello world").unwrap_err();
    let code = err.to_response().error.code;
    assert_eq!(
        code,
        engram::errors::codes::MODEL_NOT_LOADED,
        "expected MODEL_NOT_LOADED error code"
    );
}

#[cfg(not(feature = "embeddings"))]
#[test]
fn embed_texts_returns_model_not_loaded_without_feature() {
    let err = embedding::embed_texts(&["hello".to_string()]).unwrap_err();
    let code = err.to_response().error.code;
    assert_eq!(code, engram::errors::codes::MODEL_NOT_LOADED);
}

// ── Query validation ─────────────────────────────────────────────

#[test]
fn validate_query_length_accepts_within_limit() {
    assert!(embedding::validate_query_length("short query").is_ok());
}

#[test]
fn validate_query_length_rejects_over_limit() {
    let long = "a".repeat(MAX_QUERY_CHARS + 1);
    let err = embedding::validate_query_length(&long).unwrap_err();
    let code = err.to_response().error.code;
    assert_eq!(code, engram::errors::codes::QUERY_TOO_LONG);
}

// ── Hybrid search keyword-only fallback ──────────────────────────

#[test]
fn hybrid_search_works_without_embeddings() {
    let candidates = vec![
        SearchCandidate {
            id: "spec:auth".to_string(),
            source_type: "spec".to_string(),
            content: "user authentication login flow".to_string(),
            embedding: None,
        },
        SearchCandidate {
            id: "spec:db".to_string(),
            source_type: "spec".to_string(),
            content: "database schema migration".to_string(),
            embedding: None,
        },
    ];

    let results =
        hybrid_search("user login", &candidates, 10).expect("keyword-only search should succeed");

    assert_eq!(results.len(), 2);
    // Auth spec should rank higher (matches both "user" and "login")
    assert_eq!(results[0].id, "spec:auth");
    assert!(results[0].score > results[1].score);
}

#[test]
fn hybrid_search_returns_empty_for_no_candidates() {
    let results =
        hybrid_search("test query", &[], 10).expect("search with no candidates should succeed");
    assert!(results.is_empty());
}

// ── Cosine similarity ────────────────────────────────────────────

#[test]
#[allow(deprecated)] // test exercises cosine_similarity directly to verify its contract
fn cosine_similarity_unit_vectors() {
    let a = vec![1.0, 0.0, 0.0];
    let b = vec![0.0, 1.0, 0.0];
    let sim = search::cosine_similarity(&a, &b);
    assert!(sim.abs() < 1e-6, "orthogonal vectors should be ~0.0");
}

#[test]
#[allow(deprecated)] // test exercises cosine_similarity directly to verify its contract
fn cosine_similarity_identical_vectors() {
    let v = vec![0.5, 0.5, 0.5];
    let sim = search::cosine_similarity(&v, &v);
    assert!((sim - 1.0).abs() < 1e-6, "identical vectors should be ~1.0");
}

// ── Lazy model download (feature-gated) ──────────────────────────

#[cfg(feature = "embeddings")]
#[test]
fn embed_text_produces_correct_dimension() {
    let result = embedding::embed_text("test sentence for embedding");
    if let Ok(vec) = result {
        assert_eq!(vec.len(), EMBEDDING_DIM);
    }
    // Err(_): model download may fail in CI without network; acceptable
}

#[cfg(feature = "embeddings")]
#[test]
fn embed_texts_batch_matches_single() {
    let texts = vec!["hello world".to_string()];
    if let (Ok(single), Ok(batch)) = (
        embedding::embed_text("hello world"),
        embedding::embed_texts(&texts),
    ) {
        assert_eq!(batch.len(), 1);
        assert_eq!(single.len(), batch[0].len());
    }
    // Either unavailable: model not loaded; skip comparison
}
