// RwLock Deadlock Audit (T041, 2026-03-09; updated 092.001-T, 2026-07-17):
// - Most RwLock/Mutex guards are dropped before any `.await` point.
//   `record_tool_latency` explicitly calls `drop(latencies)` before the
//   atomic increment; `latency_percentiles` explicitly calls `drop(latencies)`
//   before the sort.  Most other guard acquisitions are either the sole await
//   in a method or are released via implicit drop before the next await.
// - Intentional paired-lock exception: to snapshot/publish the
//   (`active_workspace`, `workspace_config`) pair atomically,
//   `snapshot_dispatch_context` and `snapshot_workspace_and_config` (read
//   guards) and `set_workspace_and_config` (write guards) hold the
//   `active_workspace` guard across acquisition of the `workspace_config` guard.
//   All use the same lock order (`active_workspace` then `workspace_config`),
//   and no code path acquires the two locks in the reverse order, so this
//   pairing cannot deadlock. `tokio`'s async guards are `Send`, so holding one
//   across an `.await` is permitted (unlike `std::sync` guards);
//   deadlock-freedom here rests on the consistent lock order, not on a `!Send`
//   compile-time net.
// - Connection and tool-call counts use `AtomicUsize` / `AtomicU64` which
//   need no locking at all.
// - No lock is held across an I/O operation; the only await performed while a
//   guard is held is the paired lock acquisition described above.
// Verdict: no deadlock potential identified.

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
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
/// `set_workspace_and_config` (or `set_workspace_config`) call cannot change
/// the workspace/config pair that was checked at the start of a tool call.
/// See TASK-018 for full context.
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

/// Generation-tagged pending-sync queue state (105.001-T / R1).
///
/// Packs the three coalesced-sync request bits with the generation that owns
/// them, so a generation-scoped clear can distinguish an older generation's
/// stale request from a newer generation's live one. Guarded by a single mutex
/// on [`AppState`] so every read/write of `{generation, flags}` is atomic (H1):
/// a concurrent drain can never observe `pending_sync` set with a stale/missing
/// companion bit.
#[derive(Debug, Default, Clone, Copy)]
struct PendingSyncState {
    /// Generation that published the currently-queued bits (`0` = none yet).
    generation: u64,
    /// Packed request bits:
    /// `PENDING_SYNC_BIT | *_REVALIDATE_BIT | *_BACKFILL_PYTHON_BIT`.
    flags: u8,
}

impl PendingSyncState {
    /// OR `bits` into the queue, adopting `current_generation` only when the
    /// queue is currently empty (a fresh arm); a non-empty queue keeps its
    /// existing owner generation so coalesced companion bits stay grouped with
    /// the request that armed them (drain re-queue safety, 101.002-T).
    fn arm(&mut self, current_generation: u64, bits: u8) {
        if self.flags == 0 {
            self.generation = current_generation;
        }
        self.flags |= bits;
    }

    /// Publish `bits` for `current_generation`. A publish from a *newer*
    /// generation than the current owner REPLACES the stale older-generation
    /// bits (so an old binding's `--revalidate` / `--backfill-python-canonical`
    /// never leaks into a new binding's routine sync — N2/AC4); the same
    /// generation coalesces (sticky OR). `current_generation` is monotonic and
    /// captured under the same lock, so it is never below the owner.
    fn publish(&mut self, current_generation: u64, bits: u8) {
        if current_generation > self.generation {
            self.generation = current_generation;
            self.flags = bits;
        } else {
            self.flags |= bits;
        }
    }

    /// Test-and-clear `bit`, returning whether it was set.
    fn take(&mut self, bit: u8) -> bool {
        let had = self.flags & bit != 0;
        self.flags &= !bit;
        had
    }

    /// Non-consuming peek at `bit`.
    fn has(&self, bit: u8) -> bool {
        self.flags & bit != 0
    }

    /// Clear all bits iff the caller's generation still owns the queue
    /// (`owner <= caller_generation`). A newer generation's live request is
    /// preserved (N1/AC2).
    fn clear_for_generation(&mut self, caller_generation: u64) {
        if self.generation <= caller_generation {
            self.flags = 0;
        }
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
    /// Generation-tagged pending-sync request queue (104.002-T / 105.001-T):
    /// bit 0 = `pending_sync` (a `sync_workspace` was requested while indexing
    /// was active, drained after `finish_indexing()`); bit 1 = revalidation
    /// companion (`--revalidate-code-graph`); bit 2 = Python-canonical backfill
    /// companion (`--backfill-python-canonical`); tagged with the owning
    /// generation.
    ///
    /// A single mutex guards `{generation, flags}` so publish / clear / drain
    /// updates are atomic (H1): no interleaving with a concurrent publish can
    /// leave a lone companion bit without its owning `pending_sync`. Companion
    /// bits are published BEFORE `pending_sync` is consumed (N3) and drained
    /// AFTER the indexing lock is acquired (101.002-T). The generation tag lets
    /// [`AppState::clear_pending_sync_for_generation`] wipe only the owning
    /// generation's request, so a newer generation's queued intent published in
    /// the `set_workspace` cancel race window survives an older generation's
    /// clear (105.001-T / R1). See the `PENDING_SYNC_*_BIT` associated constants.
    pending_sync: Mutex<PendingSyncState>,
    /// Monotonic sync-generation counter, incremented on each
    /// [`AppState::begin_scan_generation`] (each `set_workspace` rebind). Reads
    /// give the current generation a published request is tagged with; the value
    /// returned to the hydration task lets its cancel / DB-fail clear identify
    /// which generation it owns (105.001-T / R1).
    sync_generation: AtomicU64,
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
            pending_sync: Mutex::new(PendingSyncState::default()),
            sync_generation: AtomicU64::new(0),
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

    /// Atomically snapshot the active workspace binding and loaded config.
    ///
    /// This reader acquires `active_workspace` before `workspace_config`, matching
    /// the paired-lock order used by [`AppState::set_workspace_and_config`] and
    /// [`AppState::snapshot_dispatch_context`]. Holding both read guards while
    /// cloning prevents a concurrent atomic writer from being observed as a
    /// mismatched workspace/config pair.
    ///
    /// Returns `None` when either value is absent. Background paths use that
    /// gating to skip work until both the workspace and its config are loaded;
    /// unlike [`AppState::snapshot_dispatch_context`], this method never
    /// substitutes [`WorkspaceConfig::default`]. Both guards are dropped before
    /// the caller can perform any I/O or other awaited work.
    pub async fn snapshot_workspace_and_config(
        &self,
    ) -> Option<(WorkspaceSnapshot, WorkspaceConfig)> {
        // Lock-order invariant: when workspace and config are held together,
        // acquire `active_workspace` first, then `workspace_config`.
        let workspace_guard = self.active_workspace.read().await;
        let config_guard = self.workspace_config.read().await;
        let workspace = workspace_guard.clone()?;
        let config = config_guard.clone()?;
        Some((workspace, config))
    }

    /// Atomically snapshot the active workspace binding and config for use at dispatch entry.
    ///
    /// Both read locks are held simultaneously while cloning, in a consistent order
    /// (`active_workspace` then `workspace_config`), so that a concurrent
    /// [`AppState::set_workspace_and_config`] call cannot produce a mismatched
    /// workspace/config pair from different points in time. Both guards are dropped at
    /// the end of this function.
    ///
    /// Returns `None` when no workspace is bound; `set_workspace` or
    /// `set_workspace_and_config` must be called first.
    /// When a workspace is bound but no config has been loaded, the snapshot uses
    /// [`WorkspaceConfig::default`] so that dispatch proceeds with policy disabled.
    pub async fn snapshot_dispatch_context(&self) -> Option<DispatchSnapshot> {
        // Lock-order invariant: when workspace and config are held together,
        // acquire `active_workspace` first, then `workspace_config`.
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

    /// Atomically publish a workspace binding and its config.
    ///
    /// Performs the same workspace capacity check as [`AppState::set_workspace`] before
    /// mutating either value, so a [`WorkspaceError::LimitReached`] leaves the prior
    /// workspace and config unchanged.
    pub async fn set_workspace_and_config(
        &self,
        snapshot: WorkspaceSnapshot,
        config: Option<WorkspaceConfig>,
    ) -> Result<(), WorkspaceError> {
        // Lock-order invariant: when workspace and config are held together,
        // acquire `active_workspace` first, then `workspace_config`.
        let mut workspace = self.active_workspace.write().await;
        let mut workspace_config = self.workspace_config.write().await;
        if let Some(active) = workspace.as_ref() {
            if active.workspace_id != snapshot.workspace_id && self.max_workspaces <= 1 {
                return Err(WorkspaceError::LimitReached {
                    limit: self.max_workspaces,
                });
            }
        }

        *workspace = Some(snapshot);
        *workspace_config = config;
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

    /// `pending_sync` bit in [`PendingSyncState::flags`] (104.002-T / 105.001-T).
    const PENDING_SYNC_BIT: u8 = 0b001;
    /// Revalidation companion bit in [`PendingSyncState::flags`].
    const PENDING_SYNC_REVALIDATE_BIT: u8 = 0b010;
    /// Python-canonical backfill companion bit in [`PendingSyncState::flags`].
    const PENDING_SYNC_BACKFILL_PYTHON_BIT: u8 = 0b100;

    /// Lock the pending-sync queue, recovering from a poisoned mutex.
    ///
    /// The critical sections are tiny (`Copy` bit/counter twiddling with no
    /// `.await` and no panics), so poisoning is not expected; recovering via
    /// [`std::sync::PoisonError::into_inner`] keeps the daemon live rather than
    /// propagating a panic, and satisfies the no-`unwrap`/`expect` rule.
    fn lock_pending_sync(&self) -> std::sync::MutexGuard<'_, PendingSyncState> {
        self.pending_sync
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// The current (latest) sync generation — the value most recently returned
    /// by [`AppState::begin_scan_generation`]. `0` before the first rebind.
    pub fn current_sync_generation(&self) -> u64 {
        self.sync_generation.load(Ordering::SeqCst)
    }

    /// Signal that a sync was requested while indexing was in progress.
    ///
    /// The next caller of [`finish_indexing`] that drains this flag (via
    /// [`take_pending_sync`]) is responsible for running one coalesced sync.
    /// A fresh arm adopts the current generation; a drain re-queue coalesces
    /// into the already-queued generation's request (keeps companion bits).
    pub fn set_pending_sync(&self) {
        let generation = self.current_sync_generation();
        self.lock_pending_sync()
            .arm(generation, Self::PENDING_SYNC_BIT);
    }

    /// Atomically publish a queued coalesced sync request — the requested
    /// companion bits AND `pending_sync` in ONE locked update (104.002-T),
    /// tagged with the current sync generation (105.001-T / R1).
    ///
    /// Because the whole update happens under the pending-sync lock, a
    /// concurrent [`clear_pending_sync_for_generation`](Self::clear_pending_sync_for_generation)
    /// can never interleave *between* a companion write and the `pending_sync`
    /// write: the flags are observed either fully published or (if a
    /// same-or-newer clear wins the total order) fully absent — never
    /// `pending_sync == true` with a stale/missing companion bit (N3/H1). A
    /// publish from a *newer* generation than the queue's current owner replaces
    /// the stale older-generation bits rather than OR-ing into them, so an old
    /// binding's `--revalidate` never leaks into a new binding's routine sync
    /// (N2/AC4).
    pub fn publish_pending_sync(&self, revalidate: bool, backfill_python: bool) {
        let mut mask = Self::PENDING_SYNC_BIT;
        if revalidate {
            mask |= Self::PENDING_SYNC_REVALIDATE_BIT;
        }
        if backfill_python {
            mask |= Self::PENDING_SYNC_BACKFILL_PYTHON_BIT;
        }
        let generation = self.current_sync_generation();
        self.lock_pending_sync().publish(generation, mask);
    }

    /// Publish a queued sync request AND backstop the producer/consumer drain
    /// lost-wakeup (105.002-T / R2).
    ///
    /// The bounded snapshot-loop drain (`drain_pending_sync_to_completion`)
    /// narrows but cannot close the window where a sync caller fails
    /// [`try_start_indexing`](Self::try_start_indexing) while a holder owns the
    /// lock, is descheduled BEFORE publishing its intent, and resumes only AFTER
    /// the holder ran its final [`has_pending_sync`](Self::has_pending_sync) peek
    /// and exited — stranding the request until an external index/sync/watcher
    /// tick.
    ///
    /// This closes it with an atomic producer→lock-holder handoff. After
    /// publishing the intent (under the pending mutex, so it is generation-tagged
    /// and cannot be torn — R1), it RE-ATTEMPTS the indexing lock:
    ///
    /// * Returns `true` iff the re-attempt ACQUIRED the lock. That proves the
    ///   prior holder had already released (and may already have run its final
    ///   drain-check and exited): the caller is now the **guaranteed finisher**
    ///   and MUST drain the queued request itself.
    /// * Returns `false` iff the lock is still held. The current holder is then
    ///   guaranteed to observe the just-published intent on its release-check.
    ///
    /// # Why the handoff has no lost wakeup
    ///
    /// The publish `P2` is sequenced-before this re-attempt `P3`. A holder's
    /// release `finish_indexing` (a SeqCst `store(false)`) is sequenced-before
    /// its drain-loop `has_pending_sync` read `chk`. Suppose the bad outcome:
    /// the holder exits without draining (`chk` reads `false`) AND `P3` fails
    /// (reads `indexing == true`). `P3` reading `true` places `P3` before
    /// `store(false)` in the SeqCst total order, giving
    /// `P2 →sb P3 →S store(false) →sb chk`, i.e. `P2` happens-before `chk` — which
    /// forces `chk` to observe the published bit (`true`), contradicting the
    /// assumption. So the bad interleaving is impossible: either the holder sees
    /// the intent (`false` branch) or the producer re-acquires (`true` branch).
    pub fn publish_pending_sync_and_try_reacquire(
        &self,
        revalidate: bool,
        backfill_python: bool,
    ) -> bool {
        self.publish_pending_sync(revalidate, backfill_python);
        // Backstop the drain lost-wakeup: re-attempt the indexing lock. Success
        // proves the prior holder already released, so the caller is now the
        // guaranteed finisher (see the ordering argument above).
        self.try_start_indexing()
    }

    /// Atomically clear and return the pending-sync flag.
    ///
    /// Returns `true` once after a successful publish/set of the pending bit,
    /// then `false` until set again.
    pub fn take_pending_sync(&self) -> bool {
        self.lock_pending_sync().take(Self::PENDING_SYNC_BIT)
    }

    /// Non-consuming peek at the pending-sync flag.
    ///
    /// Unlike [`take_pending_sync`], this does not clear the flag. Used by the
    /// bounded loop-drain (`drain_pending_sync_to_completion`, 104.002-T) to
    /// decide whether another drain pass is required after a re-arm.
    pub fn has_pending_sync(&self) -> bool {
        self.lock_pending_sync().has(Self::PENDING_SYNC_BIT)
    }

    /// Generation-scoped clear of `pending_sync` and both companion bits
    /// (105.001-T / R1; supersedes the 104.002-T whole-queue `store(0)` wipe).
    ///
    /// Used on the hydration cancellation and DB-connect-failure paths, which
    /// release the indexing lock WITHOUT draining. The clear zeroes the queue
    /// ONLY when the caller's generation still owns it (`owner <= caller_gen`).
    /// A request published by a *newer* generation in the `set_workspace` cancel
    /// race window (the new snapshot is installed BEFORE the old hydration is
    /// cancelled) therefore SURVIVES an older generation's clear instead of being
    /// silently dropped (N1/AC2). Stale bits from the caller's own (or an older)
    /// generation are still cleared so they cannot leak into a newer generation's
    /// routine sync (N2/AC4). The new generation's own scan re-queues whatever it
    /// actually needs (N5).
    pub fn clear_pending_sync_for_generation(&self, caller_generation: u64) {
        self.lock_pending_sync()
            .clear_for_generation(caller_generation);
    }

    /// Mark the pending coalesced sync as a *revalidation* sync (101.002-T).
    ///
    /// Published BEFORE [`set_pending_sync`] when a `--revalidate-code-graph`
    /// request is queued because indexing is active, so a concurrent drain
    /// cannot observe `pending_sync == true` while this companion bit is still
    /// false. Sticky OR-semantics within a generation: once set it stays set
    /// until drained, so any queued revalidation upgrades the coalesced drain.
    pub fn set_pending_sync_revalidate(&self) {
        let generation = self.current_sync_generation();
        self.lock_pending_sync()
            .arm(generation, Self::PENDING_SYNC_REVALIDATE_BIT);
    }

    /// Atomically clear and return the pending-sync-revalidate flag.
    ///
    /// The coalesced drain reads this AFTER acquiring the indexing lock so that,
    /// if the lock grab loses a race and the sync is re-queued, the flag is left
    /// set for the next drain.
    pub fn take_pending_sync_revalidate(&self) -> bool {
        self.lock_pending_sync()
            .take(Self::PENDING_SYNC_REVALIDATE_BIT)
    }

    /// Mark the pending coalesced sync as a Python-canonical *backfill* sync
    /// (101.002-T; mirrors [`set_pending_sync_revalidate`]).
    ///
    /// Published BEFORE `set_pending_sync` so a concurrent drain cannot observe
    /// `pending_sync == true` while this companion bit is still false.
    pub fn set_pending_sync_backfill_python(&self) {
        let generation = self.current_sync_generation();
        self.lock_pending_sync()
            .arm(generation, Self::PENDING_SYNC_BACKFILL_PYTHON_BIT);
    }

    /// Atomically clear and return the pending-sync-backfill-python flag.
    ///
    /// Read AFTER acquiring the indexing lock (same re-queue safety as
    /// [`take_pending_sync_revalidate`]).
    pub fn take_pending_sync_backfill_python(&self) -> bool {
        self.lock_pending_sync()
            .take(Self::PENDING_SYNC_BACKFILL_PYTHON_BIT)
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
    /// Increments the monotonic [`sync_generation`](Self::current_sync_generation)
    /// counter (so pending-sync requests published after this rebind are tagged
    /// with the new generation), cancels any in-flight background scan from the
    /// previous generation by sending `true` on the old cancel channel, then
    /// registers a fresh channel for the new scan.
    ///
    /// Returns `(generation, Receiver<bool>)`: the new generation number the
    /// hydration task owns (used by its cancel / DB-fail path to call
    /// [`clear_pending_sync_for_generation`](Self::clear_pending_sync_for_generation)
    /// so it never wipes a *newer* generation's queued request — 105.001-T / R1),
    /// and a `Receiver<bool>` the new scan task should watch; when it yields
    /// `true` the task should abandon its work.
    pub async fn begin_scan_generation(&self) -> (u64, tokio::sync::watch::Receiver<bool>) {
        // Bump the generation FIRST, while holding the cancel lock, so the new
        // generation number is published before the old scan is signalled to
        // cancel. A request that arrives after this point is tagged with the
        // new generation and thus survives the old generation's clear.
        let mut cancel = self.scan_cancel.write().await;
        let generation = self.sync_generation.fetch_add(1, Ordering::SeqCst) + 1;
        let (tx, rx) = tokio::sync::watch::channel(false);
        if let Some(old_tx) = cancel.take() {
            let _ = old_tx.send(true);
        }
        *cancel = Some(tx);
        (generation, rx)
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
