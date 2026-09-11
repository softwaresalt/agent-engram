//! Daemon startup activation seam: the initial startup gate and the readiness
//! view published to health probes.
//!
//! This module owns the question "has the daemon finished its initial startup
//! gate, and what readiness does it publish?". The IPC composition root
//! ([`crate::daemon::ipc_server`]) delegates every readiness decision here
//! instead of reading activation state directly.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(debug_assertions)]
use std::time::Duration;

use chrono::Utc;
use tokio::sync::{RwLock, mpsc, watch};
use tracing::{debug, error, info, warn};

use crate::daemon::lifecycle_policy::{
    drive_daemon_transferred_syncs, flush_daemon_snapshot, guarded_daemon_sync_context,
};
use crate::daemon::ttl::TtlTimer;
use crate::errors::{ActivationError, EngramError};
use crate::models::health::ScanProgress;
use crate::server::state::{
    AppState, CompletionOutcome, CoordinatorCell, DispatchSnapshot, DriverTaskGuard, OwnerKind,
    OwnerProgressScope, ReadRequestContext, SharedState,
};
use crate::services::generations::GenerationActivator;

/// Outcome of the daemon's initial startup gate.
///
/// The gate is passed once workspace hydration has reached its ready terminal.
/// Until then the daemon is bound and accepting connections but is not yet able
/// to serve real tool calls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupOutcome {
    /// The initial startup gate has not completed; hydration is still running.
    Pending,
    /// The initial startup gate completed; the daemon can serve tool calls.
    Ready,
}

/// Readiness snapshot published by the daemon for health probes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadinessView {
    /// Current outcome of the initial startup gate.
    pub startup: StartupOutcome,
}

impl ReadinessView {
    /// Return `true` when the startup gate has completed.
    #[must_use]
    pub fn is_ready(self) -> bool {
        matches!(self.startup, StartupOutcome::Ready)
    }

    /// Return `true` when read dispatch may proceed.
    ///
    /// Readiness and read dispatch are withheld together and released
    /// together: a daemon that is not ready has, by construction, no data to
    /// serve a read from. Keeping this derived from `startup` rather than
    /// tracked separately makes it impossible for the two signals to drift
    /// apart, which is exactly the failure mode that would let a request be
    /// dispatched against a generation that is not open yet.
    #[must_use]
    pub fn admits_dispatch(self) -> bool {
        self.is_ready()
    }
}

// ── Read-server startup gate (F18) ───────────────────────────────────────────

/// The phase a `ReadServer`-mode daemon has reached during startup.
///
/// Ordered by the sequence the startup path walks: the socket is bound first
/// so clients get a connection (and a `starting` health answer) rather than a
/// connection refusal, and only then is the initial generation activated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadServerPhase {
    /// The listening socket has not been bound yet.
    Binding,
    /// The socket is bound and the daemon answers `starting`; the initial
    /// generation activation has not completed.
    Starting,
    /// Exactly one generation context is open; readiness and read dispatch are
    /// released.
    Ready,
    /// The initial activation failed. The daemon stays bound so the failure is
    /// reportable, but it never publishes readiness.
    Failed,
}

/// The `ReadServer`-mode startup gate: the sole writer of [`ReadinessView`].
///
/// F18 owns exactly one decision -- "has the daemon opened its initial
/// generation, and may it therefore publish readiness?" -- and nothing else.
/// In particular it does **not** gate individual requests: admission is
/// [`crate::daemon::request_entry::admit`]'s sole authority, and that function
/// only *reads* the view this gate publishes. Splitting the write and the read
/// this way is what keeps a single, auditable answer to "is this daemon
/// serving?" rather than one answer per call site.
///
/// Managed mode never constructs this gate; its readiness continues to come
/// from [`readiness`] and is unchanged by F18.
#[derive(Debug)]
pub struct ReadServerStartupGate {
    activator: Arc<GenerationActivator>,
    branch: String,
    workspace_id: String,
    phase: RwLock<ReadServerPhase>,
    context: RwLock<Option<Arc<ReadRequestContext>>>,
    reconciliation_in_flight: AtomicBool,
}

impl ReadServerStartupGate {
    /// Construct a gate for a `ReadServer`-mode daemon.
    ///
    /// The identity this gate stamps onto captured contexts is derived from
    /// `activator`'s own [`ExpectedIdentity`][crate::services::generations::activation::ExpectedIdentity]
    /// rather than accepted as separate `branch`/`workspace_id` parameters.
    /// A single source of truth makes it structurally impossible for the
    /// gate and the activator it wraps to disagree about which identity is
    /// being served -- previously nothing prevented a caller from passing
    /// values here that diverged from what the activator actually validates
    /// manifests against.
    #[must_use]
    pub fn new(activator: Arc<GenerationActivator>) -> Self {
        let identity = activator.expected_identity();
        let branch = identity.branch().to_owned();
        let workspace_id = identity.workspace_id().to_owned();
        Self {
            activator,
            branch,
            workspace_id,
            phase: RwLock::new(ReadServerPhase::Binding),
            context: RwLock::new(None),
            reconciliation_in_flight: AtomicBool::new(false),
        }
    }

    /// Record that the listening socket has been bound.
    ///
    /// Binding first is deliberate: a client that connects during activation
    /// must receive a `starting` answer it can poll, not a connection refusal
    /// it would interpret as "no daemon".
    pub async fn socket_bound(&self) {
        let mut phase = self.phase.write().await;
        if matches!(*phase, ReadServerPhase::Binding) {
            *phase = ReadServerPhase::Starting;
        }
    }

    /// The phase this gate has reached.
    pub async fn phase(&self) -> ReadServerPhase {
        *self.phase.read().await
    }

    /// The readiness view this gate publishes.
    ///
    /// Readiness is `Ready` only in [`ReadServerPhase::Ready`]; every other
    /// phase -- including a failed activation -- publishes `Pending`, so a
    /// failed startup can never be mistaken for a serving daemon.
    pub async fn readiness(&self) -> ReadinessView {
        ReadinessView {
            startup: match self.phase().await {
                ReadServerPhase::Ready => StartupOutcome::Ready,
                ReadServerPhase::Binding | ReadServerPhase::Starting | ReadServerPhase::Failed => {
                    StartupOutcome::Pending
                }
            },
        }
    }

    /// The captured context, once the initial activation has completed.
    ///
    /// Returns `None` until readiness is published, which is what withholds
    /// read dispatch: there is simply no context for a request to capture.
    pub async fn admitted_context(&self) -> Option<Arc<ReadRequestContext>> {
        self.context.read().await.clone()
    }

    /// The activator this gate drives.
    ///
    /// Exposed for the F20 request-entry seam
    /// ([`crate::daemon::request_entry`]), which reconciles the durable
    /// manifest at read dispatch. F18 deliberately does not own that
    /// reconciliation: gating startup and admitting requests are different
    /// decisions, and collapsing them would make the startup gate a
    /// per-request authority it is explicitly not.
    #[must_use]
    pub fn activator(&self) -> &Arc<GenerationActivator> {
        &self.activator
    }

    /// The identity (branch, workspace) this gate stamps onto captured contexts.
    #[must_use]
    pub fn identity(&self) -> (&str, &str) {
        (&self.branch, &self.workspace_id)
    }

    /// Install a freshly-captured context as the one this daemon serves.
    ///
    /// Called by the F20 request-entry seam after a background activation
    /// produced a newer generation. Readiness is unaffected: this gate only
    /// ever *raises* readiness, and a daemon that is already serving stays
    /// serving across a generation swap.
    pub async fn install_context(&self, context: Arc<ReadRequestContext>) {
        let mut slot = self.context.write().await;
        *slot = Some(context);
    }

    /// Claim the single background-reconciliation slot for this gate.
    ///
    /// Returns `true` when this call successfully claimed the slot (no
    /// reconciliation task is currently in flight) and `false` when another
    /// task already holds it. Every admitted generation-backed read would
    /// otherwise spawn its own reconciliation task; the activator's
    /// single-flight mutex serializes the actual work but does not coalesce
    /// the *spawning* itself, so a slow activation (for example, blocked on a
    /// large database open) could otherwise build an unbounded task queue
    /// under sustained load. This claim happens before `tokio::spawn` is ever
    /// called, so a claim failure means no task is spawned at all rather than
    /// a task that spawns and immediately no-ops.
    pub fn try_claim_reconciliation(&self) -> bool {
        self.reconciliation_in_flight
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    /// Release the background-reconciliation slot claimed by
    /// [`try_claim_reconciliation`].
    ///
    /// Must be called on every exit path of the spawned reconciliation task
    /// -- success, `Ok(None)`, and error alike -- so the slot never stays
    /// stuck claimed. Callers should prefer an RAII guard over calling this
    /// directly so a task panic still releases the slot.
    pub fn release_reconciliation(&self) {
        self.reconciliation_in_flight
            .store(false, Ordering::Release);
    }

    /// Run the initial generation activation and release readiness on success.
    ///
    /// Exactly one generation context is opened: the activator is re-entrant,
    /// so a retried startup reuses the generation it already opened rather
    /// than opening a second copy of it.
    ///
    /// # Errors
    ///
    /// Returns the typed [`ActivationError`] produced by the activation
    /// attempt. The gate moves to [`ReadServerPhase::Failed`] and readiness
    /// stays withheld; nothing partially-activated is ever published.
    pub async fn run_initial_activation(&self) -> Result<Arc<ReadRequestContext>, ActivationError> {
        self.socket_bound().await;

        match self.activator.activate_initial().await {
            Ok(generation) => {
                let context = ReadRequestContext::from_generation(
                    generation,
                    self.branch.clone(),
                    self.workspace_id.clone(),
                );
                {
                    let mut slot = self.context.write().await;
                    *slot = Some(Arc::clone(&context));
                }
                // Readiness is published only after the context is installed,
                // so no observer can see `Ready` without a context to serve.
                *self.phase.write().await = ReadServerPhase::Ready;
                info!("read-server startup gate released: initial generation activated");
                Ok(context)
            }
            Err(error) => {
                *self.phase.write().await = ReadServerPhase::Failed;
                error!(
                    error = %error,
                    "read-server startup gate failed: initial generation activation rejected"
                );
                Err(error)
            }
        }
    }
}

/// Evaluate the daemon's initial startup gate for `state`.
///
/// The gate reflects exactly the condition the daemon has always used: the
/// retained hydration driver reached its ready terminal.
#[must_use]
pub fn run_initial_gate(state: &AppState) -> StartupOutcome {
    if state.is_hydration_ready() {
        StartupOutcome::Ready
    } else {
        StartupOutcome::Pending
    }
}

/// Build the readiness view published to health probes.
#[must_use]
pub fn readiness(state: &AppState) -> ReadinessView {
    ReadinessView {
        startup: run_initial_gate(state),
    }
}

// ── Startup driver ───────────────────────────────────────────────────────────

/// Drive daemon startup: bind the workspace, publish readiness, then run the
/// initial code-graph sync, registry ingestion, and embedding backfill.
pub(crate) async fn run_startup_driver(
    state: SharedState,
    workspace: String,
    ttl: Arc<TtlTimer>,
    shutdown_tx: Arc<watch::Sender<bool>>,
) {
    #[cfg(debug_assertions)]
    if let Some(delay_ms) = std::env::var("ENGRAM_TEST_STARTUP_DELAY_MS")
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
    {
        debug!(delay_ms, "applying test-only daemon startup delay");
        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
    }

    if let Err(error) = crate::tools::lifecycle::set_workspace(Arc::clone(&state), workspace).await
    {
        error!(%error, "workspace hydration failed — initiating shutdown");
        let _ = shutdown_tx.send(true);
        return;
    }

    info!("workspace binding published; background database hydration started");
    ttl.reset();
    let ttl_task = Arc::clone(&ttl);
    tokio::spawn(async move {
        ttl_task.run_until_expired(shutdown_tx).await;
    });

    let Some((admission, snapshot, workspace_config)) = guarded_daemon_sync_context(&state).await
    else {
        return;
    };
    let permit = match admission.acquire_background(OwnerKind::Startup).await {
        Ok(Some(permit)) => permit,
        Ok(None) => return,
        Err(error) => {
            error!(%error, "startup coordinator admission failed");
            return;
        }
    };
    let context = DispatchSnapshot {
        workspace: snapshot,
        config: workspace_config,
    };
    let Some((mut permit, context)) = (match state
        .prepare_daemon_mutation(permit, context, OwnerKind::Startup)
        .await
    {
        Ok(prepared) => prepared,
        Err(error) => {
            error!(%error, "startup branch preparation failed");
            return;
        }
    }) else {
        return;
    };
    let snapshot = context.workspace;
    let workspace_config = context.config;
    let Some(progress_scope) = permit.progress_scope() else {
        error!("startup permit lost ownership before progress relay creation");
        return;
    };
    let (mut backfill_progress, progress_tx) =
        StartupBackfillProgressRelay::spawn(Arc::clone(&state), progress_scope.clone());

    let operation = async {
        let mut backfill_result = None;
        let workspace_path = std::path::PathBuf::from(&snapshot.path);
        let should_flush = match crate::services::code_graph::sync_workspace(
            &workspace_path,
            &snapshot.data_dir,
            &snapshot.branch,
            &workspace_config.code_graph,
        )
        .await
        {
            Ok(result) => {
                info!(
                    files_added = result.files_added,
                    files_modified = result.files_modified,
                    files_unchanged = result.files_unchanged,
                    "startup auto-sync complete"
                );
                true
            }
            Err(error) => {
                warn!(%error, "startup auto-sync failed");
                false
            }
        };

        match crate::db::connect_db(&snapshot.data_dir, &snapshot.branch).await {
            Ok(db) => {
                let queries = crate::db::queries::CodeGraphQueries::new(db);
                let registry_path = workspace_path.join(".engram").join("registry.yaml");
                match crate::services::registry::load_registry(&registry_path) {
                    Ok(Some(mut config)) => {
                        let _ = crate::services::registry::validate_sources(
                            &mut config,
                            &workspace_path,
                        );
                        match crate::services::ingestion::ingest_all_sources(
                            &config,
                            &workspace_path,
                            &queries,
                        )
                        .await
                        {
                            Ok(summary) => {
                                info!(
                                    ingested = summary.ingested,
                                    unchanged = summary.unchanged,
                                    total = summary.total_files,
                                    "startup registry ingestion complete"
                                );
                            }
                            Err(error) => {
                                warn!(%error, "startup registry ingestion failed");
                            }
                        }
                    }
                    Ok(None) => {
                        debug!("no registry.yaml — skipping content ingestion");
                    }
                    Err(error) => {
                        warn!(%error, "startup registry load failed");
                    }
                }

                backfill_result = Some(backfill_with_scan_progress(&queries, &progress_tx).await);
            }
            Err(error) => {
                warn!(%error, "startup ingestion: failed to connect to database");
            }
        }

        if should_flush {
            if let Err(error) = flush_daemon_snapshot(&snapshot).await {
                warn!(%error, "startup auto-flush failed");
            }
        }
        backfill_result
    };

    let backfill_result = permit.run_until_cancelled(operation).await;
    // The cancellable future owns a producer clone. Close it before joining so
    // normal completion cannot wait forever for its own progress channel.
    drop(progress_tx);
    if backfill_result.is_none() {
        backfill_progress.abort_and_join().await;
        return;
    }
    backfill_progress.join().await;
    if let Some(result) = backfill_result.flatten() {
        match result {
            Ok(updated) => {
                if let Some(snapshot) = backfill_completion_snapshot(
                    backfill_progress.relayed_running(),
                    updated,
                    Utc::now().to_rfc3339(),
                ) {
                    let _ = state
                        .set_scan_progress_for_owner(&progress_scope, Some(snapshot))
                        .await;
                }
                if updated != 0 {
                    info!(updated, "startup content embedding backfill complete");
                }
            }
            Err(error) => {
                warn!(%error, "startup content embedding backfill failed");
            }
        }
    }
    let transferred = match CoordinatorCell::complete(permit) {
        CompletionOutcome::Transferred(successor) => Some(successor),
        CompletionOutcome::Released
        | CompletionOutcome::RetirementAcknowledged
        | CompletionOutcome::SequenceExhausted(_)
        | CompletionOutcome::Stale => None,
    };
    if let Some(successor) = transferred {
        drive_daemon_transferred_syncs(
            &state,
            &snapshot,
            &workspace_config,
            successor,
            "startup",
            #[cfg(test)]
            None,
        )
        .await;
    }
}

// ── Readiness publication helpers ────────────────────────────────────────────

/// Build a `running` scan-status snapshot reflecting embedding-backfill progress.
fn backfill_running_progress(done: usize, total: usize) -> ScanProgress {
    ScanProgress {
        running: true,
        files_scanned: done as u64,
        files_total: total as u64,
        last_completed_at: None,
    }
}

/// Build a completed scan-status snapshot for a finished embedding backfill.
fn backfill_completed_progress(done: usize, completed_at: String) -> ScanProgress {
    ScanProgress {
        running: false,
        files_scanned: done as u64,
        files_total: done as u64,
        last_completed_at: Some(completed_at),
    }
}

/// Decide the `scan_status` snapshot to write once the backfill finishes.
///
/// Returns a `running: false` completed snapshot whenever any `running`
/// progress was relayed — even if `embedded == 0` (model unavailable or every
/// write-back failed). This clears a `running: true` status that would
/// otherwise persist forever, since owner completion does not touch
/// `scan_progress`. Returns `None` when no running progress was relayed (there
/// was nothing to clear).
fn backfill_completion_snapshot(
    relayed_running: bool,
    embedded: usize,
    completed_at: String,
) -> Option<ScanProgress> {
    if relayed_running {
        Some(backfill_completed_progress(embedded, completed_at))
    } else {
        None
    }
}

/// Run the content-embedding backfill using the startup owner's progress relay.
async fn backfill_with_scan_progress(
    queries: &crate::db::queries::CodeGraphQueries,
    progress_tx: &mpsc::UnboundedSender<crate::services::ingestion::BackfillProgress>,
) -> Result<usize, EngramError> {
    crate::services::ingestion::backfill_content_embeddings(queries, Some(progress_tx)).await
}

/// Parent-owned progress child for the startup embedding phase.
///
/// The relay is created outside the cancellable startup future so cancellation
/// can abort and join it before the startup permit acknowledges retirement.
/// Every publication is fenced to the exact startup owner.
struct StartupBackfillProgressRelay {
    tx: Option<mpsc::UnboundedSender<crate::services::ingestion::BackfillProgress>>,
    relayed_running: Arc<AtomicBool>,
    child: Option<DriverTaskGuard>,
}

impl StartupBackfillProgressRelay {
    fn spawn(
        state: SharedState,
        scope: OwnerProgressScope,
    ) -> (
        Self,
        mpsc::UnboundedSender<crate::services::ingestion::BackfillProgress>,
    ) {
        let (tx, mut rx) =
            mpsc::unbounded_channel::<crate::services::ingestion::BackfillProgress>();
        let relayed_running = Arc::new(AtomicBool::new(false));
        let relayed_for_updater = Arc::clone(&relayed_running);
        let child = tokio::spawn(async move {
            while let Some(progress) = rx.recv().await {
                if state
                    .set_scan_progress_for_owner(
                        &scope,
                        Some(backfill_running_progress(progress.done, progress.total)),
                    )
                    .await
                {
                    relayed_for_updater.store(true, Ordering::Relaxed);
                }
            }
        });
        let producer = tx.clone();
        (
            Self {
                tx: Some(tx),
                relayed_running,
                child: Some(DriverTaskGuard { task: Some(child) }),
            },
            producer,
        )
    }

    fn relayed_running(&self) -> bool {
        self.relayed_running.load(Ordering::Relaxed)
    }

    async fn join(&mut self) {
        let _ = self.tx.take();
        if let Some(child) = self.child.take() {
            if let Err(error) = child.join().await {
                warn!(%error, "startup backfill progress updater failed");
            }
        }
    }

    async fn abort_and_join(&mut self) {
        let _ = self.tx.take();
        if let Some(child) = self.child.take() {
            let _ = child.abort_and_join().await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::StaleStrategy;
    use crate::models::config::{DaemonMode, WorkspaceConfig};
    use crate::server::state::WorkspaceSnapshot;

    fn coordinator_snapshot(
        workspace_id: &str,
        workspace_uuid: &str,
        path: &std::path::Path,
        data_dir: std::path::PathBuf,
    ) -> WorkspaceSnapshot {
        WorkspaceSnapshot {
            workspace_id: workspace_id.to_owned(),
            workspace_uuid: workspace_uuid.to_owned(),
            branch: "main".to_owned(),
            data_dir,
            path: path.display().to_string(),
            last_flush: None,
            stale_files: false,
            connection_count: 0,
            file_mtimes: std::collections::HashMap::new(),
        }
    }

    #[tokio::test]
    async fn initial_gate_reports_pending_until_hydration_is_ready() {
        let state = AppState::with_mode(DaemonMode::Managed, 1, StaleStrategy::Warn, 20, 60);

        assert_eq!(run_initial_gate(&state), StartupOutcome::Pending);
        assert!(
            !readiness(&state).is_ready(),
            "an unhydrated daemon must not publish readiness"
        );
    }

    #[tokio::test]
    async fn startup_prepares_current_head_before_database_or_file_mutation() {
        let metrics_guard = crate::services::metrics::test_writer_guard().await;
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = temp.path().join("workspace");
        std::fs::create_dir_all(workspace.join(".git")).expect("create git metadata");
        std::fs::write(
            workspace.join(".git").join("HEAD"),
            "ref: refs/heads/main\n",
        )
        .expect("write git HEAD");
        let state = Arc::new(AppState::with_mode(
            DaemonMode::Managed,
            1,
            StaleStrategy::Warn,
            20,
            60,
        ));
        let mut stale_snapshot = coordinator_snapshot(
            "id-stale",
            "uuid-worktree",
            &workspace,
            temp.path().join("data"),
        );
        stale_snapshot.branch = "captured-before-checkout".to_owned();
        crate::services::metrics::configure_test_disabled_writer(
            &metrics_guard,
            &workspace,
            &stale_snapshot.branch,
        )
        .await
        .expect("configure disabled startup metrics");
        let _ = state
            .publish_workspace_generation(stale_snapshot, Some(WorkspaceConfig::default()))
            .await
            .expect("publish stale capture");
        let (admission, workspace, config) = guarded_daemon_sync_context(&state)
            .await
            .expect("guarded startup context");
        let permit = admission
            .acquire_background(OwnerKind::Startup)
            .await
            .expect("startup admission")
            .expect("startup permit");

        let (_, prepared) = state
            .prepare_daemon_mutation(
                permit,
                DispatchSnapshot { workspace, config },
                OwnerKind::Startup,
            )
            .await
            .expect("prepare startup mutation")
            .expect("same worktree remains active");

        assert_eq!(
            prepared.workspace.branch, "main",
            "startup must refresh HEAD before its first DB/file mutation"
        );
        assert_eq!(
            state
                .snapshot_workspace()
                .await
                .expect("published workspace")
                .branch,
            "main",
            "the refreshed branch must be coherently published"
        );
        crate::services::metrics::shutdown()
            .await
            .expect("reset startup metrics");
    }

    #[test]
    fn backfill_running_progress_marks_scan_active_with_counts() {
        let progress = backfill_running_progress(128, 2441);
        assert!(progress.running, "backfill in flight must report running");
        assert_eq!(progress.files_scanned, 128);
        assert_eq!(progress.files_total, 2441);
        assert!(
            progress.last_completed_at.is_none(),
            "an in-flight backfill has no completion timestamp"
        );
    }

    #[test]
    fn backfill_completed_progress_marks_scan_finished() {
        let progress = backfill_completed_progress(2441, "2026-07-06T07:28:21Z".to_owned());
        assert!(!progress.running, "finished backfill must clear running");
        assert_eq!(progress.files_scanned, 2441);
        assert_eq!(
            progress.files_total, progress.files_scanned,
            "completed snapshot reports the embedded record count as the total"
        );
        assert_eq!(
            progress.last_completed_at.as_deref(),
            Some("2026-07-06T07:28:21Z")
        );
    }

    #[test]
    fn backfill_completion_snapshot_clears_running_even_when_nothing_embedded() {
        // Regression: when running progress was relayed but the model was
        // unavailable (embedded == 0), status must still be cleared to
        // `running: false` — otherwise it reports indexing forever.
        let snapshot = backfill_completion_snapshot(true, 0, "2026-07-06T07:28:21Z".to_owned())
            .expect("relayed progress must yield a completion snapshot");
        assert!(!snapshot.running, "status must be cleared to not-running");
        assert_eq!(snapshot.files_scanned, 0);
    }

    #[test]
    fn backfill_completion_snapshot_reports_embedded_count() {
        let snapshot = backfill_completion_snapshot(true, 42, "2026-07-06T07:28:21Z".to_owned())
            .expect("relayed progress must yield a completion snapshot");
        assert!(!snapshot.running);
        assert_eq!(snapshot.files_scanned, 42);
    }

    #[test]
    fn backfill_completion_snapshot_is_none_when_no_progress_relayed() {
        // Nothing was set to running (no pending records), so there is nothing
        // to clear and the existing scan_status must be left untouched.
        assert!(
            backfill_completion_snapshot(false, 0, "2026-07-06T07:28:21Z".to_owned()).is_none(),
            "no relayed progress means no completion snapshot"
        );
    }

    #[tokio::test]
    async fn backfill_progress_relay_updates_scan_status() {
        let state: SharedState = Arc::new(AppState::with_mode(
            DaemonMode::Managed,
            1,
            StaleStrategy::Warn,
            20,
            60,
        ));
        let workspace = tempfile::tempdir().expect("workspace tempdir");
        let data = tempfile::tempdir().expect("data tempdir");
        std::fs::create_dir_all(workspace.path().join(".git")).expect("create git metadata");
        std::fs::write(
            workspace.path().join(".git").join("HEAD"),
            "ref: refs/heads/main\n",
        )
        .expect("write git HEAD");
        let _ = state
            .publish_workspace_generation(
                coordinator_snapshot(
                    "id-main",
                    "uuid-worktree",
                    workspace.path(),
                    data.path().to_path_buf(),
                ),
                Some(WorkspaceConfig::default()),
            )
            .await
            .expect("publish workspace");
        let (admission, _, _) = guarded_daemon_sync_context(&state)
            .await
            .expect("guarded startup context");
        let permit = admission
            .acquire_background(OwnerKind::Startup)
            .await
            .expect("startup admission")
            .expect("startup permit");
        let scope = permit.progress_scope().expect("progress scope");
        let (mut relay, sender) = StartupBackfillProgressRelay::spawn(Arc::clone(&state), scope);

        sender
            .send(crate::services::ingestion::BackfillProgress {
                done: 256,
                total: 1000,
            })
            .expect("send progress");
        drop(sender);
        relay.join().await;

        let snapshot = state
            .scan_progress_snapshot()
            .await
            .expect("scan status populated by relay");
        assert!(snapshot.running);
        assert_eq!(snapshot.files_scanned, 256);
        assert_eq!(snapshot.files_total, 1000);
    }

    #[tokio::test]
    async fn cancelled_backfill_progress_relay_quiesces_and_rejects_stale_writes() {
        let state: SharedState = Arc::new(AppState::with_mode(
            DaemonMode::Managed,
            1,
            StaleStrategy::Warn,
            20,
            60,
        ));
        let workspace = tempfile::tempdir().expect("workspace tempdir");
        let data = tempfile::tempdir().expect("data tempdir");
        std::fs::create_dir_all(workspace.path().join(".git")).expect("create git metadata");
        std::fs::write(
            workspace.path().join(".git").join("HEAD"),
            "ref: refs/heads/main\n",
        )
        .expect("write git HEAD");
        let _ = state
            .publish_workspace_generation(
                coordinator_snapshot(
                    "id-main",
                    "uuid-worktree",
                    workspace.path(),
                    data.path().to_path_buf(),
                ),
                Some(WorkspaceConfig::default()),
            )
            .await
            .expect("publish workspace");
        let (admission, _, _) = guarded_daemon_sync_context(&state)
            .await
            .expect("guarded startup context");
        let permit = admission
            .acquire_background(OwnerKind::Startup)
            .await
            .expect("startup admission")
            .expect("startup permit");
        let scope = permit.progress_scope().expect("progress scope");
        let (mut relay, sender) = StartupBackfillProgressRelay::spawn(Arc::clone(&state), scope);

        relay.abort_and_join().await;
        assert!(
            sender
                .send(crate::services::ingestion::BackfillProgress {
                    done: 999,
                    total: 1000,
                })
                .is_err(),
            "joined cancellation must close the progress receiver"
        );
        assert!(matches!(
            CoordinatorCell::complete(permit),
            CompletionOutcome::Released
        ));
        assert!(
            state.scan_progress_snapshot().await.is_none(),
            "a quiesced stale child must not publish progress after retirement"
        );
    }
}
