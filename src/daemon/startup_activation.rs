//! Daemon startup activation seam: the initial startup gate and the readiness
//! view published to health probes.
//!
//! This module owns the question "has the daemon finished its initial startup
//! gate, and what readiness does it publish?". The IPC composition root
//! ([`crate::daemon::ipc_server`]) delegates every readiness decision here
//! instead of reading activation state directly.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use chrono::Utc;
use tokio::sync::{mpsc, watch};
use tracing::{debug, error, info, warn};

use crate::daemon::lifecycle_policy::{
    drive_daemon_transferred_syncs, flush_daemon_snapshot, guarded_daemon_sync_context,
};
use crate::daemon::ttl::TtlTimer;
use crate::errors::EngramError;
use crate::models::health::ScanProgress;
use crate::server::state::{
    AppState, CompletionOutcome, CoordinatorCell, DispatchSnapshot, DriverTaskGuard, OwnerKind,
    OwnerProgressScope, SharedState,
};

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
    use crate::models::config::WorkspaceConfig;
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
        let state = AppState::new(1);

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
        let state = Arc::new(AppState::new(1));
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
        let state: SharedState = Arc::new(AppState::new(1));
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
        let state: SharedState = Arc::new(AppState::new(1));
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
