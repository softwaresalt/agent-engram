use serde::{Deserialize, Serialize};
use sysinfo::System;

use crate::db::workspace::{canonicalize_workspace, workspace_hash};
use crate::errors::{TMemError, WorkspaceError};
use crate::server::state::{AppState, WorkspaceSnapshot};
use crate::services::{connection::validate_workspace_path, hydration::hydrate_workspace};

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

pub async fn set_workspace(state: &AppState, path: String) -> Result<WorkspaceBinding, TMemError> {
    validate_workspace_path(&path)?;

    let canonical = canonicalize_workspace(&path)?;
    let workspace_id = workspace_hash(&canonical);

    let (task_count, context_count) = hydrate_workspace(&canonical).await?;

    let snapshot = WorkspaceSnapshot {
        workspace_id: workspace_id.clone(),
        path: canonical.display().to_string(),
        task_count,
        context_count,
        last_flush: None,
        stale_files: false,
        connection_count: state.active_connections(),
    };

    state.set_workspace(snapshot).await;

    Ok(WorkspaceBinding {
        workspace_id,
        path: canonical.display().to_string(),
        task_count,
        hydrated: true,
    })
}

pub async fn get_daemon_status(state: &AppState) -> Result<DaemonStatus, TMemError> {
    let mut sys = System::new();
    sys.refresh_memory();
    let memory_bytes = sys.used_memory() * 1024; // KiB -> bytes

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

pub async fn get_workspace_status(state: &AppState) -> Result<WorkspaceStatus, TMemError> {
    if let Some(snapshot) = state.snapshot_workspace().await {
        return Ok(WorkspaceStatus {
            path: snapshot.path,
            task_count: snapshot.task_count,
            context_count: snapshot.context_count,
            last_flush: snapshot.last_flush,
            stale_files: snapshot.stale_files,
            connection_count: state.active_connections(),
        });
    }

    Err(TMemError::Workspace(WorkspaceError::NotSet))
}
