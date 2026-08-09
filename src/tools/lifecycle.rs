use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::Ordering;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::connect_db;
use crate::db::queries::CodeGraphQueries;
use crate::db::workspace::{
    canonicalize_workspace, load_or_create_workspace_id, resolve_data_dir, resolve_git_branch,
    workspace_hash,
};
use crate::errors::{EngramError, MetricsError, SystemError, WorkspaceError};
use crate::models::health::{HealthReport, ScanProgress};
use crate::server::state::CoordinatorCell;
use crate::server::state::{
    AdmissionGuard, AppState, CompletionOutcome, CoordinatorError, DispatchSnapshot,
    DriverTaskGuard, OwnerKind, OwnerPermit, OwnerProgressScope, WorkspaceSnapshot,
};
use crate::services::code_graph::sync_workspace as sync_code_graph;
use crate::services::config::parse_config;
use crate::services::connection::validate_workspace_path;
use crate::services::file_tracker::detect_offline_changes;
use crate::services::hydration::{detect_stale_since, hydrate_code_graph, hydrate_workspace};
use crate::services::registry::{load_registry, validate_sources_strict};
use crate::tools::doctor::get_health_report_for_daemon;

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkspaceBinding {
    pub workspace_id: String,
    pub path: String,
    pub hydrated: bool,
    /// Whether a background offline-change scan was queued for this binding (029-F WS-6).
    pub pending_scan: bool,
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
    /// Structured diagnostic health report covering all 8 failure modes (029-F WS-2).
    pub health: HealthReport,
    /// Process-level reliability counters (029-F WS-8).
    pub telemetry: ReliabilitySnapshot,
}

/// Serializable snapshot of `ReliabilityCounters` for `DaemonStatus`.
#[derive(Debug, Serialize, Deserialize)]
pub struct ReliabilitySnapshot {
    pub stale_pid_recovered: u64,
    pub version_mismatch_respawn: u64,
    pub registry_validation_failures: u64,
    pub duplicate_daemon_detected: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkspaceStatus {
    pub path: String,
    /// Active git branch name (used as the DB storage subdirectory).
    pub branch: String,
    /// Absolute path to the CozoDB SQLite database file for this workspace and branch.
    pub db_path: String,
    pub last_flush: Option<String>,
    pub stale_files: bool,
    pub connection_count: usize,
    pub code_graph: CodeGraphStats,
    /// Background scan progress snapshot; `null` until the first scan is queued (029-F WS-6).
    pub scan_status: Option<ScanProgress>,
    /// Whether the retrieval-evaluation subsystem is enabled for this workspace
    /// (081-F). Exposed for autoharness capability discovery without parsing
    /// `.engram/config.toml`.
    pub retrieval_eval_enabled: bool,
}

/// Summary statistics for the indexed code graph.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CodeGraphStats {
    pub code_files: u64,
    pub functions: u64,
    pub classes: u64,
    pub interfaces: u64,
    pub edges: u64,
}

#[derive(Default)]
struct WorkspaceAdmissionProbe {
    #[cfg(test)]
    passed_precheck: Option<tokio::sync::oneshot::Sender<()>>,
    #[cfg(test)]
    resume: Option<tokio::sync::oneshot::Receiver<()>>,
    #[cfg(test)]
    metrics_replaced: Option<tokio::sync::oneshot::Sender<()>>,
    #[cfg(test)]
    resume_after_metrics: Option<tokio::sync::oneshot::Receiver<()>>,
    #[cfg(test)]
    fail_publication: bool,
}

impl WorkspaceAdmissionProbe {
    fn after_precheck(&mut self) -> impl std::future::Future<Output = ()> + '_ {
        #[cfg(test)]
        {
            async move {
                if let Some(passed_precheck) = self.passed_precheck.take() {
                    let _ = passed_precheck.send(());
                }
                if let Some(resume) = self.resume.take() {
                    let _ = resume.await;
                }
            }
        }
        #[cfg(not(test))]
        {
            let _ = self;
            std::future::ready(())
        }
    }

    fn after_metrics_replacement(&mut self) -> impl std::future::Future<Output = ()> + '_ {
        #[cfg(test)]
        {
            async move {
                if let Some(metrics_replaced) = self.metrics_replaced.take() {
                    let _ = metrics_replaced.send(());
                }
                if let Some(resume) = self.resume_after_metrics.take() {
                    let _ = resume.await;
                }
            }
        }
        #[cfg(not(test))]
        {
            let _ = self;
            std::future::ready(())
        }
    }

    const fn fail_publication(&self) -> bool {
        #[cfg(test)]
        {
            self.fail_publication
        }
        #[cfg(not(test))]
        {
            let _ = self;
            false
        }
    }
}

struct WorkspaceLifecycleTransaction {
    admission: Option<tokio::sync::OwnedMutexGuard<()>>,
    prior_metrics: Option<WorkspaceMetricsConfiguration>,
    armed: bool,
}

struct WorkspaceMetricsConfiguration {
    path: PathBuf,
    branch: String,
    config: crate::models::metrics::MetricsConfig,
}

impl WorkspaceMetricsConfiguration {
    fn from_dispatch(dispatch: DispatchSnapshot) -> Self {
        Self {
            path: PathBuf::from(dispatch.workspace.path),
            branch: dispatch.workspace.branch,
            config: dispatch.config.metrics,
        }
    }

    async fn restore(self) -> Result<(), EngramError> {
        crate::services::metrics::initialize(&self.path, &self.branch, &self.config).await
    }
}

impl WorkspaceLifecycleTransaction {
    fn new(
        admission: tokio::sync::OwnedMutexGuard<()>,
        prior_metrics: Option<WorkspaceMetricsConfiguration>,
    ) -> Self {
        Self {
            admission: Some(admission),
            prior_metrics,
            armed: true,
        }
    }

    fn commit(mut self) {
        self.armed = false;
        let _ = self.admission.take();
    }

    async fn rollback(mut self) -> Result<(), EngramError> {
        let restoration = restore_prior_metrics(self.prior_metrics.take()).await;
        self.armed = false;
        let _ = self.admission.take();
        restoration
    }
}

impl Drop for WorkspaceLifecycleTransaction {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        let Some(admission) = self.admission.take() else {
            return;
        };
        let prior_metrics = self.prior_metrics.take();
        let Ok(runtime) = tokio::runtime::Handle::try_current() else {
            mark_metrics_unavailable("workspace lifecycle rollback lost its async runtime");
            drop(admission);
            return;
        };
        mark_metrics_unavailable("cancelled workspace bind is awaiting metrics restoration");
        // Detaching is intentional: the task owns the admission guard, so later
        // workspace binds use that lock as the restoration completion barrier.
        let _rollback = runtime.spawn(async move {
            if let Err(error) = restore_prior_metrics_from_unavailable(prior_metrics).await {
                tracing::error!(
                    error = %bounded_error(&error),
                    "cancelled workspace bind could not restore prior metrics; metrics left unavailable"
                );
            }
            drop(admission);
        });
    }
}

async fn restore_prior_metrics(
    prior_metrics: Option<WorkspaceMetricsConfiguration>,
) -> Result<(), EngramError> {
    mark_metrics_unavailable("workspace bind rollback is awaiting metrics restoration");
    restore_prior_metrics_from_unavailable(prior_metrics).await
}

async fn restore_prior_metrics_from_unavailable(
    prior_metrics: Option<WorkspaceMetricsConfiguration>,
) -> Result<(), EngramError> {
    let restoration = match prior_metrics {
        Some(prior) => prior.restore().await,
        None => crate::services::metrics::shutdown().await,
    };
    if restoration.is_err() {
        mark_metrics_unavailable("workspace bind rollback restoration failed");
    }
    restoration
}

fn mark_metrics_unavailable(context: &str) {
    if let Err(error) = crate::services::metrics::mark_writer_unavailable() {
        tracing::error!(
            error = %bounded_error(&error),
            context,
            "failed to advance unavailable metrics identity during workspace rollback"
        );
    }
}

fn bounded_error(error: &impl std::fmt::Display) -> String {
    const MAX_ERROR_CHARS: usize = 512;
    error.to_string().chars().take(MAX_ERROR_CHARS).collect()
}

pub async fn set_workspace(
    state: Arc<AppState>,
    path: String,
) -> Result<WorkspaceBinding, EngramError> {
    set_workspace_with_probe(state, path, WorkspaceAdmissionProbe::default()).await
}

#[cfg(test)]
async fn set_workspace_after_precheck(
    state: Arc<AppState>,
    path: String,
    passed_precheck: tokio::sync::oneshot::Sender<()>,
    resume: tokio::sync::oneshot::Receiver<()>,
) -> Result<WorkspaceBinding, EngramError> {
    set_workspace_with_probe(
        state,
        path,
        WorkspaceAdmissionProbe {
            passed_precheck: Some(passed_precheck),
            resume: Some(resume),
            ..WorkspaceAdmissionProbe::default()
        },
    )
    .await
}

#[cfg(test)]
async fn set_workspace_after_metrics_replacement(
    state: Arc<AppState>,
    path: String,
    metrics_replaced: tokio::sync::oneshot::Sender<()>,
    resume_after_metrics: tokio::sync::oneshot::Receiver<()>,
    fail_publication: bool,
) -> Result<WorkspaceBinding, EngramError> {
    set_workspace_with_probe(
        state,
        path,
        WorkspaceAdmissionProbe {
            metrics_replaced: Some(metrics_replaced),
            resume_after_metrics: Some(resume_after_metrics),
            fail_publication,
            ..WorkspaceAdmissionProbe::default()
        },
    )
    .await
}

async fn set_workspace_with_probe(
    state: Arc<AppState>,
    path: String,
    mut admission_probe: WorkspaceAdmissionProbe,
) -> Result<WorkspaceBinding, EngramError> {
    validate_workspace_path(&path)?;

    let canonical = canonicalize_workspace(&path)?;
    let canonical_path = canonical.display().to_string();
    let workspace_uuid = load_or_create_workspace_id(&canonical)?;
    let branch = resolve_git_branch(&canonical).unwrap_or_else(|_| "default".to_string());
    let workspace_id = workspace_hash(&canonical, &branch);

    if let Some(active) = state.snapshot_workspace().await {
        if active.path == canonical_path && active.workspace_uuid != workspace_uuid.to_string() {
            let expected_id = Uuid::parse_str(&active.workspace_uuid).map_err(|error| {
                EngramError::System(SystemError::InvalidParams {
                    reason: format!(
                        "active workspace state contains an invalid workspace UUID '{}': {error}",
                        active.workspace_uuid
                    ),
                })
            })?;
            return Err(EngramError::Workspace(WorkspaceError::AmbiguousBind {
                expected_id,
                found_id: workspace_uuid,
                path: canonical,
            }));
        }
    }

    let data_dir = resolve_data_dir(&canonical);

    if !state.can_bind_workspace(&workspace_id).await {
        return Err(EngramError::Workspace(WorkspaceError::LimitReached {
            limit: state.max_workspaces(),
        }));
    }
    admission_probe.after_precheck().await;

    // Fast metadata hydration: reads .engram/ files but does not open the DB.
    let hydration = hydrate_workspace(&canonical).await?;

    // Parse workspace config synchronously (fast: reads a single TOML file).
    let ws_config = parse_config(&canonical)?;

    // Run strict registry validation. A failure increments the reliability
    // counter and is logged as a warning, but does NOT abort binding — the
    // workspace is still usable with reduced source coverage.
    let registry_path = canonical.join(".engram").join("registry.yaml");
    if let Ok(Some(mut registry_config)) = load_registry(&registry_path) {
        if let Err(err) = validate_sources_strict(&mut registry_config, &canonical) {
            tracing::warn!(
                error = %err,
                workspace = %canonical_path,
                "set_workspace: strict registry validation failed — workspace bound with reduced coverage"
            );
            state
                .reliability_counters()
                .inc_registry_validation_failure();
        }
    }

    // The first capacity check is only a fast path. Recheck while owning the
    // async admission guard, then retain it through metrics replacement and
    // binding publication so a stale concurrent bind cannot replace the active
    // workspace's process-global writer before losing publication.
    let workspace_admission = state.acquire_workspace_admission().await;
    if !state.can_bind_workspace(&workspace_id).await {
        return Err(EngramError::Workspace(WorkspaceError::LimitReached {
            limit: state.max_workspaces(),
        }));
    }
    let prior_metrics = state
        .snapshot_dispatch_context()
        .await
        .map(WorkspaceMetricsConfiguration::from_dispatch);

    // Initialise metrics sink (spawns a channel + background writer, no DB).
    crate::services::metrics::initialize(&canonical, &branch, &ws_config.metrics).await?;
    let lifecycle_transaction =
        WorkspaceLifecycleTransaction::new(workspace_admission, prior_metrics);
    admission_probe.after_metrics_replacement().await;

    let snapshot = WorkspaceSnapshot {
        workspace_id: workspace_id.clone(),
        workspace_uuid: workspace_uuid.to_string(),
        branch: branch.clone(),
        data_dir: data_dir.clone(),
        path: canonical.display().to_string(),
        last_flush: hydration.last_flush.clone(),
        stale_files: hydration.stale_files,
        connection_count: state.active_connections(),
        file_mtimes: hydration.file_mtimes.clone(),
    };
    let hydration_context = DispatchSnapshot {
        workspace: snapshot.clone(),
        config: ws_config.clone(),
    };

    let publication = if admission_probe.fail_publication() {
        Err(CoordinatorError::SequenceExhausted)
    } else {
        state
            .publish_workspace_generation(snapshot, Some(ws_config.clone()))
            .await
    };
    let (binding_generation, admission) = match publication {
        Ok(committed) => committed,
        Err(error) => {
            let publication_error = match error {
                CoordinatorError::SequenceExhausted => {
                    EngramError::System(SystemError::InvalidParams {
                        reason: error.to_string(),
                    })
                }
                CoordinatorError::WorkspaceLimit { limit } => {
                    EngramError::Workspace(WorkspaceError::LimitReached { limit })
                }
            };
            if let Err(restoration_error) = lifecycle_transaction.rollback().await {
                return Err(EngramError::Metrics(MetricsError::WriteFailed {
                    reason: format!(
                        "workspace publication failed ({}); prior metrics restoration failed \
                         ({}); metrics left unavailable",
                        bounded_error(&publication_error),
                        bounded_error(&restoration_error)
                    ),
                }));
            }
            return Err(publication_error);
        }
    };
    lifecycle_transaction.commit();
    crate::services::query_stats::reset_timing();

    // Queue a background scan immediately. The DB connect + hydrate +
    // offline-change detection are moved off the hot path so that
    // set_workspace returns well within the 500 ms bind-latency SLA
    // (029-F WS-6). The `pending_scan` field signals to the caller
    // that background work has been scheduled.
    let state_bg = Arc::clone(&state);
    let task = tokio::spawn(async move {
        background_db_hydration(
            state_bg,
            hydration_context,
            admission,
            #[cfg(test)]
            None,
        )
        .await;
    });
    let driver = DriverTaskGuard { task: Some(task) };
    if let Some(displaced) = state.retain_hydration_driver(binding_generation, driver) {
        let _ = displaced.abort_and_join().await;
    }

    Ok(WorkspaceBinding {
        workspace_id,
        path: canonical.display().to_string(),
        hydrated: true,
        pending_scan: true,
    })
}

/// Background DB hydration task spawned by [`set_workspace`].
///
/// Connects to CozoDB, hydrates the code graph from JSONL files, and runs
/// offline-change detection — all off the bind-latency hot path. Updates
/// [`AppState::scan_progress`] when complete (029-F WS-6).
///
/// The move-only admission guard owns pre-acquisition cancellation and becomes
/// the exact Hydration permit before any DB/file boundary. Post-acquisition
/// cancellation drops the in-function operation future before permit cleanup.
#[cfg(test)]
#[derive(Clone, Copy)]
enum HydrationProbeExit {
    DbFailure,
    EarlyReturn,
    AwaitCancellation,
}

#[cfg(test)]
struct HydrationProbe {
    exit: HydrationProbeExit,
    io_starts: Arc<std::sync::atomic::AtomicUsize>,
    active_io: Arc<std::sync::atomic::AtomicUsize>,
    entered: Option<tokio::sync::oneshot::Sender<()>>,
}

#[cfg(test)]
#[derive(Clone, Copy)]
enum HandoffProbeExit {
    Handled,
    EarlyReturn,
    AwaitCancellation,
}

#[cfg(test)]
struct HandoffProbe {
    exit: HandoffProbeExit,
    coordinator: Arc<CoordinatorCell>,
    runs: Arc<std::sync::atomic::AtomicUsize>,
    mask_bits: Arc<std::sync::atomic::AtomicUsize>,
    active_io: Arc<std::sync::atomic::AtomicUsize>,
    owner_active: Arc<std::sync::atomic::AtomicUsize>,
    entered: Option<tokio::sync::oneshot::Sender<()>>,
}

#[cfg(test)]
impl HandoffProbe {
    async fn run(mut self, mask_bits: u8) -> HandoffTerminal {
        struct ActiveIo(Arc<std::sync::atomic::AtomicUsize>);

        impl Drop for ActiveIo {
            fn drop(&mut self) {
                self.0.store(0, Ordering::SeqCst);
            }
        }

        self.runs.fetch_add(1, Ordering::SeqCst);
        self.mask_bits
            .store(usize::from(mask_bits), Ordering::SeqCst);
        self.owner_active.store(
            usize::from(!self.coordinator.test_is_idle()),
            Ordering::SeqCst,
        );
        self.active_io.store(1, Ordering::SeqCst);
        let _active = ActiveIo(Arc::clone(&self.active_io));
        if let Some(entered) = self.entered.take() {
            let _ = entered.send(());
        }
        if matches!(self.exit, HandoffProbeExit::AwaitCancellation) {
            std::future::pending::<()>().await;
        }
        match self.exit {
            HandoffProbeExit::Handled | HandoffProbeExit::AwaitCancellation => {
                HandoffTerminal::Handled
            }
            HandoffProbeExit::EarlyReturn => HandoffTerminal::EarlyReturn,
        }
    }
}

#[cfg(test)]
impl HydrationProbe {
    async fn run(mut self) -> HydrationProbeExit {
        struct ActiveIo(Arc<std::sync::atomic::AtomicUsize>);

        impl Drop for ActiveIo {
            fn drop(&mut self) {
                self.0.store(0, Ordering::SeqCst);
            }
        }

        self.io_starts.fetch_add(1, Ordering::SeqCst);
        self.active_io.store(1, Ordering::SeqCst);
        let _active = ActiveIo(Arc::clone(&self.active_io));
        if let Some(entered) = self.entered.take() {
            let _ = entered.send(());
        }
        if matches!(self.exit, HydrationProbeExit::AwaitCancellation) {
            std::future::pending::<()>().await;
        }
        self.exit
    }
}

#[derive(Clone, Copy)]
enum HydrationTerminal {
    Handled,
    DbFailure,
    #[cfg(test)]
    EarlyReturn,
}

#[derive(Clone, Copy)]
enum HandoffTerminal {
    Handled,
    Failed,
    #[cfg(test)]
    EarlyReturn,
}

async fn set_hydration_progress(
    state: &AppState,
    scope: &OwnerProgressScope,
    progress: Option<ScanProgress>,
) -> bool {
    state.set_scan_progress_for_owner(scope, progress).await
}

async fn background_db_hydration(
    state: Arc<AppState>,
    context: DispatchSnapshot,
    admission: AdmissionGuard,
    #[cfg(test)] test_probe: Option<HydrationProbe>,
) {
    let permit = match admission.acquire_hydration().await {
        Ok(Some(permit)) => permit,
        Ok(None) => return,
        Err(error) => {
            tracing::error!(%error, "background_db_hydration: admission failed");
            return;
        }
    };
    let Some((mut permit, context)) = (match crate::tools::write::prepare_branch_owner(
        &state,
        permit,
        context,
        OwnerKind::Hydration,
    )
    .await
    {
        Ok(prepared) => prepared,
        Err(error) => {
            tracing::error!(%error, "background_db_hydration: branch preparation failed");
            return;
        }
    }) else {
        return;
    };
    let Some(binding_generation) = permit.generation() else {
        tracing::error!("background_db_hydration: prepared permit lost ownership");
        return;
    };
    let Some(progress_scope) = permit.progress_scope() else {
        tracing::error!("background_db_hydration: prepared permit lost progress scope");
        return;
    };
    let canonical = PathBuf::from(&context.workspace.path);
    let data_dir = context.workspace.data_dir;
    let branch = context.workspace.branch;
    let code_graph_config = context.config.code_graph;

    let operation = async {
        set_hydration_progress(
            &state,
            &progress_scope,
            Some(ScanProgress {
                running: true,
                files_scanned: 0,
                files_total: 0,
                last_completed_at: None,
            }),
        )
        .await;

        #[cfg(test)]
        if let Some(probe) = test_probe {
            return match probe.run().await {
                HydrationProbeExit::EarlyReturn => HydrationTerminal::EarlyReturn,
                HydrationProbeExit::DbFailure => HydrationTerminal::DbFailure,
                HydrationProbeExit::AwaitCancellation => HydrationTerminal::Handled,
            };
        }

        let db = match connect_db(&data_dir, &branch).await {
            Ok(db) => db,
            Err(e) => {
                tracing::warn!(error = %e, "background_db_hydration: DB connect failed");
                set_hydration_progress(
                    &state,
                    &progress_scope,
                    Some(ScanProgress {
                        running: false,
                        files_scanned: 0,
                        files_total: 0,
                        last_completed_at: Some(Utc::now().to_rfc3339()),
                    }),
                )
                .await;
                return HydrationTerminal::DbFailure;
            }
        };

        let cg_queries = CodeGraphQueries::new(db);

        // Signal "ready" immediately after the DB connects so the shim's
        // poll_until_ready succeeds while the longer hydration continues.
        let _ = state.set_hydration_ready_for_generation(binding_generation);

        if let Err(e) = hydrate_code_graph(&canonical, &data_dir, &branch, &cg_queries).await {
            tracing::warn!(error = %e, "background_db_hydration: code graph hydration failed");
        }

        let offline_count = match detect_offline_changes(&canonical, &cg_queries).await {
            Ok(changes) => {
                if !changes.is_empty() {
                    tracing::info!(
                        count = changes.len(),
                        "background_db_hydration: offline changes detected"
                    );
                }
                changes.len() as u64
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "background_db_hydration: offline change detection failed"
                );
                0
            }
        };

        set_hydration_progress(
            &state,
            &progress_scope,
            Some(ScanProgress {
                running: false,
                files_scanned: offline_count,
                files_total: offline_count,
                last_completed_at: Some(Utc::now().to_rfc3339()),
            }),
        )
        .await;

        let auto_reindex = std::env::var("ENGRAM_AUTO_REINDEX")
            .map(|v| matches!(v.to_lowercase().as_str(), "1" | "true" | "yes"))
            .unwrap_or(false);

        if offline_count > 0 && auto_reindex {
            tracing::info!(
                offline_count,
                "background_db_hydration: ENGRAM_AUTO_REINDEX=true, starting post-scan re-index"
            );
            match sync_code_graph(&canonical, &data_dir, &branch, &code_graph_config).await {
                Ok(result) => tracing::info!(
                    files_added = result.files_added,
                    files_modified = result.files_modified,
                    "background_db_hydration: post-scan re-index complete"
                ),
                Err(e) => tracing::warn!(
                    error = %e,
                    "background_db_hydration: post-scan re-index failed"
                ),
            }
        } else if offline_count > 0 {
            tracing::info!(
                offline_count,
                "background_db_hydration: offline changes detected; \
                 set ENGRAM_AUTO_REINDEX=true to re-index on startup, \
                 or run `engram sync` to update the code graph"
            );
        }
        HydrationTerminal::Handled
    };

    match permit.run_until_cancelled(operation).await {
        Some(HydrationTerminal::Handled | HydrationTerminal::DbFailure) => {
            if let CompletionOutcome::Transferred(successor) = CoordinatorCell::complete(permit) {
                drive_transferred_sync(
                    &state,
                    successor,
                    #[cfg(test)]
                    None,
                )
                .await;
            }
        }
        #[cfg(test)]
        Some(HydrationTerminal::EarlyReturn) => {}
        None => {
            tracing::info!("background_db_hydration: scan cancelled by new generation");
        }
    }
}

async fn drive_transferred_sync(
    state: &AppState,
    permit: OwnerPermit,
    #[cfg(test)] test_probe: Option<HandoffProbe>,
) {
    let Some(initial_ctx) = state.snapshot_dispatch_context().await else {
        return;
    };
    let mut permit = permit;
    let mut current_ctx = initial_ctx;
    #[cfg(test)]
    let mut test_probe = test_probe;
    loop {
        let prepared =
            crate::tools::write::prepare_transferred_sync(state, permit, current_ctx).await;
        let Some((prepared_permit, prepared_ctx)) = (match prepared {
            Ok(prepared) => prepared,
            Err(error) => {
                tracing::warn!(%error, "hydration transferred branch preparation failed");
                return;
            }
        }) else {
            return;
        };
        permit = prepared_permit;
        current_ctx = prepared_ctx;
        let work_mask = permit.work_bits();
        let operation = async {
            #[cfg(test)]
            if let Some(probe) = test_probe.take() {
                return probe.run(work_mask).await;
            }

            let revalidate = work_mask & 0b010 != 0;
            let backfill_python = work_mask & 0b100 != 0;
            let result = crate::services::code_graph::sync_workspace_with_progress(
                Path::new(&current_ctx.workspace.path),
                &current_ctx.workspace.data_dir,
                &current_ctx.workspace.branch,
                &current_ctx.config.code_graph,
                backfill_python,
                revalidate,
                None,
            )
            .await;
            match result {
                Ok(result) => {
                    let unfulfilled_work_bits =
                        crate::tools::write::unfulfilled_work_bits(&result.errors, work_mask);
                    tracing::info!(
                        files_added = result.files_added,
                        files_modified = result.files_modified,
                        revalidate,
                        backfill_python,
                        unfulfilled_work_bits,
                        "transferred sync complete"
                    );
                    if unfulfilled_work_bits == 0 {
                        HandoffTerminal::Handled
                    } else {
                        HandoffTerminal::Failed
                    }
                }
                Err(error) => {
                    tracing::warn!(
                        %error,
                        revalidate,
                        backfill_python,
                        "transferred sync failed"
                    );
                    HandoffTerminal::Failed
                }
            }
        };

        match permit.run_until_cancelled(operation).await {
            Some(HandoffTerminal::Handled) => {
                let _ = state.set_hydration_ready_for_permit(&permit);
                match CoordinatorCell::complete(permit) {
                    CompletionOutcome::Transferred(successor) => permit = successor,
                    CompletionOutcome::Released
                    | CompletionOutcome::RetirementAcknowledged
                    | CompletionOutcome::SequenceExhausted(_)
                    | CompletionOutcome::Stale => return,
                }
            }
            Some(HandoffTerminal::Failed) => return,
            #[cfg(test)]
            Some(HandoffTerminal::EarlyReturn) => return,
            None => return,
        }
    }
}

pub async fn get_daemon_status(state: &AppState) -> Result<DaemonStatus, EngramError> {
    let memory_bytes = crate::services::process_memory::current_process_memory_bytes().unwrap_or(0);

    let model_loaded = crate::services::embedding::is_available();
    let model_name = if model_loaded {
        Some("bge-small-en-v1.5".to_string())
    } else {
        None
    };

    let health = match get_health_report_for_daemon(state).await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(error = %e, "get_daemon_status: health report failed; using default");
            HealthReport::default()
        }
    };

    let rc = state.reliability_counters();
    let telemetry = ReliabilitySnapshot {
        stale_pid_recovered: rc.stale_pid_recovered.load(Ordering::Relaxed),
        version_mismatch_respawn: rc.version_mismatch_respawn.load(Ordering::Relaxed),
        registry_validation_failures: rc.registry_validation_failures.load(Ordering::Relaxed),
        duplicate_daemon_detected: rc.duplicate_daemon_detected.load(Ordering::Relaxed),
    };

    Ok(DaemonStatus {
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: state.uptime_seconds(),
        active_workspaces: state.active_workspaces().await,
        active_connections: state.active_connections(),
        memory_bytes,
        model_loaded,
        model_name,
        health,
        telemetry,
    })
}

pub async fn get_workspace_status(state: &AppState) -> Result<WorkspaceStatus, EngramError> {
    // 086.004-T: read the workspace binding AND its config TOGETHER at handler
    // entry via one `snapshot_dispatch_context()` (the pattern `tools/eval.rs`
    // uses), instead of reading the retrieval-eval flag from a separate
    // `workspace_config()` AFTER the `connect_db` round-trip. That wide gap let a
    // concurrent `set_workspace` pair this workspace's path/branch with a
    // DIFFERENT config's `retrieval_eval_enabled`; taking both under one
    // dispatch-context read eliminates that WIDE tear window. The writer side is
    // now atomic too: `publish_workspace_generation` publishes the binding,
    // config, coordinator floor, and cancellation channel under the fixed lock
    // order, so this handler observes a fully consistent workspace/config pair.
    let Some(ctx) = state.snapshot_dispatch_context().await else {
        return Err(EngramError::Workspace(WorkspaceError::NotSet));
    };
    let snapshot = ctx.workspace;
    let retrieval_eval_enabled = ctx.config.retrieval_eval.enabled;

    let engram_dir = Path::new(&snapshot.path).join(".engram");
    let stale_now = snapshot.stale_files || detect_stale_since(&snapshot.file_mtimes, &engram_dir);

    if stale_now != snapshot.stale_files {
        // Only write the recomputed staleness back if the SNAPSHOTTED workspace is
        // still the active binding: `update_workspace` mutates whichever workspace
        // is active when the lock is acquired, so a concurrent rebind must not
        // receive workspace A's stale flag computed against workspace A's files
        // (status polling must not contaminate a new binding — Copilot PR#249).
        let snapshot_id = snapshot.workspace_id.clone();
        let _ = state
            .update_workspace(|ws| {
                if ws.workspace_id == snapshot_id {
                    ws.stale_files = stale_now;
                }
            })
            .await;
    }

    // Gather code-graph stats from the database. The code-graph indexer is
    // always active in every build; the `git-graph` feature only gates git
    // commit-history tooling, so these counts must not be gated behind it.
    // Mirrors `get_workspace_statistics`, which reads the same counts
    // unconditionally.
    let code_graph = match connect_db(&snapshot.data_dir, &snapshot.branch).await {
        Ok(db) => {
            let cg_queries = CodeGraphQueries::new(db);
            CodeGraphStats {
                code_files: cg_queries.count_code_files().await.unwrap_or(0),
                functions: cg_queries.count_functions().await.unwrap_or(0),
                classes: cg_queries.count_classes().await.unwrap_or(0),
                interfaces: cg_queries.count_interfaces().await.unwrap_or(0),
                edges: cg_queries.count_code_edges().await.unwrap_or(0),
            }
        }
        Err(error) => {
            tracing::warn!(
                error = %error,
                "get_workspace_status: code-graph DB connect failed; reporting zero counts"
            );
            CodeGraphStats::default()
        }
    };

    let branch_safe = snapshot.branch.replace(['/', '\\', ':'], "_");
    let db_path = snapshot
        .data_dir
        .join("cozo")
        .join(&branch_safe)
        .join("engram.db")
        .display()
        .to_string();

    Ok(WorkspaceStatus {
        path: snapshot.path,
        branch: snapshot.branch,
        db_path,
        last_flush: snapshot.last_flush,
        stale_files: stale_now,
        connection_count: state.active_connections(),
        code_graph,
        scan_status: state.scan_progress_snapshot().await,
        // Read from the SAME dispatch-context snapshot as the workspace binding
        // above (not a separate later read), for capability discovery (086.004-T).
        retrieval_eval_enabled,
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use sysinfo::System;

    use super::{
        HandoffProbe, HandoffProbeExit, HydrationProbe, HydrationProbeExit,
        background_db_hydration, drive_transferred_sync, get_daemon_status, set_hydration_progress,
        set_workspace, set_workspace_after_metrics_replacement, set_workspace_after_precheck,
    };
    use crate::db::connect_db;
    use crate::db::queries::CodeGraphQueries;
    use crate::errors::{EngramError, WorkspaceError};
    use crate::models::config::{CodeGraphConfig, WorkspaceConfig};
    use crate::models::health::ScanProgress;
    use crate::server::state::{
        AppState, CompletionOutcome, CoordinatorCell, DispatchSnapshot, DriverTaskGuard, OwnerKind,
        OwnerPermit, RequestOutcome, WorkMask, WorkspaceSnapshot,
    };
    use crate::services::code_graph;

    fn coordinator_snapshot(name: &str, workspace_id: &str) -> WorkspaceSnapshot {
        WorkspaceSnapshot {
            workspace_id: workspace_id.to_owned(),
            workspace_uuid: format!("uuid-{workspace_id}"),
            branch: "main".to_owned(),
            data_dir: PathBuf::from(format!("logs/phase6-group2/{name}")),
            path: format!("C:/workspace/{name}"),
            last_flush: None,
            stale_files: false,
            connection_count: 0,
            file_mtimes: std::collections::HashMap::new(),
        }
    }

    async fn publish_test_binding(state: &AppState, snapshot: WorkspaceSnapshot) -> u64 {
        state
            .publish_workspace_generation(snapshot, Some(WorkspaceConfig::default()))
            .await
            .unwrap_or_else(|error| panic!("publish test binding: {error}"))
            .0
    }

    async fn publish_test_binding_with_disabled_metrics(
        state: &AppState,
        snapshot: WorkspaceSnapshot,
        metrics_guard: &tokio::sync::MutexGuard<'static, ()>,
    ) -> u64 {
        crate::services::metrics::configure_test_disabled_writer(
            metrics_guard,
            std::path::Path::new(&snapshot.path),
            &snapshot.branch,
        )
        .await
        .expect("configure disabled metrics writer");
        publish_test_binding(state, snapshot).await
    }

    fn request_empty(
        state: &AppState,
        kind: OwnerKind,
    ) -> Result<RequestOutcome, crate::server::state::CoordinatorError> {
        CoordinatorCell::request(state.coordinator.admission(), WorkMask::default(), kind)
    }

    fn acquired(outcome: RequestOutcome) -> OwnerPermit {
        match outcome {
            RequestOutcome::Acquired(permit) => permit,
            RequestOutcome::Waiting(_) => panic!("expected acquired hydration permit"),
            RequestOutcome::Enqueued => panic!("empty hydration request was enqueued"),
            RequestOutcome::Stale => panic!("current hydration request was stale"),
        }
    }

    fn waiting(outcome: RequestOutcome) -> crate::server::state::AdmissionGuard {
        match outcome {
            RequestOutcome::Waiting(admission) => admission,
            RequestOutcome::Acquired(_) => panic!("expected waiting hydration admission"),
            RequestOutcome::Enqueued => panic!("empty hydration request was enqueued"),
            RequestOutcome::Stale => panic!("current hydration request was stale"),
        }
    }

    fn probe(
        exit: HydrationProbeExit,
        entered: Option<tokio::sync::oneshot::Sender<()>>,
    ) -> (HydrationProbe, Arc<AtomicUsize>, Arc<AtomicUsize>) {
        let io_starts = Arc::new(AtomicUsize::new(0));
        let active_io = Arc::new(AtomicUsize::new(0));
        (
            HydrationProbe {
                exit,
                io_starts: Arc::clone(&io_starts),
                active_io: Arc::clone(&active_io),
                entered,
            },
            io_starts,
            active_io,
        )
    }

    fn handoff_probe(
        state: &AppState,
        exit: HandoffProbeExit,
        entered: Option<tokio::sync::oneshot::Sender<()>>,
    ) -> (
        HandoffProbe,
        Arc<AtomicUsize>,
        Arc<AtomicUsize>,
        Arc<AtomicUsize>,
        Arc<AtomicUsize>,
    ) {
        let runs = Arc::new(AtomicUsize::new(0));
        let mask_bits = Arc::new(AtomicUsize::new(0));
        let active_io = Arc::new(AtomicUsize::new(0));
        let owner_active = Arc::new(AtomicUsize::new(0));
        (
            HandoffProbe {
                exit,
                coordinator: Arc::clone(&state.coordinator),
                runs: Arc::clone(&runs),
                mask_bits: Arc::clone(&mask_bits),
                active_io: Arc::clone(&active_io),
                owner_active: Arc::clone(&owner_active),
                entered,
            },
            runs,
            mask_bits,
            active_io,
            owner_active,
        )
    }

    fn transferred_successor(state: &AppState, bits: u8) -> OwnerPermit {
        let owner = acquired(
            request_empty(state, OwnerKind::Hydration)
                .unwrap_or_else(|error| panic!("hydration owner request: {error}")),
        );
        assert!(matches!(
            CoordinatorCell::request(
                state.coordinator.admission(),
                WorkMask::from_bits(bits),
                OwnerKind::Sync,
            ),
            Ok(RequestOutcome::Enqueued)
        ));
        match CoordinatorCell::complete(owner) {
            CompletionOutcome::Transferred(successor) => successor,
            CompletionOutcome::Released => panic!("full mask was not transferred"),
            CompletionOutcome::RetirementAcknowledged => {
                panic!("ordinary completion acknowledged retirement")
            }
            CompletionOutcome::SequenceExhausted(_) => panic!("owner sequence exhausted"),
            CompletionOutcome::Stale => panic!("current hydration completion was stale"),
        }
    }

    fn recover_full_mask_once(state: &AppState) {
        let recovery = acquired(
            CoordinatorCell::request(
                state.coordinator.admission(),
                WorkMask::from_bits(0b001),
                OwnerKind::Sync,
            )
            .unwrap_or_else(|error| panic!("recovery request: {error}")),
        );
        assert_eq!(recovery.work_bits(), 0b111);
        assert!(matches!(
            CoordinatorCell::complete(recovery),
            CompletionOutcome::Released
        ));
        assert_eq!(state.coordinator.test_pending_bits(), 0);
    }

    async fn finish_driver(mut driver: DriverTaskGuard, abort: bool) {
        if let Some(task) = driver.task.take() {
            if abort {
                task.abort();
            }
            let _ = task.await;
        }
    }

    fn create_bind_workspace(root: &std::path::Path, name: &str, metrics_config: &str) -> PathBuf {
        let workspace = root.join(name);
        std::fs::create_dir_all(workspace.join(".git")).expect("create git metadata");
        std::fs::create_dir_all(workspace.join(".engram")).expect("create engram metadata");
        std::fs::write(
            workspace.join(".git").join("HEAD"),
            "ref: refs/heads/main\n",
        )
        .expect("write git HEAD");
        std::fs::write(
            workspace.join(".engram").join("config.toml"),
            metrics_config,
        )
        .expect("write workspace config");
        std::fs::write(workspace.join("lib.rs"), "pub fn indexed() {}\n").expect("write source");
        workspace
    }

    async fn join_hydration(state: &AppState) {
        if let Some(driver) = state.take_hydration_driver() {
            driver.join().await.expect("workspace hydration must join");
        }
    }

    async fn drive_test_transferred_sync(
        state: &AppState,
        permit: OwnerPermit,
        probe: Option<HandoffProbe>,
    ) {
        drive_transferred_sync(state, permit, probe).await;
    }

    #[tokio::test]
    async fn concurrent_bind_loser_cannot_replace_active_metrics_writer() {
        let _metrics_guard = crate::services::metrics::test_writer_guard().await;
        crate::services::metrics::shutdown()
            .await
            .expect("reset metrics writer");
        let temp = tempfile::tempdir().expect("tempdir");
        let first_workspace = temp.path().join("first");
        let second_workspace = temp.path().join("second");
        for workspace in [&first_workspace, &second_workspace] {
            std::fs::create_dir_all(workspace.join(".git")).expect("create git metadata");
            std::fs::write(
                workspace.join(".git").join("HEAD"),
                "ref: refs/heads/main\n",
            )
            .expect("write git HEAD");
            std::fs::write(workspace.join("lib.rs"), "pub fn indexed() {}\n")
                .expect("write source");
        }

        let state = Arc::new(AppState::new(1));
        let (first_passed_tx, first_passed_rx) = tokio::sync::oneshot::channel();
        let (resume_first_tx, resume_first_rx) = tokio::sync::oneshot::channel();
        let first_state = Arc::clone(&state);
        let first_path = first_workspace.display().to_string();
        let first = tokio::spawn(async move {
            set_workspace_after_precheck(first_state, first_path, first_passed_tx, resume_first_rx)
                .await
        });

        let (second_passed_tx, second_passed_rx) = tokio::sync::oneshot::channel();
        let (resume_second_tx, resume_second_rx) = tokio::sync::oneshot::channel();
        let second_state = Arc::clone(&state);
        let second_path = second_workspace.display().to_string();
        let second = tokio::spawn(async move {
            set_workspace_after_precheck(
                second_state,
                second_path,
                second_passed_tx,
                resume_second_rx,
            )
            .await
        });

        first_passed_rx
            .await
            .expect("first bind must pass the initial capacity check");
        second_passed_rx
            .await
            .expect("second bind must pass the same stale capacity check");

        resume_first_tx.send(()).expect("release first bind");
        first
            .await
            .expect("first bind task must join")
            .expect("first bind must publish");
        if let Some(driver) = state.take_hydration_driver() {
            driver.join().await.expect("first hydration must join");
        }

        resume_second_tx.send(()).expect("release losing bind");
        let losing_error = second
            .await
            .expect("losing bind task must join")
            .expect_err("second workspace must lose admission");
        assert!(matches!(
            losing_error,
            EngramError::Workspace(WorkspaceError::LimitReached { limit: 1 })
        ));

        crate::tools::write::index_workspace(Arc::clone(&state), None)
            .await
            .expect("active workspace must retain its metrics writer after losing bind");
        crate::services::metrics::shutdown()
            .await
            .expect("reset metrics writer");
    }

    #[tokio::test]
    async fn cancelled_bind_restores_enabled_metrics_before_workspace_admission_reopens() {
        let _metrics_guard = crate::services::metrics::test_writer_guard().await;
        crate::services::metrics::shutdown()
            .await
            .expect("reset metrics writer");
        let temp = tempfile::tempdir().expect("tempdir");
        let original = create_bind_workspace(
            temp.path(),
            "original",
            "[metrics]\nenabled = true\nbuffer_size = 7\nusage_path_override = \".engram/custom/usage.jsonl\"\n",
        );
        let replacement = create_bind_workspace(
            temp.path(),
            "replacement",
            "[metrics]\nenabled = true\nbuffer_size = 3\n",
        );
        let state = Arc::new(AppState::new(2));
        set_workspace(Arc::clone(&state), original.display().to_string())
            .await
            .expect("bind original workspace");
        join_hydration(&state).await;
        crate::services::metrics::reset_test_writer_activity_peak();

        let (metrics_replaced_tx, metrics_replaced_rx) = tokio::sync::oneshot::channel();
        let (_resume_tx, resume_rx) = tokio::sync::oneshot::channel();
        let bind_state = Arc::clone(&state);
        let bind = tokio::spawn(async move {
            set_workspace_after_metrics_replacement(
                bind_state,
                replacement.display().to_string(),
                metrics_replaced_tx,
                resume_rx,
                false,
            )
            .await
        });
        metrics_replaced_rx
            .await
            .expect("replacement must pause after metrics replacement");
        bind.abort();
        let _ = bind.await;

        let rollback_barrier = state.acquire_workspace_admission().await;
        drop(rollback_barrier);
        let index_result = crate::tools::write::index_workspace(Arc::clone(&state), None).await;
        let sync_result = crate::tools::write::sync_workspace(Arc::clone(&state), None).await;
        crate::services::metrics::record(crate::models::metrics::UsageEvent {
            tool_name: "restored_enabled_writer".to_owned(),
            branch: "main".to_owned(),
            ..crate::models::metrics::UsageEvent::default()
        });
        crate::services::metrics::shutdown()
            .await
            .expect("drain restored writer");

        assert!(
            index_result.is_ok(),
            "original workspace must index after cancelled bind: {index_result:?}"
        );
        assert!(
            sync_result.is_ok(),
            "original workspace must sync after cancelled bind: {sync_result:?}"
        );
        let custom_usage = original.join(".engram").join("custom").join("usage.jsonl");
        let usage = std::fs::read_to_string(&custom_usage)
            .unwrap_or_else(|error| panic!("read restored custom metrics config: {error}"));
        assert!(usage.contains("restored_enabled_writer"));
        assert_eq!(
            crate::services::metrics::max_test_writer_activity(),
            1,
            "replacement and restoration metrics writers must never overlap"
        );
    }

    #[tokio::test]
    async fn publication_error_restores_disabled_metrics_before_returning() {
        let _metrics_guard = crate::services::metrics::test_writer_guard().await;
        crate::services::metrics::shutdown()
            .await
            .expect("reset metrics writer");
        let temp = tempfile::tempdir().expect("tempdir");
        let original = create_bind_workspace(
            temp.path(),
            "original-disabled",
            "[metrics]\nenabled = false\nbuffer_size = 5\n",
        );
        let replacement = create_bind_workspace(
            temp.path(),
            "replacement-enabled",
            "[metrics]\nenabled = true\nbuffer_size = 3\n",
        );
        let state = Arc::new(AppState::new(2));
        set_workspace(Arc::clone(&state), original.display().to_string())
            .await
            .expect("bind original workspace");
        join_hydration(&state).await;
        crate::services::metrics::reset_test_writer_activity_peak();

        let (metrics_replaced_tx, metrics_replaced_rx) = tokio::sync::oneshot::channel();
        let (resume_tx, resume_rx) = tokio::sync::oneshot::channel();
        let bind_state = Arc::clone(&state);
        let bind = tokio::spawn(async move {
            set_workspace_after_metrics_replacement(
                bind_state,
                replacement.display().to_string(),
                metrics_replaced_tx,
                resume_rx,
                true,
            )
            .await
        });
        metrics_replaced_rx
            .await
            .expect("replacement must pause after metrics replacement");
        resume_tx.send(()).expect("resume failed publication");
        let bind_error = bind
            .await
            .expect("failed bind task must join")
            .expect_err("publication failpoint must reject the bind");
        let index_result = crate::tools::write::index_workspace(Arc::clone(&state), None).await;
        let sync_result = crate::tools::write::sync_workspace(Arc::clone(&state), None).await;
        let active = state.snapshot_workspace().await.expect("active workspace");
        let writer_result = crate::services::metrics::writer_control_token(
            std::path::Path::new(&active.path),
            &active.branch,
        );
        crate::services::metrics::shutdown()
            .await
            .expect("reset restored disabled writer");

        assert!(
            bind_error.to_string().contains("sequence"),
            "unexpected publication failure: {bind_error}"
        );
        assert!(
            writer_result.is_ok(),
            "disabled writer identity must be restored: {writer_result:?}"
        );
        assert!(
            index_result.is_ok(),
            "original workspace must index after publication failure: {index_result:?}"
        );
        assert!(
            sync_result.is_ok(),
            "original workspace must sync after publication failure: {sync_result:?}"
        );
        assert_eq!(
            crate::services::metrics::max_test_writer_activity(),
            1,
            "replacement and disabled restoration must never overlap"
        );
    }

    #[tokio::test]
    async fn hydration_waiting_and_stale_admission_never_reaches_io() {
        let _metrics_guard = crate::services::metrics::test_writer_guard().await;
        for held_owner in [true, false] {
            let state = Arc::new(AppState::new(2));
            let old_snapshot = coordinator_snapshot("old", "old");
            let _ = publish_test_binding(&state, old_snapshot.clone()).await;
            let owner = held_owner.then(|| {
                acquired(
                    request_empty(&state, OwnerKind::Index)
                        .unwrap_or_else(|error| panic!("owner request: {error}")),
                )
            });
            let admission = if held_owner {
                waiting(
                    request_empty(&state, OwnerKind::Hydration)
                        .unwrap_or_else(|error| panic!("hydration request: {error}")),
                )
            } else {
                state.coordinator.admission()
            };

            let _ = publish_test_binding(&state, coordinator_snapshot("new", "new")).await;
            let (probe, io_starts, active_io) = probe(HydrationProbeExit::DbFailure, None);
            background_db_hydration(
                Arc::clone(&state),
                DispatchSnapshot {
                    workspace: old_snapshot,
                    config: WorkspaceConfig::default(),
                },
                admission,
                Some(probe),
            )
            .await;

            assert_eq!(
                io_starts.load(Ordering::SeqCst),
                0,
                "pre-acquisition cancellation must exclude every hydration I/O boundary"
            );
            assert_eq!(active_io.load(Ordering::SeqCst), 0);
            assert_eq!(state.coordinator.test_notification_calls(), 0);
            if held_owner {
                assert!(state.coordinator.test_is_retiring());
            } else {
                assert!(state.coordinator.test_is_idle());
            }
            drop(owner);
        }
    }

    #[tokio::test]
    async fn hydration_progress_rejects_retired_owner_after_rebind() {
        let state = Arc::new(AppState::new(2));
        let _ = publish_test_binding(&state, coordinator_snapshot("old", "old")).await;
        let permit = acquired(
            request_empty(&state, OwnerKind::Hydration)
                .unwrap_or_else(|error| panic!("hydration request: {error}")),
        );
        let scope = permit.progress_scope().expect("hydration progress scope");
        let _ = publish_test_binding(&state, coordinator_snapshot("new", "new")).await;

        assert!(
            !set_hydration_progress(
                &state,
                &scope,
                Some(ScanProgress {
                    running: true,
                    files_scanned: 1,
                    files_total: 2,
                    last_completed_at: None,
                }),
            )
            .await,
            "retired hydration owner must not publish into the replacement binding"
        );
        assert!(state.scan_progress_snapshot().await.is_none());
        assert!(matches!(
            CoordinatorCell::complete(permit),
            CompletionOutcome::RetirementAcknowledged
        ));
    }

    #[tokio::test]
    async fn hydration_refreshes_head_before_its_first_io_boundary() {
        let metrics_guard = crate::services::metrics::test_writer_guard().await;
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace_path = temp.path().join("workspace");
        std::fs::create_dir_all(workspace_path.join(".git")).expect("create git metadata");
        std::fs::write(
            workspace_path.join(".git").join("HEAD"),
            "ref: refs/heads/main\n",
        )
        .expect("write git HEAD");
        let state = Arc::new(AppState::new(2));
        let mut stale_snapshot = coordinator_snapshot("hydration-branch", "stale-id");
        stale_snapshot.workspace_uuid = "uuid-worktree".to_owned();
        stale_snapshot.path = workspace_path.to_string_lossy().into_owned();
        stale_snapshot.data_dir = temp.path().join("data");
        stale_snapshot.branch = "captured-before-checkout".to_owned();
        let _ = publish_test_binding_with_disabled_metrics(
            &state,
            stale_snapshot.clone(),
            &metrics_guard,
        )
        .await;
        let old_admission = state.coordinator.admission();
        let admission = state.coordinator.admission();
        let (probe, io_starts, _) = probe(HydrationProbeExit::DbFailure, None);

        background_db_hydration(
            Arc::clone(&state),
            DispatchSnapshot {
                workspace: stale_snapshot,
                config: WorkspaceConfig::default(),
            },
            admission,
            Some(probe),
        )
        .await;

        assert_eq!(io_starts.load(Ordering::SeqCst), 1);
        let active = state.snapshot_workspace().await.expect("active workspace");
        assert_eq!(active.branch, "main");
        assert_eq!(
            active.workspace_id,
            crate::db::workspace::workspace_hash(&workspace_path, "main")
        );
        assert!(matches!(
            CoordinatorCell::request(old_admission, WorkMask::from_bits(0b001), OwnerKind::Sync,),
            Ok(RequestOutcome::Stale)
        ));
    }

    #[tokio::test]
    async fn spawned_hydration_rebind_is_supervised_until_quiescent_ack() {
        let metrics_guard = crate::services::metrics::test_writer_guard().await;
        for (same_binding, abort_task) in
            [(true, false), (true, true), (false, false), (false, true)]
        {
            let state = Arc::new(AppState::new(2));
            let old_snapshot = coordinator_snapshot("old", "old");
            let _ = publish_test_binding_with_disabled_metrics(
                &state,
                old_snapshot.clone(),
                &metrics_guard,
            )
            .await;
            let admission = state.coordinator.admission();
            let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
            let (probe, _io_starts, active_io) =
                probe(HydrationProbeExit::AwaitCancellation, Some(entered_tx));
            let task_state = Arc::clone(&state);
            let task = DriverTaskGuard {
                task: Some(tokio::spawn(async move {
                    background_db_hydration(
                        task_state,
                        DispatchSnapshot {
                            workspace: old_snapshot,
                            config: WorkspaceConfig::default(),
                        },
                        admission,
                        Some(probe),
                    )
                    .await;
                })),
            };
            entered_rx
                .await
                .unwrap_or_else(|error| panic!("hydration did not enter I/O: {error}"));
            assert!(
                !state.coordinator.test_is_idle(),
                "hydration must own its coordinator permit while I/O is active"
            );

            let target = if same_binding {
                coordinator_snapshot("old-rebound", "old")
            } else {
                coordinator_snapshot("new", "new")
            };
            let _ = publish_test_binding(&state, target).await;
            finish_driver(task, abort_task).await;

            assert_eq!(
                active_io.load(Ordering::SeqCst),
                0,
                "DB/file future must be gone before retirement acknowledgment"
            );
            assert!(state.coordinator.test_is_idle());
            assert_eq!(state.coordinator.test_notification_calls(), 1);
            assert_eq!(state.coordinator.test_pending_bits(), 0);
            assert!(
                !state.is_hydration_ready(),
                "cancelled hydration must not mark its replacement generation ready"
            );
        }
    }

    #[tokio::test]
    async fn hydration_db_failure_and_early_return_use_exact_terminals() {
        let metrics_guard = crate::services::metrics::test_writer_guard().await;
        for exit in [
            HydrationProbeExit::DbFailure,
            HydrationProbeExit::EarlyReturn,
        ] {
            let state = Arc::new(AppState::new(1));
            let snapshot = coordinator_snapshot("terminal", "terminal");
            let _ = publish_test_binding_with_disabled_metrics(
                &state,
                snapshot.clone(),
                &metrics_guard,
            )
            .await;
            let admission = state.coordinator.admission();
            let (probe, io_starts, active_io) = probe(exit, None);

            background_db_hydration(
                Arc::clone(&state),
                DispatchSnapshot {
                    workspace: snapshot,
                    config: WorkspaceConfig::default(),
                },
                admission,
                Some(probe),
            )
            .await;

            assert_eq!(io_starts.load(Ordering::SeqCst), 1);
            assert_eq!(active_io.load(Ordering::SeqCst), 0);
            assert!(state.coordinator.test_is_idle());
            assert_eq!(state.coordinator.test_notification_calls(), 1);
            assert_eq!(state.coordinator.test_pending_bits(), 0);
            assert!(
                !state.is_hydration_ready(),
                "DB failure and early return must not report hydration readiness"
            );
        }
    }

    #[tokio::test]
    async fn transferred_full_mask_executes_once_under_one_successor() {
        let metrics_guard = crate::services::metrics::test_writer_guard().await;
        let state = Arc::new(AppState::new(1));
        let _ = publish_test_binding_with_disabled_metrics(
            &state,
            coordinator_snapshot("handoff", "handoff"),
            &metrics_guard,
        )
        .await;
        let successor = transferred_successor(&state, 0b111);
        let (probe, runs, mask_bits, active_io, owner_active) =
            handoff_probe(&state, HandoffProbeExit::Handled, None);

        drive_test_transferred_sync(&state, successor, Some(probe)).await;

        assert_eq!(runs.load(Ordering::SeqCst), 1);
        assert_eq!(mask_bits.load(Ordering::SeqCst), 0b111);
        assert_eq!(active_io.load(Ordering::SeqCst), 0);
        assert_eq!(
            owner_active.load(Ordering::SeqCst),
            1,
            "transferred permit must remain authoritative for the whole drive"
        );
        assert!(state.coordinator.test_is_idle());
        assert_eq!(state.coordinator.test_pending_bits(), 0);
    }

    #[tokio::test]
    async fn failed_transferred_hydration_sync_recovers_its_full_mask() {
        let metrics_guard = crate::services::metrics::test_writer_guard().await;
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = temp.path().join("workspace");
        std::fs::create_dir_all(&workspace).expect("create workspace");
        let invalid_data_dir = temp.path().join("not-a-directory");
        std::fs::write(&invalid_data_dir, b"file blocks database directory")
            .expect("create invalid data path");
        let state = Arc::new(AppState::new(1));
        let _ = publish_test_binding_with_disabled_metrics(
            &state,
            WorkspaceSnapshot {
                workspace_id: "id-transfer-failure".to_owned(),
                workspace_uuid: "uuid-transfer-failure".to_owned(),
                branch: "main".to_owned(),
                data_dir: invalid_data_dir,
                path: workspace.display().to_string(),
                last_flush: None,
                stale_files: false,
                connection_count: 0,
                file_mtimes: std::collections::HashMap::new(),
            },
            &metrics_guard,
        )
        .await;
        let successor = transferred_successor(&state, 0b111);

        drive_test_transferred_sync(&state, successor, None).await;

        assert!(state.coordinator.test_is_idle());
        assert_eq!(
            state.coordinator.test_pending_bits(),
            0b111,
            "a failed transferred sync must remain recoverable through armed Drop"
        );
    }

    #[tokio::test]
    async fn transferred_partial_file_errors_recover_full_mask() {
        let metrics_guard = crate::services::metrics::test_writer_guard().await;
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = temp.path().join("workspace");
        let data_dir = temp.path().join("data");
        std::fs::create_dir_all(&workspace).expect("create workspace");
        std::fs::write(workspace.join("broken.py"), [0xff]).expect("write invalid UTF-8 fixture");
        let state = Arc::new(AppState::new(1));
        let _ = publish_test_binding_with_disabled_metrics(
            &state,
            WorkspaceSnapshot {
                workspace_id: "id-transfer-partial".to_owned(),
                workspace_uuid: "uuid-transfer-partial".to_owned(),
                branch: "main".to_owned(),
                data_dir,
                path: workspace.display().to_string(),
                last_flush: None,
                stale_files: false,
                connection_count: 0,
                file_mtimes: std::collections::HashMap::new(),
            },
            &metrics_guard,
        )
        .await;
        let successor = transferred_successor(&state, 0b111);

        drive_test_transferred_sync(&state, successor, None).await;

        assert!(state.coordinator.test_is_idle());
        assert_eq!(
            state.coordinator.test_pending_bits(),
            0b111,
            "a partial transferred sync must retain heavy work for retry"
        );
    }

    #[tokio::test]
    async fn hydration_handoff_supervises_a_second_transferred_successor() {
        let metrics_guard = crate::services::metrics::test_writer_guard().await;
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = temp.path().join("workspace");
        let data_dir = temp.path().join("data");
        std::fs::create_dir_all(&workspace).expect("create workspace");
        let state = Arc::new(AppState::new(1));
        let _ = publish_test_binding_with_disabled_metrics(
            &state,
            WorkspaceSnapshot {
                workspace_id: "id-second-transfer".to_owned(),
                workspace_uuid: "uuid-second-transfer".to_owned(),
                branch: "main".to_owned(),
                data_dir,
                path: workspace.display().to_string(),
                last_flush: None,
                stale_files: false,
                connection_count: 0,
                file_mtimes: std::collections::HashMap::new(),
            },
            &metrics_guard,
        )
        .await;
        let successor = transferred_successor(&state, 0b001);
        assert!(matches!(
            CoordinatorCell::request(
                state.coordinator.admission(),
                WorkMask::from_bits(0b010),
                OwnerKind::Sync,
            ),
            Ok(RequestOutcome::Enqueued)
        ));
        let (probe, runs, _mask_bits, _active_io, _owner_active) =
            handoff_probe(&state, HandoffProbeExit::Handled, None);

        drive_test_transferred_sync(&state, successor, Some(probe)).await;

        assert_eq!(runs.load(Ordering::SeqCst), 1);
        assert!(state.coordinator.test_is_idle());
        assert_eq!(
            state.coordinator.test_pending_bits(),
            0,
            "a second transferred successor must be driven rather than dropped"
        );
    }

    #[tokio::test]
    async fn lost_transferred_successor_republishes_once_for_one_recovery() {
        let metrics_guard = crate::services::metrics::test_writer_guard().await;
        for mode in [
            HandoffProbeExit::EarlyReturn,
            HandoffProbeExit::AwaitCancellation,
            HandoffProbeExit::Handled,
        ] {
            let state = Arc::new(AppState::new(1));
            let _ = publish_test_binding_with_disabled_metrics(
                &state,
                coordinator_snapshot("handoff-loss", "handoff-loss"),
                &metrics_guard,
            )
            .await;
            let successor = transferred_successor(&state, 0b111);
            let is_early = matches!(mode, HandoffProbeExit::EarlyReturn);
            let is_abort = matches!(mode, HandoffProbeExit::Handled);

            if is_early {
                let (probe, _runs, _mask, active_io, owner_active) =
                    handoff_probe(&state, mode, None);
                drive_test_transferred_sync(&state, successor, Some(probe)).await;
                assert_eq!(active_io.load(Ordering::SeqCst), 0);
                assert_eq!(owner_active.load(Ordering::SeqCst), 1);
            } else {
                let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
                let (probe, _runs, _mask, active_io, owner_active) = handoff_probe(
                    &state,
                    HandoffProbeExit::AwaitCancellation,
                    Some(entered_tx),
                );
                let task_state = Arc::clone(&state);
                let driver = DriverTaskGuard {
                    task: Some(tokio::spawn(async move {
                        drive_test_transferred_sync(&task_state, successor, Some(probe)).await;
                    })),
                };
                entered_rx
                    .await
                    .unwrap_or_else(|error| panic!("handoff did not enter drive: {error}"));
                assert!(
                    !state.coordinator.test_is_idle(),
                    "successor ownership ended before its DB/file future"
                );
                assert_eq!(owner_active.load(Ordering::SeqCst), 1);

                if !is_abort {
                    let _ = publish_test_binding(
                        &state,
                        coordinator_snapshot("handoff-rebound", "handoff-loss"),
                    )
                    .await;
                }
                finish_driver(driver, is_abort).await;
                assert_eq!(active_io.load(Ordering::SeqCst), 0);
            }

            assert!(state.coordinator.test_is_idle());
            assert_eq!(state.coordinator.test_pending_bits(), 0b111);
            assert_eq!(state.coordinator.test_notification_calls(), 1);
            recover_full_mask_once(&state);
            assert_eq!(state.coordinator.test_notification_calls(), 2);
        }
    }

    // ── 099.007-T: a queued MCP sync carrying backfill_python_canonical=true
    // must actually RUN the canonical backfill when it drains ────────────────
    //
    // When `sync_workspace({backfill_python_canonical:true})` arrives while a
    // coordinator owner is active, write.rs coalesces the complete work mask
    // and returns "queued". The transferred successor must carry that intent
    // into a GATED
    // `sync_workspace_with_progress(.., backfill_python, ..)` — not downgrade to
    // a routine (ungated) sync, which would defer the stale-marker migration
    // (C12-5 no-op) and silently drop the requested backfill. The production
    // carry-through is an end-to-end regression guard for the transferred mask.
    //
    // RED-capable: reverting the driver's `backfill_python` argument to `false`
    // makes the coalesced sync ungated, so the pre-upgrade canonical edge is NOT
    // restored and the extraction-version marker stays stale ("0") — both
    // assertions below then fail.
    #[tokio::test]
    async fn queued_backfill_python_runs_gated_sync_on_drain() {
        let metrics_guard = crate::services::metrics::test_writer_guard().await;
        let tmp = tempfile::tempdir().expect("tempdir");
        let ws = tmp.path().join("ws");
        let data_dir = tmp.path().join("data");
        std::fs::create_dir_all(&ws).expect("create ws dir");
        std::fs::write(
            ws.join("app.py"),
            "from helper import compute\n\n\ndef run():\n    compute()\n",
        )
        .expect("write app.py");
        std::fs::write(ws.join("helper.py"), "def compute():\n    return 1\n")
            .expect("write helper");

        let branch = "main";
        let config = CodeGraphConfig::default();

        // Fresh full index resolves the cross-module canonical edge and advances
        // the extraction-version marker to the current version.
        code_graph::index_workspace(&ws, &data_dir, branch, &config, false)
            .await
            .expect("index should succeed");

        // Simulate the pre-upgrade state a backfill migration is meant to repair:
        // retract the canonical edge and reset the marker to a stale version.
        {
            let db = connect_db(&data_dir, branch).await.expect("db connect");
            let q = CodeGraphQueries::new(db);
            q.retract_all_calls_resolved_canonical_edges()
                .await
                .expect("retract canonical edges");
            q.set_python_extraction_version("0")
                .await
                .expect("stale marker");
            let before = q
                .list_calls_edges_by_resolution("calls_resolved_canonical")
                .await
                .expect("canonical edges");
            assert!(
                before.is_empty(),
                "precondition: no canonical edge before the queued backfill drains; got {before:?}"
            );
        }

        // Bind the workspace + config so the drain enters its real sync branch.
        let state = Arc::new(AppState::new(1));
        state
            .set_workspace(WorkspaceSnapshot {
                workspace_id: "test-ws".to_owned(),
                workspace_uuid: "00000000-0000-0000-0000-000000000000".to_owned(),
                branch: branch.to_owned(),
                data_dir: data_dir.clone(),
                path: ws.display().to_string(),
                last_flush: None,
                stale_files: false,
                connection_count: 0,
                file_mtimes: std::collections::HashMap::new(),
            })
            .await
            .expect("set workspace");
        state
            .set_workspace_config(Some(WorkspaceConfig::default()))
            .await;
        crate::services::metrics::configure_test_disabled_writer(&metrics_guard, &ws, branch)
            .await
            .expect("configure disabled metrics writer");

        // Queue exactly the routine + Python-backfill work bits behind a
        // hydration owner, then drive the move-only transferred successor.
        let successor = transferred_successor(&state, 0b101);
        drive_test_transferred_sync(&state, successor, None).await;

        let db = connect_db(&data_dir, branch).await.expect("db reconnect");
        let q = CodeGraphQueries::new(db);
        let name_by_id: std::collections::HashMap<String, String> = q
            .all_functions()
            .await
            .expect("all_functions")
            .into_iter()
            .map(|f| {
                (
                    f.id,
                    format!("{}@{}", f.name, f.file_path.replace('\\', "/")),
                )
            })
            .collect();
        let restored: Vec<(String, String)> = q
            .list_calls_edges_by_resolution("calls_resolved_canonical")
            .await
            .expect("canonical edges")
            .into_iter()
            .map(|(from, to)| {
                (
                    name_by_id.get(&from).cloned().unwrap_or(from),
                    name_by_id.get(&to).cloned().unwrap_or(to),
                )
            })
            .collect();
        assert!(
            restored.contains(&("run@app.py".to_owned(), "compute@helper.py".to_owned())),
            "the queued backfill must re-materialize the cross-module canonical edge on drain; got {restored:?}"
        );
        assert_eq!(
            q.python_extraction_version().expect("read marker"),
            Some("1".to_owned()),
            "a queued backfill that drains must advance the extraction-version marker (not defer as an ungated sync)"
        );
        assert!(state.coordinator.test_is_idle());
        assert_eq!(state.coordinator.test_pending_bits(), 0);
    }

    /// Regression guard for 078.001-T: `get_daemon_status.memory_bytes` must
    /// report this process's resident memory, not system-wide RAM. Prior to the
    /// fix it returned `System::used_memory()` (whole-machine usage), which on a
    /// busy host overstated engram's footprint by more than an order of
    /// magnitude.
    #[tokio::test]
    async fn daemon_status_memory_reflects_process_not_system() {
        let state = AppState::new(10);
        let status = get_daemon_status(&state)
            .await
            .expect("get_daemon_status should succeed");

        // Independently sample this process's resident memory.
        let mut sys = System::new();
        let pid = sysinfo::get_current_pid().expect("current process pid");
        sys.refresh_process(pid);
        let process_bytes = sys
            .process(pid)
            .expect("current process present in sysinfo")
            .memory();

        let tolerance = std::cmp::max(64 * 1024 * 1024, process_bytes / 4);
        let diff = status.memory_bytes.abs_diff(process_bytes);
        assert!(
            diff <= tolerance,
            "daemon_status.memory_bytes ({}) must reflect this process's memory \
             ({process_bytes} bytes); diff {diff} exceeds tolerance {tolerance} — \
             likely reporting system-wide RAM",
            status.memory_bytes,
        );
    }
}
