use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use tokio::sync::RwLock;

#[derive(Clone, Debug)]
pub struct WorkspaceSnapshot {
    pub workspace_id: String,
    pub path: String,
    pub task_count: u64,
    pub context_count: u64,
    pub last_flush: Option<String>,
    pub stale_files: bool,
    pub connection_count: usize,
}

#[derive(Debug)]
pub struct AppState {
    start: Instant,
    active_connections: AtomicUsize,
    active_workspace: RwLock<Option<WorkspaceSnapshot>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
            active_connections: AtomicUsize::new(0),
            active_workspace: RwLock::new(None),
        }
    }

    pub fn uptime_seconds(&self) -> u64 {
        Instant::now()
            .checked_duration_since(self.start)
            .unwrap_or_default()
            .as_secs()
    }

    pub fn active_connections(&self) -> usize {
        self.active_connections.load(Ordering::SeqCst)
    }

    pub async fn active_workspaces(&self) -> usize {
        if self.active_workspace.read().await.is_some() {
            1
        } else {
            0
        }
    }

    pub async fn snapshot_workspace(&self) -> Option<WorkspaceSnapshot> {
        self.active_workspace.read().await.clone()
    }

    pub async fn set_workspace(&self, snapshot: WorkspaceSnapshot) {
        *self.active_workspace.write().await = Some(snapshot);
    }

    pub fn increment_connections(&self) {
        self.active_connections.fetch_add(1, Ordering::SeqCst);
    }

    pub fn decrement_connections(&self) {
        self.active_connections.fetch_sub(1, Ordering::SeqCst);
    }
}

pub type SharedState = Arc<AppState>;
