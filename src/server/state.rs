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
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use tokio::sync::futures::OwnedNotified;
use tokio::sync::{Notify, RwLock, watch};

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

/// Exact private identity of a workspace binding.
#[allow(dead_code)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct BindingIdentity {
    workspace_uuid: String,
    workspace_id: String,
}

#[allow(dead_code)]
impl BindingIdentity {
    fn unbound() -> Self {
        Self {
            workspace_uuid: String::new(),
            workspace_id: String::new(),
        }
    }
}

/// Opaque generation and binding snapshot used for coordinator admission.
#[allow(dead_code)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GenerationToken {
    floor: u64,
    binding_identity: BindingIdentity,
}

/// Complete coalesced work request. The three bits always move together.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct WorkMask(u8);

#[allow(dead_code)]
impl WorkMask {
    const ROUTINE: u8 = 0b001;
    const REVALIDATE: u8 = 0b010;
    const BACKFILL_PYTHON: u8 = 0b100;

    pub(crate) const fn from_bits(bits: u8) -> Self {
        Self(bits & (Self::ROUTINE | Self::REVALIDATE | Self::BACKFILL_PYTHON))
    }

    const fn is_empty(self) -> bool {
        self.0 == 0
    }

    const fn union(self, other: Self) -> Self {
        Self::from_bits(self.0 | other.0)
    }

    pub(crate) const fn bits(self) -> u8 {
        self.0
    }
}

/// Diagnostic owner category retained in the sequenced permit identity.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum OwnerKind {
    Index,
    Sync,
    Hydration,
    Startup,
    Watcher,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct OwnerIdentity {
    generation: u64,
    sequence: u64,
    kind: OwnerKind,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Eq, PartialEq)]
struct OwnerRecord {
    identity: OwnerIdentity,
    binding_identity: BindingIdentity,
    work_mask: WorkMask,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Eq, PartialEq)]
struct RetirementBarrier {
    retired_identity: OwnerIdentity,
    retired_binding: BindingIdentity,
    /// Immutable old-generation intent retained only so a later retarget back
    /// to the retired binding can reconstruct its same-binding replay.
    retired_work_mask: WorkMask,
    target_generation: u64,
    target_binding: BindingIdentity,
    deferred: WorkMask,
}

#[allow(dead_code)]
#[derive(Debug)]
enum CoordinatorPhase {
    Idle,
    Running(OwnerRecord),
    Retiring(RetirementBarrier),
}

#[allow(dead_code)]
#[derive(Debug)]
struct SyncCoordinator {
    floor: u64,
    binding_identity: BindingIdentity,
    next_sequence: u64,
    phase: CoordinatorPhase,
    pending: WorkMask,
    generation_cancel: watch::Sender<bool>,
    last_indexed_at: Option<DateTime<Utc>>,
}

/// Single private authority for coordinator admission and owner lifecycle.
#[allow(dead_code)]
#[derive(Debug)]
pub(crate) struct CoordinatorCell {
    state: Mutex<SyncCoordinator>,
    notify: Arc<Notify>,
    #[cfg(test)]
    notification_calls: AtomicUsize,
    #[cfg(test)]
    timestamp_writes: AtomicUsize,
}

/// Move-only pre-acquisition ownership, including an enabled waiter registration.
#[allow(dead_code)]
pub(crate) struct AdmissionGuard {
    cell: Arc<CoordinatorCell>,
    token: GenerationToken,
    binding_snapshot: BindingIdentity,
    cancel_rx: watch::Receiver<bool>,
    enabled_notification: Pin<Box<OwnedNotified>>,
}

#[allow(dead_code)]
struct PermitOwnership {
    cell: Arc<CoordinatorCell>,
    token: GenerationToken,
    binding_snapshot: BindingIdentity,
    cancel_rx: watch::Receiver<bool>,
}

/// Move-only exact owner permit with mandatory synchronous abandonment cleanup.
#[allow(dead_code)]
pub(crate) struct OwnerPermit {
    ownership: Option<PermitOwnership>,
    identity: OwnerIdentity,
    work_mask: WorkMask,
    cleanup_armed: bool,
}

/// Parent-owned supervision for one mutation-capable driver task.
#[allow(dead_code)]
#[derive(Debug)]
pub(crate) struct DriverTaskGuard {
    pub(crate) task: Option<tokio::task::JoinHandle<()>>,
}

impl DriverTaskGuard {
    pub(crate) async fn abort_and_join(mut self) -> Result<(), tokio::task::JoinError> {
        let Some(task) = self.task.as_mut() else {
            return Ok(());
        };
        task.abort();
        let result = task.await;
        let _ = self.task.take();
        result
    }

    pub(crate) async fn join(mut self) -> Result<(), tokio::task::JoinError> {
        let Some(task) = self.task.as_mut() else {
            return Ok(());
        };
        let result = task.await;
        let _ = self.task.take();
        result
    }
}

impl Drop for DriverTaskGuard {
    fn drop(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}

#[allow(dead_code)]
pub(crate) enum RequestOutcome {
    Acquired(OwnerPermit),
    Waiting(AdmissionGuard),
    Enqueued,
    Stale,
}

#[allow(dead_code)]
pub(crate) enum ClaimOutcome {
    Acquired(OwnerPermit),
    Retained,
    Missing,
    Stale,
}

#[allow(dead_code)]
pub(crate) enum CompletionOutcome {
    Transferred(OwnerPermit),
    Released,
    RetirementAcknowledged,
    SequenceExhausted(OwnerPermit),
    Stale,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub(crate) enum CoordinatorError {
    #[error("coordinator owner sequence exhausted")]
    SequenceExhausted,
    #[error("workspace limit reached (limit {limit})")]
    WorkspaceLimit { limit: usize },
}

#[allow(dead_code)]
impl CoordinatorCell {
    fn new(binding_identity: BindingIdentity) -> Self {
        let (generation_cancel, _cancel_rx) = watch::channel(false);
        Self {
            state: Mutex::new(SyncCoordinator {
                floor: 0,
                binding_identity,
                next_sequence: 0,
                phase: CoordinatorPhase::Idle,
                pending: WorkMask::default(),
                generation_cancel,
                last_indexed_at: None,
            }),
            notify: Arc::new(Notify::new()),
            #[cfg(test)]
            notification_calls: AtomicUsize::new(0),
            #[cfg(test)]
            timestamp_writes: AtomicUsize::new(0),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, SyncCoordinator> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    #[cfg(test)]
    pub(crate) fn test_is_idle(&self) -> bool {
        matches!(self.lock().phase, CoordinatorPhase::Idle)
    }

    #[cfg(test)]
    pub(crate) fn test_is_retiring(&self) -> bool {
        matches!(self.lock().phase, CoordinatorPhase::Retiring(_))
    }

    #[cfg(test)]
    pub(crate) fn test_notification_calls(&self) -> usize {
        self.notification_calls.load(Ordering::SeqCst)
    }

    #[cfg(test)]
    pub(crate) fn test_pending_bits(&self) -> u8 {
        self.lock().pending.bits()
    }

    pub(crate) fn admission(self: &Arc<Self>) -> AdmissionGuard {
        let state = self.lock();
        let token = GenerationToken {
            floor: state.floor,
            binding_identity: state.binding_identity.clone(),
        };
        let binding_snapshot = state.binding_identity.clone();
        let cancel_rx = state.generation_cancel.subscribe();
        drop(state);

        let mut enabled_notification = Box::pin(Arc::clone(&self.notify).notified_owned());
        enabled_notification.as_mut().enable();
        AdmissionGuard {
            cell: Arc::clone(self),
            token,
            binding_snapshot,
            cancel_rx,
            enabled_notification,
        }
    }

    pub(crate) fn request(
        admission: AdmissionGuard,
        work_mask: WorkMask,
        kind: OwnerKind,
    ) -> Result<RequestOutcome, CoordinatorError> {
        enum Decision {
            Acquired {
                identity: OwnerIdentity,
                work_mask: WorkMask,
            },
            Waiting,
            Enqueued,
            Stale,
        }

        let cell = Arc::clone(&admission.cell);
        let mut state = cell.lock();
        let is_current = admission.token.floor == state.floor
            && admission.token.binding_identity == state.binding_identity
            && admission.binding_snapshot == state.binding_identity;
        if !is_current {
            drop(state);
            return Ok(RequestOutcome::Stale);
        }

        let decision = if matches!(state.phase, CoordinatorPhase::Idle) {
            if state.next_sequence == u64::MAX {
                drop(state);
                return Err(CoordinatorError::SequenceExhausted);
            }

            state.next_sequence += 1;
            let identity = OwnerIdentity {
                generation: state.floor,
                sequence: state.next_sequence,
                kind,
            };
            let selected_work = if work_mask.is_empty() {
                WorkMask::default()
            } else {
                let selected = state.pending.union(work_mask);
                state.pending = WorkMask::default();
                selected
            };
            state.phase = CoordinatorPhase::Running(OwnerRecord {
                identity,
                binding_identity: state.binding_identity.clone(),
                work_mask: selected_work,
            });
            Decision::Acquired {
                identity,
                work_mask: selected_work,
            }
        } else if matches!(state.phase, CoordinatorPhase::Running(_)) {
            if work_mask.is_empty() {
                Decision::Waiting
            } else {
                state.pending = state.pending.union(work_mask);
                Decision::Enqueued
            }
        } else if work_mask.is_empty() {
            Decision::Waiting
        } else {
            match &mut state.phase {
                CoordinatorPhase::Retiring(barrier) => {
                    barrier.deferred = barrier.deferred.union(work_mask);
                    Decision::Enqueued
                }
                CoordinatorPhase::Idle | CoordinatorPhase::Running(_) => Decision::Stale,
            }
        };
        drop(state);

        match decision {
            Decision::Acquired {
                identity,
                work_mask,
            } => {
                let AdmissionGuard {
                    cell,
                    token,
                    binding_snapshot,
                    cancel_rx,
                    enabled_notification,
                } = admission;
                drop(enabled_notification);
                Ok(RequestOutcome::Acquired(OwnerPermit {
                    ownership: Some(PermitOwnership {
                        cell,
                        token,
                        binding_snapshot,
                        cancel_rx,
                    }),
                    identity,
                    work_mask,
                    cleanup_armed: true,
                }))
            }
            Decision::Waiting => Ok(RequestOutcome::Waiting(admission)),
            Decision::Enqueued => Ok(RequestOutcome::Enqueued),
            Decision::Stale => Ok(RequestOutcome::Stale),
        }
    }

    /// Claim an already-published Sync request without publishing it again.
    pub(crate) fn claim_reissued_sync(
        admission: AdmissionGuard,
    ) -> Result<ClaimOutcome, CoordinatorError> {
        let cell = Arc::clone(&admission.cell);
        let mut state = cell.lock();
        let is_current = admission.token.floor == state.floor
            && admission.token.binding_identity == state.binding_identity
            && admission.binding_snapshot == state.binding_identity;
        if !is_current {
            drop(state);
            return Ok(ClaimOutcome::Stale);
        }
        if !matches!(state.phase, CoordinatorPhase::Idle) {
            drop(state);
            return Ok(ClaimOutcome::Retained);
        }
        if state.pending.is_empty() {
            drop(state);
            return Ok(ClaimOutcome::Missing);
        }
        if state.next_sequence == u64::MAX {
            drop(state);
            return Err(CoordinatorError::SequenceExhausted);
        }

        state.next_sequence += 1;
        let identity = OwnerIdentity {
            generation: state.floor,
            sequence: state.next_sequence,
            kind: OwnerKind::Sync,
        };
        let work_mask = state.pending;
        state.pending = WorkMask::default();
        state.phase = CoordinatorPhase::Running(OwnerRecord {
            identity,
            binding_identity: state.binding_identity.clone(),
            work_mask,
        });
        drop(state);

        let AdmissionGuard {
            cell,
            token,
            binding_snapshot,
            cancel_rx,
            enabled_notification,
        } = admission;
        drop(enabled_notification);
        Ok(ClaimOutcome::Acquired(OwnerPermit {
            ownership: Some(PermitOwnership {
                cell,
                token,
                binding_snapshot,
                cancel_rx,
            }),
            identity,
            work_mask,
            cleanup_armed: true,
        }))
    }

    pub(crate) fn complete(mut permit: OwnerPermit) -> CompletionOutcome {
        let Some(ownership) = permit.ownership.as_ref() else {
            permit.cleanup_armed = false;
            return CompletionOutcome::Stale;
        };
        let cell = Arc::clone(&ownership.cell);
        let mut state = cell.lock();
        let retiring_deferred = match &state.phase {
            CoordinatorPhase::Retiring(barrier)
                if barrier.retired_identity == permit.identity
                    && barrier.retired_binding == ownership.binding_snapshot
                    && ownership.token.floor == permit.identity.generation
                    && ownership.token.binding_identity == barrier.retired_binding =>
            {
                Some(barrier.deferred)
            }
            CoordinatorPhase::Idle
            | CoordinatorPhase::Running(_)
            | CoordinatorPhase::Retiring(_) => None,
        };
        if let Some(deferred) = retiring_deferred {
            state.pending = deferred;
            state.phase = CoordinatorPhase::Idle;
            drop(state);

            permit.cleanup_armed = false;
            let _ = permit.ownership.take();
            #[cfg(test)]
            cell.notification_calls.fetch_add(1, Ordering::SeqCst);
            cell.notify.notify_one();
            return CompletionOutcome::RetirementAcknowledged;
        }

        let exact = matches!(
            &state.phase,
            CoordinatorPhase::Running(owner)
                if owner.identity == permit.identity
                    && owner.binding_identity == ownership.binding_snapshot
                    && ownership.token.floor == state.floor
                    && ownership.token.binding_identity == state.binding_identity
        );
        if !exact {
            drop(state);
            permit.cleanup_armed = false;
            let _ = permit.ownership.take();
            return CompletionOutcome::Stale;
        }

        if state.pending.is_empty() {
            state.phase = CoordinatorPhase::Idle;
            state.last_indexed_at = Some(Utc::now());
            #[cfg(test)]
            cell.timestamp_writes.fetch_add(1, Ordering::SeqCst);
            drop(state);

            permit.cleanup_armed = false;
            let _ = permit.ownership.take();
            #[cfg(test)]
            cell.notification_calls.fetch_add(1, Ordering::SeqCst);
            cell.notify.notify_one();
            CompletionOutcome::Released
        } else {
            if state.next_sequence == u64::MAX {
                drop(state);
                return CompletionOutcome::SequenceExhausted(permit);
            }
            state.next_sequence += 1;
            let identity = OwnerIdentity {
                generation: state.floor,
                sequence: state.next_sequence,
                kind: OwnerKind::Sync,
            };
            let work_mask = state.pending;
            state.pending = WorkMask::default();
            state.phase = CoordinatorPhase::Running(OwnerRecord {
                identity,
                binding_identity: state.binding_identity.clone(),
                work_mask,
            });
            state.last_indexed_at = Some(Utc::now());
            #[cfg(test)]
            cell.timestamp_writes.fetch_add(1, Ordering::SeqCst);
            drop(state);

            permit.cleanup_armed = false;
            let Some(ownership) = permit.ownership.take() else {
                return CompletionOutcome::Stale;
            };
            CompletionOutcome::Transferred(OwnerPermit {
                ownership: Some(ownership),
                identity,
                work_mask,
                cleanup_armed: true,
            })
        }
    }
}

#[allow(dead_code)]
impl AdmissionGuard {
    pub(crate) const fn generation(&self) -> u64 {
        self.token.floor
    }

    pub(crate) async fn acquire_hydration(self) -> Result<Option<OwnerPermit>, CoordinatorError> {
        self.acquire_background(OwnerKind::Hydration).await
    }

    pub(crate) async fn acquire_background(
        mut self,
        kind: OwnerKind,
    ) -> Result<Option<OwnerPermit>, CoordinatorError> {
        loop {
            if *self.cancel_rx.borrow() {
                return Ok(None);
            }
            match CoordinatorCell::request(self, WorkMask::default(), kind)? {
                RequestOutcome::Acquired(permit) => return Ok(Some(permit)),
                RequestOutcome::Waiting(mut waiting) => {
                    let cancelled = tokio::select! {
                        biased;
                        changed = waiting.cancel_rx.changed() => {
                            changed.is_err() || *waiting.cancel_rx.borrow()
                        }
                        () = waiting.enabled_notification.as_mut() => false,
                    };
                    if cancelled {
                        return Ok(None);
                    }
                    waiting.rearm();
                    self = waiting;
                }
                RequestOutcome::Enqueued | RequestOutcome::Stale => return Ok(None),
            }
        }
    }

    pub(crate) fn rearm(&mut self) {
        let mut notification = Box::pin(Arc::clone(&self.cell.notify).notified_owned());
        notification.as_mut().enable();
        self.enabled_notification = notification;
    }
}

impl OwnerPermit {
    pub(crate) const fn work_bits(&self) -> u8 {
        self.work_mask.bits()
    }

    pub(crate) fn generation(&self) -> Option<u64> {
        self.ownership
            .as_ref()
            .map(|ownership| ownership.token.floor)
    }

    pub(crate) async fn run_until_cancelled<F>(&mut self, operation: F) -> Option<F::Output>
    where
        F: std::future::Future,
    {
        let ownership = self.ownership.as_mut()?;
        if *ownership.cancel_rx.borrow() {
            return None;
        }
        tokio::pin!(operation);
        tokio::select! {
            biased;
            _ = ownership.cancel_rx.changed() => None,
            output = &mut operation => Some(output),
        }
    }
}

impl Drop for OwnerPermit {
    fn drop(&mut self) {
        if !self.cleanup_armed {
            return;
        }
        let Some(ownership) = self.ownership.as_ref() else {
            self.cleanup_armed = false;
            return;
        };
        let cell = Arc::clone(&ownership.cell);
        let mut state = cell.lock();
        let retiring_deferred = match &state.phase {
            CoordinatorPhase::Retiring(barrier)
                if barrier.retired_identity == self.identity
                    && barrier.retired_binding == ownership.binding_snapshot
                    && ownership.token.floor == self.identity.generation
                    && ownership.token.binding_identity == barrier.retired_binding =>
            {
                Some(barrier.deferred)
            }
            CoordinatorPhase::Idle
            | CoordinatorPhase::Running(_)
            | CoordinatorPhase::Retiring(_) => None,
        };
        if let Some(deferred) = retiring_deferred {
            state.pending = deferred;
            state.phase = CoordinatorPhase::Idle;
            drop(state);
            #[cfg(test)]
            cell.notification_calls.fetch_add(1, Ordering::SeqCst);
            cell.notify.notify_one();
            self.cleanup_armed = false;
            return;
        }

        let owner_work = match &state.phase {
            CoordinatorPhase::Running(owner)
                if owner.identity == self.identity
                    && owner.binding_identity == ownership.binding_snapshot
                    && ownership.token.floor == state.floor
                    && ownership.token.binding_identity == state.binding_identity =>
            {
                Some(owner.work_mask)
            }
            CoordinatorPhase::Idle
            | CoordinatorPhase::Running(_)
            | CoordinatorPhase::Retiring(_) => None,
        };
        let should_notify = if let Some(owner_work) = owner_work {
            state.pending = owner_work.union(state.pending);
            state.phase = CoordinatorPhase::Idle;
            true
        } else {
            false
        };
        drop(state);

        if should_notify {
            #[cfg(test)]
            cell.notification_calls.fetch_add(1, Ordering::SeqCst);
            cell.notify.notify_one();
        }
        self.cleanup_armed = false;
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
    #[allow(dead_code)]
    pub(crate) coordinator: Arc<CoordinatorCell>,
    /// Parent-retained hydration task. The guard aborts on parent Drop.
    hydration_driver: Mutex<Option<(u64, DriverTaskGuard)>>,
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
    /// Lock-free process-level reliability counters (029-F WS-8).
    reliability: ReliabilityCounters,
    /// Set to `true` once the current generation has initialized its branch DB.
    /// `_health` gates "ready" on this flag so polling clients do not issue
    /// workspace calls against an uninitialized branch.
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
            coordinator: Arc::new(CoordinatorCell::new(BindingIdentity::unbound())),
            hydration_driver: Mutex::new(None),
            query_latencies: RwLock::new(VecDeque::new()),
            tool_call_count: AtomicU64::new(0),
            watcher_event_count: AtomicU64::new(0),
            last_watcher_event: RwLock::new(None),
            scan_progress: RwLock::new(None),
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

    /// Capture one immutable dispatch payload and its matching admission guard.
    pub(crate) async fn guarded_dispatch_context(
        &self,
    ) -> Option<(AdmissionGuard, DispatchSnapshot)> {
        // Publication takes these locks in the same order before the coordinator
        // mutex, so the payload and admission token belong to one binding view.
        let workspace_guard = self.active_workspace.read().await;
        let config_guard = self.workspace_config.read().await;
        let workspace = workspace_guard.clone()?;
        let config = config_guard.clone().unwrap_or_default();
        let admission = self.coordinator.admission();
        Some((admission, DispatchSnapshot { workspace, config }))
    }

    /// Capture one configured workspace payload and its matching admission guard.
    pub(crate) async fn guarded_workspace_and_config(
        &self,
    ) -> Option<(AdmissionGuard, WorkspaceSnapshot, WorkspaceConfig)> {
        let workspace_guard = self.active_workspace.read().await;
        let config_guard = self.workspace_config.read().await;
        let workspace = workspace_guard.clone()?;
        let config = config_guard.clone()?;
        let admission = self.coordinator.admission();
        Some((admission, workspace, config))
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

    /// Publish a binding/config pair and begin its coordinator generation.
    ///
    /// Task 109.017-T replaces this compile-only RED bridge with one atomic
    /// binding/coordinator publication.
    pub(crate) async fn publish_workspace_generation(
        &self,
        snapshot: WorkspaceSnapshot,
        config: Option<WorkspaceConfig>,
    ) -> Result<(u64, AdmissionGuard), CoordinatorError> {
        self.publish_workspace_generation_transition(snapshot, config, WorkMask::default())
            .await
    }

    /// Publish a binding and atomically install an explicit request for that binding.
    ///
    /// A distinct binding still inherits no old-binding work. The supplied mask
    /// is qualified to the newly published binding and is installed under the
    /// same coordinator lock as publication.
    pub(crate) async fn publish_workspace_generation_with_reissue(
        &self,
        snapshot: WorkspaceSnapshot,
        config: Option<WorkspaceConfig>,
        reissued_work: WorkMask,
    ) -> Result<(u64, AdmissionGuard), CoordinatorError> {
        self.publish_workspace_generation_transition(snapshot, config, reissued_work)
            .await
    }

    async fn publish_workspace_generation_transition(
        &self,
        snapshot: WorkspaceSnapshot,
        config: Option<WorkspaceConfig>,
        reissued_work: WorkMask,
    ) -> Result<(u64, AdmissionGuard), CoordinatorError> {
        let target_binding = BindingIdentity {
            workspace_uuid: snapshot.workspace_uuid.clone(),
            workspace_id: snapshot.workspace_id.clone(),
        };
        let (new_cancel, new_cancel_rx) = watch::channel(false);

        // Fixed lock order: binding, config, then the synchronous coordinator.
        // No await occurs after the coordinator mutex is acquired.
        let mut workspace = self.active_workspace.write().await;
        let mut workspace_config = self.workspace_config.write().await;
        if let Some(active) = workspace.as_ref() {
            if active.workspace_id != snapshot.workspace_id
                && (active.workspace_uuid != snapshot.workspace_uuid
                    || active.path != snapshot.path)
                && self.max_workspaces <= 1
            {
                return Err(CoordinatorError::WorkspaceLimit {
                    limit: self.max_workspaces,
                });
            }
        }

        let mut coordinator = self
            .coordinator
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(target_generation) = coordinator.floor.checked_add(1) else {
            return Err(CoordinatorError::SequenceExhausted);
        };
        let same_binding = coordinator.binding_identity == target_binding;
        let old_cancel = std::mem::replace(&mut coordinator.generation_cancel, new_cancel);
        let prior_phase = std::mem::replace(&mut coordinator.phase, CoordinatorPhase::Idle);
        coordinator.phase = match prior_phase {
            CoordinatorPhase::Idle => {
                if !same_binding {
                    coordinator.pending = WorkMask::default();
                }
                coordinator.pending = coordinator.pending.union(reissued_work);
                CoordinatorPhase::Idle
            }
            CoordinatorPhase::Running(owner) => {
                let retired_work_mask = owner.work_mask.union(coordinator.pending);
                let inherited = if same_binding {
                    retired_work_mask
                } else {
                    WorkMask::default()
                };
                let deferred = inherited.union(reissued_work);
                coordinator.pending = WorkMask::default();
                CoordinatorPhase::Retiring(RetirementBarrier {
                    retired_identity: owner.identity,
                    retired_binding: owner.binding_identity,
                    retired_work_mask,
                    target_generation,
                    target_binding: target_binding.clone(),
                    deferred,
                })
            }
            CoordinatorPhase::Retiring(mut barrier) => {
                if barrier.target_binding != target_binding {
                    barrier.deferred = if target_binding == barrier.retired_binding {
                        barrier.retired_work_mask
                    } else {
                        WorkMask::default()
                    };
                }
                barrier.deferred = barrier.deferred.union(reissued_work);
                barrier.target_generation = target_generation;
                barrier.target_binding = target_binding.clone();
                CoordinatorPhase::Retiring(barrier)
            }
        };
        coordinator.floor = target_generation;
        coordinator.binding_identity = target_binding;
        self.hydration_ready.store(false, Ordering::Release);
        *workspace = Some(snapshot);
        *workspace_config = config;

        let mut enabled_notification =
            Box::pin(Arc::clone(&self.coordinator.notify).notified_owned());
        enabled_notification.as_mut().enable();
        let admission = AdmissionGuard {
            cell: Arc::clone(&self.coordinator),
            token: GenerationToken {
                floor: target_generation,
                binding_identity: coordinator.binding_identity.clone(),
            },
            binding_snapshot: coordinator.binding_identity.clone(),
            cancel_rx: new_cancel_rx,
            enabled_notification,
        };

        // Make the visible binding readable before allowing new admission.
        drop(workspace_config);
        drop(workspace);
        drop(coordinator);
        let _ = old_cancel.send(true);
        Ok((target_generation, admission))
    }

    /// Retain only the newest hydration generation without awaiting under the slot lock.
    pub(crate) fn retain_hydration_driver(
        &self,
        generation: u64,
        driver: DriverTaskGuard,
    ) -> Option<DriverTaskGuard> {
        let mut retained = self
            .hydration_driver
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if retained
            .as_ref()
            .is_some_and(|(retained_generation, _)| *retained_generation > generation)
        {
            Some(driver)
        } else {
            retained
                .replace((generation, driver))
                .map(|(_generation, guard)| guard)
        }
    }

    /// Remove the retained hydration task so its caller can abort/join without the slot lock.
    pub(crate) fn take_hydration_driver(&self) -> Option<DriverTaskGuard> {
        self.hydration_driver
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()
            .map(|(_generation, driver)| driver)
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
        !matches!(self.coordinator.lock().phase, CoordinatorPhase::Idle)
    }

    /// Get the timestamp of the last completed indexing operation.
    #[allow(clippy::unused_async)] // Preserve the existing async state API contract.
    pub async fn last_indexed_at(&self) -> Option<DateTime<Utc>> {
        self.coordinator.lock().last_indexed_at
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

    /// Returns a reference to the process-level reliability counters (029-F WS-8).
    ///
    /// Callers use the returned reference to increment specific counters without
    /// acquiring any locks (all fields are `AtomicU64`).
    pub fn reliability_counters(&self) -> &ReliabilityCounters {
        &self.reliability
    }

    /// Mark the current branch DB as initialized.
    ///
    /// Generation-owned production paths use
    /// [`set_hydration_ready_for_generation`](Self::set_hydration_ready_for_generation)
    /// so stale work cannot mark a newer binding ready.
    pub fn set_hydration_ready(&self) {
        self.hydration_ready.store(true, Ordering::Release);
    }

    /// Mark readiness only when `generation` is still the published generation.
    ///
    /// The coordinator lock linearizes this store with generation publication:
    /// an older hydration cannot restore readiness after a branch/workspace
    /// publication has cleared it.
    pub(crate) fn set_hydration_ready_for_generation(&self, generation: u64) -> bool {
        let coordinator = self.coordinator.lock();
        if coordinator.floor != generation {
            return false;
        }
        self.hydration_ready.store(true, Ordering::Release);
        true
    }

    /// Mark readiness only for a permit belonging to the current generation.
    pub(crate) fn set_hydration_ready_for_permit(&self, permit: &OwnerPermit) -> bool {
        permit
            .generation()
            .is_some_and(|generation| self.set_hydration_ready_for_generation(generation))
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

#[cfg(test)]
mod coordinator_tests {
    use super::*;

    fn binding(name: &str) -> BindingIdentity {
        BindingIdentity {
            workspace_uuid: format!("uuid-{name}"),
            workspace_id: format!("workspace-{name}"),
        }
    }

    fn coordinator_cell(name: &str) -> Arc<CoordinatorCell> {
        Arc::new(CoordinatorCell::new(binding(name)))
    }

    fn workspace(name: &str, workspace_uuid: &str, workspace_id: &str) -> WorkspaceSnapshot {
        WorkspaceSnapshot {
            workspace_id: workspace_id.to_string(),
            workspace_uuid: workspace_uuid.to_string(),
            branch: "main".to_string(),
            data_dir: PathBuf::from(format!("logs/phase6-group1/{name}")),
            path: format!("C:/workspace/{name}"),
            last_flush: None,
            stale_files: false,
            connection_count: 0,
            file_mtimes: HashMap::new(),
        }
    }

    async fn publish(state: &AppState, snapshot: WorkspaceSnapshot) -> (u64, AdmissionGuard) {
        match state
            .publish_workspace_generation(snapshot, Some(WorkspaceConfig::default()))
            .await
        {
            Ok(publication) => publication,
            Err(error) => panic!("unexpected publication error: {error}"),
        }
    }

    fn admission(cell: &Arc<CoordinatorCell>) -> AdmissionGuard {
        cell.admission()
    }

    fn publish_idle_generation(cell: &CoordinatorCell, new_binding: BindingIdentity) {
        let old_cancel = {
            let mut state = cell.lock();
            let (new_cancel, _new_rx) = watch::channel(false);
            let old_cancel = std::mem::replace(&mut state.generation_cancel, new_cancel);
            state.floor += 1;
            state.binding_identity = new_binding;
            old_cancel
        };
        let _ = old_cancel.send(true);
    }

    fn acquired(outcome: RequestOutcome) -> OwnerPermit {
        match outcome {
            RequestOutcome::Acquired(permit) => permit,
            RequestOutcome::Waiting(_) => panic!("expected acquired permit, got waiting"),
            RequestOutcome::Enqueued => panic!("expected acquired permit, got enqueued"),
            RequestOutcome::Stale => panic!("expected acquired permit, got stale"),
        }
    }

    fn waiting(outcome: RequestOutcome) -> AdmissionGuard {
        match outcome {
            RequestOutcome::Waiting(admission) => admission,
            RequestOutcome::Acquired(_) => panic!("expected waiting guard, got permit"),
            RequestOutcome::Enqueued => panic!("expected waiting guard, got enqueued"),
            RequestOutcome::Stale => panic!("expected waiting guard, got stale"),
        }
    }

    fn request(admission: AdmissionGuard, mask: WorkMask, kind: OwnerKind) -> RequestOutcome {
        match CoordinatorCell::request(admission, mask, kind) {
            Ok(outcome) => outcome,
            Err(error) => panic!("unexpected request error: {error:?}"),
        }
    }

    fn rearm(admission: &mut AdmissionGuard) {
        admission.rearm();
    }

    async fn notification_is_ready(notification: &mut Pin<Box<OwnedNotified>>) -> bool {
        tokio::select! {
            biased;
            () = notification.as_mut() => true,
            () = std::future::ready(()) => false,
        }
    }

    #[test]
    fn admission_guard_cancels_and_busy_work_is_coordinator_owned() {
        let cell = coordinator_cell("old");
        let stale_admission = admission(&cell);
        publish_idle_generation(&cell, binding("new"));
        assert!(*stale_admission.cancel_rx.borrow());
        assert!(matches!(
            request(stale_admission, WorkMask::default(), OwnerKind::Hydration),
            RequestOutcome::Stale
        ));

        let owner = acquired(request(
            admission(&cell),
            WorkMask::from_bits(0b101),
            OwnerKind::Index,
        ));
        let enqueued = request(
            admission(&cell),
            WorkMask::from_bits(0b010),
            OwnerKind::Sync,
        );
        assert!(matches!(enqueued, RequestOutcome::Enqueued));

        let waiting_guard = waiting(request(
            admission(&cell),
            WorkMask::default(),
            OwnerKind::Startup,
        ));
        let before_drop = {
            let state = cell.lock();
            (
                state.pending,
                cell.notification_calls.load(Ordering::SeqCst),
            )
        };
        drop(waiting_guard);
        let after_drop = {
            let state = cell.lock();
            (
                state.pending,
                cell.notification_calls.load(Ordering::SeqCst),
            )
        };
        assert_eq!(before_drop, after_drop);
        assert_eq!(after_drop.0.bits(), 0b010);
        drop(owner);
    }

    #[test]
    fn owner_kind_completion_drop_and_sequence_matrix() {
        for kind in [
            OwnerKind::Index,
            OwnerKind::Sync,
            OwnerKind::Hydration,
            OwnerKind::Startup,
            OwnerKind::Watcher,
        ] {
            let cell = coordinator_cell("kind");
            let permit = acquired(request(admission(&cell), WorkMask::from_bits(0b001), kind));
            assert_eq!(permit.identity.kind, kind);
            assert!(matches!(
                CoordinatorCell::complete(permit),
                CompletionOutcome::Released
            ));
            let state = cell.lock();
            assert!(matches!(state.phase, CoordinatorPhase::Idle));
            assert!(state.last_indexed_at.is_some());
            assert_eq!(cell.timestamp_writes.load(Ordering::SeqCst), 1);
            assert_eq!(cell.notification_calls.load(Ordering::SeqCst), 1);
        }

        let cell = coordinator_cell("transfer");
        let permit = acquired(request(
            admission(&cell),
            WorkMask::from_bits(0b101),
            OwnerKind::Index,
        ));
        assert!(matches!(
            request(
                admission(&cell),
                WorkMask::from_bits(0b010),
                OwnerKind::Sync
            ),
            RequestOutcome::Enqueued
        ));
        let successor = match CoordinatorCell::complete(permit) {
            CompletionOutcome::Transferred(successor) => successor,
            CompletionOutcome::Released => panic!("expected transferred successor"),
            CompletionOutcome::SequenceExhausted(_) => {
                panic!("unexpected sequence exhaustion")
            }
            CompletionOutcome::RetirementAcknowledged => {
                panic!("unexpected retirement acknowledgment")
            }
            CompletionOutcome::Stale => panic!("expected current completion"),
        };
        assert_eq!(successor.work_mask.bits(), 0b010);
        assert_eq!(successor.identity.kind, OwnerKind::Sync);
        assert_eq!(cell.timestamp_writes.load(Ordering::SeqCst), 1);
        drop(successor);
        let state = cell.lock();
        assert!(matches!(state.phase, CoordinatorPhase::Idle));
        assert_eq!(state.pending.bits(), 0b010);
        assert_eq!(cell.notification_calls.load(Ordering::SeqCst), 1);
        drop(state);

        let stale_ownership = PermitOwnership {
            cell: Arc::clone(&cell),
            token: GenerationToken {
                floor: 0,
                binding_identity: binding("transfer"),
            },
            binding_snapshot: binding("transfer"),
            cancel_rx: cell.lock().generation_cancel.subscribe(),
        };
        let stale_permit = OwnerPermit {
            ownership: Some(stale_ownership),
            identity: OwnerIdentity {
                generation: 0,
                sequence: u64::MAX,
                kind: OwnerKind::Sync,
            },
            work_mask: WorkMask::from_bits(0b111),
            cleanup_armed: true,
        };
        assert!(matches!(
            CoordinatorCell::complete(stale_permit),
            CompletionOutcome::Stale
        ));
        assert_eq!(cell.lock().pending.bits(), 0b010);
        assert_eq!(cell.notification_calls.load(Ordering::SeqCst), 1);

        let exhausted = coordinator_cell("exhausted");
        exhausted.lock().next_sequence = u64::MAX;
        let result = CoordinatorCell::request(
            admission(&exhausted),
            WorkMask::from_bits(0b001),
            OwnerKind::Index,
        );
        assert_eq!(result.err(), Some(CoordinatorError::SequenceExhausted));
        let state = exhausted.lock();
        assert!(matches!(state.phase, CoordinatorPhase::Idle));
        assert!(state.pending.is_empty());
        drop(state);

        let transfer_exhausted = coordinator_cell("transfer-exhausted");
        transfer_exhausted.lock().next_sequence = u64::MAX - 1;
        let permit = acquired(request(
            admission(&transfer_exhausted),
            WorkMask::from_bits(0b001),
            OwnerKind::Index,
        ));
        assert!(matches!(
            request(
                admission(&transfer_exhausted),
                WorkMask::from_bits(0b010),
                OwnerKind::Sync
            ),
            RequestOutcome::Enqueued
        ));
        let permit = match CoordinatorCell::complete(permit) {
            CompletionOutcome::SequenceExhausted(permit) => permit,
            CompletionOutcome::Transferred(_) => panic!("sequence exhaustion transferred"),
            CompletionOutcome::Released => panic!("sequence exhaustion released"),
            CompletionOutcome::RetirementAcknowledged => {
                panic!("sequence exhaustion acknowledged retirement")
            }
            CompletionOutcome::Stale => panic!("sequence exhaustion reported stale"),
        };
        {
            let state = transfer_exhausted.lock();
            assert!(matches!(state.phase, CoordinatorPhase::Running(_)));
            assert_eq!(state.pending.bits(), 0b010);
            assert!(state.last_indexed_at.is_none());
        }
        drop(permit);
        let state = transfer_exhausted.lock();
        assert!(matches!(state.phase, CoordinatorPhase::Idle));
        assert_eq!(state.pending.bits(), 0b011);
        assert_eq!(
            transfer_exhausted.notification_calls.load(Ordering::SeqCst),
            1
        );
    }

    #[test]
    fn compatibility_stale_terminals_and_drop_recovery_are_identity_guarded() {
        let cell = coordinator_cell("compatibility");
        let owner = acquired(request(
            admission(&cell),
            WorkMask::from_bits(0b101),
            OwnerKind::Index,
        ));
        assert!(matches!(
            request(
                admission(&cell),
                WorkMask::from_bits(0b010),
                OwnerKind::Sync
            ),
            RequestOutcome::Enqueued
        ));

        drop(owner);
        {
            let state = cell.lock();
            assert!(matches!(state.phase, CoordinatorPhase::Idle));
            assert_eq!(state.pending.bits(), 0b111);
            assert!(state.last_indexed_at.is_none());
            assert_eq!(cell.notification_calls.load(Ordering::SeqCst), 1);
        }

        let successor = acquired(request(
            admission(&cell),
            WorkMask::from_bits(0b001),
            OwnerKind::Sync,
        ));
        assert_eq!(successor.work_mask.bits(), 0b111);
        let waiting_guard = waiting(request(
            admission(&cell),
            WorkMask::default(),
            OwnerKind::Startup,
        ));
        drop(waiting_guard);

        let snapshot = || {
            let state = cell.lock();
            let identity = match &state.phase {
                CoordinatorPhase::Running(owner) => owner.identity,
                CoordinatorPhase::Idle | CoordinatorPhase::Retiring(_) => {
                    panic!("recovered successor lost ownership")
                }
            };
            (
                identity,
                state.pending,
                state.last_indexed_at,
                cell.notification_calls.load(Ordering::SeqCst),
                cell.timestamp_writes.load(Ordering::SeqCst),
            )
        };
        let before_stale = snapshot();
        let stale_permit = || OwnerPermit {
            ownership: Some(PermitOwnership {
                cell: Arc::clone(&cell),
                token: GenerationToken {
                    floor: 0,
                    binding_identity: binding("compatibility"),
                },
                binding_snapshot: binding("compatibility"),
                cancel_rx: cell.lock().generation_cancel.subscribe(),
            }),
            identity: OwnerIdentity {
                generation: 0,
                sequence: u64::MAX,
                kind: OwnerKind::Sync,
            },
            work_mask: WorkMask::from_bits(0b111),
            cleanup_armed: true,
        };

        assert!(matches!(
            CoordinatorCell::complete(stale_permit()),
            CompletionOutcome::Stale
        ));
        assert_eq!(snapshot(), before_stale);
        drop(stale_permit());
        assert_eq!(snapshot(), before_stale);

        assert!(matches!(
            CoordinatorCell::complete(successor),
            CompletionOutcome::Released
        ));
        let state = cell.lock();
        assert!(matches!(state.phase, CoordinatorPhase::Idle));
        assert!(state.pending.is_empty());
        assert!(state.last_indexed_at.is_some());
        assert_eq!(cell.timestamp_writes.load(Ordering::SeqCst), 1);
        assert_eq!(cell.notification_calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn acquired_registration_cannot_steal_release_baton() {
        let cell = coordinator_cell("baton");
        let owner = acquired(request(
            admission(&cell),
            WorkMask::default(),
            OwnerKind::Hydration,
        ));
        let mut first = waiting(request(
            admission(&cell),
            WorkMask::default(),
            OwnerKind::Startup,
        ));
        let mut second = waiting(request(
            admission(&cell),
            WorkMask::default(),
            OwnerKind::Watcher,
        ));

        assert!(matches!(
            CoordinatorCell::complete(owner),
            CompletionOutcome::Released
        ));
        assert!(notification_is_ready(&mut first.enabled_notification).await);
        assert!(!notification_is_ready(&mut second.enabled_notification).await);

        rearm(&mut first);
        let first_owner = acquired(request(first, WorkMask::default(), OwnerKind::Startup));
        assert!(matches!(
            CoordinatorCell::complete(first_owner),
            CompletionOutcome::Released
        ));
        assert!(notification_is_ready(&mut second.enabled_notification).await);
        rearm(&mut second);
        let second_owner = acquired(request(second, WorkMask::default(), OwnerKind::Watcher));
        assert!(matches!(
            CoordinatorCell::complete(second_owner),
            CompletionOutcome::Released
        ));
        assert_eq!(cell.notification_calls.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn binding_publication_is_coherent_and_overflow_safe() {
        let state = AppState::new(1);
        let old_waiter = admission(&state.coordinator);
        let alpha = workspace("alpha", "uuid-alpha", "id-alpha");
        let (generation, new_cancel_rx) = publish(&state, alpha.clone()).await;
        assert_eq!(generation, 1);
        assert_eq!(generation, 1);
        assert!(*old_waiter.cancel_rx.borrow());
        assert!(!*new_cancel_rx.cancel_rx.borrow());
        assert!(matches!(
            request(old_waiter, WorkMask::default(), OwnerKind::Hydration),
            RequestOutcome::Stale
        ));
        {
            let coordinator = state.coordinator.lock();
            assert_eq!(coordinator.floor, 1);
            assert_eq!(
                coordinator.binding_identity,
                BindingIdentity {
                    workspace_uuid: alpha.workspace_uuid.clone(),
                    workspace_id: alpha.workspace_id.clone(),
                }
            );
            assert!(matches!(coordinator.phase, CoordinatorPhase::Idle));
            assert_eq!(
                state.coordinator.notification_calls.load(Ordering::SeqCst),
                0
            );
        }
        let Some(dispatch) = state.snapshot_dispatch_context().await else {
            panic!("binding publication did not publish dispatch state");
        };
        assert_eq!(dispatch.workspace.workspace_uuid, "uuid-alpha");
        assert_eq!(dispatch.workspace.workspace_id, "id-alpha");

        state.coordinator.lock().floor = u64::MAX;
        let before = state.snapshot_workspace_and_config().await;
        let result = state
            .publish_workspace_generation(
                workspace("beta", "uuid-beta", "id-beta"),
                Some(WorkspaceConfig::default()),
            )
            .await;
        assert!(result.is_err());
        let after = state.snapshot_workspace_and_config().await;
        assert_eq!(
            before.as_ref().map(|(snapshot, _)| &snapshot.workspace_id),
            after.as_ref().map(|(snapshot, _)| &snapshot.workspace_id)
        );
        let coordinator = state.coordinator.lock();
        assert_eq!(coordinator.floor, u64::MAX);
        assert_eq!(coordinator.binding_identity.workspace_id, "id-alpha");
    }

    #[tokio::test]
    async fn qualified_reissue_survives_same_target_republication_exactly_once() {
        for qualified_first in [true, false] {
            let state = AppState::new(2);
            let old = workspace("old", "uuid-old", "id-old");
            let _ = publish(&state, old.clone()).await;
            let old_owner = acquired(request(
                admission(&state.coordinator),
                WorkMask::from_bits(0b001),
                OwnerKind::Sync,
            ));
            let mut target = old;
            target.workspace_id = "id-target".to_owned();
            target.branch = "feature".to_owned();

            if qualified_first {
                let _ = state
                    .publish_workspace_generation_with_reissue(
                        target.clone(),
                        Some(WorkspaceConfig::default()),
                        WorkMask::from_bits(0b110),
                    )
                    .await
                    .unwrap_or_else(|error| panic!("qualified publication failed: {error}"));
                let _ = publish(&state, target).await;
            } else {
                let _ = publish(&state, target.clone()).await;
                let _ = state
                    .publish_workspace_generation_with_reissue(
                        target,
                        Some(WorkspaceConfig::default()),
                        WorkMask::from_bits(0b110),
                    )
                    .await
                    .unwrap_or_else(|error| panic!("qualified publication failed: {error}"));
            }

            assert!(matches!(
                CoordinatorCell::complete(old_owner),
                CompletionOutcome::RetirementAcknowledged
            ));
            assert_eq!(
                state.coordinator.test_pending_bits(),
                0b110,
                "either same-target publication order must retain the explicit request"
            );

            let reissued = match CoordinatorCell::claim_reissued_sync(admission(&state.coordinator))
                .unwrap_or_else(|error| panic!("qualified claim failed: {error}"))
            {
                ClaimOutcome::Acquired(permit) => permit,
                ClaimOutcome::Retained => panic!("qualified request remained behind an owner"),
                ClaimOutcome::Missing => panic!("qualified request was lost"),
                ClaimOutcome::Stale => panic!("qualified request admission was stale"),
            };
            assert_eq!(reissued.work_bits(), 0b110);
            assert!(matches!(
                CoordinatorCell::complete(reissued),
                CompletionOutcome::Released
            ));
            assert_eq!(
                state.coordinator.test_pending_bits(),
                0,
                "the qualified request must drain exactly once"
            );
        }
    }

    #[tokio::test]
    async fn stale_generation_cannot_restore_hydration_readiness() {
        let state = AppState::new(2);
        let (old_generation, _) = publish(&state, workspace("old", "uuid-old", "id-old")).await;
        assert!(state.set_hydration_ready_for_generation(old_generation));
        assert!(state.is_hydration_ready());

        let (current_generation, _) = publish(&state, workspace("new", "uuid-new", "id-new")).await;
        assert!(!state.is_hydration_ready());
        assert!(!state.set_hydration_ready_for_generation(old_generation));
        assert!(
            !state.is_hydration_ready(),
            "cancelled old-generation work must not mark the new binding ready"
        );
        assert!(state.set_hydration_ready_for_generation(current_generation));
        assert!(state.is_hydration_ready());
    }

    #[tokio::test]
    async fn guarded_dispatch_keeps_immutable_payload_when_publication_cancels_owner() {
        let state = AppState::new(2);
        let alpha = workspace("alpha", "uuid-alpha", "id-alpha");
        let _ = publish(&state, alpha).await;
        let Some((admission, dispatch)) = state.guarded_dispatch_context().await else {
            panic!("guarded dispatch missing");
        };

        let _ = publish(&state, workspace("beta", "uuid-beta", "id-beta")).await;

        assert_eq!(dispatch.workspace.workspace_id, "id-alpha");
        assert!(*admission.cancel_rx.borrow());
        assert!(matches!(
            request(admission, WorkMask::default(), OwnerKind::Startup),
            RequestOutcome::Stale
        ));
    }

    #[tokio::test]
    async fn older_hydration_driver_cannot_displace_newer_retained_generation() {
        let state = AppState::new(1);
        let newer = DriverTaskGuard {
            task: Some(tokio::spawn(std::future::pending::<()>())),
        };
        assert!(
            state.retain_hydration_driver(2, newer).is_none(),
            "newest driver should occupy an empty slot"
        );
        let older = DriverTaskGuard {
            task: Some(tokio::spawn(std::future::pending::<()>())),
        };
        let displaced = state
            .retain_hydration_driver(1, older)
            .expect("stale incoming driver must be displaced");

        assert_eq!(
            state
                .hydration_driver
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .as_ref()
                .map(|(generation, _)| *generation),
            Some(2)
        );
        let _ = displaced.abort_and_join().await;
        let retained = state
            .take_hydration_driver()
            .expect("newer driver remains retained");
        let _ = retained.abort_and_join().await;
    }

    #[tokio::test]
    async fn active_rebind_matrix_is_acknowledgment_gated() {
        for same_binding in [true, false] {
            for kind in [
                OwnerKind::Index,
                OwnerKind::Sync,
                OwnerKind::Hydration,
                OwnerKind::Startup,
                OwnerKind::Watcher,
            ] {
                for explicit_ack in [true, false] {
                    let state = AppState::new(2);
                    let old = workspace("old", "uuid-old", "id-old");
                    let _ = publish(&state, old.clone()).await;
                    let permit = acquired(request(
                        admission(&state.coordinator),
                        WorkMask::from_bits(0b101),
                        kind,
                    ));
                    assert!(matches!(
                        request(
                            admission(&state.coordinator),
                            WorkMask::from_bits(0b010),
                            OwnerKind::Sync
                        ),
                        RequestOutcome::Enqueued
                    ));

                    let target = if same_binding {
                        workspace("old-rebound", "uuid-old", "id-old")
                    } else {
                        workspace("new", "uuid-new", "id-new")
                    };
                    let (target_generation, _) = publish(&state, target).await;
                    assert!(
                        *permit
                            .ownership
                            .as_ref()
                            .map(|ownership| ownership.cancel_rx.borrow())
                            .unwrap_or_else(|| panic!("permit lost cancellation ownership"))
                    );

                    let retired_identity = {
                        let coordinator = state.coordinator.lock();
                        let barrier = match &coordinator.phase {
                            CoordinatorPhase::Retiring(barrier) => barrier,
                            CoordinatorPhase::Idle | CoordinatorPhase::Running(_) => {
                                panic!("active rebind did not install retirement barrier")
                            }
                        };
                        assert_eq!(barrier.retired_identity, permit.identity);
                        assert_eq!(barrier.target_generation, target_generation);
                        assert_eq!(
                            barrier.deferred.bits(),
                            if same_binding { 0b111 } else { 0 }
                        );
                        assert_eq!(
                            state.coordinator.notification_calls.load(Ordering::SeqCst),
                            0
                        );
                        barrier.retired_identity
                    };

                    assert!(matches!(
                        request(
                            admission(&state.coordinator),
                            WorkMask::from_bits(0b100),
                            OwnerKind::Sync
                        ),
                        RequestOutcome::Enqueued
                    ));
                    let mut waiting_guard = waiting(request(
                        admission(&state.coordinator),
                        WorkMask::default(),
                        OwnerKind::Startup,
                    ));
                    let active_drivers = AtomicUsize::new(1);
                    assert_eq!(active_drivers.load(Ordering::SeqCst), 1);
                    active_drivers.store(0, Ordering::SeqCst);

                    if explicit_ack {
                        assert!(matches!(
                            CoordinatorCell::complete(permit),
                            CompletionOutcome::RetirementAcknowledged
                        ));
                    } else {
                        drop(permit);
                    }
                    assert_eq!(active_drivers.load(Ordering::SeqCst), 0);
                    {
                        let coordinator = state.coordinator.lock();
                        assert!(matches!(coordinator.phase, CoordinatorPhase::Idle));
                        assert_eq!(
                            coordinator.pending.bits(),
                            if same_binding { 0b111 } else { 0b100 }
                        );
                        assert!(coordinator.last_indexed_at.is_none());
                        assert_eq!(
                            state.coordinator.notification_calls.load(Ordering::SeqCst),
                            1
                        );
                    }

                    assert!(notification_is_ready(&mut waiting_guard.enabled_notification).await);
                    let successor = acquired(request(
                        admission(&state.coordinator),
                        WorkMask::from_bits(0b001),
                        OwnerKind::Sync,
                    ));
                    assert_eq!(successor.identity.generation, target_generation);
                    assert_ne!(successor.identity, retired_identity);
                    active_drivers.store(1, Ordering::SeqCst);
                    assert_eq!(active_drivers.load(Ordering::SeqCst), 1);
                    drop(successor);
                }
            }
        }
    }

    #[tokio::test]
    async fn repeated_rebind_retargets_one_barrier_and_stale_terminals_do_nothing() {
        let state = AppState::new(2);
        let old = workspace("old", "uuid-old", "id-old");
        let _ = publish(&state, old.clone()).await;
        let permit = acquired(request(
            admission(&state.coordinator),
            WorkMask::from_bits(0b101),
            OwnerKind::Index,
        ));
        assert!(matches!(
            request(
                admission(&state.coordinator),
                WorkMask::from_bits(0b010),
                OwnerKind::Sync
            ),
            RequestOutcome::Enqueued
        ));

        let _ = publish(&state, old.clone()).await;
        let first_identity = match &state.coordinator.lock().phase {
            CoordinatorPhase::Retiring(barrier) => {
                assert_eq!(barrier.deferred.bits(), 0b111);
                barrier.retired_identity
            }
            CoordinatorPhase::Idle | CoordinatorPhase::Running(_) => {
                panic!("first rebind did not retire owner")
            }
        };
        let target_waiter = waiting(request(
            admission(&state.coordinator),
            WorkMask::default(),
            OwnerKind::Watcher,
        ));

        let (same_target_generation, _) = publish(&state, old.clone()).await;
        assert!(*target_waiter.cancel_rx.borrow());
        {
            let coordinator = state.coordinator.lock();
            let barrier = match &coordinator.phase {
                CoordinatorPhase::Retiring(barrier) => barrier,
                CoordinatorPhase::Idle | CoordinatorPhase::Running(_) => {
                    panic!("same-target rebind cleared retirement barrier")
                }
            };
            assert_eq!(barrier.retired_identity, first_identity);
            assert_eq!(barrier.target_generation, same_target_generation);
            assert_eq!(barrier.deferred.bits(), 0b111);
        }

        let newest = workspace("newest", "uuid-newest", "id-newest");
        let (newest_generation, _) = publish(&state, newest.clone()).await;
        {
            let coordinator = state.coordinator.lock();
            let barrier = match &coordinator.phase {
                CoordinatorPhase::Retiring(barrier) => barrier,
                CoordinatorPhase::Idle | CoordinatorPhase::Running(_) => {
                    panic!("distinct retarget cleared retirement barrier")
                }
            };
            assert_eq!(barrier.retired_identity, first_identity);
            assert_eq!(barrier.target_generation, newest_generation);
            assert_eq!(barrier.target_binding.workspace_id, "id-newest");
            assert!(barrier.deferred.is_empty());
        }
        assert!(matches!(
            request(
                admission(&state.coordinator),
                WorkMask::from_bits(0b010),
                OwnerKind::Sync
            ),
            RequestOutcome::Enqueued
        ));

        let (restored_generation, _) = publish(&state, old).await;
        {
            let coordinator = state.coordinator.lock();
            let barrier = match &coordinator.phase {
                CoordinatorPhase::Retiring(barrier) => barrier,
                CoordinatorPhase::Idle | CoordinatorPhase::Running(_) => {
                    panic!("return to retired binding cleared retirement barrier")
                }
            };
            assert_eq!(barrier.retired_identity, first_identity);
            assert_eq!(barrier.target_generation, restored_generation);
            assert_eq!(barrier.target_binding.workspace_id, "id-old");
            assert_eq!(barrier.deferred.bits(), 0b111);
        }

        let (final_generation, _) = publish(&state, newest).await;
        {
            let coordinator = state.coordinator.lock();
            let barrier = match &coordinator.phase {
                CoordinatorPhase::Retiring(barrier) => barrier,
                CoordinatorPhase::Idle | CoordinatorPhase::Running(_) => {
                    panic!("final distinct retarget cleared retirement barrier")
                }
            };
            assert_eq!(barrier.retired_identity, first_identity);
            assert_eq!(barrier.target_generation, final_generation);
            assert!(barrier.deferred.is_empty());
        }
        assert!(matches!(
            request(
                admission(&state.coordinator),
                WorkMask::from_bits(0b010),
                OwnerKind::Sync
            ),
            RequestOutcome::Enqueued
        ));

        drop(permit);
        let before_stale = {
            let coordinator = state.coordinator.lock();
            assert!(matches!(coordinator.phase, CoordinatorPhase::Idle));
            assert_eq!(coordinator.pending.bits(), 0b010);
            (
                coordinator.floor,
                coordinator.binding_identity.clone(),
                coordinator.pending,
                coordinator.last_indexed_at,
                state.coordinator.notification_calls.load(Ordering::SeqCst),
            )
        };
        let stale_permit = OwnerPermit {
            ownership: Some(PermitOwnership {
                cell: Arc::clone(&state.coordinator),
                token: GenerationToken {
                    floor: 1,
                    binding_identity: binding("old"),
                },
                binding_snapshot: binding("old"),
                cancel_rx: state.coordinator.lock().generation_cancel.subscribe(),
            }),
            identity: first_identity,
            work_mask: WorkMask::from_bits(0b111),
            cleanup_armed: true,
        };
        assert!(matches!(
            CoordinatorCell::complete(stale_permit),
            CompletionOutcome::Stale
        ));
        let after_stale = {
            let coordinator = state.coordinator.lock();
            (
                coordinator.floor,
                coordinator.binding_identity.clone(),
                coordinator.pending,
                coordinator.last_indexed_at,
                state.coordinator.notification_calls.load(Ordering::SeqCst),
            )
        };
        assert_eq!(before_stale, after_stale);
    }

    #[test]
    fn write_admission_preserves_direct_kind_and_rejects_stale_snapshot() {
        for kind in [OwnerKind::Index, OwnerKind::Sync] {
            let cell = coordinator_cell("write");
            let stale = admission(&cell);
            publish_idle_generation(&cell, binding("replacement"));
            assert!(matches!(
                request(stale, WorkMask::from_bits(0b111), kind),
                RequestOutcome::Stale
            ));
            assert!(cell.test_is_idle());

            let permit = acquired(request(admission(&cell), WorkMask::from_bits(0b111), kind));
            assert_eq!(permit.identity.kind, kind);
            assert!(matches!(
                CoordinatorCell::complete(permit),
                CompletionOutcome::Released
            ));
        }
    }

    #[tokio::test]
    async fn startup_and_watcher_empty_owner_release_passes_one_waiter_baton() {
        for owner_kind in [OwnerKind::Hydration, OwnerKind::Startup, OwnerKind::Watcher] {
            let cell = coordinator_cell("daemon-baton");
            let owner = acquired(request(admission(&cell), WorkMask::default(), owner_kind));
            let mut startup = waiting(request(
                admission(&cell),
                WorkMask::default(),
                OwnerKind::Startup,
            ));
            let mut watcher = waiting(request(
                admission(&cell),
                WorkMask::default(),
                OwnerKind::Watcher,
            ));

            assert!(matches!(
                CoordinatorCell::complete(owner),
                CompletionOutcome::Released
            ));
            assert!(notification_is_ready(&mut startup.enabled_notification).await);
            assert!(!notification_is_ready(&mut watcher.enabled_notification).await);

            rearm(&mut startup);
            let startup_owner = acquired(request(startup, WorkMask::default(), OwnerKind::Startup));
            assert!(matches!(
                CoordinatorCell::complete(startup_owner),
                CompletionOutcome::Released
            ));
            assert!(notification_is_ready(&mut watcher.enabled_notification).await);
            rearm(&mut watcher);
            let watcher_owner = acquired(request(watcher, WorkMask::default(), OwnerKind::Watcher));
            assert!(matches!(
                CoordinatorCell::complete(watcher_owner),
                CompletionOutcome::Released
            ));
            assert_eq!(cell.test_notification_calls(), 3);
        }
    }

    #[tokio::test]
    async fn startup_and_watcher_rebind_waits_for_drop_ack_and_isolates_stale_terminal() {
        for kind in [OwnerKind::Startup, OwnerKind::Watcher] {
            for same_binding in [true, false] {
                let state = AppState::new(2);
                let _ = publish(&state, workspace("old", "uuid-old", "id-old")).await;
                let permit = acquired(request(
                    admission(&state.coordinator),
                    WorkMask::default(),
                    kind,
                ));
                let retired_identity = permit.identity;
                assert!(matches!(
                    request(
                        admission(&state.coordinator),
                        WorkMask::from_bits(0b111),
                        OwnerKind::Sync
                    ),
                    RequestOutcome::Enqueued
                ));

                let target = if same_binding {
                    workspace("same", "uuid-old", "id-old")
                } else {
                    workspace("new", "uuid-new", "id-new")
                };
                let _ = publish(&state, target).await;
                assert!(state.coordinator.test_is_retiring());
                assert_eq!(state.coordinator.test_notification_calls(), 0);
                drop(permit);
                assert!(state.coordinator.test_is_idle());
                assert_eq!(state.coordinator.test_notification_calls(), 1);
                assert_eq!(
                    state.coordinator.test_pending_bits(),
                    if same_binding { 0b111 } else { 0 }
                );

                let before_stale = {
                    let coordinator = state.coordinator.lock();
                    (
                        coordinator.floor,
                        coordinator.binding_identity.clone(),
                        coordinator.pending,
                        coordinator.last_indexed_at,
                        state.coordinator.test_notification_calls(),
                    )
                };
                let stale_permit = OwnerPermit {
                    ownership: Some(PermitOwnership {
                        cell: Arc::clone(&state.coordinator),
                        token: GenerationToken {
                            floor: 1,
                            binding_identity: binding("old"),
                        },
                        binding_snapshot: binding("old"),
                        cancel_rx: state.coordinator.lock().generation_cancel.subscribe(),
                    }),
                    identity: retired_identity,
                    work_mask: WorkMask::default(),
                    cleanup_armed: true,
                };
                assert!(matches!(
                    CoordinatorCell::complete(stale_permit),
                    CompletionOutcome::Stale
                ));
                let after_stale = {
                    let coordinator = state.coordinator.lock();
                    (
                        coordinator.floor,
                        coordinator.binding_identity.clone(),
                        coordinator.pending,
                        coordinator.last_indexed_at,
                        state.coordinator.test_notification_calls(),
                    )
                };
                assert_eq!(before_stale, after_stale);
            }
        }
    }
}
