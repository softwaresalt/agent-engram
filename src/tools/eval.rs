//! Retrieval-evaluation MCP tool handlers (081-F).
//!
//! Implements `run_retrieval_eval` (compute a run) and
//! `get_retrieval_eval_report` (return the latest run). This module is
//! deliberately separate from the agent-efficiency evaluation surface in
//! [`crate::tools::read::get_evaluation_report`].
//!
//! For the empty-state milestone (081.002-T) both handlers return a well-formed
//! empty [`RetrievalEvalReport`] that reflects the configured `enabled` flag.
//! Semantic and graph compute plus persistence land in later tasks.

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
    let snapshot = state
        .snapshot_workspace()
        .await
        .ok_or(EngramError::Workspace(WorkspaceError::NotSet))?;
    let config = state
        .workspace_config()
        .await
        .unwrap_or_default()
        .retrieval_eval;
    Ok(SnapshotParts {
        workspace_path: PathBuf::from(snapshot.path),
        data_dir: snapshot.data_dir,
        branch: snapshot.branch,
        config,
    })
}

/// Count the parser call-site inventory across indexed source files.
///
/// Reads each indexed file that passes the language gate and parses it to count
/// `ExtractedEdge::Calls` occurrences (the graph-metric denominator). File-read
/// and parse failures are skipped so a single bad file never aborts the run.
async fn count_workspace_call_sites(
    workspace_path: &Path,
    queries: &CodeGraphQueries,
    config: &RetrievalEvalConfig,
) -> usize {
    let files = queries.list_code_files().await.unwrap_or_default();
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
        if let Ok(source) = tokio::fs::read_to_string(&full).await {
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
    .unwrap_or(0)
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
/// function corpus (081.004-T); graph metrics remain zeroed until 081.005-T.
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

    // Read the indexed function corpus. A brand-new or un-indexed workspace has
    // no function relations yet; treat that as an empty corpus (zero report)
    // rather than surfacing a database error.
    let db = connect_db(&parts.data_dir, &parts.branch).await?;
    let queries = CodeGraphQueries::new(db);
    let functions = queries.all_functions().await.unwrap_or_default();

    let semantic = retrieval_eval::evaluate_semantic(&functions, &parts.config)?;

    // Graph resolution metrics (081.005-T): denominator = parser call-site
    // inventory over indexed source; numerator = resolved `calls` edges;
    // false edges = resolved edges whose callee matches no known definition.
    let call_sites =
        count_workspace_call_sites(&parts.workspace_path, &queries, &parts.config).await;
    let resolved = queries.count_calls_edges().await.unwrap_or(0);
    let false_edges = queries.count_dangling_calls_edges().await.unwrap_or(0);
    let graph = retrieval_eval::compute_graph_metrics(call_sites, resolved, false_edges);

    let mut report = RetrievalEvalReport::empty(true, parts.branch);
    report.k = parts.config.k;
    report.sample_size = parts.config.sample_size;
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
