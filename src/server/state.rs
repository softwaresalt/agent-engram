use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use tokio::sync::RwLock;

use crate::config::StaleStrategy;
use crate::errors::WorkspaceError;
use crate::models::config::WorkspaceConfig;
use crate::services::connection::ConnectionRegistry;
use crate::services::hydration::FileFingerprint;

#[derive(Clone, Debug)]
pub struct WorkspaceSnapshot {
    pub workspace_id: String,
    pub path: String,
    pub task_count: u64,
    pub context_count: u64,
    pub last_flush: Option<String>,
    pub stale_files: bool,
    pub connection_count: usize,
    pub file_mtimes: HashMap<String, FileFingerprint>,
}

/// Sliding-window rate limiter for SSE connections (FR-025/T118).
///
/// Tracks connection timestamps and rejects new connections when the
/// maximum per window is exceeded. Uses wall-clock time (`std::time::Instant`)
/// so it is unaffected by tokio time mocking in tests.
#[derive(Debug)]
pub struct RateLimiter {
    max_per_window: usize,
    window: Duration,
    timestamps: tokio::sync::Mutex<Vec<Instant>>,
}

impl RateLimiter {
    /// Create a rate limiter allowing `max_per_window` connections per `window_secs`.
    pub fn new(max_per_window: usize, window_secs: u64) -> Self {
        Self {
            max_per_window,
            window: Duration::from_secs(window_secs),
            timestamps: tokio::sync::Mutex::new(Vec::new()),
        }
    }

    /// Check whether a new connection is allowed and record its timestamp.
    ///
    /// Returns `true` if within limits, `false` if rate exceeded.
    pub async fn check_and_record(&self) -> bool {
        let mut ts = self.timestamps.lock().await;
        let now = Instant::now();
        if let Some(cutoff) = now.checked_sub(self.window) {
            ts.retain(|t| *t > cutoff);
        }
        if ts.len() >= self.max_per_window {
            return false;
        }
        ts.push(now);
        true
    }
}

#[derive(Debug)]
pub struct AppState {
    start: Instant,
    active_connections: AtomicUsize,
    active_workspace: RwLock<Option<WorkspaceSnapshot>>,
    workspace_config: RwLock<Option<WorkspaceConfig>>,
    max_workspaces: usize,
    stale_strategy: StaleStrategy,
    connection_registry: ConnectionRegistry,
    rate_limiter: RateLimiter,
}

impl AppState {
    pub fn new(max_workspaces: usize) -> Self {
        Self::with_options(max_workspaces, StaleStrategy::Warn, 20, 60)
    }

    pub fn with_stale_strategy(max_workspaces: usize, stale_strategy: StaleStrategy) -> Self {
        Self::with_options(max_workspaces, stale_strategy, 20, 60)
    }

    /// Create `AppState` with full configuration including rate limit parameters.
    pub fn with_options(
        max_workspaces: usize,
        stale_strategy: StaleStrategy,
        rate_limit_max: usize,
        rate_limit_window_secs: u64,
    ) -> Self {
        Self {
            start: Instant::now(),
            active_connections: AtomicUsize::new(0),
            active_workspace: RwLock::new(None),
            workspace_config: RwLock::new(None),
            max_workspaces,
            stale_strategy,
            connection_registry: ConnectionRegistry::new(),
            rate_limiter: RateLimiter::new(rate_limit_max, rate_limit_window_secs),
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
        usize::from(self.active_workspace.read().await.is_some())
    }

    pub async fn snapshot_workspace(&self) -> Option<WorkspaceSnapshot> {
        self.active_workspace.read().await.clone()
    }

    pub async fn set_workspace(&self, snapshot: WorkspaceSnapshot) -> Result<(), WorkspaceError> {
        let mut workspace = self.active_workspace.write().await;
        let active = usize::from(workspace.is_some());
        if active >= self.max_workspaces {
            return Err(WorkspaceError::LimitReached {
                limit: self.max_workspaces,
            });
        }

        *workspace = Some(snapshot);
        Ok(())
    }

    pub fn increment_connections(&self) {
        self.active_connections.fetch_add(1, Ordering::SeqCst);
    }

    pub fn decrement_connections(&self) {
        self.active_connections.fetch_sub(1, Ordering::SeqCst);
    }

    pub fn max_workspaces(&self) -> usize {
        self.max_workspaces
    }

    pub async fn has_workspace_capacity(&self) -> bool {
        self.active_workspaces().await < self.max_workspaces
    }

    pub fn stale_strategy(&self) -> StaleStrategy {
        self.stale_strategy
    }

    pub async fn update_workspace<F>(&self, f: F) -> Result<(), WorkspaceError>
    where
        F: FnOnce(&mut WorkspaceSnapshot),
    {
        let mut workspace = self.active_workspace.write().await;
        if let Some(snapshot) = workspace.as_mut() {
            f(snapshot);
            Ok(())
        } else {
            Err(WorkspaceError::NotSet)
        }
    }

    /// Register a new SSE connection in the registry (US5/T091).
    pub async fn register_connection(&self, id: String) {
        self.connection_registry.register(id).await;
        self.increment_connections();
    }

    /// Unregister an SSE connection on disconnect (US5/T095).
    pub async fn unregister_connection(&self, id: &str) {
        self.connection_registry.unregister(id).await;
        self.decrement_connections();
    }

    /// Check connection rate limit (FR-025/T118).
    pub async fn check_rate_limit(&self) -> bool {
        self.rate_limiter.check_and_record().await
    }

    /// Access the connection registry.
    pub fn connection_registry(&self) -> &ConnectionRegistry {
        &self.connection_registry
    }

    /// Get the current workspace config.
    pub async fn workspace_config(&self) -> Option<WorkspaceConfig> {
        self.workspace_config.read().await.clone()
    }

    /// Set the workspace config.
    pub async fn set_workspace_config(&self, config: Option<WorkspaceConfig>) {
        *self.workspace_config.write().await = config;
    }
}

pub type SharedState = Arc<AppState>;
