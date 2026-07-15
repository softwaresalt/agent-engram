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
/// Connects to CozoDB, hydrates the code graph from JSONL files, and runs
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
    // Acquire the indexing lock immediately so the startup auto-sync task
    // (spawned after set_workspace returns) cannot grab it and open a competing
    // DB connection while we are still doing schema bootstrap.  Concurrent
    // writers on the same CozoDB/SQLite file cause SQLITE_BUSY retries in
    // run_schema_bootstrap (up to 500ms × 20 attempts × 23 scripts ≈ minutes)
    // which prevents set_hydration_ready() from being called within the shim's
    // poll_until_ready timeout.  Holding the lock here forces the auto-sync
    // task to see try_start_indexing() == false and exit immediately; the lock
    // is released after we call finish_indexing() at the end of this function.
    //
    // `acquired_lock` tracks whether THIS task holds the flag so that only the
    // holder calls finish_indexing() — releasing another task's lock would break
    // the concurrency guard.
    let acquired_lock = state.try_start_indexing();

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
                // Only release the lock if this task acquired it.
                if acquired_lock {
                    state.finish_indexing().await;
                }
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
            if acquired_lock {
                state.finish_indexing().await;
            }
            return;
        }
    };

    check_cancel!();

    let cg_queries = CodeGraphQueries::new(db);

    // Signal "ready" immediately after the DB connects so the shim's
    // poll_until_ready succeeds as soon as the daemon is responsive.
    // JSONL code-graph hydration and offline-change detection may take
    // minutes on large workspaces; keeping the daemon in "starting" for that
    // entire duration causes the shim to time out.  Read-only tool handlers
    // no longer gate on is_indexing(), so they return available (possibly
    // partial) data while the background hydration and re-index complete.
    state.set_hydration_ready();

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

    // Trigger a code-graph re-index when offline changes were found, but only
    // when ENGRAM_AUTO_REINDEX=true is set.  Without the opt-in, startup
    // re-indexing on a large workspace (e.g. 1 000+ files) can consume
    // several gigabytes of RAM and block the daemon for many minutes.
    // Users can trigger a re-index explicitly with `engram sync` or
    // `engram index` at any time.
    let auto_reindex = std::env::var("ENGRAM_AUTO_REINDEX")
        .map(|v| matches!(v.to_lowercase().as_str(), "1" | "true" | "yes"))
        .unwrap_or(false);

    if offline_count > 0 && auto_reindex {
        check_cancel!();
        if let (Some(snapshot), Some(ws_config)) = (
            state.snapshot_workspace().await,
            state.workspace_config().await,
        ) {
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
    // Release the indexing lock only if this task acquired it, then drain any
    // pending sync that was queued while we held the lock.
    if acquired_lock {
        state.finish_indexing().await;
        drain_pending_sync(&state).await;
    }
}

/// Drain a sync request that was queued while an indexing operation held the lock.
///
/// Called from every [`AppState::finish_indexing`] site so that a
/// `sync_workspace` request queued during indexing is eventually executed.
/// Multiple concurrent queue requests are coalesced into a single sync run.
///
/// # Race safety
///
/// The pending-sync flag is only consumed *after* the indexing lock is
/// successfully acquired, preventing the flag from being cleared when the
/// lock is unavailable. If `try_start_indexing` fails, the flag is re-set
/// so the next `finish_indexing` caller can drain it.
pub async fn drain_pending_sync(state: &AppState) {
    if !state.take_pending_sync() {
        return;
    }
    tracing::info!("drain_pending_sync: running coalesced sync after indexing completed");
    if let (Some(snapshot), Some(ws_config)) = (
        state.snapshot_workspace().await,
        state.workspace_config().await,
    ) {
        if state.try_start_indexing() {
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
                    "drain_pending_sync: coalesced sync complete"
                ),
                Err(e) => tracing::warn!(
                    error = %e,
                    "drain_pending_sync: coalesced sync failed"
                ),
            }
            state.finish_indexing().await;
        } else {
            // Another indexer grabbed the lock before we could; re-queue so
            // the next finish_indexing caller can drain it.
            state.set_pending_sync();
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
    // dispatch-context read eliminates that WIDE tear window. NOTE: `set_workspace`
    // still publishes the binding and its config in two separate awaits, so this
    // is not a strict atomic publish/observe guarantee — the residual, much
    // narrower writer-side window is a tracked follow-up (082-S review F4), not
    // closed here.
    let Some(ctx) = state.snapshot_dispatch_context().await else {
        return Err(EngramError::Workspace(WorkspaceError::NotSet));
    };
    let snapshot = ctx.workspace;
    let retrieval_eval_enabled = ctx.config.retrieval_eval.enabled;

    let engram_dir = Path::new(&snapshot.path).join(".engram");
    let stale_now = snapshot.stale_files || detect_stale_since(&snapshot.file_mtimes, &engram_dir);

    if stale_now != snapshot.stale_files {
        let _ = state
            .update_workspace(|ws| ws.stale_files = stale_now)
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
    use sysinfo::System;

    use super::get_daemon_status;
    use crate::server::state::AppState;

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
