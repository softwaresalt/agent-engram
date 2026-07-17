//! Retrieval-evaluation MCP tool handlers (081-F).
//!
//! Implements `run_retrieval_eval` (compute a run) and
//! `get_retrieval_eval_report` (return the latest run). This module is
//! deliberately separate from the agent-efficiency evaluation surface in
//! [`crate::tools::read::get_evaluation_report`].
//!
//! When the subsystem is disabled (the default), `run_retrieval_eval` returns a
//! well-formed empty [`RetrievalEvalReport`] whose `enabled` flag is `false`.
//! When enabled, `run_retrieval_eval` computes semantic self-retrieval and
//! graph resolution metrics and persists the run under `.engram/eval/{branch}/`.
//! `get_retrieval_eval_report` reads persistence first, so it returns the latest
//! persisted run — which may be an earlier `enabled: true` run captured before
//! the subsystem was disabled — and only falls back to an empty report when no
//! run has ever been persisted for the branch.

use std::path::PathBuf;

use serde_json::Value;

use crate::db::connect_db;
use crate::db::queries::CodeGraphQueries;
use crate::errors::{EngramError, SystemError, WorkspaceError};
use crate::models::retrieval_eval::{RetrievalEvalConfig, RetrievalEvalReport};
use crate::server::state::SharedState;
use crate::services::retrieval_eval;

/// Workspace facts needed to run a retrieval-evaluation.
struct SnapshotParts {
    /// Absolute workspace root (for reading indexed source files).
    workspace_path: PathBuf,
    /// `.engram` data directory (for the code-graph database).
    data_dir: PathBuf,
    /// Active branch.
    branch: String,
    /// Retrieval-eval configuration.
    config: RetrievalEvalConfig,
}

/// Resolve the workspace paths, active branch and retrieval-eval config,
/// requiring a bound workspace.
///
/// # Errors
/// Returns [`WorkspaceError::NotSet`] when no workspace is bound.
async fn snapshot_parts(state: &SharedState) -> Result<SnapshotParts, EngramError> {
    // Clone the workspace binding and config under a single lock window so a
    // concurrent `set_workspace` / `set_workspace_config` cannot pair a snapshot
    // with a config from a different update (which could run or persist an
    // evaluation with mismatched settings).
    let ctx = state
        .snapshot_dispatch_context()
        .await
        .ok_or(EngramError::Workspace(WorkspaceError::NotSet))?;
    Ok(SnapshotParts {
        workspace_path: PathBuf::from(ctx.workspace.path),
        data_dir: ctx.workspace.data_dir,
        branch: ctx.workspace.branch,
        config: ctx.config.retrieval_eval,
    })
}

/// Serialize a report to a JSON value, mapping failures to a database error.
fn to_value(report: &RetrievalEvalReport) -> Result<Value, EngramError> {
    serde_json::to_value(report).map_err(|e| {
        EngramError::System(SystemError::DatabaseError {
            reason: format!("failed to serialize retrieval eval report: {e}"),
        })
    })
}

/// `run_retrieval_eval` — compute a retrieval-evaluation run.
///
/// When the subsystem is disabled, returns an empty [`RetrievalEvalReport`].
/// When enabled, computes semantic self-retrieval metrics over the indexed
/// function corpus and graph resolution metrics (resolution-recall and
/// false-edge-rate) from the parser call-site inventory, then persists the run.
/// Unknown params are ignored.
///
/// # Errors
/// Returns [`WorkspaceError::NotSet`] when no workspace is bound, a database
/// error if the corpus cannot be read, or a serialization error if the report
/// cannot be encoded.
pub async fn run_retrieval_eval(
    state: SharedState,
    _params: Option<Value>,
) -> Result<Value, EngramError> {
    let parts = snapshot_parts(&state).await?;
    if !parts.config.enabled {
        return to_value(&RetrievalEvalReport::empty(false, parts.branch));
    }

    // Read the indexed function corpus. An initialized but un-indexed workspace
    // returns an empty vector normally, so an actual query error must propagate
    // (a database failure must not masquerade as a zero-metric success report).
    // 084.009-T: use the LEFT-JOIN corpus so a partially-written function
    // (function_meta present, code/embedding row absent) still counts toward the
    // semantic-eval denominator (78AA205D) instead of being silently dropped.
    let db = connect_db(&parts.data_dir, &parts.branch).await?;
    let queries = CodeGraphQueries::new(db);
    let functions = queries.all_functions_for_eval().await?;

    let semantic = retrieval_eval::evaluate_semantic(&functions, &parts.config)?;

    // Graph resolution metrics (081.005-T): denominator = parser call-site
    // inventory over indexed source; numerator = resolved `calls` edges;
    // false edges = resolved edges whose callee matches no known definition.
    // Database read errors propagate rather than degrading to fabricated zeros.
    let files = queries.list_code_files().await?;
    let mut resolved_edges = std::collections::HashSet::new();
    for resolution in [
        "direct",
        "calls_resolved_singleton",
        "calls_resolved_canonical",
    ] {
        resolved_edges.extend(queries.list_calls_edges_by_resolution(resolution).await?);
    }
    let resolution_context = retrieval_eval::CallSiteResolutionContext::new(
        functions,
        resolved_edges,
        queries.function_ids_by_canonical_path().await?,
        queries.load_index_unsafe_module_prefixes().await?,
    );
    let inventory = retrieval_eval::scan_call_site_inventory_with_resolution(
        &parts.workspace_path,
        &files,
        &parts.config.languages,
        &resolution_context,
    )
    .await?;
    // Numerator is gated to the *same* configured caller languages as the
    // call-site denominator (084.002-T / D6F70DCC) so recall is a ratio of
    // identically-scoped units; an empty language list counts every edge.
    let resolved = queries
        .count_calls_edges_in_languages(&parts.config.languages)
        .await?;
    // Count dangling (false) edges under the SAME caller-language gate as the
    // resolved numerator (084.008-T / Thread-8) so `false_edge_rate` is a ratio
    // of identically-scoped units. A dangling edge in an unconfigured language
    // must not inflate the configured language's false-edge rate.
    let false_edges = queries
        .count_dangling_calls_edges_in_languages(&parts.config.languages)
        .await?;
    let mut graph =
        retrieval_eval::compute_graph_metrics(inventory.call_sites, resolved, false_edges);
    // Surface the honest index-consistency signals alongside the ratio so a
    // consumer can distinguish a genuine recall from one computed against a tree
    // that drifted from the indexed revision, or over fewer files than were
    // indexed (084.003-T). The `[0, 1]` clamp stays as a defensive floor.
    graph.index_stale = inventory.index_stale;
    graph.unreadable_files = inventory.unreadable_files;

    let mut report = RetrievalEvalReport::empty(true, parts.branch);
    // Record the *effective* cutoff actually used by the semantic compute
    // (`evaluate_semantic` normalizes `k = 0` to `1`), so the reported `k`
    // matches the metrics that were computed against it.
    report.k = parts.config.k.max(1);
    // Report the actual number of known-item queries evaluated (functions with
    // a non-empty derived query), not the configured cap, so `sample_size`
    // matches its documented meaning and `semantic.queries`.
    report.sample_size = semantic.queries;
    report.languages.clone_from(&parts.config.languages);
    report.semantic = semantic;
    report.graph = graph;

    // Gate the run against the configured thresholds (084.006-T / 14B33F9F).
    // A run that evaluated nothing (empty / un-indexed corpus: no sampled
    // queries and no call sites) is NOT gated, so an unmeasured floor cannot
    // fire a false breach; disabled runs already returned early above. Default
    // thresholds are permissive (floors 0.0, ceiling 1.0), so an unconfigured
    // workspace passes unchanged (back-compat). The `engram eval` CLI maps
    // `thresholds_breached` onto its exit code (084.007-T).
    if report.sample_size > 0 || report.graph.call_sites > 0 {
        // Reject non-finite thresholds before enforcing them: TOML/JSON accept
        // `nan`/`inf`, and every `<`/`>` comparison against NaN is false, so a
        // malformed floor or ceiling would silently report `thresholds_breached
        // = false` and disable the gate (084.006-T). A bad config value must fail
        // the run loudly, not defeat the gate quietly.
        retrieval_eval::validate_thresholds(&parts.config.thresholds)?;
        let check = retrieval_eval::check_thresholds(&report, &parts.config.thresholds);
        report.thresholds_breached = !check.passed;
        report.threshold_breaches = check.breaches;
    }

    // Persist the run under `.engram/eval/{branch}/` for autoharness feedback
    // and so `get_retrieval_eval_report` can return the latest run.
    let engram_dir = parts.workspace_path.join(".engram");
    retrieval_eval::persist_report(&engram_dir, &report).await?;

    to_value(&report)
}

/// `get_retrieval_eval_report` — return the latest retrieval-evaluation report.
///
/// Reads the newest run persisted under `.engram/eval/{branch}/`. When no run
/// has been persisted, returns an empty [`RetrievalEvalReport`] reflecting the
/// configured `enabled` flag. Unknown params are ignored.
///
/// # Errors
/// Returns [`WorkspaceError::NotSet`] when no workspace is bound, a system error
/// if a persisted run cannot be read, or a serialization error if the report
/// cannot be encoded.
pub async fn get_retrieval_eval_report(
    state: SharedState,
    _params: Option<Value>,
) -> Result<Value, EngramError> {
    let parts = snapshot_parts(&state).await?;
    let engram_dir = parts.workspace_path.join(".engram");
    if let Some(report) = retrieval_eval::latest_report(&engram_dir, &parts.branch).await? {
        return to_value(&report);
    }
    to_value(&RetrievalEvalReport::empty(
        parts.config.enabled,
        parts.branch,
    ))
}
