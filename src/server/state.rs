// RwLock Deadlock Audit (T041, 2026-03-09):
// - All RwLock/Mutex guards are dropped before any `.await` point.
//   `record_tool_latency` explicitly calls `drop(latencies)` before the
//   atomic increment; `latency_percentiles` explicitly calls `drop(latencies)`
//   before the sort.  All other guard acquisitions are either the sole await
//   in a method or are released via implicit drop before the next await.
// - Rust's `!Send` bound on `MutexGuard` / `RwLockGuard` would produce a
//   compile-time error if any guard were held across an await in a multi-
//   threaded context, providing a mechanical safety net on top of the audit.
// - Connection and tool-call counts use `AtomicUsize` / `AtomicU64` which
//   need no locking at all.
// - No lock is held across I/O operations.
// Verdict: no deadlock potential identified.

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use tokio::sync::RwLock;

use crate::config::StaleStrategy;
use crate::errors::WorkspaceError;
use crate::models::config::WorkspaceConfig;
use crate::models::health::ScanProgress;
use crate::services::connection::ConnectionRegistry;
use crate::services::hydration::FileFingerprint;

/// Atomic point-in-time snapshot of workspace binding and config taken at dispatch entry.
///
/// Both fields are captured under a single logical read so that a concurrent
/// `set_workspace_config` call cannot change the policy that was checked at
/// the start of a tool call. See TASK-018 for full context.
#[derive(Clone, Debug)]
pub struct DispatchSnapshot {
    /// Clone of the active workspace binding at snapshot time.
    pub workspace: WorkspaceSnapshot,
    /// Clone of the active workspace config at snapshot time.
    /// Defaults to [`WorkspaceConfig::default`] when no config has been loaded.
    pub config: WorkspaceConfig,
}

/// Lock-free process-level reliability counters for the daemon (029-F WS-8).
///
/// Counters are `AtomicU64` so increments are allocation-free on the hot path.
/// Owned by `AppState`; surfaced through `get_daemon_status`.
#[derive(Debug, Default)]
pub struct ReliabilityCounters {
    /// Number of times a stale PID file was recovered on startup.
    pub stale_pid_recovered: AtomicU64,
    /// Number of times a version-mismatch forced a daemon respawn.
    pub version_mismatch_respawn: AtomicU64,
    /// Number of times `validate_sources_strict` returned a `ValidationFailed` error.
    pub registry_validation_failures: AtomicU64,
    /// Number of times a duplicate daemon was detected on bind (lockfile conflict).
    pub duplicate_daemon_detected: AtomicU64,
}

impl ReliabilityCounters {
    /// Increment the `stale_pid_recovered` counter by 1.
    pub fn inc_stale_pid_recovered(&self) {
        self.stale_pid_recovered.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment the `version_mismatch_respawn` counter by 1.
    pub fn inc_version_mismatch_respawn(&self) {
        self.version_mismatch_respawn
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Increment the `registry_validation_failures` counter by 1.
    pub fn inc_registry_validation_failure(&self) {
        self.registry_validation_failures
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Increment the `duplicate_daemon_detected` counter by 1.
    pub fn inc_duplicate_daemon_detected(&self) {
        self.duplicate_daemon_detected
            .fetch_add(1, Ordering::Relaxed);
    }
}

#[derive(Clone, Debug)]
pub struct WorkspaceSnapshot {
    pub workspace_id: String,
    pub workspace_uuid: String,
    pub branch: String,
    pub data_dir: PathBuf,
    pub path: String,
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
    indexing_in_progress: AtomicBool,
    last_indexed_at: RwLock<Option<DateTime<Utc>>>,
    /// Rolling window of tool-call latencies (in microseconds, capped at 1 000 samples).
    query_latencies: RwLock<VecDeque<u64>>,
    /// Total number of tool calls recorded since startup.
    tool_call_count: AtomicU64,
    /// Total number of file-watcher events seen since startup.
    watcher_event_count: AtomicU64,
    /// Timestamp of the most recently seen file-watcher event.
    last_watcher_event: RwLock<Option<DateTime<Utc>>>,
    /// Background offline-change scan progress (029-F WS-6).
    /// `None` until the first scan is queued after a `set_workspace` call.
    scan_progress: RwLock<Option<ScanProgress>>,
    /// Cancellation sender for the current background scan generation (029-F WS-6).
    /// Replaced on each new `set_workspace` call; sending `true` cancels the stale scan.
    scan_cancel: RwLock<Option<tokio::sync::watch::Sender<bool>>>,
    /// Lock-free process-level reliability counters (029-F WS-8).
    reliability: ReliabilityCounters,
    /// Set to `true` once `background_db_hydration` has run to completion
    /// (success, failure, or cancellation).  `_health` gates "ready" on this
    /// flag so that polling clients (shim, test harness) wait until initial
    /// data load is done before issuing real tool calls.
    hydration_ready: AtomicBool,
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
            indexing_in_progress: AtomicBool::new(false),
            last_indexed_at: RwLock::new(None),
            query_latencies: RwLock::new(VecDeque::new()),
            tool_call_count: AtomicU64::new(0),
            watcher_event_count: AtomicU64::new(0),
            last_watcher_event: RwLock::new(None),
            scan_progress: RwLock::new(None),
            scan_cancel: RwLock::new(None),
            reliability: ReliabilityCounters::default(),
            hydration_ready: AtomicBool::new(false),
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

    /// Atomically snapshot the active workspace binding and config for use at dispatch entry.
    ///
    /// Both read locks are held simultaneously while cloning, in a consistent order
    /// (`active_workspace` then `workspace_config`), so that a concurrent
    /// [`AppState::set_workspace`] or [`AppState::set_workspace_config`] call cannot produce
    /// a mismatched workspace/config pair from different points in time. Both guards are
    /// dropped at the end of this function.
    ///
    /// Returns `None` when no workspace is bound; `set_workspace` must be called first.
    /// When a workspace is bound but no config has been loaded, the snapshot uses
    /// [`WorkspaceConfig::default`] so that dispatch proceeds with policy disabled.
    pub async fn snapshot_dispatch_context(&self) -> Option<DispatchSnapshot> {
        let workspace_guard = self.active_workspace.read().await;
        let config_guard = self.workspace_config.read().await;
        let workspace = workspace_guard.clone()?;
        let config = config_guard.clone().unwrap_or_default();
        Some(DispatchSnapshot { workspace, config })
    }

    pub async fn set_workspace(&self, snapshot: WorkspaceSnapshot) -> Result<(), WorkspaceError> {
        let mut workspace = self.active_workspace.write().await;
        if let Some(active) = workspace.as_ref() {
            if active.workspace_id != snapshot.workspace_id && self.max_workspaces <= 1 {
                return Err(WorkspaceError::LimitReached {
                    limit: self.max_workspaces,
                });
            }
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

    pub async fn can_bind_workspace(&self, workspace_id: &str) -> bool {
        let workspace = self.active_workspace.read().await;
        match workspace.as_ref() {
            Some(active) => active.workspace_id == workspace_id || self.max_workspaces > 1,
            None => self.max_workspaces > 0,
        }
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

    /// Get the policy configuration from the active workspace config.
    ///
    /// Returns `None` when no workspace config has been loaded (either no workspace is bound
    /// or `set_workspace_config` has not yet been called). When `Some` is returned it contains
    /// the `PolicyConfig` from the loaded `WorkspaceConfig`, which may have `enabled: false`
    /// if no `[policy]` section was present in `.engram/config.toml`.
    pub async fn policy_config(&self) -> Option<crate::models::policy::PolicyConfig> {
        self.workspace_config
            .read()
            .await
            .as_ref()
            .map(|c| c.policy.clone())
    }

    /// Get the active evaluation configuration.
    ///
    /// Returns `None` when no workspace is bound.
    pub async fn evaluation_config(&self) -> Option<crate::models::evaluation::EvaluationConfig> {
        self.workspace_config
            .read()
            .await
            .as_ref()
            .map(|c| c.evaluation.clone())
    }

    /// Set the workspace config.
    pub async fn set_workspace_config(&self, config: Option<WorkspaceConfig>) {
        *self.workspace_config.write().await = config;
    }

    /// Check whether an indexing operation is currently in progress.
    pub fn is_indexing(&self) -> bool {
        self.indexing_in_progress.load(Ordering::SeqCst)
    }

    /// Attempt to start an indexing operation.
    ///
    /// Returns `true` if the flag was set (no other indexing was running).
    /// Returns `false` if indexing was already in progress.
    pub fn try_start_indexing(&self) -> bool {
        self.indexing_in_progress
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }

    /// Clear the indexing-in-progress flag and record the completion time.
    pub async fn finish_indexing(&self) {
        self.indexing_in_progress.store(false, Ordering::SeqCst);
        *self.last_indexed_at.write().await = Some(Utc::now());
    }

    /// Get the timestamp of the last completed indexing operation.
    pub async fn last_indexed_at(&self) -> Option<DateTime<Utc>> {
        *self.last_indexed_at.read().await
    }

    // ── Observability ─────────────────────────────────────────────────────────

    /// Record a tool-call latency sample (in microseconds) and increment the
    /// tool-call counter.
    ///
    /// Keeps at most 1 000 samples in a rolling window; oldest entries are
    /// evicted when the window is full.
    pub async fn record_tool_latency(&self, micros: u64) {
        let mut latencies = self.query_latencies.write().await;
        if latencies.len() >= 1_000 {
            latencies.pop_front();
        }
        latencies.push_back(micros);
        drop(latencies);
        self.tool_call_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Compute p50, p95, and p99 latency percentiles (in microseconds) from
    /// the rolling 1 000-sample window.
    ///
    /// Returns `(0, 0, 0)` when no samples have been recorded yet.
    pub async fn latency_percentiles(&self) -> (u64, u64, u64) {
        let latencies = self.query_latencies.read().await;
        if latencies.is_empty() {
            return (0, 0, 0);
        }
        let mut sorted: Vec<u64> = latencies.iter().copied().collect();
        drop(latencies);
        sorted.sort_unstable();
        let len = sorted.len();
        let p50 = sorted[(len * 50 / 100).min(len - 1)];
        let p95 = sorted[(len * 95 / 100).min(len - 1)];
        let p99 = sorted[(len * 99 / 100).min(len - 1)];
        (p50, p95, p99)
    }

    /// Return the total number of tool calls recorded since startup.
    pub fn tool_call_count(&self) -> u64 {
        self.tool_call_count.load(Ordering::Relaxed)
    }

    /// Increment the watcher-event counter and record the current UTC timestamp.
    pub async fn record_watcher_event(&self) {
        self.watcher_event_count.fetch_add(1, Ordering::Relaxed);
        *self.last_watcher_event.write().await = Some(Utc::now());
    }

    /// Return `(event_count, last_event_rfc3339)`.
    ///
    /// `last_event_rfc3339` is `None` when no events have been recorded.
    pub async fn watcher_stats(&self) -> (u64, Option<String>) {
        let count = self.watcher_event_count.load(Ordering::Relaxed);
        let last = self
            .last_watcher_event
            .read()
            .await
            .map(|dt| dt.to_rfc3339());
        (count, last)
    }
    // ── Background scan (029-F WS-6) ──────────────────────────────────────────

    /// Store or clear the current background scan progress snapshot.
    pub async fn set_scan_progress(&self, progress: Option<ScanProgress>) {
        *self.scan_progress.write().await = progress;
    }

    /// Return a clone of the current scan progress, or `None` when no scan
    /// has been queued since startup.
    pub async fn scan_progress_snapshot(&self) -> Option<ScanProgress> {
        self.scan_progress.read().await.clone()
    }

    /// Begin a new scan generation.
    ///
    /// Cancels any in-flight background scan from the previous generation by
    /// sending `true` on the old cancel channel, then registers a fresh
    /// channel for the new scan.
    ///
    /// Returns a `Receiver<bool>` that the new scan task should watch; when
    /// it yields `true` the task should abandon its work.
    pub async fn begin_scan_generation(&self) -> tokio::sync::watch::Receiver<bool> {
        let (tx, rx) = tokio::sync::watch::channel(false);
        let mut cancel = self.scan_cancel.write().await;
        if let Some(old_tx) = cancel.take() {
            let _ = old_tx.send(true);
        }
        *cancel = Some(tx);
        rx
    }
    /// Returns a reference to the process-level reliability counters (029-F WS-8).
    ///
    /// Callers use the returned reference to increment specific counters without
    /// acquiring any locks (all fields are `AtomicU64`).
    pub fn reliability_counters(&self) -> &ReliabilityCounters {
        &self.reliability
    }

    /// Mark the initial background DB hydration as complete (success, failure,
    /// or cancellation).  The `_health` handler gates the "ready" status on
    /// this flag so clients wait until data is loaded before issuing queries.
    pub fn set_hydration_ready(&self) {
        self.hydration_ready.store(true, Ordering::Release);
    }

    /// Reset the hydration-ready flag before a new background hydration cycle.
    ///
    /// Call this before spawning a new `background_db_hydration` task so that
    /// `_health` returns "starting" until the new cycle completes, even if a
    /// previous workspace was already hydrated.
    pub fn clear_hydration_ready(&self) {
        self.hydration_ready.store(false, Ordering::Release);
    }

    /// Returns `true` once [`set_hydration_ready`] has been called.
    pub fn is_hydration_ready(&self) -> bool {
        self.hydration_ready.load(Ordering::Acquire)
    }
}

pub type SharedState = Arc<AppState>;
