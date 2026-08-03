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
use crate::errors::{EngramError, SystemError, WorkspaceError};
use crate::models::health::{HealthReport, ScanProgress};
use crate::server::state::CoordinatorCell;
use crate::server::state::{
    AdmissionGuard, AppState, CompletionOutcome, CoordinatorError, DriverTaskGuard, OwnerPermit,
    WorkspaceSnapshot,
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

pub async fn set_workspace(
    state: Arc<AppState>,
    path: String,
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

    // Initialise metrics sink (spawns a channel + background writer, no DB).
    crate::services::metrics::initialize(&canonical, &branch, &ws_config.metrics).await?;

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

    let (binding_generation, _cancel_rx) = state
        .publish_workspace_generation(snapshot, Some(ws_config.clone()))
        .await
        .map_err(|error| match error {
            CoordinatorError::SequenceExhausted => {
                EngramError::System(SystemError::InvalidParams {
                    reason: error.to_string(),
                })
            }
            CoordinatorError::WorkspaceLimit { limit } => {
                EngramError::Workspace(WorkspaceError::LimitReached { limit })
            }
        })?;
    crate::services::query_stats::reset_timing();

    let previous_driver = {
        state
            .hydration_driver
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()
    };
    if let Some((_generation, mut previous_driver)) = previous_driver {
        if let Some(task) = previous_driver.task.take() {
            if !task.is_finished() {
                task.abort();
            }
            let _ = task.await;
        }
    }

    // Queue a background scan immediately. The DB connect + hydrate +
    // offline-change detection are moved off the hot path so that
    // set_workspace returns well within the 500 ms bind-latency SLA
    // (029-F WS-6). The `pending_scan` field signals to the caller
    // that background work has been scheduled.
    let initial_progress = ScanProgress {
        running: true,
        files_scanned: 0,
        files_total: 0,
        last_completed_at: None,
    };
    state.set_scan_progress(Some(initial_progress)).await;

    let state_bg = Arc::clone(&state);
    let canonical_bg = canonical.clone();
    let data_dir_bg = data_dir.clone();
    let branch_bg = branch.clone();
    let admission = state.coordinator.admission();
    let task = tokio::spawn(async move {
        background_db_hydration(
            state_bg,
            canonical_bg,
            data_dir_bg,
            branch_bg,
            admission,
            #[cfg(test)]
            None,
        )
        .await;
    });
    let mut driver = Some(DriverTaskGuard { task: Some(task) });
    let displaced = {
        let mut retained = state
            .hydration_driver
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if retained
            .as_ref()
            .is_some_and(|(generation, _)| *generation > binding_generation)
        {
            driver.take()
        } else {
            retained
                .replace((
                    binding_generation,
                    driver
                        .take()
                        .unwrap_or_else(|| unreachable!("driver is installed once")),
                ))
                .map(|(_generation, guard)| guard)
        }
    };
    if let Some(mut displaced) = displaced {
        if let Some(task) = displaced.task.take() {
            task.abort();
            let _ = task.await;
        }
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
    #[cfg(test)]
    EarlyReturn,
}

async fn background_db_hydration(
    state: Arc<AppState>,
    canonical: PathBuf,
    data_dir: PathBuf,
    branch: String,
    admission: AdmissionGuard,
    #[cfg(test)] test_probe: Option<HydrationProbe>,
) {
    let mut permit = match admission.acquire_hydration().await {
        Ok(Some(permit)) => permit,
        Ok(None) => return,
        Err(error) => {
            tracing::error!(%error, "background_db_hydration: admission failed");
            return;
        }
    };

    let operation = async {
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
                state
                    .set_scan_progress(Some(ScanProgress {
                        running: false,
                        files_scanned: 0,
                        files_total: 0,
                        last_completed_at: Some(Utc::now().to_rfc3339()),
                    }))
                    .await;
                state.set_hydration_ready();
                return HydrationTerminal::DbFailure;
            }
        };

        let cg_queries = CodeGraphQueries::new(db);

        // Signal "ready" immediately after the DB connects so the shim's
        // poll_until_ready succeeds while the longer hydration continues.
        state.set_hydration_ready();

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

        state
            .set_scan_progress(Some(ScanProgress {
                running: false,
                files_scanned: offline_count,
                files_total: offline_count,
                last_completed_at: Some(Utc::now().to_rfc3339()),
            }))
            .await;

        let auto_reindex = std::env::var("ENGRAM_AUTO_REINDEX")
            .map(|v| matches!(v.to_lowercase().as_str(), "1" | "true" | "yes"))
            .unwrap_or(false);

        if offline_count > 0 && auto_reindex {
            if let Some((snapshot, ws_config)) = state.snapshot_workspace_and_config().await {
                let ws_path = PathBuf::from(&snapshot.path);
                tracing::info!(
                    offline_count,
                    "background_db_hydration: ENGRAM_AUTO_REINDEX=true, starting post-scan re-index"
                );
                match sync_code_graph(
                    &ws_path,
                    &snapshot.data_dir,
                    &snapshot.branch,
                    &ws_config.code_graph,
                )
                .await
                {
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
    mut permit: OwnerPermit,
    #[cfg(test)] test_probe: Option<HandoffProbe>,
) {
    let work_mask = permit.work_bits();
    let operation = async {
        #[cfg(test)]
        if let Some(probe) = test_probe {
            return probe.run(work_mask).await;
        }

        let Some((snapshot, ws_config)) = state.snapshot_workspace_and_config().await else {
            return HandoffTerminal::Handled;
        };
        let ws_path = PathBuf::from(&snapshot.path);
        let revalidate = work_mask & 0b010 != 0;
        let backfill_python = work_mask & 0b100 != 0;
        match crate::services::code_graph::sync_workspace_with_progress(
            &ws_path,
            &snapshot.data_dir,
            &snapshot.branch,
            &ws_config.code_graph,
            backfill_python,
            revalidate,
            None,
        )
        .await
        {
            Ok(result) => tracing::info!(
                files_added = result.files_added,
                files_modified = result.files_modified,
                revalidate,
                backfill_python,
                "transferred sync complete"
            ),
            Err(e) => tracing::warn!(
                error = %e,
                revalidate,
                backfill_python,
                "transferred sync failed"
            ),
        }
        HandoffTerminal::Handled
    };

    match permit.run_until_cancelled(operation).await {
        Some(HandoffTerminal::Handled) => {
            if let CompletionOutcome::Transferred(successor) = CoordinatorCell::complete(permit) {
                drop(successor);
            }
        }
        #[cfg(test)]
        Some(HandoffTerminal::EarlyReturn) => {}
        None => {}
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
        background_db_hydration, drive_transferred_sync, get_daemon_status,
    };
    use crate::db::connect_db;
    use crate::db::queries::CodeGraphQueries;
    use crate::models::config::{CodeGraphConfig, WorkspaceConfig};
    use crate::server::state::{
        AppState, CompletionOutcome, CoordinatorCell, DriverTaskGuard, OwnerKind, OwnerPermit,
        RequestOutcome, WorkMask, WorkspaceSnapshot,
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

    #[tokio::test]
    async fn hydration_waiting_and_stale_admission_never_reaches_io() {
        for held_owner in [true, false] {
            let state = Arc::new(AppState::new(2));
            let _ = publish_test_binding(&state, coordinator_snapshot("old", "old")).await;
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
                PathBuf::from("C:/workspace/old"),
                PathBuf::from("logs/phase6-group2/old-data"),
                "main".to_owned(),
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
    async fn spawned_hydration_rebind_is_supervised_until_quiescent_ack() {
        for (same_binding, abort_task) in
            [(true, false), (true, true), (false, false), (false, true)]
        {
            let state = Arc::new(AppState::new(2));
            let _ = publish_test_binding(&state, coordinator_snapshot("old", "old")).await;
            let admission = state.coordinator.admission();
            let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
            let (probe, _io_starts, active_io) =
                probe(HydrationProbeExit::AwaitCancellation, Some(entered_tx));
            let task_state = Arc::clone(&state);
            let task = DriverTaskGuard {
                task: Some(tokio::spawn(async move {
                    background_db_hydration(
                        task_state,
                        PathBuf::from("C:/workspace/old"),
                        PathBuf::from("logs/phase6-group2/old-data"),
                        "main".to_owned(),
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
        }
    }

    #[tokio::test]
    async fn hydration_db_failure_and_early_return_use_exact_terminals() {
        for exit in [
            HydrationProbeExit::DbFailure,
            HydrationProbeExit::EarlyReturn,
        ] {
            let state = Arc::new(AppState::new(1));
            let _ =
                publish_test_binding(&state, coordinator_snapshot("terminal", "terminal")).await;
            let admission = state.coordinator.admission();
            let (probe, io_starts, active_io) = probe(exit, None);

            background_db_hydration(
                Arc::clone(&state),
                PathBuf::from("C:/workspace/terminal"),
                PathBuf::from("logs/phase6-group2/terminal-data"),
                "main".to_owned(),
                admission,
                Some(probe),
            )
            .await;

            assert_eq!(io_starts.load(Ordering::SeqCst), 1);
            assert_eq!(active_io.load(Ordering::SeqCst), 0);
            assert!(state.coordinator.test_is_idle());
            assert_eq!(state.coordinator.test_notification_calls(), 1);
            assert_eq!(state.coordinator.test_pending_bits(), 0);
        }
    }

    #[tokio::test]
    async fn transferred_full_mask_executes_once_under_one_successor() {
        let state = Arc::new(AppState::new(1));
        let _ = publish_test_binding(&state, coordinator_snapshot("handoff", "handoff")).await;
        let successor = transferred_successor(&state, 0b111);
        let (probe, runs, mask_bits, active_io, owner_active) =
            handoff_probe(&state, HandoffProbeExit::Handled, None);

        drive_transferred_sync(&state, successor, Some(probe)).await;

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
    async fn lost_transferred_successor_republishes_once_for_one_recovery() {
        for mode in [
            HandoffProbeExit::EarlyReturn,
            HandoffProbeExit::AwaitCancellation,
            HandoffProbeExit::Handled,
        ] {
            let state = Arc::new(AppState::new(1));
            let _ =
                publish_test_binding(&state, coordinator_snapshot("handoff-loss", "handoff-loss"))
                    .await;
            let successor = transferred_successor(&state, 0b111);
            let is_early = matches!(mode, HandoffProbeExit::EarlyReturn);
            let is_abort = matches!(mode, HandoffProbeExit::Handled);

            if is_early {
                let (probe, _runs, _mask, active_io, owner_active) =
                    handoff_probe(&state, mode, None);
                drive_transferred_sync(&state, successor, Some(probe)).await;
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
                        drive_transferred_sync(&task_state, successor, Some(probe)).await;
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

        // Queue exactly the routine + Python-backfill work bits behind a
        // hydration owner, then drive the move-only transferred successor.
        let successor = transferred_successor(&state, 0b101);
        drive_transferred_sync(&state, successor, None).await;

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
