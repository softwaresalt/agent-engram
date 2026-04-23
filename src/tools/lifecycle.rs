use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::Ordering;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sysinfo::System;
use uuid::Uuid;

use crate::db::connect_db;
use crate::db::queries::CodeGraphQueries;
use crate::db::workspace::{
    canonicalize_workspace, load_or_create_workspace_id, resolve_data_dir, resolve_git_branch,
    workspace_hash,
};
use crate::errors::{EngramError, SystemError, WorkspaceError};
use crate::models::health::{HealthReport, ScanProgress};
use crate::server::state::{AppState, WorkspaceSnapshot};
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
    /// Absolute path to the SurrealDB storage directory for this workspace and branch.
    pub db_path: String,
    pub last_flush: Option<String>,
    pub stale_files: bool,
    pub connection_count: usize,
    pub code_graph: CodeGraphStats,
    /// Background scan progress snapshot; `null` until the first scan is queued (029-F WS-6).
    pub scan_status: Option<ScanProgress>,
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

    state.set_workspace(snapshot).await?;
    state.set_workspace_config(Some(ws_config.clone())).await;
    crate::services::query_stats::reset_timing();

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

    // Cancel any stale scan from a prior set_workspace call, then register
    // a fresh cancellation receiver for this generation.
    // Also reset the hydration-ready flag so `_health` gates "ready" on the
    // new cycle completing rather than inheriting the prior workspace's state.
    state.clear_hydration_ready();
    let cancel_rx = state.begin_scan_generation().await;

    let state_bg = Arc::clone(&state);
    let canonical_bg = canonical.clone();
    let data_dir_bg = data_dir.clone();
    let branch_bg = branch.clone();
    let _task = tokio::spawn(async move {
        background_db_hydration(state_bg, canonical_bg, data_dir_bg, branch_bg, cancel_rx).await;
    });

    Ok(WorkspaceBinding {
        workspace_id,
        path: canonical.display().to_string(),
        hydrated: true,
        pending_scan: true,
    })
}

/// Background DB hydration task spawned by [`set_workspace`].
///
/// Connects to SurrealDB, hydrates the code graph from JSONL files, and runs
/// offline-change detection — all off the bind-latency hot path. Updates
/// [`AppState::scan_progress`] when complete (029-F WS-6).
///
/// Checks `cancel_rx` at each major step; when `true` is signalled by
/// [`AppState::begin_scan_generation`], the task abandons its work (029-F WS-6
/// CancellationToken requirement).
async fn background_db_hydration(
    state: Arc<AppState>,
    canonical: PathBuf,
    data_dir: PathBuf,
    branch: String,
    mut cancel_rx: tokio::sync::watch::Receiver<bool>,
) {
    macro_rules! check_cancel {
        () => {
            if *cancel_rx.borrow_and_update() {
                tracing::info!("background_db_hydration: scan cancelled by new generation");
                state
                    .set_scan_progress(Some(ScanProgress {
                        running: false,
                        files_scanned: 0,
                        files_total: 0,
                        last_completed_at: Some(Utc::now().to_rfc3339()),
                    }))
                    .await;
                state.set_hydration_ready();
                return;
            }
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
            return;
        }
    };

    check_cancel!();

    let cg_queries = CodeGraphQueries::new(db);

    if let Err(e) = hydrate_code_graph(&canonical, &data_dir, &branch, &cg_queries).await {
        tracing::warn!(error = %e, "background_db_hydration: code graph hydration failed");
    }

    check_cancel!();

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
            tracing::warn!(error = %e, "background_db_hydration: offline change detection failed");
            0
        }
    };

    check_cancel!();

    state
        .set_scan_progress(Some(ScanProgress {
            running: false,
            files_scanned: offline_count,
            files_total: offline_count,
            last_completed_at: Some(Utc::now().to_rfc3339()),
        }))
        .await;

    // Trigger a code-graph re-index when offline changes were found.
    // Guarded by the try_start_indexing flag so concurrent indexing is prevented.
    if offline_count > 0 && state.try_start_indexing() {
        check_cancel!();
        if let (Some(snapshot), Some(ws_config)) = (
            state.snapshot_workspace().await,
            state.workspace_config().await,
        ) {
            let ws_path = PathBuf::from(&snapshot.path);
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
        state.finish_indexing().await;
    }

    state.set_hydration_ready();
}

pub async fn get_daemon_status(state: &AppState) -> Result<DaemonStatus, EngramError> {
    let mut sys = System::new();
    sys.refresh_memory();
    let memory_bytes = sys.used_memory(); // sysinfo 0.30+ returns bytes

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
    if let Some(snapshot) = state.snapshot_workspace().await {
        let engram_dir = Path::new(&snapshot.path).join(".engram");
        let stale_now =
            snapshot.stale_files || detect_stale_since(&snapshot.file_mtimes, &engram_dir);

        if stale_now != snapshot.stale_files {
            let _ = state
                .update_workspace(|ws| ws.stale_files = stale_now)
                .await;
        }

        // Gather code graph stats from the database
        let code_graph = if let Ok(db) = connect_db(&snapshot.data_dir, &snapshot.branch).await {
            let cg_queries = CodeGraphQueries::new(db);
            let code_files = cg_queries.count_code_files().await.unwrap_or(0);
            let functions = cg_queries.count_functions().await.unwrap_or(0);
            let classes = cg_queries.count_classes().await.unwrap_or(0);
            let interfaces = cg_queries.count_interfaces().await.unwrap_or(0);
            let edges = cg_queries.count_code_edges().await.unwrap_or(0);
            CodeGraphStats {
                code_files,
                functions,
                classes,
                interfaces,
                edges,
            }
        } else {
            CodeGraphStats::default()
        };

        let db_path = snapshot
            .data_dir
            .join("db")
            .join(&snapshot.branch)
            .display()
            .to_string();

        return Ok(WorkspaceStatus {
            path: snapshot.path,
            branch: snapshot.branch,
            db_path,
            last_flush: snapshot.last_flush,
            stale_files: stale_now,
            connection_count: state.active_connections(),
            code_graph,
            scan_status: state.scan_progress_snapshot().await,
        });
    }

    Err(EngramError::Workspace(WorkspaceError::NotSet))
}
