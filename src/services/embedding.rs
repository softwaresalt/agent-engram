//! Embedding generation service for semantic search.
//!
//! Wraps `fastembed-rs` behind the `embeddings` feature flag. When the
//! feature is disabled, all calls return `Err(QueryError::ModelNotLoaded)`.
//!
//! The model (`all-MiniLM-L6-v2`, 384 dimensions) is lazily downloaded
//! on first use and cached under `~/.local/share/engram/models/`.

use std::path::PathBuf;

use crate::errors::{EngramError, QueryError};

/// Embedding vector dimension for `all-MiniLM-L6-v2`.
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

        let options = fastembed::InitOptions::new(fastembed::EmbeddingModel::AllMiniLML6V2)
            .with_cache_dir(cache)
            .with_show_download_progress(true);

        fastembed::TextEmbedding::try_new(options).map_err(|e| e.to_string())
    });

    match result {
        Ok(model) => Ok(model),
        Err(reason) => Err(EngramError::Query(QueryError::ModelNotLoaded)),
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
