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

use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::db::connect_db;
use crate::db::queries::CodeGraphQueries;
use crate::errors::{EngramError, SystemError, WorkspaceError};
use crate::models::retrieval_eval::{RetrievalEvalConfig, RetrievalEvalReport};
use crate::server::state::SharedState;
use crate::services::parsing::Language;
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

/// Count the parser call-site inventory across indexed source files.
///
/// Reads each indexed file that passes the language gate and parses it to count
/// `ExtractedEdge::Calls` occurrences (the graph-metric denominator). File-read
/// and parse failures are skipped so a single bad file never aborts the run.
///
/// # Errors
/// Propagates a database error if the indexed-file listing fails, or a system
/// error if the off-runtime parse task panics — a failed read must not be
/// silently reported as an empty inventory.
async fn count_workspace_call_sites(
    workspace_path: &Path,
    queries: &CodeGraphQueries,
    config: &RetrievalEvalConfig,
) -> Result<usize, EngramError> {
    let files = queries.list_code_files().await?;
    // Canonical workspace root for the containment check below. The bound
    // workspace is canonicalized at `set_workspace` time, so this is expected
    // to succeed; a failure is surfaced rather than silently skewing metrics.
    let ws_root = tokio::fs::canonicalize(workspace_path).await.map_err(|e| {
        EngramError::System(SystemError::DatabaseError {
            reason: format!("cannot resolve workspace root for eval containment check: {e}"),
        })
    })?;
    let mut sources: Vec<(String, String)> = Vec::new();
    for file in files {
        let gated = config.languages.is_empty()
            || config
                .languages
                .iter()
                .any(|lang| lang.eq_ignore_ascii_case(&file.language));
        if !gated {
            continue;
        }
        let full = workspace_path.join(&file.path);
        // Workspace-isolation invariant (no traversal): resolve the target
        // (following symlinks and `..`) and require it to stay under the
        // canonical workspace root before reading. An absolute path, `..`
        // component, or in-workspace symlink that escapes the workspace is
        // skipped rather than read. Unresolvable paths are skipped too, which
        // matches this function's existing skip-on-read-failure contract.
        let Ok(canon) = tokio::fs::canonicalize(&full).await else {
            continue;
        };
        if !canon.starts_with(&ws_root) {
            continue;
        }
        if let Ok(source) = tokio::fs::read_to_string(&canon).await {
            sources.push((file.language, source));
        }
    }

    // Parsing is CPU-bound; run the whole batch off the async runtime.
    tokio::task::spawn_blocking(move || {
        sources
            .iter()
            .filter_map(|(lang, source)| {
                Language::try_from(lang.as_str())
                    .ok()
                    .map(|language| retrieval_eval::count_call_sites(source, language))
            })
            .sum()
    })
    .await
    .map_err(|e| {
        EngramError::System(SystemError::DatabaseError {
            reason: format!("retrieval eval call-site parse task failed: {e}"),
        })
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
    let db = connect_db(&parts.data_dir, &parts.branch).await?;
    let queries = CodeGraphQueries::new(db);
    let functions = queries.all_functions().await?;

    let semantic = retrieval_eval::evaluate_semantic(&functions, &parts.config)?;

    // Graph resolution metrics (081.005-T): denominator = parser call-site
    // inventory over indexed source; numerator = resolved `calls` edges;
    // false edges = resolved edges whose callee matches no known definition.
    // Database read errors propagate rather than degrading to fabricated zeros.
    let call_sites =
        count_workspace_call_sites(&parts.workspace_path, &queries, &parts.config).await?;
    let resolved = queries.count_calls_edges().await?;
    let false_edges = queries.count_dangling_calls_edges().await?;
    let graph = retrieval_eval::compute_graph_metrics(call_sites, resolved, false_edges);

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
