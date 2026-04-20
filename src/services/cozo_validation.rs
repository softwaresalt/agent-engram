//! Input validation for CozoDB ingestion.
//!
//! Phase 2 (U2.7) enforces NaN/Inf checks, dimensionality guards, and
//! empty-ID rejection at the ingestion boundary before any vector is
//! persisted to the database.

use crate::errors::{EngramError, SystemError};

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
    embedding: &[f32],
    expected_dim: usize,
    id: &str,
) -> Result<(), EngramError> {
    if id.is_empty() {
        return Err(EngramError::System(SystemError::InvalidParams {
            reason: "embedding ID must not be empty".into(),
        }));
    }

    if embedding.len() != expected_dim {
        return Err(EngramError::System(SystemError::InvalidParams {
            reason: format!(
                "embedding dimension mismatch: expected {expected_dim}, got {}",
                embedding.len()
            ),
        }));
    }

    for (i, &v) in embedding.iter().enumerate() {
        if v.is_nan() {
            return Err(EngramError::System(SystemError::InvalidParams {
                reason: format!("embedding[{i}] is NaN"),
            }));
        }
        if v.is_infinite() {
            return Err(EngramError::System(SystemError::InvalidParams {
                reason: format!("embedding[{i}] is infinite ({v})"),
            }));
        }
    }

    Ok(())
}
