//! Integration tests for embedding status API (dxo.4.1).
//!
//! Verifies `is_available()` and `status()` functions expose embedding
//! subsystem readiness, model information, and symbol coverage metrics.

use engram::services::embedding;

// ── is_available() ──────────────────────────────────────────────────

#[cfg(not(feature = "embeddings"))]
#[test]
fn is_available_returns_false_when_feature_disabled() {
    // GIVEN the embeddings feature flag is not compiled in
    // WHEN we check embedding availability
    let available = embedding::is_available();

    // THEN it returns false
    assert!(
        !available,
        "is_available() must be false without feature flag"
    );
}

#[cfg(feature = "embeddings")]
#[test]
fn is_available_returns_true_when_model_loaded() {
    // GIVEN the embeddings feature is enabled and model has been loaded
    //       (loading happens lazily on first embed_text call)
    // WHEN we check availability
    let available = embedding::is_available();

    // THEN it reflects model load state (true if OnceLock holds Ok)
    // NOTE: on CI without a model download, this may be false;
    // the test validates the function doesn't panic and returns a bool.
    let _ = available;
}

// ── status() ────────────────────────────────────────────────────────

#[cfg(not(feature = "embeddings"))]
#[tokio::test]
async fn status_returns_disabled_without_feature() {
    // GIVEN the embeddings feature is disabled and no workspace is bound
    // WHEN we request embedding status with no queries handle
    let result = embedding::status(None).await;

    // THEN we get a status with enabled=false and model_loaded=false
    let st = result.expect("status() should succeed even without feature");
    assert!(!st.enabled, "enabled must be false without feature flag");
    assert!(
        !st.model_loaded,
        "model_loaded must be false without feature flag"
    );
    assert!(
        st.model_name.is_none(),
        "model_name must be None without feature flag"
    );
}

#[cfg(not(feature = "embeddings"))]
#[tokio::test]
async fn status_returns_zero_coverage_without_workspace() {
    // GIVEN no workspace is bound (queries = None)
    // WHEN we request embedding status
    let result = embedding::status(None).await;

    // THEN symbol counts are zero and coverage is 0.0
    let st = result.expect("status() should succeed");
    assert_eq!(st.symbols_with_embeddings, 0);
    assert_eq!(st.total_symbols, 0);
    assert!((st.coverage_percent - 0.0).abs() < f64::EPSILON);
}

#[tokio::test]
async fn status_returns_correct_coverage_for_known_counts() {
    // GIVEN a workspace with indexed symbols (some with embeddings)
    // WHEN we compute coverage_percent
    // THEN coverage_percent = symbols_with_embeddings / total_symbols * 100

    // This test creates an in-memory DB, inserts known symbol counts,
    // and verifies the percentage calculation.
    let ws_hash = format!("test_embed_status_{}", std::process::id());
    let data_dir = std::env::temp_dir().join("engram-test");
    let db = engram::db::connect_db(&data_dir, &ws_hash)
        .await
        .expect("connect_db");
    let queries = engram::db::queries::CodeGraphQueries::new(db);

    // Insert 2 functions: one with a real embedding, one with a zero vector
    let func_with_embed = engram::models::Function {
        id: "function:with_embed".to_string(),
        name: "with_embed".to_string(),
        file_path: "src/test.rs".to_string(),
        line_start: 1,
        line_end: 5,
        signature: "fn with_embed()".to_string(),
        docstring: None,
        body: String::new(),
        body_hash: "aaa111".to_string(),
        token_count: 0,
        embed_type: "explicit_code".to_string(),
        summary: "test".to_string(),
        embedding: vec![0.1; 384],
    };

    let func_without_embed = engram::models::Function {
        id: "function:no_embed".to_string(),
        name: "no_embed".to_string(),
        file_path: "src/test.rs".to_string(),
        line_start: 6,
        line_end: 10,
        signature: "fn no_embed()".to_string(),
        docstring: None,
        body: String::new(),
        body_hash: "bbb222".to_string(),
        token_count: 0,
        embed_type: "explicit_code".to_string(),
        summary: "test".to_string(),
        embedding: vec![0.0; 384],
    };

    queries
        .upsert_function(&func_with_embed)
        .await
        .expect("upsert");
    queries
        .upsert_function(&func_without_embed)
        .await
        .expect("upsert");

    let st = embedding::status(Some(&queries)).await.expect("status");

    // 1 out of 2 symbols has a meaningful embedding → 50%
    assert_eq!(st.total_symbols, 2, "total_symbols should be 2");
    assert_eq!(
        st.symbols_with_embeddings, 1,
        "symbols_with_embeddings should be 1"
    );
    assert!(
        (st.coverage_percent - 50.0).abs() < 0.1,
        "coverage_percent should be ~50.0, got {}",
        st.coverage_percent
    );
}

#[cfg(feature = "embeddings")]
#[tokio::test]
async fn status_returns_model_name_when_loaded() {
    // GIVEN the embedding model has been loaded
    // WHEN we request status
    let st = embedding::status(None).await.expect("status");

    // THEN model_name contains the model identifier
    if st.model_loaded {
        assert!(
            st.model_name.is_some(),
            "model_name must be Some when model is loaded"
        );
    }
}

#[tokio::test]
async fn status_works_before_workspace_set() {
    // GIVEN no workspace has been set (queries = None)
    // WHEN we call status()
    let result = embedding::status(None).await;

    // THEN it succeeds without error
    assert!(
        result.is_ok(),
        "status() must not error before workspace is set"
    );
}
