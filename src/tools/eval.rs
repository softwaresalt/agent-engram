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

use serde_json::Value;

use crate::errors::{EngramError, SystemError, WorkspaceError};
use crate::models::retrieval_eval::{RetrievalEvalConfig, RetrievalEvalReport};
use crate::server::state::SharedState;

/// Resolve the active branch and retrieval-eval config, requiring a workspace.
///
/// # Errors
/// Returns [`WorkspaceError::NotSet`] when no workspace is bound.
async fn branch_and_config(
    state: &SharedState,
) -> Result<(String, RetrievalEvalConfig), EngramError> {
    let snapshot = state
        .snapshot_workspace()
        .await
        .ok_or(EngramError::Workspace(WorkspaceError::NotSet))?;
    let config = state
        .workspace_config()
        .await
        .unwrap_or_default()
        .retrieval_eval;
    Ok((snapshot.branch, config))
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
/// Empty-state milestone: returns an empty [`RetrievalEvalReport`] reflecting
/// the configured `enabled` flag. Unknown params are ignored.
///
/// # Errors
/// Returns [`WorkspaceError::NotSet`] when no workspace is bound, or a
/// serialization error if the report cannot be encoded.
pub async fn run_retrieval_eval(
    state: SharedState,
    _params: Option<Value>,
) -> Result<Value, EngramError> {
    let (branch, config) = branch_and_config(&state).await?;
    let report = RetrievalEvalReport::empty(config.enabled, branch);
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
