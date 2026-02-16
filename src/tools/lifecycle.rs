use std::path::Path;

use serde::{Deserialize, Serialize};
use sysinfo::System;

use crate::db::connect_db;
use crate::db::queries::Queries;
use crate::db::workspace::{canonicalize_workspace, workspace_hash};
use crate::errors::{EngramError, WorkspaceError};
use crate::server::state::{AppState, WorkspaceSnapshot};
use crate::services::config::parse_config;
use crate::services::connection::validate_workspace_path;
use crate::services::hydration::{
    backfill_embeddings, detect_stale_since, hydrate_into_db, hydrate_workspace,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkspaceBinding {
    pub workspace_id: String,
    pub path: String,
    pub task_count: u64,
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
    pub task_count: u64,
    pub context_count: u64,
    pub last_flush: Option<String>,
    pub stale_files: bool,
    pub connection_count: usize,
}

pub async fn set_workspace(
    state: &AppState,
    path: String,
) -> Result<WorkspaceBinding, EngramError> {
    validate_workspace_path(&path)?;

    let canonical = canonicalize_workspace(&path)?;
    let workspace_id = workspace_hash(&canonical);

    if !state.has_workspace_capacity().await {
        return Err(EngramError::Workspace(WorkspaceError::LimitReached {
            limit: state.max_workspaces(),
        }));
    }

    let hydration = hydrate_workspace(&canonical).await?;

    // Connect to DB and load .engram/ data into SurrealDB (T072)
    let db = connect_db(&workspace_id).await?;
    let queries = Queries::new(db.clone());
    let db_result = hydrate_into_db(&canonical, &queries).await?;

    // Backfill embeddings for specs/contexts that lack them (T086)
    backfill_embeddings(&queries).await;

    let task_count = if db_result.tasks_loaded > 0 {
        db_result.tasks_loaded as u64
    } else {
        hydration.task_count
    };

    // Load and validate workspace config BEFORE committing the snapshot.
    // If config validation fails, we must not leave the workspace partially bound.
    let ws_config = parse_config(&canonical)?;
    // validate_config is now called inside parse_config (RI-11)

    let snapshot = WorkspaceSnapshot {
        workspace_id: workspace_id.clone(),
        path: canonical.display().to_string(),
        task_count,
        context_count: hydration.context_count,
        last_flush: hydration.last_flush.clone(),
        stale_files: hydration.stale_files,
        connection_count: state.active_connections(),
        file_mtimes: hydration.file_mtimes.clone(),
    };

    state.set_workspace(snapshot).await?;
    state.set_workspace_config(Some(ws_config)).await;

    Ok(WorkspaceBinding {
        workspace_id,
        path: canonical.display().to_string(),
        task_count,
        hydrated: true,
    })
}

pub async fn get_daemon_status(state: &AppState) -> Result<DaemonStatus, EngramError> {
    let mut sys = System::new();
    sys.refresh_memory();
    let memory_bytes = sys.used_memory(); // sysinfo 0.30+ returns bytes

    Ok(DaemonStatus {
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: state.uptime_seconds(),
        active_workspaces: state.active_workspaces().await,
        active_connections: state.active_connections(),
        memory_bytes,
        model_loaded: false,
        model_name: None,
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

        return Ok(WorkspaceStatus {
            path: snapshot.path,
            task_count: snapshot.task_count,
            context_count: snapshot.context_count,
            last_flush: snapshot.last_flush,
            stale_files: stale_now,
            connection_count: state.active_connections(),
        });
    }

    Err(EngramError::Workspace(WorkspaceError::NotSet))
}
