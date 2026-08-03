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
use crate::server::state::{AppState, CoordinatorError, WorkspaceSnapshot};
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

    let (sync_generation, cancel_rx) = state
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
    let _task = tokio::spawn(async move {
        background_db_hydration(
            state_bg,
            canonical_bg,
            data_dir_bg,
            branch_bg,
            sync_generation,
            cancel_rx,
        )
        .await;
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
///
/// `sync_generation` is the generation this task owns (returned alongside
/// `cancel_rx` by [`AppState::begin_scan_generation`]). The cancel and
/// DB-connect-failure paths clear the pending-sync queue scoped to THIS
/// generation ([`AppState::clear_pending_sync_for_generation`]) so they never
/// wipe a *newer* generation's request published in the `set_workspace` cancel
/// race window (105.001-T / R1).
async fn background_db_hydration(
    state: Arc<AppState>,
    canonical: PathBuf,
    data_dir: PathBuf,
    branch: String,
    sync_generation: u64,
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
                    // 104.002-T / 105.001-T (N1/N5): a cancelled generation must
                    // not drain the queued heavy sync against a torn-down state —
                    // clear the pending + companion bits scoped to THIS
                    // generation. A newer generation's request published in the
                    // cancel race window survives; this generation's own stale
                    // bits are dropped. The new generation's scan re-queues
                    // whatever it actually needs.
                    state.clear_pending_sync_for_generation(sync_generation);
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
                // 104.002-T / 105.001-T (N1): DB connect failed — clear the
                // queued pending + companion bits scoped to THIS generation
                // rather than leak them into a later unrelated sync (there is no
                // live DB to drain against anyway). A newer generation's request
                // is preserved.
                state.clear_pending_sync_for_generation(sync_generation);
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
    // Release the indexing lock only if this task acquired it, then drain any
    // pending sync that was queued while we held the lock.
    if acquired_lock {
        state.finish_indexing().await;
        drain_pending_sync_to_completion(&state).await;
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
    if let Some((snapshot, ws_config)) = state.snapshot_workspace_and_config().await {
        if state.try_start_indexing() {
            let ws_path = PathBuf::from(&snapshot.path);
            // 101.002-T: drain the pending gate flags AFTER acquiring the
            // indexing lock so that, if the lock grab below had failed and the
            // sync were re-queued, the flags would survive for the next drain.
            // A coalesced revalidation/backfill must not be downgraded to a
            // routine sync that silently drops the requested migration.
            let revalidate = state.take_pending_sync_revalidate();
            let backfill_python = state.take_pending_sync_backfill_python();
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
            // the next finish_indexing caller can drain it. The
            // pending-revalidate flag was intentionally NOT taken above, so it
            // remains set for the next drain.
            state.set_pending_sync();
        }
    }
}

/// Drain the coalesced pending sync, looping until nothing remains (104.002-T).
///
/// A single [`drain_pending_sync`] pass is single-shot: a `pending_sync`
/// re-armed *during* the drain (either a fresh request or the lost-lock
/// re-queue) is left for an unspecified "next `finish_indexing` caller", which
/// can stall the queued sync. This wrapper loops so the re-arm is handled here
/// instead (N2). The loop is bounded by `MAX_DRAIN_ITERATIONS` with a warn guard
/// against a pathological set/drain livelock (H3), and yields cooperatively each
/// pass so a contended indexing lock lets the competing indexer make progress
/// rather than being busy-spun.
pub async fn drain_pending_sync_to_completion(state: &AppState) {
    /// Upper bound on drain passes before deferring to the next
    /// `finish_indexing` caller, guarding against a set/drain livelock (N2/H3).
    const MAX_DRAIN_ITERATIONS: u32 = 64;

    for _ in 0..MAX_DRAIN_ITERATIONS {
        if !state.has_pending_sync() {
            return;
        }
        drain_pending_sync(state).await;
        // Cooperative yield: if the indexing lock is contended, drain_pending_sync
        // re-queues rather than draining, so yield to let the holder finish
        // instead of busy-spinning; also lets a re-armed pending settle.
        tokio::task::yield_now().await;
    }

    if state.has_pending_sync() {
        tracing::warn!(
            max_iterations = MAX_DRAIN_ITERATIONS,
            "drain_pending_sync_to_completion: reached iteration bound with a \
             pending sync still queued; deferring to the next finish_indexing caller"
        );
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
    use std::sync::Arc;

    use sysinfo::System;

    use super::{background_db_hydration, drain_pending_sync_to_completion, get_daemon_status};
    use crate::db::connect_db;
    use crate::db::queries::CodeGraphQueries;
    use crate::models::config::{CodeGraphConfig, WorkspaceConfig};
    use crate::server::state::{AppState, WorkspaceSnapshot};
    use crate::services::code_graph;

    /// Minimal bound-workspace fixture pointing at a scratch tempdir so the
    /// coalesced drain enters its sync branch (and can re-queue on a lost lock).
    fn test_snapshot(tmp: &tempfile::TempDir) -> WorkspaceSnapshot {
        WorkspaceSnapshot {
            workspace_id: "test-ws".to_owned(),
            workspace_uuid: "00000000-0000-0000-0000-000000000000".to_owned(),
            branch: "main".to_owned(),
            data_dir: tmp.path().to_path_buf(),
            path: tmp.path().display().to_string(),
            last_flush: None,
            stale_files: false,
            connection_count: 0,
            file_mtimes: std::collections::HashMap::new(),
        }
    }

    /// 104.001-T (T-leak, RED): a revalidation sync queued while indexing must
    /// not leave its companion bits sticky when the hydration is cancelled. The
    /// pre-fix cancel path releases the indexing lock without clearing the
    /// pending/companion flags, so they leak into a later unrelated sync.
    #[tokio::test]
    async fn hydration_cancel_path_clears_pending_companion_bits() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data_dir = tmp.path().join("data");
        let canonical = tmp.path().join("ws");
        std::fs::create_dir_all(&canonical).expect("create ws dir");
        let state = Arc::new(AppState::new(1));

        // Queue a revalidation sync (companion + pending) as if requested while
        // the daemon was indexing.
        state.set_pending_sync_revalidate();
        state.set_pending_sync();

        // Cancellation is already signalled for this generation, so the first
        // check_cancel! after the DB connects takes the cancel path.
        let (_cancel_tx, cancel_rx) = tokio::sync::watch::channel(true);

        background_db_hydration(
            Arc::clone(&state),
            canonical,
            data_dir,
            "main".to_owned(),
            0,
            cancel_rx,
        )
        .await;

        assert!(
            !state.take_pending_sync(),
            "cancel path must not leave pending_sync set (companion-bit leak)"
        );
        assert!(
            !state.take_pending_sync_revalidate(),
            "cancel path must not leak the pending_sync_revalidate companion bit"
        );
        assert!(
            !state.take_pending_sync_backfill_python(),
            "cancel path must not leak the pending_sync_backfill_python companion bit"
        );
    }

    /// 104.001-T (T-dbfail, RED): the DB-connect-failure path must also clear
    /// the queued pending/companion flags rather than leak them. Reproduced by
    /// pointing `data_dir` at a regular file so `connect_db` fails.
    #[tokio::test]
    async fn hydration_db_connect_failure_clears_pending_companion_bits() {
        let tmp = tempfile::tempdir().expect("tempdir");
        // Point data_dir at a regular FILE so connect_db's create_dir_all fails
        // deterministically, exercising the DB-connect-failure path.
        let data_file = tmp.path().join("not-a-directory");
        std::fs::write(&data_file, b"x").expect("write file");
        let canonical = tmp.path().join("ws");
        std::fs::create_dir_all(&canonical).expect("create ws dir");
        let state = Arc::new(AppState::new(1));

        state.set_pending_sync_backfill_python();
        state.set_pending_sync();

        // Cancellation stays false so hydration reaches connect_db (which fails).
        let (_cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);

        background_db_hydration(
            Arc::clone(&state),
            canonical,
            data_file,
            "main".to_owned(),
            0,
            cancel_rx,
        )
        .await;

        assert!(
            !state.take_pending_sync(),
            "DB-connect-failure path must not leave pending_sync set (companion-bit leak)"
        );
        assert!(
            !state.take_pending_sync_revalidate(),
            "DB-connect-failure path must not leak the revalidate companion bit"
        );
        assert!(
            !state.take_pending_sync_backfill_python(),
            "DB-connect-failure path must not leak the backfill companion bit"
        );
    }

    // ── 105.001-T / R1: generation-scoped pending-sync clear ─────────────────
    //
    // The cancel/DB-fail clears (lifecycle.rs:255/:280) must be scoped to the
    // generation that OWNS the queue, not a whole-queue wipe. In the
    // set_workspace cancel race window the new snapshot is installed (and its
    // generation counter bumped) BEFORE the old hydration is cancelled; a sync
    // against the NEW binding can lose the still-held indexing lock and publish
    // its pending+companion bits, which the OLD generation's clear must NOT
    // erase (source_stash_id B7F52777, PR #297 Copilot thread).

    /// 105.001-T AC1 (RED FIRST) + AC2 — cancel-path variant. Drive the OLD
    /// generation's cancel-path clear (lifecycle.rs:255) while the queue is
    /// owned by a NEWER generation, and assert the newer generation's pending +
    /// both companion bits SURVIVE. Fails pre-fix (whole-queue `store(0)` wipe
    /// erases them); the RED scaffold's unconditional `clear_for_generation`
    /// reproduces that failure. Deterministic — no wall-clock sleeps.
    #[tokio::test]
    async fn r1_cancel_path_preserves_newer_generation_pending_bits() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data_dir = tmp.path().join("data");
        let canonical = tmp.path().join("ws");
        std::fs::create_dir_all(&canonical).expect("create ws dir");
        let state = Arc::new(AppState::new(1));

        // OLD generation begins and captures its cancel receiver (generation 1).
        let (old_gen, old_cancel_rx) = state.begin_scan_generation().await;

        // set_workspace installs the NEW snapshot and cancels the old hydration:
        // a fresh begin_scan_generation bumps to generation 2 and signals `true`
        // on the old cancel channel, so the old task takes its cancel path.
        let (_new_gen, _new_cancel_rx) = state.begin_scan_generation().await;

        // A sync against the NEW binding loses the still-held indexing lock and
        // publishes pending + BOTH companion bits (tagged with generation 2).
        state.publish_pending_sync(true, true);

        // Drive the OLD generation's cancel-path clear (lifecycle.rs:255).
        background_db_hydration(
            Arc::clone(&state),
            canonical,
            data_dir,
            "main".to_owned(),
            old_gen,
            old_cancel_rx,
        )
        .await;

        assert!(
            state.take_pending_sync(),
            "AC1/AC2: newer-generation pending_sync must survive the older generation's cancel-path clear"
        );
        assert!(
            state.take_pending_sync_revalidate(),
            "AC1/AC2: newer-generation --revalidate-code-graph intent must survive the older generation's clear"
        );
        assert!(
            state.take_pending_sync_backfill_python(),
            "AC1/AC2: newer-generation --backfill-python-canonical intent must survive the older generation's clear"
        );
    }

    /// 105.001-T AC1 (RED FIRST) + AC2 — DB-connect-failure-path variant. Same
    /// invariant driven through the `:280` clear: point `data_dir` at a regular
    /// file so `connect_db` fails, and assert the newer generation's bits SURVIVE.
    #[tokio::test]
    async fn r1_db_connect_failure_path_preserves_newer_generation_pending_bits() {
        let tmp = tempfile::tempdir().expect("tempdir");
        // Regular file at the data_dir path → connect_db's create_dir_all fails.
        let data_file = tmp.path().join("not-a-directory");
        std::fs::write(&data_file, b"x").expect("write file");
        let canonical = tmp.path().join("ws");
        std::fs::create_dir_all(&canonical).expect("create ws dir");
        let state = Arc::new(AppState::new(1));

        let (old_gen, old_cancel_rx) = state.begin_scan_generation().await;
        let (_new_gen, _new_cancel_rx) = state.begin_scan_generation().await;

        // NEW binding publishes pending + both companions (generation 2).
        state.publish_pending_sync(true, true);

        // DB-connect-failure fires before any cancel check, driving the :280 clear.
        background_db_hydration(
            Arc::clone(&state),
            canonical,
            data_file,
            "main".to_owned(),
            old_gen,
            old_cancel_rx,
        )
        .await;

        assert!(
            state.take_pending_sync(),
            "AC1/AC2: newer-generation pending_sync must survive the older generation's DB-fail clear"
        );
        assert!(
            state.take_pending_sync_revalidate(),
            "AC1/AC2: newer-generation revalidate intent must survive the older generation's DB-fail clear"
        );
        assert!(
            state.take_pending_sync_backfill_python(),
            "AC1/AC2: newer-generation backfill intent must survive the older generation's DB-fail clear"
        );
    }

    /// 105.001-T AC4 (RED FIRST) — no false heavy sync. An OLD generation's
    /// stale `--revalidate` / `--backfill-python` companion bits must NOT leak
    /// into a NEWER generation's routine sync: a newer-generation publish
    /// REPLACES the older generation's bits rather than OR-ing into them. Fails
    /// under the RED scaffold's plain sticky-OR `publish`.
    #[tokio::test]
    async fn r1_older_generation_companion_does_not_leak_into_newer_routine_sync() {
        let state = Arc::new(AppState::new(1));

        // OLD generation (1) queues a revalidation + backfill sync.
        let (_old_gen, _old_rx) = state.begin_scan_generation().await;
        state.publish_pending_sync(true, true);

        // NEW generation (2) binds and queues a ROUTINE sync (no companions).
        let (_new_gen, _new_rx) = state.begin_scan_generation().await;
        state.publish_pending_sync(false, false);

        assert!(
            state.take_pending_sync(),
            "the newer generation's routine sync is still queued"
        );
        assert!(
            !state.take_pending_sync_revalidate(),
            "AC4: an older generation's --revalidate-code-graph must not leak into the newer generation's routine sync"
        );
        assert!(
            !state.take_pending_sync_backfill_python(),
            "AC4: an older generation's --backfill-python-canonical must not leak into the newer generation's routine sync"
        );
    }

    /// 105.001-T AC3 — preserve the 104-F publish-order atomicity invariant. A
    /// same-generation publish sets pending + companions as an indivisible unit,
    /// and a same-generation clear removes them as an indivisible unit, so a
    /// concurrent drain never observes `pending_sync == true` with a
    /// stale/missing companion bit. The single mutex on `{generation, flags}`
    /// makes the update all-or-nothing.
    #[tokio::test]
    async fn r1_same_generation_publish_and_clear_are_all_or_nothing() {
        let state = Arc::new(AppState::new(1));
        let (owner_gen, _rx) = state.begin_scan_generation().await;

        // Publish sets pending + both companions together.
        state.publish_pending_sync(true, true);
        assert!(
            state.has_pending_sync(),
            "AC3: pending must be visible after an atomic publish"
        );

        // A same-generation clear removes the whole owned queue as a unit.
        state.clear_pending_sync_for_generation(owner_gen);
        assert!(
            !state.take_pending_sync(),
            "AC3: same-generation clear removes pending atomically"
        );
        assert!(
            !state.take_pending_sync_revalidate(),
            "AC3: same-generation clear removes the revalidate companion atomically"
        );
        assert!(
            !state.take_pending_sync_backfill_python(),
            "AC3: same-generation clear removes the backfill companion atomically"
        );
    }

    // ── 105.002-T / R2: producer/consumer drain handoff (close the TOCTOU) ────
    //
    // The bounded snapshot-loop drain cannot close the window where a sync caller
    // fails try_start_indexing while a holder owns the lock, is descheduled
    // BEFORE publishing pending_sync, and resumes only AFTER the holder's final
    // has_pending_sync peek returned false — stranding the request until an
    // external index/sync/watcher tick. The fix is an atomic producer->lock-holder
    // handoff: publish_pending_sync_and_try_reacquire re-attempts the lock after
    // publishing; acquiring it makes the producer the guaranteed finisher
    // (source_stash_id 0B5AAAD2, PR #297 Copilot thread).

    /// 105.002-T AC1 (RED FIRST) + AC2 — intent-after-final-peek. Reproduce the
    /// TOCTOU: a holder acquired, drained, released, and ran its final
    /// has_pending peek (nothing queued) then exited; the descheduled producer
    /// resumes and publishes intent only NOW. The backstop must re-acquire the
    /// lock so a finisher is guaranteed WITHOUT an external tick. Fails under the
    /// RED scaffold (publish-only, no re-attempt → returns false → no finisher).
    #[tokio::test]
    async fn r2_backstop_guarantees_finisher_when_intent_lands_after_final_peek() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let state = Arc::new(AppState::new(1));
        state
            .set_workspace(test_snapshot(&tmp))
            .await
            .expect("set workspace");
        state
            .set_workspace_config(Some(WorkspaceConfig::default()))
            .await;

        // A holder acquired the lock, did its work, released, and ran its final
        // drain-check (nothing queued) then exited — the lock is now free & empty.
        assert!(
            state.try_start_indexing(),
            "holder acquires the indexing lock"
        );
        state.finish_indexing().await; // holder releases
        assert!(
            !state.has_pending_sync(),
            "holder's final peek observes an empty queue → holder exits"
        );

        // The descheduled producer resumes and publishes intent + backstops.
        let must_drain = state.publish_pending_sync_and_try_reacquire(false, false);
        assert!(
            must_drain,
            "AC1: a publish landing after the holder exited must re-acquire the lock so a finisher is guaranteed (no lost wakeup)"
        );
        assert!(
            state.is_indexing(),
            "the backstop must hold the re-acquired indexing lock"
        );

        // The guaranteed finisher drains the queued request with no external tick.
        state.finish_indexing().await;
        drain_pending_sync_to_completion(&state).await;
        assert!(
            !state.has_pending_sync(),
            "AC2: the backstopped request is guaranteed drained"
        );
        assert!(
            !state.is_indexing(),
            "the drain releases the indexing lock on completion"
        );
    }

    /// 105.002-T AC4 — no lost-wakeup: BOTH interleavings + the R1 generation
    /// seam. (1) intent-BEFORE-release: while a holder still holds the lock, the
    /// backstop must NOT acquire (returns false) and the holder drains on release.
    /// (2) generation seam: a backstopped request tagged with a newer generation
    /// is NOT wiped by an older generation's cancel-clear (composes with R1).
    #[tokio::test]
    async fn r2_backstop_defers_to_active_holder_and_is_generation_aware() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let state = Arc::new(AppState::new(1));
        state
            .set_workspace(test_snapshot(&tmp))
            .await
            .expect("set workspace");
        state
            .set_workspace_config(Some(WorkspaceConfig::default()))
            .await;

        // (1) intent-before-release: a holder currently holds the lock.
        assert!(state.try_start_indexing(), "holder holds the indexing lock");
        let must_drain = state.publish_pending_sync_and_try_reacquire(false, false);
        assert!(
            !must_drain,
            "AC4: while a holder still holds the lock, the backstop must NOT acquire — the holder drains the queued intent on release"
        );
        assert!(
            state.has_pending_sync(),
            "the intent is queued for the current holder"
        );

        // The holder releases and drains the queued intent (no external tick).
        state.finish_indexing().await;
        drain_pending_sync_to_completion(&state).await;
        assert!(
            !state.has_pending_sync(),
            "the holder drains the backstopped intent on release"
        );

        // (2) generation seam with R1: a newer-generation backstopped request
        // must survive an older generation's cancel-clear.
        let (old_gen, _old_rx) = state.begin_scan_generation().await;
        let (_new_gen, _new_rx) = state.begin_scan_generation().await;
        let _ = state.publish_pending_sync_and_try_reacquire(false, false); // tagged new gen
        state.clear_pending_sync_for_generation(old_gen); // older-generation clear
        assert!(
            state.has_pending_sync(),
            "AC4 seam with R1: an older generation's clear must not wipe a newer-generation backstopped request"
        );
        // Release the lock if the backstop acquired it.
        if state.is_indexing() {
            state.finish_indexing().await;
        }
    }

    /// 104.001-T (T-loop, RED): a `pending_sync` re-armed during a drain (here
    /// the lost-lock re-queue) must be self-drained by the loop wrapper without
    /// relying on an external `finish_indexing` caller. Deterministic via the
    /// current-thread runtime + explicit yields (no wall-clock sleeps): the test
    /// holds the indexing lock so the first drain pass loses the race and
    /// re-queues, then releases it so a *looping* drain can finish the work. The
    /// single-shot placeholder stalls here (fails); the 104.002-T loop drains it.
    #[tokio::test]
    async fn loop_drain_self_drains_pending_rearmed_during_a_drain() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let state = Arc::new(AppState::new(1));

        // Bind a workspace + config so the drain enters its sync branch and, on
        // a lost indexing-lock race, re-queues pending.
        state
            .set_workspace(test_snapshot(&tmp))
            .await
            .expect("set workspace");
        state
            .set_workspace_config(Some(WorkspaceConfig::default()))
            .await;

        // Hold the indexing lock so the first drain pass loses the race and
        // re-queues the pending sync (re-arm-during-drain, deterministic).
        assert!(state.try_start_indexing(), "test holds the indexing lock");
        state.set_pending_sync();

        let drain_state = Arc::clone(&state);
        let drain = tokio::spawn(async move {
            drain_pending_sync_to_completion(&drain_state).await;
        });

        // Let the spawned drain run its first pass: it takes pending, fails to
        // acquire the (held) lock, and re-queues pending.
        tokio::task::yield_now().await;
        assert!(
            state.has_pending_sync(),
            "first drain pass must re-queue pending while the indexing lock is contended"
        );

        // Release the lock; a loop-drain must now self-drain the re-queued
        // pending WITHOUT any external finish_indexing caller.
        state.finish_indexing().await;

        drain.await.expect("drain task joins");

        assert!(
            !state.has_pending_sync(),
            "loop-drain must self-drain the re-queued pending sync to completion"
        );
        assert!(
            !state.is_indexing(),
            "loop-drain must release the indexing lock after draining"
        );
    }

    /// 104.002-T (H3): the loop self-drains after a pending sync is re-armed
    /// *twice* during draining (two lost-lock re-queues), then terminates.
    #[tokio::test]
    async fn loop_drain_self_drains_after_two_rearms() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let state = Arc::new(AppState::new(1));
        state
            .set_workspace(test_snapshot(&tmp))
            .await
            .expect("set workspace");
        state
            .set_workspace_config(Some(WorkspaceConfig::default()))
            .await;

        assert!(state.try_start_indexing(), "test holds the indexing lock");
        state.set_pending_sync();

        let drain_state = Arc::clone(&state);
        let drain = tokio::spawn(async move {
            drain_pending_sync_to_completion(&drain_state).await;
        });

        // Two drain passes while the lock is held, each re-queues pending.
        tokio::task::yield_now().await;
        assert!(state.has_pending_sync(), "first pass must re-queue pending");
        tokio::task::yield_now().await;
        assert!(
            state.has_pending_sync(),
            "second pass must re-queue pending"
        );

        // Release the lock; the loop must self-drain to completion.
        state.finish_indexing().await;
        drain.await.expect("drain task joins");

        assert!(
            !state.has_pending_sync(),
            "loop-drain must self-drain after two re-arms"
        );
        assert!(!state.is_indexing(), "loop-drain must release the lock");
    }

    /// 104.002-T (H3): when the indexing lock is never released, every drain
    /// pass loses the race and re-queues; the bounded loop must still TERMINATE
    /// (deferring to the lock holder) rather than spin forever.
    #[tokio::test]
    async fn loop_drain_is_bounded_when_lock_is_never_released() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let state = Arc::new(AppState::new(1));
        state
            .set_workspace(test_snapshot(&tmp))
            .await
            .expect("set workspace");
        state
            .set_workspace_config(Some(WorkspaceConfig::default()))
            .await;

        // Hold the indexing lock for the entire drain so every pass re-queues.
        assert!(state.try_start_indexing(), "test holds the indexing lock");
        state.set_pending_sync();

        // Must return (bounded) despite pending never clearing — a hang here
        // would fail the test via the harness timeout.
        drain_pending_sync_to_completion(&state).await;

        assert!(
            state.has_pending_sync(),
            "pending remains queued for the externally-held lock holder to drain"
        );
        assert!(
            state.is_indexing(),
            "the externally-held indexing lock must be left untouched"
        );
    }

    // ── 099.007-T: a queued MCP sync carrying backfill_python_canonical=true
    // must actually RUN the canonical backfill when it drains ────────────────
    //
    // When `sync_workspace({backfill_python_canonical:true})` arrives while the
    // daemon is already indexing, write.rs coalesces it via
    // `publish_pending_sync(revalidate, backfill_python)` and returns "queued".
    // The drain must carry that intent into a GATED
    // `sync_workspace_with_progress(.., backfill_python, ..)` — not downgrade to
    // a routine (ungated) sync, which would defer the stale-marker migration
    // (C12-5 no-op) and silently drop the requested backfill. The production
    // carry-through landed in 101.002-T (drain took `backfill_python` instead of
    // a hardcoded `false`); this is its missing END-TO-END regression guard.
    //
    // RED-capable: reverting the drain's `backfill_python` argument to `false`
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

        // Exactly what write.rs::sync_workspace publishes for a queued
        // `sync_workspace({backfill_python_canonical:true})` while indexing runs.
        state.publish_pending_sync(false, true);

        // Drain: the coalesced sync must run the GATED backfill.
        drain_pending_sync_to_completion(&state).await;

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
        assert!(
            !state.has_pending_sync(),
            "the drain must fully consume the queued pending sync"
        );
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
