//! Input validation for CozoDB ingestion.
//!
//! Phase 2 (U2.7) enforces NaN/Inf checks, dimensionality guards, and
//! empty-ID rejection at the ingestion boundary before any vector is
//! persisted to the database.

use crate::errors::EngramError;

/// Validate an embedding vector before ingestion into CozoDB.
///
/// Checks performed:
/// - `id` must not be empty.
/// - `embedding.len()` must equal `expected_dim`.
/// - No element of `embedding` may be `NaN` or infinite.
///
/// # Errors
/// Returns an [`EngramError`] when any check fails.
pub fn validate_cozo_embedding(
    _embedding: &[f32],
    _expected_dim: usize,
    _id: &str,
) -> Result<(), EngramError> {
    unimplemented!("Worker: validate_cozo_embedding not implemented (Phase 2 U2.7)")
}
