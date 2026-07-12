//! Retrieval-evaluation data models (081-F).
//!
//! Defines the configuration and report shapes for the portable, in-product
//! retrieval + graph-recall evaluation subsystem. The subsystem measures how
//! well the workspace index answers known-item queries (semantic
//! self-retrieval) and how completely the code graph resolves syntactic call
//! sites (graph resolution recall).
//!
//! This surface is intentionally distinct from the agent-efficiency
//! [`crate::models::evaluation`] surface (`EvaluationConfig` /
//! `EvaluationReport` / `get_evaluation_report`). The two subsystems measure
//! different things and must not be conflated; naming here is `retrieval_eval`
//! everywhere.
//!
//! # Ground truth is auto-derived (no manual labels)
//!
//! * **Semantic** — each indexed function's docstring (falling back to its name)
//!   becomes a known-item query whose single expected hit is that same function.
//!   (The semantic corpus is scoped to functions in this baseline; broader
//!   symbol kinds are follow-up work.)
//! * **Graph** — the tree-sitter call-site inventory is the denominator; the
//!   resolved `calls` edges are the numerator.

use serde::{Deserialize, Serialize};

/// Configuration for the retrieval-evaluation subsystem.
///
/// Read from the `[retrieval_eval]` section of `.engram/config.toml`. The
/// subsystem is **disabled by default**; a workspace opts in by setting
/// `enabled = true`. Follows the `#[serde(default)]` section pattern used by
/// [`crate::models::config::CodeGraphConfig`] and
/// [`crate::models::evaluation::EvaluationConfig`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetrievalEvalConfig {
    /// Whether the subsystem is enabled. Defaults to `false` (opt-in).
    #[serde(default)]
    pub enabled: bool,
    /// Languages to include when deriving semantic queries and call-site
    /// inventories. Defaults to `["rust"]`.
    #[serde(default = "default_languages")]
    pub languages: Vec<String>,
    /// Cut-off rank `k` for precision@k / recall@k / nDCG@k. Defaults to `10`.
    #[serde(default = "default_k")]
    pub k: usize,
    /// Maximum number of symbols sampled as known-item queries. Bounds eval
    /// cost on large workspaces. Defaults to `200`.
    #[serde(default = "default_sample_size")]
    pub sample_size: usize,
    /// Baseline thresholds used by the regression tier to detect metric
    /// regressions.
    #[serde(default)]
    pub thresholds: RetrievalEvalThresholds,
}

impl Default for RetrievalEvalConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            languages: default_languages(),
            k: default_k(),
            sample_size: default_sample_size(),
            thresholds: RetrievalEvalThresholds::default(),
        }
    }
}

fn default_languages() -> Vec<String> {
    vec!["rust".to_owned()]
}

const fn default_k() -> usize {
    10
}

const fn default_sample_size() -> usize {
    200
}

/// Baseline thresholds for the retrieval-evaluation regression tier.
///
/// Semantic metrics have `min_*` floors (higher is better); the false-edge rate
/// has a `max_*` ceiling (lower is better). Defaults are permissive
/// (all floors `0.0`, the ceiling `1.0`) so an unconfigured workspace never
/// fails a threshold check.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetrievalEvalThresholds {
    /// Minimum acceptable precision@k.
    #[serde(default)]
    pub min_precision_at_k: f64,
    /// Minimum acceptable recall@k.
    #[serde(default)]
    pub min_recall_at_k: f64,
    /// Minimum acceptable mean reciprocal rank.
    #[serde(default)]
    pub min_mrr: f64,
    /// Minimum acceptable nDCG@k.
    #[serde(default)]
    pub min_ndcg: f64,
    /// Minimum acceptable graph resolution recall.
    #[serde(default)]
    pub min_resolution_recall: f64,
    /// Maximum acceptable graph false-edge rate.
    #[serde(default = "default_max_false_edge_rate")]
    pub max_false_edge_rate: f64,
}

impl Default for RetrievalEvalThresholds {
    fn default() -> Self {
        Self {
            min_precision_at_k: 0.0,
            min_recall_at_k: 0.0,
            min_mrr: 0.0,
            min_ndcg: 0.0,
            min_resolution_recall: 0.0,
            max_false_edge_rate: default_max_false_edge_rate(),
        }
    }
}

fn default_max_false_edge_rate() -> f64 {
    1.0
}

/// Retrieval mode actually exercised by the semantic self-retrieval eval
/// (Cluster E, 084.008-T).
///
/// Records whether the run used true hybrid retrieval (keyword + embedding KNN)
/// or fell back to keyword-only ranking (embeddings unavailable / absent). This
/// makes reports comparable across environments and prevents a silently broken
/// embedding path from masquerading as a passing hybrid run.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetrievalMode {
    /// Retrieval mode was not recorded (legacy report or empty-state report).
    #[default]
    Unknown,
    /// Hybrid retrieval (keyword + embedding KNN) was exercised.
    Hybrid,
    /// Only keyword retrieval ran — the embedding KNN path was not exercised,
    /// either because no candidate carried an embedding or because the query
    /// itself failed to embed. Reported whenever the corpus is non-empty but the
    /// hybrid precondition (`corpus_has_vectors && query_embeds`) does not hold
    /// (a fallback, not true hybrid).
    KeywordOnly,
}

/// Semantic self-retrieval metrics over known-item queries.
///
/// Each query has exactly one relevant document (its source symbol), so the
/// metrics reduce to the single-relevant-item forms:
/// * `recall_at_k` — fraction of queries whose symbol appears in the top `k`.
/// * `precision_at_k` — mean of `1/k` over hits (single relevant per query).
/// * `mrr` — mean reciprocal rank of the source symbol.
/// * `ndcg` — mean `1/log2(rank+1)` for hits within `k` (ideal DCG is `1`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SemanticMetrics {
    /// Precision@k averaged over evaluated queries.
    pub precision_at_k: f64,
    /// Recall@k averaged over evaluated queries.
    pub recall_at_k: f64,
    /// Mean reciprocal rank.
    pub mrr: f64,
    /// Normalized discounted cumulative gain at `k`.
    pub ndcg: f64,
    /// Number of known-item queries evaluated.
    pub queries: usize,
    /// Retrieval mode actually exercised (hybrid vs keyword-only fallback).
    /// Additive (084.008-T); legacy reports default to [`RetrievalMode::Unknown`].
    #[serde(default)]
    pub retrieval_mode: RetrievalMode,
}

/// Graph resolution-recall metrics derived from the call-site inventory.
///
/// * `resolution_recall` — resolved `calls` edges ÷ visible call sites.
/// * `false_edge_rate` — resolved edges whose callee resolves to no known
///   definition ÷ resolved edges. This is a **dangling-only lower bound**; it
///   cannot flag a call resolved to a wrong-but-existing target. True
///   target-correctness requires the fixture-manifest assertion recorded in
///   `target_correct` / `target_mismatch` (084.004-T).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GraphMetrics {
    /// Resolved edges ÷ visible call sites (`0.0` when no call sites).
    pub resolution_recall: f64,
    /// False edges ÷ resolved edges (`0.0` when no resolved edges). Dangling
    /// callees only — a lower bound on the true false-edge rate.
    pub false_edge_rate: f64,
    /// Total syntactic call sites inventoried by the parser (denominator).
    pub call_sites: usize,
    /// Total resolved `calls` edges in the graph (numerator).
    pub resolved: u64,
    /// Resolved edges whose callee matches no known definition (dangling).
    pub false_edges: u64,
    /// Whether the working tree drifted from the indexed revision, or indexed
    /// files were unreadable, so `resolution_recall`'s disk-parsed denominator
    /// may not match the indexed numerator. An honest staleness signal emitted
    /// in place of a silent `[0,1]` clamp (Cluster A3, 084.003-T). Additive;
    /// legacy reports default to `false`.
    #[serde(default)]
    pub index_stale: bool,
    /// Count of indexed files that could not be read at eval time. Accounted
    /// explicitly (rather than silently dropped from the denominator) so the
    /// recall number stays honest (084.003-T). Additive; defaults to `0`.
    #[serde(default)]
    pub unreadable_files: usize,
    /// Resolved singleton edges whose target matched the expected-target
    /// manifest by exact identity (target-correctness numerator, 084.004-T).
    /// Populated only when a ground-truth manifest is supplied (fixture /
    /// regression path); `0` in production runs, which have no manifest.
    /// Additive; defaults to `0`.
    #[serde(default)]
    pub target_correct: u64,
    /// Resolved singleton edges whose target did NOT match the expected target
    /// (wrong-but-existing or dangling). Unlike `false_edge_rate` (a
    /// dangling-only lower bound), this catches mis-resolution to an existing
    /// function — but only against a supplied manifest (084.004-T). Additive;
    /// defaults to `0`.
    #[serde(default)]
    pub target_mismatch: u64,
}

/// A single retrieval-evaluation run report.
///
/// Emitted as structured JSON by the `run_retrieval_eval` /
/// `get_retrieval_eval_report` MCP tools and the `engram eval` CLI. The
/// empty-state shape (via [`RetrievalEvalReport::empty`]) is defined before any
/// compute so autoharness can integrate against a stable contract.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetrievalEvalReport {
    /// Whether the subsystem was enabled for this run.
    pub enabled: bool,
    /// Branch the run was evaluated on.
    pub branch: String,
    /// Evaluation timestamp (RFC 3339).
    pub evaluated_at: String,
    /// Cut-off rank `k` used for the semantic metrics.
    pub k: usize,
    /// Number of symbols sampled as known-item queries.
    pub sample_size: usize,
    /// Languages included in this run.
    #[serde(default)]
    pub languages: Vec<String>,
    /// Semantic self-retrieval metrics.
    pub semantic: SemanticMetrics,
    /// Graph resolution-recall metrics.
    pub graph: GraphMetrics,
    /// Whether any configured threshold was breached this run (Cluster D,
    /// 084.006-T). `false` for legacy reports, unconfigured thresholds, and
    /// empty/disabled runs. The `engram eval` CLI maps this to its exit code
    /// (084.007-T). Additive; defaults to `false`.
    #[serde(default)]
    pub thresholds_breached: bool,
    /// Human-readable descriptions of each breached threshold (empty on pass
    /// or when no thresholds are configured). Additive; defaults to empty.
    #[serde(default)]
    pub threshold_breaches: Vec<String>,
}

impl RetrievalEvalReport {
    /// Build an empty-state report with zeroed metrics.
    ///
    /// Used when the subsystem is disabled, when no run has been persisted yet,
    /// or when the workspace has nothing to evaluate. The shape matches a
    /// populated report so consumers parse one schema regardless of state.
    #[must_use]
    pub fn empty(enabled: bool, branch: impl Into<String>) -> Self {
        Self {
            enabled,
            branch: branch.into(),
            evaluated_at: chrono::Utc::now().to_rfc3339(),
            k: 0,
            sample_size: 0,
            languages: Vec::new(),
            semantic: SemanticMetrics::default(),
            graph: GraphMetrics::default(),
            thresholds_breached: false,
            threshold_breaches: Vec::new(),
        }
    }
}
