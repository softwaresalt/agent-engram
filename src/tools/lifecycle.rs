use std::path::Path;

use serde::{Deserialize, Serialize};
use sysinfo::System;

use crate::db::connect_db;
use crate::db::queries::CodeGraphQueries;
use crate::db::workspace::{
    canonicalize_workspace, resolve_data_dir, resolve_git_branch, workspace_hash,
};
use crate::errors::{EngramError, WorkspaceError};
use crate::server::state::{AppState, WorkspaceSnapshot};
use crate::services::config::parse_config;
use crate::services::connection::validate_workspace_path;
use crate::services::hydration::{detect_stale_since, hydrate_code_graph, hydrate_workspace};

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkspaceBinding {
    pub workspace_id: String,
    pub path: String,
    pub hydrated: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DaemonStatus {
    pub version: String,
    pub uptime_seconds: u64,
    pub active_workspaces: usize,
    pub active_connections: usize,
    pub memory_bytes: u64,
    pub model_loaded: bool,
    pub model_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkspaceStatus {
    pub path: String,
    pub last_flush: Option<String>,
    pub stale_files: bool,
    pub connection_count: usize,
    pub code_graph: CodeGraphStats,
}

/// Summary statistics for the indexed code graph.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CodeGraphStats {
    pub code_files: u64,
    pub functions: u64,
    pub classes: u64,
    pub interfaces: u64,
    pub edges: u64,
}

pub async fn set_workspace(
    state: &AppState,
    path: String,
) -> Result<WorkspaceBinding, EngramError> {
    validate_workspace_path(&path)?;

    let canonical = canonicalize_workspace(&path)?;
    let workspace_id = workspace_hash(&canonical);
    let branch = resolve_git_branch(&canonical).unwrap_or_else(|_| "default".to_string());
    let data_dir = resolve_data_dir(&canonical);

    if !state.has_workspace_capacity().await {
        return Err(EngramError::Workspace(WorkspaceError::LimitReached {
            limit: state.max_workspaces(),
        }));
    }

    let hydration = hydrate_workspace(&canonical).await?;

    // Connect to DB and hydrate code graph from .engram/code-graph/ JSONL files (FR-132)
    let db = connect_db(&data_dir, &branch).await?;
    let cg_queries = CodeGraphQueries::new(db);
    let _cg_result = hydrate_code_graph(&canonical, &cg_queries).await?;

    // Load and validate workspace config BEFORE committing the snapshot.
    // If config validation fails, we must not leave the workspace partially bound.
    let ws_config = parse_config(&canonical)?;

    let snapshot = WorkspaceSnapshot {
        workspace_id: workspace_id.clone(),
        branch: branch.clone(),
        data_dir: data_dir.clone(),
        path: canonical.display().to_string(),
        last_flush: hydration.last_flush.clone(),
        stale_files: hydration.stale_files,
        connection_count: state.active_connections(),
        file_mtimes: hydration.file_mtimes.clone(),
    };

    state.set_workspace(snapshot).await?;
    state.set_workspace_config(Some(ws_config)).await;
    crate::services::query_stats::reset_timing();

    Ok(WorkspaceBinding {
        workspace_id,
        path: canonical.display().to_string(),
        hydrated: true,
    })
}

pub async fn get_daemon_status(state: &AppState) -> Result<DaemonStatus, EngramError> {
    let mut sys = System::new();
    sys.refresh_memory();
    let memory_bytes = sys.used_memory(); // sysinfo 0.30+ returns bytes

    let model_loaded = crate::services::embedding::is_available();
    let model_name = if model_loaded {
        Some("bge-small-en-v1.5".to_string())
    } else {
        None
    };

    Ok(DaemonStatus {
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: state.uptime_seconds(),
        active_workspaces: state.active_workspaces().await,
        active_connections: state.active_connections(),
        memory_bytes,
        model_loaded,
        model_name,
    })
}

pub async fn get_workspace_status(state: &AppState) -> Result<WorkspaceStatus, EngramError> {
    if let Some(snapshot) = state.snapshot_workspace().await {
        let engram_dir = Path::new(&snapshot.path).join(".engram");
        let stale_now =
            snapshot.stale_files || detect_stale_since(&snapshot.file_mtimes, &engram_dir);

        if stale_now != snapshot.stale_files {
            let _ = state
                .update_workspace(|ws| ws.stale_files = stale_now)
                .await;
        }

        // Gather code graph stats from the database
        let code_graph = if let Ok(db) = connect_db(&snapshot.data_dir, &snapshot.branch).await {
            let cg_queries = CodeGraphQueries::new(db);
            let code_files = cg_queries.count_code_files().await.unwrap_or(0);
            let functions = cg_queries.count_functions().await.unwrap_or(0);
            let classes = cg_queries.count_classes().await.unwrap_or(0);
            let interfaces = cg_queries.count_interfaces().await.unwrap_or(0);
            let edges = cg_queries.count_code_edges().await.unwrap_or(0);
            CodeGraphStats {
                code_files,
                functions,
                classes,
                interfaces,
                edges,
            }
        } else {
            CodeGraphStats::default()
        };

        return Ok(WorkspaceStatus {
            path: snapshot.path,
            last_flush: snapshot.last_flush,
            stale_files: stale_now,
            connection_count: state.active_connections(),
            code_graph,
        });
    }

    Err(EngramError::Workspace(WorkspaceError::NotSet))
}
