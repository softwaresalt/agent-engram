//! Hybrid search combining vector similarity and keyword matching.
//!
//! Scoring formula: `0.7 * vector_score + 0.3 * keyword_score`
//!
//! When the `embeddings` feature is disabled, the engine falls back to
//! keyword-only ranking (the vector component is zero).

use serde::{Deserialize, Serialize};

use crate::errors::EngramError;
use crate::services::embedding;

/// Weight for vector similarity in final score.
const VECTOR_WEIGHT: f32 = 0.7;
/// Weight for keyword matching in final score.
const KEYWORD_WEIGHT: f32 = 0.3;

/// A single search hit returned to the caller.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Source entity ID (e.g. `spec:abc`, `context:xyz`).
    pub id: String,
    /// Source type: `"spec"`, `"task"`, or `"context"`.
    pub source_type: String,
    /// The text content that matched.
    pub content: String,
    /// Combined relevance score in `[0.0, 1.0]`.
    pub score: f32,
}

/// Searchable item fed into the ranking pipeline.
#[derive(Debug, Clone)]
pub struct SearchCandidate {
    pub id: String,
    pub source_type: String,
    pub content: String,
    pub embedding: Option<Vec<f32>>,
}

// ── Unified Semantic Search Types (Phase 7 — US5) ────────────────────────

/// Region tag for unified search results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchRegion {
    Code,
    Task,
}

/// A single result from unified cross-region search (FR-128/FR-131).
///
/// Returns summary text only, not full bodies (FR-148 exemption).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedSearchResult {
    /// Which region this result comes from.
    pub region: SearchRegion,
    /// Cosine similarity score in `[0.0, 1.0]`.
    pub score: f32,
    /// Node type: function, class, interface, task, context, spec.
    pub node_type: String,
    /// Entity ID (e.g. `function:abc123`, `task:xyz`).
    pub id: String,
    /// Symbol name or task title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// File path (code nodes only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
    /// Line range string, e.g. `"L42-L78"` (code nodes only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_range: Option<String>,
    /// Summary text (FR-148: no full bodies).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// Task status (task nodes only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// Linked code symbol names (task nodes only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linked_symbols: Option<Vec<String>>,
}

/// Merge code-region and task-region results into a single list sorted by
/// descending cosine score, truncated to `limit` (FR-131).
///
/// No cross-region normalization or boosting in v0.
#[must_use]
pub fn merge_unified_results(
    code_results: Vec<UnifiedSearchResult>,
    task_results: Vec<UnifiedSearchResult>,
    limit: usize,
) -> Vec<UnifiedSearchResult> {
    let mut merged: Vec<UnifiedSearchResult> =
        Vec::with_capacity(code_results.len() + task_results.len());
    merged.extend(code_results);
    merged.extend(task_results);
    merged.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    merged.truncate(limit);
    merged
}

/// Compute cosine similarity between two vectors.
///
/// Returns `0.0` when either vector is zero-length or dimensions mismatch.
#[must_use]
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot / (norm_a * norm_b)
}

/// BM25-inspired keyword score.
///
/// Each query term that appears in the document contributes
/// `1 / (1 + doc_word_count)` — a lightweight IDF-style boost
/// that favours shorter, more focused documents.
#[must_use]
#[allow(clippy::cast_precision_loss)] // word counts well within f32 precision
pub fn keyword_score(query: &str, document: &str) -> f32 {
    let query_lower = query.to_lowercase();
    let doc_lower = document.to_lowercase();
    let terms: Vec<&str> = query_lower.split_whitespace().collect();

    if terms.is_empty() {
        return 0.0;
    }

    let doc_words: Vec<&str> = doc_lower.split_whitespace().collect();
    let doc_word_count = doc_words.len().max(1) as f32;

    let mut matches: usize = 0;
    for term in &terms {
        if doc_lower.contains(term) {
            matches += 1;
        }
    }

    let term_coverage = matches as f32 / terms.len() as f32;
    // length-normalised score (shorter docs score higher per match)
    term_coverage / (1.0 + doc_word_count.ln())
}

/// Run hybrid search over the given candidates.
///
/// 1. Embed the query (skip if embeddings feature is off).
/// 2. For each candidate compute `0.7 * vector + 0.3 * keyword`.
/// 3. Return results sorted descending by score, capped at `limit`.
///
/// # Errors
/// Returns `QueryError::QueryTooLong` if the query exceeds the token budget.
pub fn hybrid_search(
    query: &str,
    candidates: &[SearchCandidate],
    limit: usize,
) -> Result<Vec<SearchResult>, EngramError> {
    embedding::validate_query_length(query)?;

    let query_embedding: Option<Vec<f32>> = embedding::embed_text(query).ok();

    let mut scored: Vec<SearchResult> = candidates
        .iter()
        .map(|c| {
            let vs = match (&query_embedding, &c.embedding) {
                (Some(qe), Some(ce)) => cosine_similarity(qe, ce).max(0.0),
                _ => 0.0,
            };
            let ks = keyword_score(query, &c.content);
            let combined = VECTOR_WEIGHT * vs + KEYWORD_WEIGHT * ks;

            SearchResult {
                id: c.id.clone(),
                source_type: c.source_type.clone(),
                content: c.content.clone(),
                score: combined,
            }
        })
        .collect();

    scored.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    scored.truncate(limit);
    Ok(scored)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── cosine_similarity ────────────────────────────────────────

    #[test]
    fn identical_vectors_return_one() {
        let v = vec![1.0, 0.0, 0.0];
        let sim = cosine_similarity(&v, &v);
        assert!((sim - 1.0).abs() < 1e-6, "expected 1.0, got {sim}");
    }

    #[test]
    fn orthogonal_vectors_return_zero() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 1e-6, "expected 0.0, got {sim}");
    }

    #[test]
    fn mismatched_dims_return_zero() {
        let a = vec![1.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < f32::EPSILON, "expected 0.0, got {sim}");
    }

    #[test]
    fn zero_vector_returns_zero() {
        let a = vec![0.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < f32::EPSILON, "expected 0.0, got {sim}");
    }

    // ── keyword_score ────────────────────────────────────────────

    #[test]
    fn exact_single_term_scores_positive() {
        let score = keyword_score("login", "user login page");
        assert!(score > 0.0, "expected positive score, got {score}");
    }

    #[test]
    fn no_match_scores_zero() {
        let score = keyword_score("authentication", "the quick brown fox");
        assert!(score.abs() < 1e-6, "expected ~0.0, got {score}");
    }

    #[test]
    fn case_insensitive_matching() {
        let score = keyword_score("LOGIN", "User Login Page");
        assert!(score > 0.0, "expected positive score, got {score}");
    }

    #[test]
    fn partial_term_coverage() {
        let full = keyword_score("user login", "user login page");
        let partial = keyword_score("user login", "user dashboard");
        assert!(
            full > partial,
            "full coverage ({full}) should beat partial ({partial})"
        );
    }

    // ── hybrid scoring weights ───────────────────────────────────

    #[test]
    fn hybrid_weights_are_correct() {
        // Without embeddings feature, vector component is 0.
        // The hybrid score should equal KEYWORD_WEIGHT * keyword_score.
        let candidates = vec![SearchCandidate {
            id: "spec:1".to_string(),
            source_type: "spec".to_string(),
            content: "user login authentication".to_string(),
            embedding: None,
        }];

        let results = hybrid_search("user login", &candidates, 10).unwrap();
        assert_eq!(results.len(), 1);

        let expected_ks = keyword_score("user login", "user login authentication");
        let expected = KEYWORD_WEIGHT * expected_ks;
        let actual = results[0].score;
        assert!(
            (actual - expected).abs() < 1e-6,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn hybrid_results_sorted_descending() {
        let candidates = vec![
            SearchCandidate {
                id: "spec:low".to_string(),
                source_type: "spec".to_string(),
                content: "the quick brown fox".to_string(),
                embedding: None,
            },
            SearchCandidate {
                id: "spec:high".to_string(),
                source_type: "spec".to_string(),
                content: "user login authentication flow".to_string(),
                embedding: None,
            },
        ];

        let results = hybrid_search("user login", &candidates, 10).unwrap();
        assert!(results.len() == 2);
        assert!(
            results[0].score >= results[1].score,
            "results should be sorted descending"
        );
        assert_eq!(results[0].id, "spec:high");
    }

    #[test]
    fn hybrid_respects_limit() {
        let candidates: Vec<SearchCandidate> = (0..20)
            .map(|i| SearchCandidate {
                id: format!("spec:{i}"),
                source_type: "spec".to_string(),
                content: format!("document number {i} about login"),
                embedding: None,
            })
            .collect();

        let results = hybrid_search("login", &candidates, 5).unwrap();
        assert_eq!(results.len(), 5);
    }

    #[test]
    fn hybrid_rejects_long_query() {
        let long_query = "a ".repeat(embedding::MAX_QUERY_CHARS + 1);
        let candidates = vec![];
        let err = hybrid_search(&long_query, &candidates, 10).unwrap_err();
        let code = err.to_response().error.code;
        assert_eq!(code, crate::errors::codes::QUERY_TOO_LONG);
    }
}
