use std::path::PathBuf;

use serde::Deserialize;
use serde_json::{Value, json};

use crate::config::StaleStrategy;
use crate::db::connect_db;
use crate::db::queries::Queries;
use crate::errors::{CodeGraphError, EngramError, SystemError, WorkspaceError};
use crate::server::state::SharedState;
use crate::services::dehydration;
use crate::services::hydration;

async fn workspace_path(state: &SharedState) -> Result<PathBuf, EngramError> {
    if let Some(snapshot) = state.snapshot_workspace().await {
        return Ok(PathBuf::from(snapshot.path));
    }
    Err(EngramError::Workspace(WorkspaceError::NotSet))
}

async fn workspace_id(state: &SharedState) -> Result<String, EngramError> {
    if let Some(snapshot) = state.snapshot_workspace().await {
        return Ok(snapshot.workspace_id);
    }
    Err(EngramError::Workspace(WorkspaceError::NotSet))
}

pub async fn flush_state(state: SharedState, params: Option<Value>) -> Result<Value, EngramError> {
    // FR-153: Reject flush while indexing — code graph may be in inconsistent state
    if state.is_indexing() {
        return Err(EngramError::CodeGraph(CodeGraphError::IndexInProgress));
    }

    // T092: Acquire per-workspace write lock for FIFO serialization of concurrent flushes
    let _flush_guard = dehydration::acquire_flush_lock().await;
    let snapshot = state
        .snapshot_workspace()
        .await
        .ok_or(EngramError::Workspace(WorkspaceError::NotSet))?;

    let path = PathBuf::from(&snapshot.path);
    let workspace_id = snapshot.workspace_id.clone();
    let engram_dir = path.join(".engram");
    let stale_strategy = state.stale_strategy();
    let mut warnings: Vec<String> = Vec::new();
    let is_stale =
        snapshot.stale_files || hydration::detect_stale_since(&snapshot.file_mtimes, &engram_dir);

    let _ = params;

    let db = connect_db(&workspace_id).await?;
    let queries = Queries::new(db.clone());

    // Determine staleness action from strategy before touching the DB
    match (is_stale, stale_strategy) {
        (true, StaleStrategy::Fail) => {
            return Err(EngramError::Hydration(
                crate::errors::HydrationError::StaleWorkspace,
            ));
        }
        (true, StaleStrategy::Warn) => {
            warnings.push("2004 StaleWorkspace: .engram files modified externally".to_string());
        }
        (true, StaleStrategy::Rehydrate) => {
            hydration::hydrate_into_db(&path, &queries).await?;
        }
        (false, _) => {}
    }

    let result = dehydration::dehydrate_workspace(&queries, &path).await?;

    // Code graph serialization (FR-132, FR-133, FR-134)
    let cg_queries = crate::db::queries::CodeGraphQueries::new(db);
    let cg_result = dehydration::dehydrate_code_graph(&cg_queries, &path).await?;

    let mut all_files = result.files_written.clone();
    all_files.extend(cg_result.files_written);

    let new_mtimes = hydration::collect_file_mtimes(&engram_dir);

    let _ = state
        .update_workspace(|ws| {
            ws.last_flush = Some(result.flush_timestamp.clone());
            ws.stale_files = false;
            ws.file_mtimes = new_mtimes;
        })
        .await;

    Ok(json!({
        "files_written": all_files,
        "warnings": warnings,
        "flush_timestamp": result.flush_timestamp,
        "code_graph": {
            "nodes_written": cg_result.nodes_written,
            "edges_written": cg_result.edges_written,
        },
    }))
}

// ── index_workspace ─────────────────────────────────────────────────

#[derive(Deserialize)]
struct IndexWorkspaceParams {
    #[serde(default)]
    force: bool,
}

/// Parse all supported source files and populate the code knowledge graph.
///
/// Returns a structured summary of files parsed, symbols indexed, edges
/// created, and any per-file errors encountered.
pub async fn index_workspace(
    state: SharedState,
    params: Option<Value>,
) -> Result<Value, EngramError> {
    let ws_path = workspace_path(&state).await?;
    let ws_id = workspace_id(&state).await?;

    // Reject if indexing is already running.
    if !state.try_start_indexing() {
        return Err(EngramError::CodeGraph(CodeGraphError::IndexInProgress));
    }

    // Run the indexing logic, ensuring the flag is cleared on all exit paths.
    let result = index_workspace_inner(&state, &ws_path, &ws_id, params).await;
    state.finish_indexing().await;
    result
}

/// Inner indexing logic separated to guarantee `finish_indexing()` runs.
async fn index_workspace_inner(
    state: &SharedState,
    ws_path: &std::path::Path,
    ws_id: &str,
    params: Option<Value>,
) -> Result<Value, EngramError> {
    let parsed: IndexWorkspaceParams = serde_json::from_value(params.unwrap_or_else(|| json!({})))
        .map_err(|e| {
            EngramError::System(SystemError::InvalidParams {
                reason: e.to_string(),
            })
        })?;

    let config = state
        .workspace_config()
        .await
        .map(|c| c.code_graph.clone())
        .unwrap_or_default();

    let result =
        crate::services::code_graph::index_workspace(ws_path, ws_id, &config, parsed.force).await?;

    serde_json::to_value(result).map_err(|e| {
        EngramError::System(SystemError::DatabaseError {
            reason: format!("result serialization failed: {e}"),
        })
    })
}

// ── sync_workspace (T045) ───────────────────────────────────────────

/// Detect changed, added, and deleted files since the last index and
/// update only affected nodes in the code graph.
///
/// Uses two-level hashing (file-level `content_hash` then per-symbol
/// `body_hash`) to minimise re-embedding. Preserves `concerns` edges
/// across file moves via hash-resilient identity matching (FR-124).
pub async fn sync_workspace(
    state: SharedState,
    params: Option<Value>,
) -> Result<Value, EngramError> {
    let ws_path = workspace_path(&state).await?;
    let ws_id = workspace_id(&state).await?;

    // Reject if indexing is already running (FR-121 / 7003).
    if !state.try_start_indexing() {
        return Err(EngramError::CodeGraph(CodeGraphError::IndexInProgress));
    }

    // Run the sync logic, ensuring the flag is cleared on all exit paths.
    let result = sync_workspace_inner(&state, &ws_path, &ws_id, params).await;
    state.finish_indexing().await;
    result
}

/// Inner sync logic separated to guarantee `finish_indexing()` runs.
async fn sync_workspace_inner(
    state: &SharedState,
    ws_path: &std::path::Path,
    ws_id: &str,
    params: Option<Value>,
) -> Result<Value, EngramError> {
    let _ = params; // no params for sync_workspace currently

    let config = state
        .workspace_config()
        .await
        .map(|c| c.code_graph.clone())
        .unwrap_or_default();

    let result = crate::services::code_graph::sync_workspace(ws_path, ws_id, &config).await?;

    serde_json::to_value(result).map_err(|e| {
        EngramError::System(SystemError::DatabaseError {
            reason: format!("result serialization failed: {e}"),
        })
    })
}

// ── index_git_history (T042) ──────────────────────────────────────────────────

/// Parameters for the `index_git_history` MCP tool.
#[cfg(feature = "git-graph")]
#[derive(serde::Deserialize)]
struct IndexGitHistoryParams {
    /// Number of commits to walk from HEAD (default: 500).
    #[serde(default)]
    depth: Option<u32>,
    /// When true, re-index all commits even if already stored.
    #[serde(default)]
    force: bool,
}

/// Index the workspace's git commit history into the `commit_node` table.
///
/// Requires the `git-graph` feature flag and a workspace that is a valid git
/// repository. Returns a summary of the indexing run.
#[cfg(feature = "git-graph")]
pub async fn index_git_history(
    state: SharedState,
    params: Option<Value>,
) -> Result<Value, EngramError> {
    let ws_path = workspace_path(&state).await?;
    let ws_id = workspace_id(&state).await?;

    let parsed: IndexGitHistoryParams = serde_json::from_value(params.unwrap_or_else(|| json!({})))
        .map_err(|e| {
            EngramError::System(SystemError::InvalidParams {
                reason: e.to_string(),
            })
        })?;

    if parsed.depth == Some(0) {
        return Err(EngramError::System(SystemError::InvalidParams {
            reason: "depth must be greater than 0 when provided".to_owned(),
        }));
    }

    let depth = parsed.depth.unwrap_or(0); // None → service uses default 500

    let db = connect_db(&ws_id).await?;
    let queries = Queries::new(db);

    let summary =
        crate::services::git_graph::index_git_history(&queries, &ws_path, depth, parsed.force)
            .await?;

    serde_json::to_value(&summary).map_err(|e| {
        EngramError::System(SystemError::DatabaseError {
            reason: format!("index_git_history serialization failed: {e}"),
        })
    })
}
