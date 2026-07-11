//! Retrieval-evaluation compute (081-F).
//!
//! Semantic self-retrieval: each indexed symbol's docstring / qualified name
//! becomes a known-item query whose single expected hit is that same symbol.
//! Running those queries through [`hybrid_search`] and recording the rank of the
//! source symbol yields precision@k, recall@k, MRR and nDCG@k.
//!
//! Ground truth is auto-derived (no manual labels): the query and its expected
//! answer both come from the same indexed symbol, so the corpus is entirely
//! self-describing. Self-retrieval is a *proxy* signal — it rewards name/doc
//! recall — and is therefore reported as one signal alongside the graph
//! resolution metrics rather than as a standalone quality score.
//!
//! This module is intentionally distinct from the agent-efficiency
//! [`crate::services::evaluation`] surface. The two subsystems measure different
//! things (`retrieval_eval` vs `evaluation`) and must not be conflated.

use std::path::{Path, PathBuf};

use crate::errors::{EngramError, SystemError};
use crate::models::Function;
use crate::models::retrieval_eval::{
    GraphMetrics, RetrievalEvalConfig, RetrievalEvalReport, SemanticMetrics,
};
use crate::services::parsing::{ExtractedEdge, Language, parse_source};
use crate::services::search::{SearchCandidate, hybrid_search};

/// Maximum bytes retained from a derived known-item query.
///
/// Well under [`crate::services::embedding::MAX_QUERY_CHARS`] so a long
/// docstring never trips the query-length guard.
const MAX_QUERY_BYTES: usize = 1900;

/// Map a file path's extension to a coarse language identifier.
///
/// Used to gate the semantic corpus by [`RetrievalEvalConfig::languages`].
/// Returns `"unknown"` for unrecognized extensions.
#[must_use]
pub fn language_of(path: &str) -> &'static str {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    match ext {
        "rs" => "rust",
        "py" => "python",
        "js" | "jsx" => "javascript",
        "ts" | "tsx" => "typescript",
        "go" => "go",
        "cs" => "csharp",
        "c" | "h" => "c",
        "cpp" | "cc" | "cxx" | "hpp" | "hh" | "hxx" => "cpp",
        "swift" => "swift",
        "kt" | "kts" => "kotlin",
        "rb" => "ruby",
        "java" => "java",
        _ => "unknown",
    }
}

/// Trim and byte-bound a raw query string on a UTF-8 char boundary.
fn bound_query(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.len() <= MAX_QUERY_BYTES {
        return trimmed.to_owned();
    }
    let mut end = MAX_QUERY_BYTES;
    while end > 0 && !trimmed.is_char_boundary(end) {
        end -= 1;
    }
    trimmed[..end].to_owned()
}

/// Derive a known-item query for a function symbol.
///
/// Prefers the first non-empty line of the docstring (natural-language intent),
/// falling back to the symbol name when no docstring is present. The result is
/// trimmed and length-bounded.
#[must_use]
pub fn derive_query(function: &Function) -> String {
    let raw = match function.docstring.as_deref() {
        Some(doc) if !doc.trim().is_empty() => doc
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty())
            .unwrap_or("")
            .to_owned(),
        _ => function.name.clone(),
    };
    bound_query(&raw)
}

/// Build a search candidate from a function record.
///
/// Content combines the symbol name, signature and docstring so a known-item
/// query derived from any of those fields can retrieve the symbol. The
/// embedding is included only when present, so an un-embedded corpus stays
/// keyword-only (no model load).
fn func_to_candidate(function: &Function) -> SearchCandidate {
    let content = format!(
        "{} {} {}",
        function.name,
        function.signature,
        function.docstring.as_deref().unwrap_or("")
    );
    SearchCandidate {
        id: function.id.clone(),
        source_type: "function".to_owned(),
        content,
        embedding: if function.embedding.is_empty() {
            None
        } else {
            Some(function.embedding.clone())
        },
        title: Some(function.name.clone()),
        file_path: Some(function.file_path.clone()),
        line_range: None,
        record_kind: Some("function".to_owned()),
        heading_path: Vec::new(),
        fallback_reason: None,
        lint_summary: None,
        suggestions: Vec::new(),
    }
}

/// Compute semantic self-retrieval metrics from per-query ranks.
///
/// `ranks[i]` is the 1-based rank of query `i`'s known item within the top `k`
/// results, or `None` when the item did not appear. Each query has exactly one
/// relevant document, so the metrics reduce to their single-relevant forms:
///
/// * `recall_at_k` = hits ÷ queries;
/// * `precision_at_k` = mean of `1/k` over hits;
/// * `mrr` = mean of `1/rank` over hits;
/// * `ndcg` = mean of `1/log2(rank+1)` over hits (ideal DCG is `1`).
///
/// Returns a zeroed [`SemanticMetrics`] (with `queries = 0`) for empty input.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn compute_semantic_metrics(ranks: &[Option<usize>], k: usize) -> SemanticMetrics {
    let queries = ranks.len();
    if queries == 0 {
        return SemanticMetrics::default();
    }
    let n = queries as f64;
    let k_f = if k == 0 { 1.0 } else { k as f64 };

    let mut hits = 0.0_f64;
    let mut precision = 0.0_f64;
    let mut mrr = 0.0_f64;
    let mut ndcg = 0.0_f64;
    for rank in ranks.iter().flatten() {
        let r = *rank as f64;
        hits += 1.0;
        precision += 1.0 / k_f;
        mrr += 1.0 / r;
        ndcg += 1.0 / (r + 1.0).log2();
    }

    SemanticMetrics {
        precision_at_k: precision / n,
        recall_at_k: hits / n,
        mrr: mrr / n,
        ndcg: ndcg / n,
        queries,
    }
}

/// Evaluate semantic self-retrieval over a function corpus.
///
/// The corpus is first gated by [`RetrievalEvalConfig::languages`] (an empty
/// list disables gating). A search candidate is built per surviving function; a
/// known-item query is derived per sampled function (up to
/// [`RetrievalEvalConfig::sample_size`]); [`hybrid_search`] ranks the corpus and
/// the rank of the source symbol is recorded. Ranks aggregate into
/// [`SemanticMetrics`]. Functions with an empty derived query are skipped.
///
/// # Errors
/// Returns an error if [`hybrid_search`] rejects a derived query (e.g. it
/// exceeds the query-length budget).
pub fn evaluate_semantic(
    functions: &[Function],
    config: &RetrievalEvalConfig,
) -> Result<SemanticMetrics, EngramError> {
    let k = config.k.max(1);

    let selected: Vec<&Function> = functions
        .iter()
        .filter(|f| {
            config.languages.is_empty()
                || config
                    .languages
                    .iter()
                    .any(|lang| lang.eq_ignore_ascii_case(language_of(&f.file_path)))
        })
        .collect();

    let candidates: Vec<SearchCandidate> = selected.iter().map(|f| func_to_candidate(f)).collect();

    let mut ranks: Vec<Option<usize>> = Vec::new();
    for function in selected.iter().take(config.sample_size) {
        let query = derive_query(function);
        if query.is_empty() {
            continue;
        }
        let results = hybrid_search(&query, &candidates, k)?;
        let rank = results
            .iter()
            .position(|hit| hit.id == function.id)
            .map(|idx| idx + 1);
        ranks.push(rank);
    }

    Ok(compute_semantic_metrics(&ranks, k))
}

/// Count identifier / path call sites in a source string.
///
/// This is the graph metric *denominator*: the parser call-site inventory
/// (`ExtractedEdge::Calls`) discovered by [`parse_source`]. Method calls and
/// blocklisted helpers (`clone`, `unwrap`, …) are excluded by the parser, so
/// the count reflects the same visible call sites the indexer attempts to
/// resolve. A parse failure yields `0` (the file contributes nothing).
#[must_use]
pub fn count_call_sites(source: &str, language: Language) -> usize {
    parse_source(source, language).map_or(0, |result| {
        result
            .edges
            .iter()
            .filter(|edge| matches!(edge, ExtractedEdge::Calls { .. }))
            .count()
    })
}

/// Compute graph resolution metrics from raw counts.
///
/// * `resolution_recall` = resolved ÷ visible call sites, clamped to `[0, 1]`
///   (`0.0` when there are no call sites);
/// * `false_edge_rate` = false edges ÷ resolved (`0.0` when nothing resolved).
///
/// `resolution_recall` is clamped because `resolved` (distinct edges in the
/// graph) and `call_sites` (raw parser occurrences re-counted from disk) are
/// gathered independently and could momentarily disagree.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn compute_graph_metrics(call_sites: usize, resolved: u64, false_edges: u64) -> GraphMetrics {
    let resolution_recall = if call_sites == 0 {
        0.0
    } else {
        (resolved as f64 / call_sites as f64).clamp(0.0, 1.0)
    };
    let false_edge_rate = if resolved == 0 {
        0.0
    } else {
        (false_edges as f64 / resolved as f64).clamp(0.0, 1.0)
    };
    GraphMetrics {
        resolution_recall,
        false_edge_rate,
        call_sites,
        resolved,
        false_edges,
    }
}

// ── Persistence (081.006-T) ──────────────────────────────────────────────

/// Wrap a filesystem error as an [`EngramError`] with contextual detail.
fn io_err(context: &str, error: &std::io::Error) -> EngramError {
    EngramError::System(SystemError::DatabaseError {
        reason: format!("retrieval eval {context}: {error}"),
    })
}

/// Sanitize a branch name for use as a single filesystem path component.
///
/// Mirrors the branch sanitization used for the code-graph DB subdirectory so a
/// branch like `feat/foo` maps to `feat_foo`.
fn branch_dir_component(branch: &str) -> String {
    branch.replace(['/', '\\', ':'], "_")
}

/// Directory holding persisted retrieval-eval runs for a branch:
/// `{engram_dir}/eval/{branch}`.
#[must_use]
pub fn eval_dir(engram_dir: &Path, branch: &str) -> PathBuf {
    engram_dir.join("eval").join(branch_dir_component(branch))
}

/// Persist a retrieval-eval run as JSON under `{engram_dir}/eval/{branch}/`.
///
/// The filename is a high-resolution timestamp so runs sort chronologically and
/// [`latest_report`] can pick the newest. Returns the written file path.
///
/// # Errors
/// Returns a system error if the directory cannot be created or the report
/// cannot be serialized or written.
pub async fn persist_report(
    engram_dir: &Path,
    report: &RetrievalEvalReport,
) -> Result<PathBuf, EngramError> {
    let dir = eval_dir(engram_dir, &report.branch);
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| io_err("create run directory", &e))?;

    let stamp = chrono::Utc::now()
        .timestamp_nanos_opt()
        .unwrap_or_else(|| chrono::Utc::now().timestamp_micros());
    let path = dir.join(format!("{stamp}.json"));

    let json = serde_json::to_string_pretty(report).map_err(|e| {
        EngramError::System(SystemError::DatabaseError {
            reason: format!("failed to serialize retrieval eval report: {e}"),
        })
    })?;
    tokio::fs::write(&path, json)
        .await
        .map_err(|e| io_err("write run file", &e))?;
    Ok(path)
}

/// Read the most recent persisted retrieval-eval run for a branch.
///
/// Returns `None` when no run has been persisted. The newest run is the file
/// with the greatest timestamp stem.
///
/// # Errors
/// Returns a system error if the directory listing or a file read fails, or if
/// the newest run cannot be deserialized.
pub async fn latest_report(
    engram_dir: &Path,
    branch: &str,
) -> Result<Option<RetrievalEvalReport>, EngramError> {
    let dir = eval_dir(engram_dir, branch);
    let mut entries = match tokio::fs::read_dir(&dir).await {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(io_err("open run directory", &e)),
    };

    let mut best: Option<(i128, PathBuf)> = None;
    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e| io_err("read run directory", &e))?
    {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let stamp = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .and_then(|stem| stem.parse::<i128>().ok())
            .unwrap_or(0);
        if best.as_ref().is_none_or(|(current, _)| stamp >= *current) {
            best = Some((stamp, path));
        }
    }

    let Some((_, path)) = best else {
        return Ok(None);
    };
    let contents = tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| io_err("read run file", &e))?;
    let report = serde_json::from_str(&contents).map_err(|e| {
        EngramError::System(SystemError::DatabaseError {
            reason: format!("failed to deserialize retrieval eval report: {e}"),
        })
    })?;
    Ok(Some(report))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_of_maps_known_extensions() {
        assert_eq!(language_of("src/lib.rs"), "rust");
        assert_eq!(language_of("pkg/main.go"), "go");
        assert_eq!(language_of("app/index.ts"), "typescript");
        assert_eq!(language_of("noext"), "unknown");
    }

    #[test]
    fn derive_query_prefers_docstring_first_line() {
        let f = Function {
            id: "function:x".to_owned(),
            name: "do_thing".to_owned(),
            file_path: "src/x.rs".to_owned(),
            line_start: 1,
            line_end: 2,
            signature: "fn do_thing()".to_owned(),
            docstring: Some("First line.\nSecond line.".to_owned()),
            body: String::new(),
            body_hash: String::new(),
            token_count: 0,
            embed_type: "explicit_code".to_owned(),
            embedding: Vec::new(),
            summary: String::new(),
        };
        assert_eq!(derive_query(&f), "First line.");
    }

    #[test]
    fn derive_query_falls_back_to_name() {
        let f = Function {
            id: "function:y".to_owned(),
            name: "helper".to_owned(),
            file_path: "src/y.rs".to_owned(),
            line_start: 1,
            line_end: 2,
            signature: "fn helper()".to_owned(),
            docstring: None,
            body: String::new(),
            body_hash: String::new(),
            token_count: 0,
            embed_type: "explicit_code".to_owned(),
            embedding: Vec::new(),
            summary: String::new(),
        };
        assert_eq!(derive_query(&f), "helper");
    }

    #[test]
    fn compute_metrics_perfect_when_all_rank_one() {
        let ranks = [Some(1_usize), Some(1_usize)];
        let m = compute_semantic_metrics(&ranks, 10);
        assert!((m.recall_at_k - 1.0).abs() < 1e-9);
        assert!((m.mrr - 1.0).abs() < 1e-9);
        assert!((m.ndcg - 1.0).abs() < 1e-9);
        assert_eq!(m.queries, 2);
    }
}
