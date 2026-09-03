//! Daemon lifecycle policy seam: start and shutdown lifecycle for the IPC
//! composition root.
//!
//! The composition root ([`crate::daemon::ipc_server`]) owns framing and the
//! accept loop; every lifecycle edge it crosses is delegated here.

use std::collections::BTreeSet;
use std::sync::Arc;
#[cfg(test)]
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::daemon::ttl::TtlTimer;
use crate::errors::EngramError;
use crate::models::WatcherEvent;
use crate::models::config::WorkspaceConfig;
use crate::server::state::{
    AppState, CompletionOutcome, CoordinatorCell, DispatchSnapshot, DriverTaskGuard, OwnerKind,
    OwnerPermit, SharedState, WorkspaceSnapshot,
};

/// Record the daemon lifecycle start edge.
///
/// Called by the composition root once the IPC endpoint is bound and the daemon
/// begins accepting connections.
pub fn on_start(state: &AppState) {
    debug!(
        active_connections = state.active_connections(),
        "daemon lifecycle start"
    );
}

/// Record the daemon lifecycle shutdown edge.
///
/// Called by the composition root after the accept loop has quiesced and before
/// durable shutdown work runs.
pub fn on_shutdown(state: &AppState) {
    debug!(
        uptime_seconds = state.uptime_seconds(),
        "daemon lifecycle shutdown"
    );
}

// ── Shared daemon driver plumbing ────────────────────────────────────────────

/// Atomically snapshot a daemon driver's immutable payload and admission guard.
///
/// This is the single acquisition seam shared by the auto-sync and file-watcher
/// closures in the daemon entry points (092.003-T). Routing all four sites
/// through one helper keeps the atomicity guarantee testable:
/// [`AppState::guarded_workspace_and_config`] holds both read locks together, so
/// a concurrent [`AppState::set_workspace_and_config`] cannot yield a torn
/// `(workspace_i, config_j)` pair. Reverting this body to two separate reads
/// would reopen that window — the `daemon_sync_context_never_tears_pair` unit
/// test guards against exactly that regression.
pub(crate) async fn guarded_daemon_sync_context(
    state: &AppState,
) -> Option<(
    crate::server::state::AdmissionGuard,
    WorkspaceSnapshot,
    WorkspaceConfig,
)> {
    state.guarded_workspace_and_config().await
}

/// Return `true` when both snapshots describe the same bound worktree instance.
pub(crate) fn same_workspace_instance(left: &WorkspaceSnapshot, right: &WorkspaceSnapshot) -> bool {
    left.workspace_uuid == right.workspace_uuid && left.path == right.path
}

impl AppState {
    pub(crate) async fn prepare_daemon_mutation(
        self: &Arc<Self>,
        permit: OwnerPermit,
        context: DispatchSnapshot,
        kind: OwnerKind,
    ) -> Result<Option<(OwnerPermit, DispatchSnapshot)>, EngramError> {
        crate::tools::write::prepare_branch_owner(self, permit, context, kind).await
    }
}

/// Dehydrate the code graph and refresh metrics for a bound workspace snapshot.
pub(crate) async fn flush_daemon_snapshot(snapshot: &WorkspaceSnapshot) -> Result<(), EngramError> {
    let _guard = crate::services::dehydration::acquire_flush_lock().await;
    let db = crate::db::connect_db(&snapshot.data_dir, &snapshot.branch).await?;
    let queries = crate::db::queries::CodeGraphQueries::new(db);
    crate::services::dehydration::dehydrate_code_graph(
        &queries,
        &snapshot.data_dir,
        &snapshot.branch,
    )
    .await?;
    if let Err(error) = crate::services::metrics::compute_and_write_summary(
        std::path::Path::new(&snapshot.path),
        &snapshot.branch,
    )
    .await
    {
        if !matches!(
            error,
            EngramError::Metrics(crate::errors::MetricsError::NotFound { .. })
        ) {
            warn!(
                %error,
                branch = %snapshot.branch,
                "metrics summary write failed during flush"
            );
        }
    }
    Ok(())
}

/// Spawn a daemon driver whose handle owns the task's lifetime.
pub(crate) fn spawn_daemon_driver<F>(future: F) -> DriverTaskGuard
where
    F: std::future::Future<Output = ()> + Send + 'static,
{
    DriverTaskGuard {
        task: Some(tokio::spawn(future)),
    }
}

/// Join a retained hydration driver to its safe terminal before shutdown.
pub(crate) async fn join_retained_hydration(state: &AppState) -> Result<(), EngramError> {
    if let Some(driver) = state.take_hydration_driver() {
        driver.join().await.map_err(|error| {
            EngramError::System(crate::errors::SystemError::DatabaseError {
                reason: format!("retained hydration failed before safe terminal: {error}"),
            })
        })?;
    }
    Ok(())
}

// ── Shutdown cleanup ─────────────────────────────────────────────────────────

#[cfg(test)]
static SHUTDOWN_FLUSH_CALLS: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

async fn flush_all_workspaces_for_shutdown(state: &SharedState) -> Result<(), EngramError> {
    #[cfg(test)]
    SHUTDOWN_FLUSH_CALLS.fetch_add(1, Ordering::SeqCst);
    crate::services::dehydration::flush_all_workspaces(state).await
}

const SHUTDOWN_ERROR_EVIDENCE_BYTES: usize = 512;

fn bounded_shutdown_error_evidence(error: &EngramError) -> String {
    let mut evidence = error.to_string();
    if evidence.len() <= SHUTDOWN_ERROR_EVIDENCE_BYTES {
        return evidence;
    }

    let mut end = SHUTDOWN_ERROR_EVIDENCE_BYTES - 3;
    while !evidence.is_char_boundary(end) {
        end -= 1;
    }
    evidence.truncate(end);
    evidence.push_str("...");
    evidence
}

fn combine_shutdown_results(
    metrics_result: Result<(), EngramError>,
    flush_result: Result<(), EngramError>,
) -> Result<(), EngramError> {
    match (metrics_result, flush_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(metrics_error), Ok(())) => Err(metrics_error),
        (Ok(()), Err(flush_error)) => Err(flush_error),
        (Err(metrics_error), Err(flush_error)) => {
            let durable_evidence = bounded_shutdown_error_evidence(&flush_error);
            let metrics_evidence = bounded_shutdown_error_evidence(&metrics_error);
            Err(EngramError::System(
                crate::errors::SystemError::FlushFailed {
                    path: format!(
                        "shutdown cleanup: durable workspace flush failed: {durable_evidence}; \
                         metrics teardown also failed: {metrics_evidence}"
                    ),
                },
            ))
        }
    }
}

/// Run daemon shutdown cleanup: metrics teardown followed by a durable flush.
pub(crate) async fn shutdown_services(state: &SharedState) -> Result<(), EngramError> {
    let metrics_result = crate::services::metrics::shutdown().await;
    let flush_result = flush_all_workspaces_for_shutdown(state).await;
    combine_shutdown_results(metrics_result, flush_result)
}

// ── Transferred-sync driver ──────────────────────────────────────────────────

/// Drive successor owners handed over by coordinator completion.
pub(crate) async fn drive_daemon_transferred_syncs(
    state: &SharedState,
    snapshot: &WorkspaceSnapshot,
    workspace_config: &WorkspaceConfig,
    successor: OwnerPermit,
    driver: &'static str,
    #[cfg(test)] operation_reached: Option<&AtomicBool>,
) {
    let mut successor = successor;
    let mut current_ctx = DispatchSnapshot {
        workspace: snapshot.clone(),
        config: workspace_config.clone(),
    };
    loop {
        let prepared =
            crate::tools::write::prepare_transferred_sync(state, successor, current_ctx).await;
        let Some((prepared_successor, prepared_ctx)) = (match prepared {
            Ok(prepared) => prepared,
            Err(error) => {
                warn!(%error, driver, "daemon transferred branch preparation failed");
                return;
            }
        }) else {
            return;
        };
        successor = prepared_successor;
        current_ctx = prepared_ctx;
        let work_bits = successor.work_bits();
        let operation = async {
            #[cfg(test)]
            if let Some(operation_reached) = operation_reached {
                operation_reached.store(true, Ordering::SeqCst);
            }
            let workspace_path = std::path::PathBuf::from(&current_ctx.workspace.path);
            let result = crate::services::code_graph::sync_workspace_with_progress(
                &workspace_path,
                &current_ctx.workspace.data_dir,
                &current_ctx.workspace.branch,
                &current_ctx.config.code_graph,
                work_bits & 0b100 != 0,
                work_bits & 0b010 != 0,
                None,
            )
            .await;
            match &result {
                Ok(result) => {
                    let unfulfilled_work_bits =
                        crate::tools::write::unfulfilled_work_bits(&result.errors, work_bits);
                    info!(
                        driver,
                        files_added = result.files_added,
                        files_modified = result.files_modified,
                        unfulfilled_work_bits,
                        "daemon transferred sync complete"
                    );
                    if let Err(error) = flush_daemon_snapshot(&current_ctx.workspace).await {
                        warn!(%error, driver, "daemon transferred flush failed");
                    }
                    unfulfilled_work_bits == 0
                }
                Err(error) => {
                    warn!(%error, driver, "daemon transferred sync failed");
                    false
                }
            }
        };
        if !matches!(successor.run_until_cancelled(operation).await, Some(true)) {
            return;
        }
        let _ = state.set_hydration_ready_for_permit(&successor);
        successor = match CoordinatorCell::complete(successor) {
            CompletionOutcome::Transferred(next) => next,
            CompletionOutcome::Released
            | CompletionOutcome::RetirementAcknowledged
            | CompletionOutcome::SequenceExhausted(_)
            | CompletionOutcome::Stale => return,
        };
    }
}

// ── File-watcher reconciliation driver ───────────────────────────────────────

/// Reconcile debounced watcher events into code-graph and content updates.
pub(crate) async fn run_watcher_driver(
    state: SharedState,
    ttl: Arc<TtlTimer>,
    mut event_rx: mpsc::UnboundedReceiver<WatcherEvent>,
    reactive_markdown: bool,
) {
    while let Some(event) = event_rx.recv().await {
        ttl.reset();
        let Some((mut admission, mut snapshot, mut workspace_config)) =
            guarded_daemon_sync_context(&state).await
        else {
            continue;
        };
        let mut pending_reingest = BTreeSet::new();
        let mut pending_reindex = match crate::daemon::debounce::adapt_event(&event) {
            crate::daemon::debounce::ServiceAction::ReindexFile { .. } => true,
            crate::daemon::debounce::ServiceAction::ReingestContent { path }
                if reactive_markdown =>
            {
                pending_reingest.insert(path);
                false
            }
            crate::daemon::debounce::ServiceAction::ReingestContent { .. }
            | crate::daemon::debounce::ServiceAction::Skip => false,
        };

        while let Ok(Some(event)) =
            tokio::time::timeout(Duration::from_secs(2), event_rx.recv()).await
        {
            ttl.reset();
            match crate::daemon::debounce::adapt_event(&event) {
                crate::daemon::debounce::ServiceAction::ReindexFile { .. } => {
                    pending_reindex = true;
                }
                crate::daemon::debounce::ServiceAction::ReingestContent { path }
                    if reactive_markdown =>
                {
                    pending_reingest.insert(path);
                }
                crate::daemon::debounce::ServiceAction::ReingestContent { .. }
                | crate::daemon::debounce::ServiceAction::Skip => {}
            }
        }
        if !pending_reindex && pending_reingest.is_empty() {
            continue;
        }

        'reconcile_batch: loop {
            let mut permit = match admission.acquire_background(OwnerKind::Watcher).await {
                Ok(Some(permit)) => permit,
                Ok(None) => {
                    let Some((next_admission, next_snapshot, next_config)) =
                        guarded_daemon_sync_context(&state).await
                    else {
                        break 'reconcile_batch;
                    };
                    if !same_workspace_instance(&snapshot, &next_snapshot) {
                        break 'reconcile_batch;
                    }
                    admission = next_admission;
                    snapshot = next_snapshot;
                    workspace_config = next_config;
                    continue 'reconcile_batch;
                }
                Err(error) => {
                    error!(%error, "watcher coordinator admission failed");
                    break 'reconcile_batch;
                }
            };
            let context = DispatchSnapshot {
                workspace: snapshot,
                config: workspace_config,
            };
            let Some((prepared_permit, context)) = (match state
                .prepare_daemon_mutation(permit, context, OwnerKind::Watcher)
                .await
            {
                Ok(prepared) => prepared,
                Err(error) => {
                    error!(%error, "watcher branch preparation failed");
                    break 'reconcile_batch;
                }
            }) else {
                break 'reconcile_batch;
            };
            permit = prepared_permit;
            snapshot = context.workspace;
            workspace_config = context.config;
            let operation = async {
                let workspace_path = std::path::PathBuf::from(&snapshot.path);
                let should_flush = if pending_reindex {
                    match crate::services::code_graph::sync_workspace(
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
                                "file-change auto-sync complete"
                            );
                            true
                        }
                        Err(error) => {
                            warn!(%error, "file-change auto-sync failed");
                            false
                        }
                    }
                } else {
                    false
                };

                if !pending_reingest.is_empty() {
                    crate::services::reactive_sync::reingest_pending_markdown(
                        &workspace_path,
                        &snapshot.data_dir,
                        &snapshot.branch,
                        &pending_reingest,
                    )
                    .await;
                }

                if should_flush {
                    if let Err(error) = flush_daemon_snapshot(&snapshot).await {
                        warn!(%error, "file-change auto-flush failed");
                    }
                }
            };
            if permit.run_until_cancelled(operation).await.is_none() {
                drop(permit);
                let Some((next_admission, next_snapshot, next_config)) =
                    guarded_daemon_sync_context(&state).await
                else {
                    break 'reconcile_batch;
                };
                if !same_workspace_instance(&snapshot, &next_snapshot) {
                    break 'reconcile_batch;
                }
                admission = next_admission;
                snapshot = next_snapshot;
                workspace_config = next_config;
                continue 'reconcile_batch;
            }

            if let CompletionOutcome::Transferred(successor) = CoordinatorCell::complete(permit) {
                drive_daemon_transferred_syncs(
                    &state,
                    &snapshot,
                    &workspace_config,
                    successor,
                    "watcher",
                    #[cfg(test)]
                    None,
                )
                .await;
            }
            break 'reconcile_batch;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::server::state::{RequestOutcome, WorkMask};

    fn coordinator_snapshot(
        workspace_id: &str,
        workspace_uuid: &str,
        path: &Path,
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

    /// 092.003-T: the shared daemon `(workspace, config)` acquisition seam must
    /// never expose a torn pair to a background-sync closure while a concurrent
    /// bind is flipping the active workspace. This is the regression guard for
    /// [`guarded_daemon_sync_context`]: reverting its body to two separate
    /// reads reopens the tear window and makes this test fail.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn daemon_sync_context_never_tears_pair() {
        use crate::models::config::CodeGraphConfig;
        use std::collections::HashMap;

        const LARGE: u64 = 8_000_000;
        const SMALL: u64 = 1_024;

        fn snapshot_for(path: &str) -> WorkspaceSnapshot {
            WorkspaceSnapshot {
                workspace_id: format!("ws-{path}"),
                workspace_uuid: format!("uuid-{path}"),
                branch: "main".to_owned(),
                data_dir: std::path::PathBuf::from("unused"),
                path: path.to_owned(),
                last_flush: None,
                stale_files: false,
                connection_count: 0,
                file_mtimes: HashMap::new(),
            }
        }
        fn config_with(max_file_size_bytes: u64) -> WorkspaceConfig {
            WorkspaceConfig {
                code_graph: CodeGraphConfig {
                    max_file_size_bytes,
                    ..CodeGraphConfig::default()
                },
                ..WorkspaceConfig::default()
            }
        }

        // Two internally-consistent (workspace, config) states. A torn read
        // pairs one state's `path` with the other's `max_file_size_bytes`.
        let snapshot_a = snapshot_for("/ws/a");
        let snapshot_b = snapshot_for("/ws/b");
        let config_a = config_with(LARGE);
        let config_b = config_with(SMALL);

        let state = Arc::new(AppState::new(10));
        state
            .set_workspace_and_config(snapshot_a.clone(), Some(config_a.clone()))
            .await
            .expect("seed bind A");

        let stop = Arc::new(AtomicBool::new(false));
        let writer_state = state.clone();
        let writer_stop = stop.clone();
        let writer = tokio::spawn(async move {
            while !writer_stop.load(Ordering::Relaxed) {
                writer_state
                    .set_workspace_and_config(snapshot_b.clone(), Some(config_b.clone()))
                    .await
                    .expect("bind B");
                tokio::task::yield_now().await;
                writer_state
                    .set_workspace_and_config(snapshot_a.clone(), Some(config_a.clone()))
                    .await
                    .expect("bind A");
                tokio::task::yield_now().await;
            }
        });

        let mut torn = 0u32;
        let mut observed_a = 0u32;
        let mut observed_b = 0u32;
        let mut iterations = 0u32;
        while (iterations < 2_000 || observed_a == 0 || observed_b == 0) && iterations < 50_000 {
            iterations += 1;
            let Some((_admission, snap, cfg)) = guarded_daemon_sync_context(&state).await else {
                continue;
            };
            let max = cfg.code_graph.max_file_size_bytes;
            if snap.path == "/ws/a" {
                observed_a += 1;
                if max != LARGE {
                    torn += 1;
                }
            } else if snap.path == "/ws/b" {
                observed_b += 1;
                if max != SMALL {
                    torn += 1;
                }
            }
            tokio::task::yield_now().await;
        }

        stop.store(true, Ordering::Relaxed);
        writer.await.expect("writer task joins");

        assert!(
            observed_a > 0 && observed_b > 0,
            "vacuous test: must observe both bound states (A={observed_a}, B={observed_b})"
        );
        assert_eq!(
            torn, 0,
            "daemon sync context tore {torn} (workspace, config) pair(s) across \
             {iterations} samples (A={observed_a}, B={observed_b})"
        );
    }

    #[tokio::test]
    async fn daemon_driver_handle_loss_cannot_detach_mutation() {
        struct Termination(Option<tokio::sync::oneshot::Sender<()>>);

        impl Drop for Termination {
            fn drop(&mut self) {
                if let Some(terminated) = self.0.take() {
                    let _ = terminated.send(());
                }
            }
        }

        let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = tokio::sync::oneshot::channel();
        let (terminated_tx, terminated_rx) = tokio::sync::oneshot::channel();
        let writes = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let writes_for_driver = Arc::clone(&writes);
        let driver = spawn_daemon_driver(async move {
            let _termination = Termination(Some(terminated_tx));
            let _ = entered_tx.send(());
            let _ = release_rx.await;
            writes_for_driver.fetch_add(1, Ordering::SeqCst);
        });

        entered_rx.await.expect("driver should enter");
        drop(driver);
        let _ = release_tx.send(());
        terminated_rx.await.expect("driver should terminate");
        assert_eq!(
            writes.load(Ordering::SeqCst),
            0,
            "a lost parent handle must abort before later mutation"
        );
    }

    #[tokio::test]
    async fn normal_shutdown_joins_retained_hydration_to_its_safe_terminal() {
        let state = AppState::new(1);
        let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = tokio::sync::oneshot::channel();
        let reached_terminal = Arc::new(AtomicBool::new(false));
        let terminal_for_driver = Arc::clone(&reached_terminal);
        let driver = spawn_daemon_driver(async move {
            let _ = entered_tx.send(());
            let _ = release_rx.await;
            terminal_for_driver.store(true, Ordering::SeqCst);
        });
        assert!(state.retain_hydration_driver(1, driver).is_none());
        entered_rx.await.expect("hydration should enter");
        let releaser = tokio::spawn(async move {
            let _ = release_tx.send(());
        });

        join_retained_hydration(&state)
            .await
            .expect("hydration should reach its safe terminal");

        releaser.await.expect("hydration releaser should join");
        assert!(
            reached_terminal.load(Ordering::SeqCst),
            "normal shutdown must not abort hydration before its safe terminal"
        );
        assert!(state.take_hydration_driver().is_none());
    }

    async fn state_with_flush_failure() -> (tempfile::TempDir, SharedState) {
        let temp = tempfile::tempdir().expect("shutdown cleanup tempdir");
        let workspace = temp.path().join("workspace");
        std::fs::create_dir_all(&workspace).expect("create shutdown cleanup workspace");
        let blocked_data_dir = temp.path().join("blocked-data-dir");
        std::fs::write(&blocked_data_dir, "not a directory")
            .expect("create blocked shutdown data directory");
        let state = Arc::new(AppState::new(1));
        let snapshot = coordinator_snapshot(
            "shutdown-cleanup",
            "shutdown-cleanup-uuid",
            &workspace,
            blocked_data_dir,
        );
        let _ = state
            .publish_workspace_generation(snapshot, Some(WorkspaceConfig::default()))
            .await
            .expect("publish shutdown cleanup workspace");
        (temp, state)
    }

    #[tokio::test]
    async fn shutdown_cleanup_preserves_success_and_invokes_flush() {
        let _metrics_guard = crate::services::metrics::test_writer_guard().await;
        crate::services::metrics::shutdown()
            .await
            .expect("reset metrics writer");
        let state = Arc::new(AppState::new(1));
        let flush_calls_before = SHUTDOWN_FLUSH_CALLS.load(Ordering::SeqCst);

        shutdown_services(&state)
            .await
            .expect("clean shutdown cleanup");

        assert_eq!(
            SHUTDOWN_FLUSH_CALLS.load(Ordering::SeqCst),
            flush_calls_before + 1
        );
    }

    #[tokio::test]
    async fn shutdown_cleanup_attempts_flush_after_stalled_metrics_failure() {
        let metrics_guard = crate::services::metrics::test_writer_guard().await;
        crate::services::metrics::shutdown()
            .await
            .expect("reset metrics writer");
        let stalled = crate::services::metrics::configure_test_stalled_shutdown_writer(
            &metrics_guard,
            Path::new("stalled-shutdown-writer"),
            "main",
        )
        .expect("configure stalled metrics shutdown");
        let state = Arc::new(AppState::new(1));
        let flush_calls_before = SHUTDOWN_FLUSH_CALLS.load(Ordering::SeqCst);

        let error = shutdown_services(&state)
            .await
            .expect_err("metrics-only cleanup failure");
        drop(stalled);

        assert_eq!(
            SHUTDOWN_FLUSH_CALLS.load(Ordering::SeqCst),
            flush_calls_before + 1,
            "durable flush must still be attempted after metrics teardown aborts"
        );
        assert!(
            matches!(
                &error,
                EngramError::Metrics(crate::errors::MetricsError::WriteFailed { reason })
                    if reason.contains("shutdown timed out")
            ),
            "metrics-only failure must be preserved: {error}"
        );
    }

    #[tokio::test]
    async fn shutdown_cleanup_returns_flush_only_failure() {
        let _metrics_guard = crate::services::metrics::test_writer_guard().await;
        crate::services::metrics::shutdown()
            .await
            .expect("reset metrics writer");
        let (_temp, state) = state_with_flush_failure().await;
        let flush_calls_before = SHUTDOWN_FLUSH_CALLS.load(Ordering::SeqCst);

        let error = shutdown_services(&state)
            .await
            .expect_err("flush-only cleanup failure");

        assert_eq!(
            SHUTDOWN_FLUSH_CALLS.load(Ordering::SeqCst),
            flush_calls_before + 1
        );
        assert!(
            matches!(
                &error,
                EngramError::System(crate::errors::SystemError::DatabaseError { .. })
            ),
            "flush-only failure must be returned unchanged: {error}"
        );
    }

    #[tokio::test]
    async fn shutdown_cleanup_combines_both_failures_with_durable_evidence_first() {
        let metrics_guard = crate::services::metrics::test_writer_guard().await;
        crate::services::metrics::shutdown()
            .await
            .expect("reset metrics writer");
        let stalled = crate::services::metrics::configure_test_stalled_shutdown_writer(
            &metrics_guard,
            Path::new("stalled-shutdown-writer"),
            "main",
        )
        .expect("configure stalled metrics shutdown");
        let (_temp, state) = state_with_flush_failure().await;
        let flush_calls_before = SHUTDOWN_FLUSH_CALLS.load(Ordering::SeqCst);

        let error = shutdown_services(&state)
            .await
            .expect_err("combined cleanup failure");
        drop(stalled);
        let message = error.to_string();
        let durable_position = message
            .find("durable workspace flush")
            .expect("combined error must retain durable flush evidence");
        let metrics_position = message
            .find("metrics teardown")
            .expect("combined error must retain metrics teardown evidence");

        assert_eq!(
            SHUTDOWN_FLUSH_CALLS.load(Ordering::SeqCst),
            flush_calls_before + 1,
            "durable flush must be attempted when metrics teardown fails"
        );
        assert!(
            matches!(
                error,
                EngramError::System(crate::errors::SystemError::FlushFailed { .. })
            ),
            "combined failure must retain durable flush classification"
        );
        assert!(
            durable_position < metrics_position,
            "durable evidence must precede optional metrics evidence: {message}"
        );
        assert!(
            message.contains("Database operation failed"),
            "combined error must retain the durable failure: {message}"
        );
        assert!(
            message.contains("metrics writer shutdown timed out"),
            "combined error must retain the metrics failure: {message}"
        );
        assert!(
            message.len() <= 1_200,
            "combined shutdown error must remain bounded: {} bytes",
            message.len()
        );

        let oversized = combine_shutdown_results(
            Err(EngramError::Metrics(
                crate::errors::MetricsError::WriteFailed {
                    reason: "metrics".repeat(400),
                },
            )),
            Err(EngramError::System(
                crate::errors::SystemError::DatabaseError {
                    reason: "durable".repeat(400),
                },
            )),
        )
        .expect_err("oversized failures must remain errors")
        .to_string();
        assert!(
            oversized.len() <= 1_200,
            "oversized combined evidence must be truncated: {} bytes",
            oversized.len()
        );
        assert!(
            oversized.matches("...").count() >= 2,
            "both oversized error excerpts must show truncation: {oversized}"
        );
    }

    #[tokio::test]
    async fn daemon_transferred_failure_recovers_full_mask() {
        let metrics_guard = crate::services::metrics::test_writer_guard().await;
        crate::services::metrics::shutdown()
            .await
            .expect("reset metrics writer");
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = temp.path().join("workspace");
        std::fs::create_dir_all(&workspace).expect("create workspace");
        let invalid_data_dir = temp.path().join("not-a-directory");
        std::fs::write(&invalid_data_dir, b"file blocks database directory")
            .expect("create invalid data path");
        let snapshot =
            coordinator_snapshot("id-transfer", "uuid-transfer", &workspace, invalid_data_dir);
        crate::services::metrics::configure_test_disabled_writer(
            &metrics_guard,
            &workspace,
            &snapshot.branch,
        )
        .await
        .expect("configure fixture metrics writer");
        let config = WorkspaceConfig::default();
        let state = Arc::new(AppState::new(1));
        let _ = state
            .publish_workspace_generation(snapshot.clone(), Some(config.clone()))
            .await
            .expect("publish binding");
        let owner = match CoordinatorCell::request(
            state.coordinator.admission(),
            WorkMask::default(),
            OwnerKind::Startup,
        )
        .expect("request startup owner")
        {
            RequestOutcome::Acquired(permit) => permit,
            RequestOutcome::Waiting(_) | RequestOutcome::Enqueued | RequestOutcome::Stale => {
                panic!("startup owner should acquire")
            }
        };
        assert!(matches!(
            CoordinatorCell::request(
                state.coordinator.admission(),
                WorkMask::from_bits(0b111),
                OwnerKind::Sync,
            ),
            Ok(RequestOutcome::Enqueued)
        ));
        let successor = match CoordinatorCell::complete(owner) {
            CompletionOutcome::Transferred(successor) => successor,
            CompletionOutcome::Released
            | CompletionOutcome::RetirementAcknowledged
            | CompletionOutcome::SequenceExhausted(_)
            | CompletionOutcome::Stale => panic!("full mask should transfer"),
        };
        let operation_reached = AtomicBool::new(false);

        drive_daemon_transferred_syncs(
            &state,
            &snapshot,
            &config,
            successor,
            "test",
            Some(&operation_reached),
        )
        .await;

        let operation_was_reached = operation_reached.load(Ordering::SeqCst);
        let coordinator_is_idle = state.coordinator.test_is_idle();
        let pending_bits = state.coordinator.test_pending_bits();
        crate::services::metrics::shutdown()
            .await
            .expect("clean fixture metrics writer");

        assert!(
            operation_was_reached,
            "fixture must reach the intended database failure"
        );
        assert!(coordinator_is_idle);
        assert_eq!(pending_bits, 0b111);
    }

    #[tokio::test]
    async fn daemon_transferred_partial_file_errors_recover_full_mask() {
        let metrics_guard = crate::services::metrics::test_writer_guard().await;
        crate::services::metrics::shutdown()
            .await
            .expect("reset metrics writer");
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = temp.path().join("workspace");
        let data_dir = temp.path().join("data");
        std::fs::create_dir_all(&workspace).expect("create workspace");
        std::fs::write(workspace.join("broken.py"), [0xff]).expect("write invalid UTF-8 fixture");
        let snapshot = coordinator_snapshot(
            "id-transfer-partial",
            "uuid-transfer-partial",
            &workspace,
            data_dir,
        );
        crate::services::metrics::configure_test_disabled_writer(
            &metrics_guard,
            &workspace,
            &snapshot.branch,
        )
        .await
        .expect("configure fixture metrics writer");
        let config = WorkspaceConfig::default();
        let state = Arc::new(AppState::new(1));
        let _ = state
            .publish_workspace_generation(snapshot.clone(), Some(config.clone()))
            .await
            .expect("publish binding");
        let owner = match CoordinatorCell::request(
            state.coordinator.admission(),
            WorkMask::default(),
            OwnerKind::Startup,
        )
        .expect("request startup owner")
        {
            RequestOutcome::Acquired(permit) => permit,
            RequestOutcome::Waiting(_) | RequestOutcome::Enqueued | RequestOutcome::Stale => {
                panic!("startup owner should acquire")
            }
        };
        assert!(matches!(
            CoordinatorCell::request(
                state.coordinator.admission(),
                WorkMask::from_bits(0b111),
                OwnerKind::Sync,
            ),
            Ok(RequestOutcome::Enqueued)
        ));
        let successor = match CoordinatorCell::complete(owner) {
            CompletionOutcome::Transferred(successor) => successor,
            CompletionOutcome::Released
            | CompletionOutcome::RetirementAcknowledged
            | CompletionOutcome::SequenceExhausted(_)
            | CompletionOutcome::Stale => panic!("full mask should transfer"),
        };
        let operation_reached = AtomicBool::new(false);

        drive_daemon_transferred_syncs(
            &state,
            &snapshot,
            &config,
            successor,
            "test",
            Some(&operation_reached),
        )
        .await;

        let operation_was_reached = operation_reached.load(Ordering::SeqCst);
        let coordinator_is_idle = state.coordinator.test_is_idle();
        let pending_bits = state.coordinator.test_pending_bits();
        crate::services::metrics::shutdown()
            .await
            .expect("clean fixture metrics writer");

        assert!(
            operation_was_reached,
            "fixture must reach the intended per-file failure"
        );
        assert!(coordinator_is_idle);
        assert_eq!(
            pending_bits, 0b111,
            "a partial daemon transfer must retain heavy work for retry"
        );
    }

    #[test]
    fn watcher_batch_retries_only_for_same_worktree_path_and_uuid() {
        let root = Path::new("C:/workspace");
        let original = coordinator_snapshot(
            "id-original",
            "uuid-original",
            root,
            std::path::PathBuf::from("data/original"),
        );
        let mut checked_out_branch = original.clone();
        checked_out_branch.branch = "feature".to_owned();
        checked_out_branch.workspace_id = "id-feature".to_owned();
        let mut different_path = checked_out_branch.clone();
        different_path.path = "C:/workspace/rebound".to_owned();
        let mut different_uuid = checked_out_branch.clone();
        different_uuid.workspace_uuid = "uuid-replacement".to_owned();

        assert!(same_workspace_instance(&original, &checked_out_branch));
        assert!(!same_workspace_instance(&original, &different_path));
        assert!(!same_workspace_instance(&original, &different_uuid));
    }
}
