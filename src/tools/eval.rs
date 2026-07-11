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

use std::path::PathBuf;

use serde_json::Value;

use crate::db::connect_db;
use crate::db::queries::CodeGraphQueries;
use crate::errors::{EngramError, SystemError, WorkspaceError};
use crate::models::retrieval_eval::{RetrievalEvalConfig, RetrievalEvalReport};
use crate::server::state::SharedState;
use crate::services::retrieval_eval;

/// Resolve the active branch and retrieval-eval config, requiring a workspace.
///
/// # Errors
/// Returns [`WorkspaceError::NotSet`] when no workspace is bound.
async fn branch_and_config(
    state: &SharedState,
) -> Result<(String, RetrievalEvalConfig), EngramError> {
    let (_, branch, config) = snapshot_parts(state).await?;
    Ok((branch, config))
}

/// Resolve the workspace data directory, active branch and retrieval-eval
/// config, requiring a workspace.
///
/// # Errors
/// Returns [`WorkspaceError::NotSet`] when no workspace is bound.
async fn snapshot_parts(
    state: &SharedState,
) -> Result<(PathBuf, String, RetrievalEvalConfig), EngramError> {
    let snapshot = state
        .snapshot_workspace()
        .await
        .ok_or(EngramError::Workspace(WorkspaceError::NotSet))?;
    let config = state
        .workspace_config()
        .await
        .unwrap_or_default()
        .retrieval_eval;
    Ok((snapshot.data_dir, snapshot.branch, config))
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
    let (data_dir, branch, config) = snapshot_parts(&state).await?;
    if !config.enabled {
        return to_value(&RetrievalEvalReport::empty(false, branch));
    }

    // Read the indexed function corpus. A brand-new or un-indexed workspace has
    // no function relations yet; treat that as an empty corpus (zero report)
    // rather than surfacing a database error.
    let db = connect_db(&data_dir, &branch).await?;
    let queries = CodeGraphQueries::new(db);
    let functions = queries.all_functions().await.unwrap_or_default();

    let semantic = retrieval_eval::evaluate_semantic(&functions, &config)?;

    let mut report = RetrievalEvalReport::empty(true, branch);
    report.k = config.k;
    report.sample_size = config.sample_size;
    report.languages.clone_from(&config.languages);
    report.semantic = semantic;
    to_value(&report)
}

/// `get_retrieval_eval_report` — return the latest retrieval-evaluation report.
///
/// Empty-state milestone: returns an empty [`RetrievalEvalReport`] because no
/// run has been persisted yet. Unknown params are ignored.
///
/// # Errors
/// Returns [`WorkspaceError::NotSet`] when no workspace is bound, or a
/// serialization error if the report cannot be encoded.
pub async fn get_retrieval_eval_report(
    state: SharedState,
    _params: Option<Value>,
) -> Result<Value, EngramError> {
    let (branch, config) = branch_and_config(&state).await?;
    let report = RetrievalEvalReport::empty(config.enabled, branch);
    to_value(&report)
}
