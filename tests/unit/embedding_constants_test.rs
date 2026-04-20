//! Unit tests for embedding constants (Task 001.003.001-T — U2.8).
//!
//! Verifies that `EMBEDDING_MODEL` exists, is non-empty, and names the
//! expected model so that schema bootstrap and metadata persistence can
//! reference a stable provenance string.

use engram::services::embedding::{EMBEDDING_DIM, EMBEDDING_MODEL};

#[test]
fn embedding_model_constant_names_bge_small() {
    assert!(
        EMBEDDING_MODEL.contains("bge-small"),
        "EMBEDDING_MODEL must reference bge-small-en-v1.5, got: '{EMBEDDING_MODEL}'"
    );
}

#[test]
fn embedding_model_constant_is_not_empty() {
    assert!(
        !EMBEDDING_MODEL.is_empty(),
        "EMBEDDING_MODEL must be a non-empty string"
    );
}

#[test]
fn embedding_dim_is_384() {
    // Confirm the existing constant aligns with the model's native output size.
    assert_eq!(EMBEDDING_DIM, 384, "bge-small-en-v1.5 outputs 384 dimensions");
}
