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
    /// Display title for the result when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Workspace-relative file path when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
    /// One-based line range string, e.g. `"L3-L5"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_range: Option<String>,
    /// Retrieval granularity for content-backed results.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub record_kind: Option<String>,
    /// Heading ancestry for Markdown chunk results.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub heading_path: Vec<String>,
    /// Explicit fallback reason when chunking degrades to file-level retrieval.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_reason: Option<String>,
    /// Advisory lint summary for the matched record.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lint_summary: Option<String>,
    /// Advisory lint suggestions for the matched record.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub suggestions: Vec<String>,
}

/// Searchable item fed into the ranking pipeline.
#[derive(Debug, Clone)]
pub struct SearchCandidate {
    pub id: String,
    pub source_type: String,
    pub content: String,
    pub embedding: Option<Vec<f32>>,
    pub title: Option<String>,
    pub file_path: Option<String>,
    pub line_range: Option<String>,
    pub record_kind: Option<String>,
    pub heading_path: Vec<String>,
    pub fallback_reason: Option<String>,
    pub lint_summary: Option<String>,
    pub suggestions: Vec<String>,
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
    /// Retrieval granularity for content-backed results.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub record_kind: Option<String>,
    /// Heading ancestry for Markdown chunk results.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub heading_path: Vec<String>,
    /// Explicit fallback reason when chunking degrades to file-level retrieval.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_reason: Option<String>,
    /// Advisory lint summary for the matched record.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lint_summary: Option<String>,
    /// Advisory lint suggestions for the matched record.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub suggestions: Vec<String>,
}

/// Additive ranking boost applied to code-region results so code ranks above
/// docs/backlog unless a content result is more relevant by more than this
/// margin (a "score gap"). engram's primary purpose is code search; docs and
/// backlog are secondary. Provisional and tunable — the merge ordering tests
/// encode the semantics independently of the exact value.
pub(crate) const CODE_RANK_BOOST: f32 = 0.10;

/// Whether `unified_search` should include content (docs/backlog) results.
///
/// `region = "code"` restricts results to code symbols only; any other value
/// (e.g. `"all"`) includes content. Pure, so it is unit-testable without a
/// workspace.
#[must_use]
pub(crate) fn should_include_content(region: &str) -> bool {
    region != "code"
}

/// Rank key used to order [`UnifiedSearchResult`]s: the result's per-source
/// score plus a code-region boost (the "score gap"). Ordering only — the
/// result's reported `score` field is left unchanged (raw cosine for embedding
/// KNN, or a keyword ratio for the content fallback path).
fn rank_key(result: &UnifiedSearchResult) -> f32 {
    match result.region {
        SearchRegion::Code => result.score + CODE_RANK_BOOST,
        SearchRegion::Task => result.score,
    }
}

/// Merge code-region and content-region results into a single list, ranked by a
/// code-biased key (per-source `score` plus [`CODE_RANK_BOOST`] for code),
/// truncated to `limit` (FR-131). Code ranks above content unless a content
/// result is more relevant by more than the boost. The reported `score` on each
/// result is left unchanged (raw cosine for KNN, or a keyword ratio for the
/// content fallback) — the boost affects ordering only.
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
    // Descending by rank key (note `b, a` order). Requires a STABLE sort so that
    // gap-boundary ties (equal rank keys) keep code — extended first — ahead of
    // content. Do not switch to `sort_unstable_by` or flip the argument order.
    merged.sort_by(|a, b| rank_key(b).total_cmp(&rank_key(a)));
    merged.truncate(limit);
    merged
}

/// Compute cosine similarity between two vectors.
///
/// Returns `0.0` when either vector is zero-length or dimensions mismatch.
///
/// # Deprecation
///
/// Code symbol search should use
/// [`CodeGraphQueries::vector_search_symbols_native`] which delegates to
/// SurrealDB's native `<|K,COSINE|>` KNN operator. This function is retained
/// only for content-record scoring in [`hybrid_search`], which applies
/// application-level scoring rather than DB-native KNN.
#[deprecated(
    note = "Use SurrealDB native KNN via CodeGraphQueries::vector_search_symbols_native() \
            for code symbol similarity. Only hybrid_search content scoring may use this."
)]
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

/// Embed `query` only when at least one candidate carries a vector.
///
/// Shared by [`hybrid_search`] and [`hybrid_rank_of`] so both gate the
/// query-embedding model identically: an all-keyword corpus never loads the
/// model, and a corpus with vectors but an unusable model silently falls back
/// to keyword-only scoring (the failure is swallowed via `.ok()`).
fn embed_query_for_corpus(query: &str, candidates: &[SearchCandidate]) -> Option<Vec<f32>> {
    if candidates.iter().any(|c| c.embedding.is_some()) {
        embedding::embed_text(query).ok()
    } else {
        None
    }
}

/// Combined hybrid score for one candidate: `0.7 * vector + 0.3 * keyword`.
///
/// The single source of truth for hybrid scoring, shared by [`hybrid_search`]
/// and [`hybrid_rank_of`] so production search and retrieval-eval ranking can
/// never diverge (084.010-T).
#[allow(deprecated)] // cosine_similarity: content records lack MTREE index, app-level scoring required
fn hybrid_candidate_score(
    query: &str,
    query_embedding: Option<&[f32]>,
    candidate: &SearchCandidate,
) -> f32 {
    let vs = match (query_embedding, &candidate.embedding) {
        (Some(qe), Some(ce)) => cosine_similarity(qe, ce).max(0.0),
        _ => 0.0,
    };
    let ks = keyword_score(query, &candidate.content);
    VECTOR_WEIGHT * vs + KEYWORD_WEIGHT * ks
}

/// Rank (1-based) of `target_id` within a bounded top-`limit` hybrid ranking,
/// or `None` when the target falls outside the top-`limit` (or is absent).
///
/// This is retrieval-eval's known-item probe (084.010-T / 4CF046A5). It shares
/// [`hybrid_search`]'s exact scoring ([`hybrid_candidate_score`]) and stable
/// descending tie-break, but tracks **only a single counter** rather than
/// cloning and sorting the full [`SearchResult`] set: the rank equals one plus
/// the number of candidates that outrank the target. A candidate outranks the
/// target when its score is strictly greater, or — on a tie — when it appears
/// earlier in candidate order (exactly matching the stable sort in
/// [`hybrid_search`], which preserves input order among equal scores). Per-query
/// work is `O(n)` time and `O(1)` extra space, with no full-corpus clone.
///
/// # Errors
/// Returns `QueryError::QueryTooLong` if the query exceeds the token budget
/// (validated identically to [`hybrid_search`]).
pub fn hybrid_rank_of(
    query: &str,
    candidates: &[SearchCandidate],
    target_id: &str,
    limit: usize,
) -> Result<Option<usize>, EngramError> {
    embedding::validate_query_length(query)?;
    if limit == 0 {
        // `hybrid_search` truncates to `limit`, so a zero limit yields no hits.
        return Ok(None);
    }

    let Some(target_idx) = candidates.iter().position(|c| c.id == target_id) else {
        return Ok(None);
    };

    let query_embedding = embed_query_for_corpus(query, candidates);
    let qe = query_embedding.as_deref();
    let target_score = hybrid_candidate_score(query, qe, &candidates[target_idx]);

    // Count candidates that outrank the target under the stable descending sort:
    // strictly-higher score, or an equal score at an earlier candidate index.
    let mut better = 0_usize;
    for (i, candidate) in candidates.iter().enumerate() {
        if i == target_idx {
            continue;
        }
        let score = hybrid_candidate_score(query, qe, candidate);
        let outranks = match score.partial_cmp(&target_score) {
            Some(std::cmp::Ordering::Greater) => true,
            Some(std::cmp::Ordering::Less) => false,
            // Tie (equal, or a NaN the sort treats as equal): earlier index wins.
            _ => i < target_idx,
        };
        if outranks {
            better += 1;
        }
    }

    Ok(if better < limit {
        Some(better + 1)
    } else {
        None
    })
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

    // Only load the embedding model when at least one candidate has a vector.
    let query_embedding = embed_query_for_corpus(query, candidates);
    let qe = query_embedding.as_deref();

    let mut scored: Vec<SearchResult> = candidates
        .iter()
        .map(|c| {
            let combined = hybrid_candidate_score(query, qe, c);

            SearchResult {
                id: c.id.clone(),
                source_type: c.source_type.clone(),
                content: c.content.clone(),
                score: combined,
                title: c.title.clone(),
                file_path: c.file_path.clone(),
                line_range: c.line_range.clone(),
                record_kind: c.record_kind.clone(),
                heading_path: c.heading_path.clone(),
                fallback_reason: c.fallback_reason.clone(),
                lint_summary: c.lint_summary.clone(),
                suggestions: c.suggestions.clone(),
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
#[allow(deprecated)] // tests exercise cosine_similarity directly to verify its correctness
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
            title: None,
            file_path: None,
            line_range: None,
            record_kind: None,
            heading_path: Vec::new(),
            fallback_reason: None,
            lint_summary: None,
            suggestions: Vec::new(),
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
                title: None,
                file_path: None,
                line_range: None,
                record_kind: None,
                heading_path: Vec::new(),
                fallback_reason: None,
                lint_summary: None,
                suggestions: Vec::new(),
            },
            SearchCandidate {
                id: "spec:high".to_string(),
                source_type: "spec".to_string(),
                content: "user login authentication flow".to_string(),
                embedding: None,
                title: None,
                file_path: None,
                line_range: None,
                record_kind: None,
                heading_path: Vec::new(),
                fallback_reason: None,
                lint_summary: None,
                suggestions: Vec::new(),
            },
        ];

        let results = hybrid_search("user login", &candidates, 10).unwrap();
        assert_eq!(results.len(), 2);
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
                title: None,
                file_path: None,
                line_range: None,
                record_kind: None,
                heading_path: Vec::new(),
                fallback_reason: None,
                lint_summary: None,
                suggestions: Vec::new(),
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

    // ── merge_unified_results: code-first (score gap) ─────────────────

    fn unified(region: SearchRegion, id: &str, score: f32) -> UnifiedSearchResult {
        UnifiedSearchResult {
            region,
            score,
            node_type: "function".to_owned(),
            id: id.to_owned(),
            title: None,
            file_path: None,
            line_range: None,
            summary: None,
            status: None,
            linked_symbols: None,
            record_kind: None,
            heading_path: Vec::new(),
            fallback_reason: None,
            lint_summary: None,
            suggestions: Vec::new(),
        }
    }

    #[test]
    fn code_ranks_before_content_within_gap() {
        // Content beats code on raw score, but by less than the gap → code first.
        let code = unified(SearchRegion::Code, "code", 0.70);
        let content = unified(SearchRegion::Task, "doc", 0.70 + CODE_RANK_BOOST / 2.0);
        let merged = merge_unified_results(vec![code], vec![content], 10);
        assert_eq!(
            merged[0].region,
            SearchRegion::Code,
            "code must rank first within the score gap"
        );
        assert_eq!(merged[1].region, SearchRegion::Task);
    }

    #[test]
    fn boundary_content_equal_to_gap_ranks_code_first() {
        // Strict `>`: content must exceed code + gap; equality keeps code first.
        let code = unified(SearchRegion::Code, "code", 0.70);
        let content = unified(SearchRegion::Task, "doc", 0.70 + CODE_RANK_BOOST);
        let merged = merge_unified_results(vec![code], vec![content], 10);
        assert_eq!(
            merged[0].region,
            SearchRegion::Code,
            "a tie at the gap boundary must prefer code"
        );
    }

    #[test]
    fn strongly_better_content_outranks_code() {
        // Content beats code by more than the gap → content first (escape hatch).
        let code = unified(SearchRegion::Code, "code", 0.70);
        let content = unified(SearchRegion::Task, "doc", 0.70 + CODE_RANK_BOOST * 2.0);
        let merged = merge_unified_results(vec![code], vec![content], 10);
        assert_eq!(
            merged[0].region,
            SearchRegion::Task,
            "clearly-more-relevant content must surface above code"
        );
    }

    #[test]
    fn code_results_sorted_by_score_within_region() {
        let lo = unified(SearchRegion::Code, "lo", 0.60);
        let hi = unified(SearchRegion::Code, "hi", 0.90);
        let merged = merge_unified_results(vec![lo, hi], vec![], 10);
        assert_eq!(merged[0].id, "hi");
        assert_eq!(merged[1].id, "lo");
    }

    #[test]
    fn content_results_sorted_by_score_within_region() {
        let lo = unified(SearchRegion::Task, "lo", 0.60);
        let hi = unified(SearchRegion::Task, "hi", 0.90);
        let merged = merge_unified_results(vec![], vec![lo, hi], 10);
        assert_eq!(merged[0].id, "hi");
        assert_eq!(merged[1].id, "lo");
    }

    #[test]
    fn reported_score_is_unboosted_cosine() {
        let code = unified(SearchRegion::Code, "code", 0.72);
        let merged = merge_unified_results(vec![code], vec![], 10);
        assert!(
            (merged[0].score - 0.72).abs() < f32::EPSILON,
            "reported score must be the raw cosine, not the boosted rank key"
        );
    }

    #[test]
    fn truncation_keeps_code_when_content_within_gap() {
        let code = unified(SearchRegion::Code, "code", 0.70);
        let content = unified(SearchRegion::Task, "doc", 0.70 + CODE_RANK_BOOST / 2.0);
        let merged = merge_unified_results(vec![code], vec![content], 1);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].region, SearchRegion::Code);
    }

    #[test]
    fn truncation_keeps_content_when_beyond_gap() {
        let code = unified(SearchRegion::Code, "code", 0.70);
        let content = unified(SearchRegion::Task, "doc", 0.70 + CODE_RANK_BOOST * 2.0);
        let merged = merge_unified_results(vec![code], vec![content], 1);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].region, SearchRegion::Task);
    }

    #[test]
    fn should_include_content_gates_on_region() {
        assert!(should_include_content("all"));
        assert!(!should_include_content("code"));
    }

    // ── hybrid_rank_of: bounded top-k parity (084.010-T / 4CF046A5) ──────────

    /// Build a keyword-only fixture corpus with deterministic, distinct scores
    /// (no embeddings → vector component is 0, so ranks are pure keyword order).
    fn rank_fixture() -> Vec<SearchCandidate> {
        let contents = [
            ("c:a", "user login authentication flow session"),
            ("c:b", "user login authentication flow"),
            ("c:c", "user login authentication"),
            ("c:d", "user login"),
            ("c:e", "the quick brown fox jumps"),
            // Deliberate keyword tie with c:d (identical term coverage & length)
            // to exercise the stable tie-break against original order.
            ("c:f", "user login"),
        ];
        contents
            .iter()
            .map(|(id, content)| SearchCandidate {
                id: (*id).to_owned(),
                source_type: "code".to_owned(),
                content: (*content).to_owned(),
                embedding: None,
                title: None,
                file_path: None,
                line_range: None,
                record_kind: None,
                heading_path: Vec::new(),
                fallback_reason: None,
                lint_summary: None,
                suggestions: Vec::new(),
            })
            .collect()
    }

    /// Reference rank via the original clone+sort path: 1-based position of
    /// `target_id` in the top-`limit` `hybrid_search` result, or `None`.
    fn oracle_rank(
        query: &str,
        cands: &[SearchCandidate],
        target_id: &str,
        limit: usize,
    ) -> Option<usize> {
        let results = hybrid_search(query, cands, limit).unwrap();
        results
            .iter()
            .position(|hit| hit.id == target_id)
            .map(|idx| idx + 1)
    }

    #[test]
    fn hybrid_rank_of_matches_hybrid_search_for_every_id() {
        let cands = rank_fixture();
        // Cover k below, at, and above the corpus size, plus every candidate id
        // and a non-existent id — ranks must match the clone+sort oracle exactly,
        // including the stable tie-break between c:d and c:f.
        for query in ["user login authentication", "login", "user"] {
            for limit in [1_usize, 2, 3, 6, 20] {
                for c in &cands {
                    let expected = oracle_rank(query, &cands, &c.id, limit);
                    let got = hybrid_rank_of(query, &cands, &c.id, limit).unwrap();
                    assert_eq!(
                        got, expected,
                        "rank mismatch for id={} query={query:?} limit={limit}",
                        c.id
                    );
                }
                assert_eq!(
                    hybrid_rank_of(query, &cands, "c:missing", limit).unwrap(),
                    None,
                    "absent target must rank None (query={query:?} limit={limit})"
                );
            }
        }
    }

    #[test]
    fn hybrid_rank_of_rejects_long_query() {
        let long_query = "a ".repeat(embedding::MAX_QUERY_CHARS + 1);
        let cands = rank_fixture();
        let err = hybrid_rank_of(&long_query, &cands, "c:a", 10).unwrap_err();
        assert_eq!(
            err.to_response().error.code,
            crate::errors::codes::QUERY_TOO_LONG,
            "over-budget query must surface QUERY_TOO_LONG, matching hybrid_search"
        );
    }

    #[test]
    fn hybrid_rank_of_empty_corpus_is_none() {
        let cands: Vec<SearchCandidate> = Vec::new();
        assert_eq!(hybrid_rank_of("login", &cands, "c:a", 5).unwrap(), None);
    }
}
