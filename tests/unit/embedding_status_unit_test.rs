//! Unit tests for the embedding status API (dxo.4.4).
//!
//! Tests `is_available()`, `status()`, `compute_coverage()`, and
//! `has_meaningful_embedding()` in isolation without a live database.

#[cfg(not(feature = "embeddings"))]
use engram::services::embedding::is_available;
use engram::services::embedding::{EmbeddingStatus, compute_coverage, has_meaningful_embedding};

// ── is_available() ────────────────────────────────────────────────────

#[cfg(not(feature = "embeddings"))]
#[test]
fn is_available_false_without_feature_flag() {
    // GIVEN the embeddings feature is not compiled in
    // WHEN we check availability
    let result = is_available();
    // THEN it is always false
    assert!(!result);
}

// ── has_meaningful_embedding() ────────────────────────────────────────

#[test]
fn meaningful_embedding_empty_vec_is_false() {
    // GIVEN an empty embedding vector
    // WHEN we check if it is meaningful
    // THEN it returns false (no information content)
    assert!(!has_meaningful_embedding(&[]));
}

#[test]
fn meaningful_embedding_zero_vec_is_false() {
    // GIVEN an all-zero embedding vector (placeholder)
    let zeros = vec![0.0_f32; 384];
    // WHEN we check if it is meaningful
    // THEN it returns false (zero vectors carry no semantic signal)
    assert!(!has_meaningful_embedding(&zeros));
}

#[test]
fn meaningful_embedding_nonzero_vec_is_true() {
    // GIVEN an embedding vector with at least one non-zero component
    let mut v = vec![0.0_f32; 384];
    v[0] = 0.1;
    // WHEN we check if it is meaningful
    // THEN it returns true
    assert!(has_meaningful_embedding(&v));
}

#[test]
fn meaningful_embedding_tiny_value_near_epsilon_is_true() {
    // GIVEN a vector where only the smallest non-epsilon value is set
    let mut v = vec![0.0_f32; 4];
    v[2] = f32::EPSILON * 2.0;
    // WHEN we check
    // THEN values > EPSILON are considered meaningful
    assert!(has_meaningful_embedding(&v));
}

#[test]
fn meaningful_embedding_exactly_epsilon_is_false() {
    // GIVEN a vector where the only non-zero value equals exactly EPSILON
    let mut v = vec![0.0_f32; 4];
    v[2] = f32::EPSILON;
    // WHEN we check (comparison is strict >)
    // THEN values == EPSILON are not considered meaningful
    assert!(!has_meaningful_embedding(&v));
}

// ── compute_coverage() ────────────────────────────────────────────────

#[test]
fn coverage_zero_total_returns_zero() {
    // GIVEN no symbols in the database
    // WHEN we compute coverage
    // THEN we get 0.0 (not NaN or div-by-zero panic)
    let result = compute_coverage(0, 0);
    assert!((result - 0.0).abs() < f64::EPSILON);
}

#[test]
fn coverage_all_with_embeddings_is_100() {
    // GIVEN 5 symbols all with meaningful embeddings
    let result = compute_coverage(5, 5);
    // WHEN we compute coverage
    // THEN we get exactly 100.0
    assert!((result - 100.0).abs() < f64::EPSILON);
}

#[test]
fn coverage_none_with_embeddings_is_zero() {
    // GIVEN 10 symbols, none with meaningful embeddings
    let result = compute_coverage(0, 10);
    // WHEN we compute coverage
    // THEN we get exactly 0.0
    assert!((result - 0.0).abs() < f64::EPSILON);
}

#[test]
fn coverage_half_with_embeddings_is_50() {
    // GIVEN 2 out of 4 symbols have embeddings
    let result = compute_coverage(2, 4);
    // WHEN we compute coverage
    // THEN we get 50.0
    assert!((result - 50.0).abs() < 0.001);
}

#[test]
fn coverage_one_of_three_is_33_percent() {
    // GIVEN 1 out of 3 symbols has an embedding
    let result = compute_coverage(1, 3);
    // WHEN we compute coverage
    // THEN we get ~33.33%
    assert!((result - 33.333_333).abs() < 0.001);
}

// ── EmbeddingStatus struct ────────────────────────────────────────────

#[test]
fn embedding_status_clone_produces_equal_values() {
    // GIVEN an EmbeddingStatus value
    let original = EmbeddingStatus {
        enabled: false,
        model_loaded: false,
        model_name: Some("test-model".to_string()),
        symbols_with_embeddings: 3,
        total_symbols: 10,
        coverage_percent: 30.0,
    };
    // WHEN we clone it
    let cloned = original.clone();
    // THEN all fields match
    assert_eq!(cloned.enabled, original.enabled);
    assert_eq!(cloned.model_loaded, original.model_loaded);
    assert_eq!(cloned.model_name, original.model_name);
    assert_eq!(
        cloned.symbols_with_embeddings,
        original.symbols_with_embeddings
    );
    assert_eq!(cloned.total_symbols, original.total_symbols);
    assert!((cloned.coverage_percent - original.coverage_percent).abs() < f64::EPSILON);
}

#[test]
fn embedding_status_debug_format_contains_key_fields() {
    // GIVEN an EmbeddingStatus
    let status = EmbeddingStatus {
        enabled: true,
        model_loaded: false,
        model_name: None,
        symbols_with_embeddings: 0,
        total_symbols: 5,
        coverage_percent: 0.0,
    };
    // WHEN we format with Debug
    let debug_str = format!("{status:?}");
    // THEN key field names appear in the output
    assert!(debug_str.contains("enabled"));
    assert!(debug_str.contains("model_loaded"));
    assert!(debug_str.contains("coverage_percent"));
}

#[test]
fn embedding_status_serializes_to_json() {
    // GIVEN a fully populated EmbeddingStatus
    let status = EmbeddingStatus {
        enabled: false,
        model_loaded: false,
        model_name: None,
        symbols_with_embeddings: 2,
        total_symbols: 4,
        coverage_percent: 50.0,
    };
    // WHEN we serialize to JSON
    let json = serde_json::to_string(&status).expect("serialize");
    // THEN key fields appear in the JSON output
    assert!(json.contains("\"enabled\""));
    assert!(json.contains("\"model_loaded\""));
    assert!(json.contains("\"coverage_percent\""));
    assert!(json.contains("50.0"));
}

#[test]
fn embedding_status_deserializes_from_json() {
    // GIVEN a JSON representation of EmbeddingStatus
    let json = r#"{
        "enabled": false,
        "model_loaded": false,
        "model_name": "bge-small-en-v1.5",
        "symbols_with_embeddings": 10,
        "total_symbols": 20,
        "coverage_percent": 50.0
    }"#;
    // WHEN we deserialize
    let status: EmbeddingStatus = serde_json::from_str(json).expect("deserialize");
    // THEN the fields are populated correctly
    assert!(!status.enabled);
    assert_eq!(status.model_name.as_deref(), Some("bge-small-en-v1.5"));
    assert_eq!(status.symbols_with_embeddings, 10);
    assert_eq!(status.total_symbols, 20);
    assert!((status.coverage_percent - 50.0).abs() < f64::EPSILON);
}

// ── status(None) async ────────────────────────────────────────────────

#[cfg(not(feature = "embeddings"))]
#[test]
fn status_none_returns_disabled_with_zero_coverage() {
    // GIVEN no workspace and embeddings disabled
    // WHEN we call status(None) synchronously via block_on
    let result = tokio_test::block_on(engram::services::embedding::status(None));
    // THEN we get a valid EmbeddingStatus with disabled fields
    let st = result.expect("status should succeed without workspace");
    assert!(!st.enabled);
    assert!(!st.model_loaded);
    assert!(st.model_name.is_none());
    assert_eq!(st.symbols_with_embeddings, 0);
    assert_eq!(st.total_symbols, 0);
    assert!((st.coverage_percent - 0.0).abs() < f64::EPSILON);
}
