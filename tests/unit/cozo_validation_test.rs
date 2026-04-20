//! Unit tests for CozoDB embedding validation (Task 001.003.008-T — U2.7).
//!
//! Verifies that `validate_cozo_embedding` rejects:
//! - empty IDs
//! - dimension mismatches
//! - vectors containing NaN
//! - vectors containing Inf
//!
//! These tests are backend-agnostic (no `cozo-backend` feature required)
//! because validation is pure logic with no database dependency.

use engram::services::cozo_validation::validate_cozo_embedding;
use engram::services::embedding::EMBEDDING_DIM;

#[test]
fn rejects_empty_id() {
    let embedding = vec![0.0_f32; EMBEDDING_DIM];
    let result = validate_cozo_embedding(&embedding, EMBEDDING_DIM, "");
    assert!(result.is_err(), "empty ID must be rejected");
}

#[test]
fn rejects_dimension_mismatch() {
    let wrong_dim = EMBEDDING_DIM + 1;
    let embedding = vec![0.0_f32; wrong_dim];
    let result = validate_cozo_embedding(&embedding, EMBEDDING_DIM, "fn:test-id");
    assert!(
        result.is_err(),
        "embedding length {wrong_dim} must be rejected when expected {EMBEDDING_DIM}"
    );
}

#[test]
fn rejects_nan_in_embedding() {
    let mut embedding = vec![0.0_f32; EMBEDDING_DIM];
    embedding[0] = f32::NAN;
    let result = validate_cozo_embedding(&embedding, EMBEDDING_DIM, "fn:test-id");
    assert!(result.is_err(), "NaN in embedding must be rejected");
}

#[test]
fn rejects_inf_in_embedding() {
    let mut embedding = vec![0.0_f32; EMBEDDING_DIM];
    embedding[EMBEDDING_DIM / 2] = f32::INFINITY;
    let result = validate_cozo_embedding(&embedding, EMBEDDING_DIM, "fn:test-id");
    assert!(result.is_err(), "Inf in embedding must be rejected");
}

#[test]
fn rejects_neg_inf_in_embedding() {
    let mut embedding = vec![0.0_f32; EMBEDDING_DIM];
    embedding[1] = f32::NEG_INFINITY;
    let result = validate_cozo_embedding(&embedding, EMBEDDING_DIM, "class:test-id");
    assert!(result.is_err(), "negative Inf in embedding must be rejected");
}

#[test]
fn accepts_valid_embedding() {
    let embedding = vec![0.1_f32; EMBEDDING_DIM];
    let result = validate_cozo_embedding(&embedding, EMBEDDING_DIM, "fn:valid-id");
    assert!(result.is_ok(), "valid embedding must be accepted: {result:?}");
}
