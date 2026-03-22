//! Embedding generation service for semantic search.
//!
//! Wraps `fastembed-rs` behind the `embeddings` feature flag. When the
//! feature is disabled, all calls return `Err(QueryError::ModelNotLoaded)`.
//!
//! The model (`bge-small-en-v1.5`, 384 dimensions) is lazily downloaded
//! on first use and cached under `~/.local/share/engram/models/`.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::errors::{EngramError, QueryError};

/// Embedding vector dimension for `bge-small-en-v1.5`.
pub const EMBEDDING_DIM: usize = 384;

/// Maximum query length in characters (rough proxy for 500 tokens).
pub const MAX_QUERY_CHARS: usize = 2000;

/// Return the model cache directory, creating it if needed.
#[must_use]
pub fn model_cache_dir() -> PathBuf {
    let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("engram").join("models")
}

/// Global, lazily-initialised embedding model.
///
/// `OnceLock` guarantees the model is loaded exactly once; subsequent calls
/// return the cached handle.
#[cfg(feature = "embeddings")]
static MODEL: std::sync::OnceLock<Result<fastembed::TextEmbedding, String>> =
    std::sync::OnceLock::new();

/// Initialise (or return) the cached embedding model.
///
/// # Errors
/// Returns `QueryError::ModelNotLoaded` when:
/// - The `embeddings` feature is disabled.
/// - The ONNX model fails to download or load.
#[cfg(feature = "embeddings")]
fn get_model() -> Result<&'static fastembed::TextEmbedding, EngramError> {
    let result = MODEL.get_or_init(|| {
        let cache = model_cache_dir();
        std::fs::create_dir_all(&cache).ok();

        let options = fastembed::TextInitOptions::new(fastembed::EmbeddingModel::BGESmallENV15)
            .with_cache_dir(cache)
            .with_show_download_progress(true);

        fastembed::TextEmbedding::try_new(options).map_err(|e| e.to_string())
    });

    match result {
        Ok(model) => Ok(model),
        Err(reason) => {
            tracing::error!(reason = %reason, "embedding model initialisation failed");
            Err(EngramError::System(
                crate::errors::SystemError::ModelLoadFailed {
                    reason: reason.clone(),
                },
            ))
        }
    }
}

/// Generate an embedding vector for a single piece of text.
///
/// # Errors
/// - `QueryError::ModelNotLoaded` when the model cannot be initialised.
/// - `QueryError::SearchFailed` when generation itself fails.
#[cfg(feature = "embeddings")]
pub fn embed_text(text: &str) -> Result<Vec<f32>, EngramError> {
    let model = get_model()?;
    let embeddings = model.embed(vec![text.to_string()], None).map_err(|e| {
        EngramError::Query(QueryError::SearchFailed {
            reason: e.to_string(),
        })
    })?;

    embeddings.into_iter().next().ok_or_else(|| {
        EngramError::Query(QueryError::SearchFailed {
            reason: "model returned no embeddings".to_string(),
        })
    })
}

/// Batch-embed multiple texts in one call for efficiency.
///
/// # Errors
/// Same as [`embed_text`].
#[cfg(feature = "embeddings")]
pub fn embed_texts(texts: &[String]) -> Result<Vec<Vec<f32>>, EngramError> {
    if texts.is_empty() {
        return Ok(Vec::new());
    }
    let model = get_model()?;
    model.embed(texts.to_vec(), None).map_err(|e| {
        EngramError::Query(QueryError::SearchFailed {
            reason: e.to_string(),
        })
    })
}

// ── Feature-gated stubs ─────────────────────────────────────────────

/// Stub: always returns `ModelNotLoaded` when the `embeddings` feature is off.
#[cfg(not(feature = "embeddings"))]
pub fn embed_text(_text: &str) -> Result<Vec<f32>, EngramError> {
    Err(EngramError::Query(QueryError::ModelNotLoaded))
}

/// Stub: always returns `ModelNotLoaded` when the `embeddings` feature is off.
#[cfg(not(feature = "embeddings"))]
pub fn embed_texts(_texts: &[String]) -> Result<Vec<Vec<f32>>, EngramError> {
    Err(EngramError::Query(QueryError::ModelNotLoaded))
}

// ── Embedding status API (dxo.4.1) ──────────────────────────────────

/// Runtime status of the embedding subsystem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingStatus {
    /// Whether the `embeddings` Cargo feature flag is enabled.
    pub enabled: bool,
    /// Whether the ONNX model is loaded and ready.
    pub model_loaded: bool,
    /// Model identifier (e.g., `bge-small-en-v1.5`), if loaded.
    pub model_name: Option<String>,
    /// Number of symbols with non-zero embedding vectors.
    pub symbols_with_embeddings: usize,
    /// Total symbol count across all tables.
    pub total_symbols: usize,
    /// `symbols_with_embeddings / total_symbols * 100`, or 0.0 when
    /// `total_symbols` is zero.
    pub coverage_percent: f64,
}

/// Returns `true` when the embeddings feature is compiled-in **and** the
/// ONNX model has been successfully loaded.
///
/// This is a cheap, synchronous check suitable for guard clauses.
#[cfg(feature = "embeddings")]
pub fn is_available() -> bool {
    MODEL.get().is_some_and(Result::is_ok)
}

/// Stub: always `false` when the `embeddings` feature is off.
#[cfg(not(feature = "embeddings"))]
pub fn is_available() -> bool {
    false
}

/// Collect full embedding subsystem status, including symbol coverage.
///
/// When `queries` is `None` (no workspace bound yet), symbol counts are
/// reported as zero.
///
/// # Errors
///
/// Returns `EngramError` if the database queries fail.
#[cfg(feature = "embeddings")]
pub async fn status(
    queries: Option<&crate::db::queries::CodeGraphQueries>,
) -> Result<EmbeddingStatus, EngramError> {
    let model_loaded = is_available();
    let model_name = if model_loaded {
        Some("bge-small-en-v1.5".to_string())
    } else {
        None
    };

    let (symbols_with_embeddings, total_symbols) = match queries {
        Some(q) => count_symbol_embeddings(q).await?,
        None => (0, 0),
    };

    Ok(EmbeddingStatus {
        enabled: true,
        model_loaded,
        model_name,
        symbols_with_embeddings,
        total_symbols,
        coverage_percent: compute_coverage(symbols_with_embeddings, total_symbols),
    })
}

/// Stub: returns disabled status when the `embeddings` feature is off.
///
/// # Errors
///
/// Returns `EngramError` if the database queries fail.
#[cfg(not(feature = "embeddings"))]
pub async fn status(
    queries: Option<&crate::db::queries::CodeGraphQueries>,
) -> Result<EmbeddingStatus, EngramError> {
    let (symbols_with_embeddings, total_symbols) = match queries {
        Some(q) => count_symbol_embeddings(q).await?,
        None => (0, 0),
    };

    Ok(EmbeddingStatus {
        enabled: false,
        model_loaded: false,
        model_name: None,
        symbols_with_embeddings,
        total_symbols,
        coverage_percent: compute_coverage(symbols_with_embeddings, total_symbols),
    })
}

// ── Shared helpers ──────────────────────────────────────────────────

/// Count total symbols and those with non-zero embedding vectors.
async fn count_symbol_embeddings(
    queries: &crate::db::queries::CodeGraphQueries,
) -> Result<(usize, usize), EngramError> {
    let functions = queries.all_functions().await?;
    let classes = queries.all_classes().await?;
    let interfaces = queries.all_interfaces().await?;

    let total = functions.len() + classes.len() + interfaces.len();
    let with_embeddings = functions
        .iter()
        .filter(|f| has_meaningful_embedding(&f.embedding))
        .count()
        + classes
            .iter()
            .filter(|c| has_meaningful_embedding(&c.embedding))
            .count()
        + interfaces
            .iter()
            .filter(|i| has_meaningful_embedding(&i.embedding))
            .count();

    Ok((with_embeddings, total))
}

/// An embedding is "meaningful" when at least one element is non-zero.
pub fn has_meaningful_embedding(embedding: &[f32]) -> bool {
    embedding.iter().any(|v| v.abs() > f32::EPSILON)
}

/// Compute coverage percentage, returning 0.0 when `total` is zero.
#[allow(clippy::cast_precision_loss)]
pub fn compute_coverage(with_embeddings: usize, total: usize) -> f64 {
    if total == 0 {
        0.0
    } else {
        (with_embeddings as f64 / total as f64) * 100.0
    }
}

/// Validate that a query string is within the token-length budget.
///
/// # Errors
/// Returns `QueryError::QueryTooLong` when the text exceeds [`MAX_QUERY_CHARS`].
pub fn validate_query_length(query: &str) -> Result<(), EngramError> {
    if query.len() > MAX_QUERY_CHARS {
        return Err(EngramError::Query(QueryError::QueryTooLong));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_query_accepted() {
        assert!(validate_query_length("hello world").is_ok());
    }

    #[test]
    fn exactly_max_query_accepted() {
        let text = "a".repeat(MAX_QUERY_CHARS);
        assert!(validate_query_length(&text).is_ok());
    }

    #[test]
    fn oversized_query_rejected() {
        let text = "a".repeat(MAX_QUERY_CHARS + 1);
        let err = validate_query_length(&text).unwrap_err();
        let code = err.to_response().error.code;
        assert_eq!(code, crate::errors::codes::QUERY_TOO_LONG);
    }

    #[cfg(not(feature = "embeddings"))]
    #[test]
    fn stub_embed_text_returns_model_not_loaded() {
        let err = embed_text("test").unwrap_err();
        let code = err.to_response().error.code;
        assert_eq!(code, crate::errors::codes::MODEL_NOT_LOADED);
    }

    #[cfg(not(feature = "embeddings"))]
    #[test]
    fn stub_embed_texts_returns_model_not_loaded() {
        let err = embed_texts(&["a".to_string()]).unwrap_err();
        let code = err.to_response().error.code;
        assert_eq!(code, crate::errors::codes::MODEL_NOT_LOADED);
    }
}
