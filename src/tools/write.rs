use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;

use chrono::Utc;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::config::StaleStrategy;
use crate::db::connect_db;
#[cfg(feature = "git-graph")]
use crate::db::queries::CodeGraphQueries;
use crate::db::workspace::{resolve_git_branch, workspace_hash};
use crate::errors::{CodeGraphError, EngramError, SystemError, WorkspaceError};
use crate::models::config::CodeGraphConfig;
use crate::models::health::ScanProgress;
use crate::server::state::SharedState;
use crate::services::dehydration;
use crate::services::hydration;
use crate::tools::lifecycle::drain_pending_sync;

#[cfg(feature = "git-graph")]
async fn workspace_path(state: &SharedState) -> Result<PathBuf, EngramError> {
    if let Some(snapshot) = state.snapshot_workspace().await {
        return Ok(PathBuf::from(snapshot.path));
    }
    Err(EngramError::Workspace(WorkspaceError::NotSet))
}

pub async fn flush_state(state: SharedState, params: Option<Value>) -> Result<Value, EngramError> {
    // FR-153: Reject flush while indexing — code graph may be in inconsistent state
    if state.is_indexing() {
        return Err(EngramError::CodeGraph(CodeGraphError::IndexInProgress));
    }

    // T092: Acquire per-workspace write lock for FIFO serialization of concurrent flushes
    let _flush_guard = dehydration::acquire_flush_lock().await;
    let snapshot = state
        .snapshot_workspace()
        .await
        .ok_or(EngramError::Workspace(WorkspaceError::NotSet))?;

    let path = PathBuf::from(&snapshot.path);
    let data_dir = snapshot.data_dir.clone();
    let branch = snapshot.branch.clone();
    let engram_dir = path.join(".engram");
    let stale_strategy = state.stale_strategy();
    let mut warnings: Vec<String> = Vec::new();
    let is_stale =
        snapshot.stale_files || hydration::detect_stale_since(&snapshot.file_mtimes, &engram_dir);

    let _ = params;

    let db = connect_db(&data_dir, &branch).await?;
    let cg_queries = crate::db::queries::CodeGraphQueries::new(db.clone());

    // Determine staleness action from strategy before touching the DB
    match (is_stale, stale_strategy) {
        (true, StaleStrategy::Fail) => {
            return Err(EngramError::Hydration(
                crate::errors::HydrationError::StaleWorkspace,
            ));
        }
        (true, StaleStrategy::Warn) => {
            warnings.push("2004 StaleWorkspace: .engram files modified externally".to_string());
        }
        (true, StaleStrategy::Rehydrate) => {
            hydration::hydrate_code_graph(&path, Path::new(&data_dir), &branch, &cg_queries)
                .await?;
        }
        (false, _) => {}
    }

    // Code graph serialization (FR-132, FR-133, FR-134)
    let cg_result =
        dehydration::dehydrate_code_graph(&cg_queries, Path::new(&data_dir), &branch).await?;
    let metrics_written =
        match crate::services::metrics::compute_and_write_summary(&path, &branch).await {
            Ok(()) => true,
            Err(crate::errors::EngramError::Metrics(crate::errors::MetricsError::NotFound {
                ..
            })) => false, // no usage events yet — skip summary
            Err(error) => return Err(error),
        };
    let flush_timestamp = chrono::Utc::now().to_rfc3339();

    let mut all_files = cg_result.files_written.clone();
    if metrics_written {
        all_files.push(format!(".engram/metrics/{branch}/summary.json"));
    }

    let new_mtimes = hydration::collect_file_mtimes(&engram_dir);

    let _ = state
        .update_workspace(|ws| {
            ws.last_flush = Some(flush_timestamp.clone());
            ws.stale_files = false;
            ws.file_mtimes = new_mtimes;
        })
        .await;

    Ok(json!({
        "files_written": all_files,
        "warnings": warnings,
        "flush_timestamp": flush_timestamp,
        "code_graph": {
            "nodes_written": cg_result.nodes_written,
            "edges_written": cg_result.edges_written,
        },
    }))
}

// ── index_workspace ─────────────────────────────────────────────────

#[derive(Deserialize)]
struct IndexWorkspaceParams {
    #[serde(default)]
    force: bool,
}

/// Parameters for the `sync_workspace` MCP tool.
///
/// `backfill_python_canonical` gates the T7 rollout backfill (096.010-T): when
/// `true` and the durable Python extraction-version marker is stale, every
/// indexed `.py` file is force re-extracted and the canonical post-pass runs to
/// materialize the upgraded cross-module edges. Defaults to `false` so routine
/// auto-sync never silently re-extracts or churns canonical edges (C12-5).
///
/// `revalidate_code_graph` gates the 101-F code-graph extraction-generation
/// backfill: when `true` and the durable generation marker is stale, every
/// indexed file is force re-extracted so the 100-F fail-closed same-file guard
/// re-runs over stale wrong same-file direct edges persisted before the fix.
/// Defaults to `false` (a stale generation is a no-op deferral on routine sync).
#[derive(Deserialize, Default)]
struct SyncWorkspaceParams {
    #[serde(default)]
    backfill_python_canonical: bool,
    #[serde(default)]
    revalidate_code_graph: bool,
}

/// Parse all supported source files and populate the code knowledge graph.
///
/// Returns a structured summary of files parsed, symbols indexed, edges
/// created, and any per-file errors encountered.
pub async fn index_workspace(
    state: SharedState,
    params: Option<Value>,
) -> Result<Value, EngramError> {
    // 092.004-T: atomic (workspace, config) capture via the shared graph-handler
    // seam so the `code_graph` config used for indexing cannot tear away from the
    // workspace path/data_dir/branch under a concurrent bind.
    let ctx = crate::tools::snapshot_graph_handler_context(&state).await?;
    let ws_path = PathBuf::from(&ctx.workspace.path);
    let data_dir = ctx.workspace.data_dir.clone();
    let branch = ctx.workspace.branch.clone();
    let code_graph = ctx.config.code_graph.clone();

    // Reject if indexing is already running.
    if !state.try_start_indexing() {
        return Err(EngramError::CodeGraph(CodeGraphError::IndexInProgress));
    }

    begin_indexing_scan_progress(&state).await;

    // Run the indexing logic, ensuring the flag is cleared on all exit paths.
    let result =
        index_workspace_inner(&state, &ws_path, &data_dir, &branch, code_graph, params).await;
    finalize_indexing_request(&state, &result, true, |state| {
        Box::pin(drain_pending_sync(state))
    })
    .await;
    result
}

/// Inner indexing logic separated to guarantee `finish_indexing()` runs.
async fn index_workspace_inner(
    state: &SharedState,
    ws_path: &std::path::Path,
    data_dir: &std::path::Path,
    branch: &str,
    config: CodeGraphConfig,
    params: Option<Value>,
) -> Result<Value, EngramError> {
    let parsed: IndexWorkspaceParams = serde_json::from_value(params.unwrap_or_else(|| json!({})))
        .map_err(|e| {
            EngramError::System(SystemError::InvalidParams {
                reason: e.to_string(),
            })
        })?;

    let last_completed_at = state
        .scan_progress_snapshot()
        .await
        .and_then(|progress| progress.last_completed_at);
    let (progress_tx, progress_task) = spawn_scan_progress_updater(state.clone());
    let result = {
        let mut progress_callback = move |files_scanned, files_total| {
            let _ = progress_tx.send(running_scan_progress(
                files_scanned,
                files_total,
                last_completed_at.clone(),
            ));
        };
        crate::services::code_graph::index_workspace_with_progress(
            ws_path,
            data_dir,
            branch,
            &config,
            parsed.force,
            Some(&mut progress_callback),
        )
        .await
    };
    progress_task.await.map_err(|e| {
        EngramError::System(SystemError::DatabaseError {
            reason: format!("scan progress updater failed: {e}"),
        })
    })?;
    let result = result?;

    serde_json::to_value(result).map_err(|e| {
        EngramError::System(SystemError::DatabaseError {
            reason: format!("result serialization failed: {e}"),
        })
    })
}

// ── sync_workspace (T045) ───────────────────────────────────────────

/// Detect changed, added, and deleted files since the last index and
/// update only affected nodes in the code graph.
///
/// Uses two-level hashing (file-level `content_hash` then per-symbol
/// `body_hash`) to minimise re-embedding. Preserves `concerns` edges
/// across file moves via hash-resilient identity matching (FR-124).
pub async fn sync_workspace(
    state: SharedState,
    params: Option<Value>,
) -> Result<Value, EngramError> {
    // 092.004-T: atomic (workspace, config) capture via the shared graph-handler
    // seam (see index_workspace). The branch-resolution block below may re-point
    // the active workspace branch, but the captured `code_graph` is unaffected.
    let ctx = crate::tools::snapshot_graph_handler_context(&state).await?;
    let ws_path = PathBuf::from(&ctx.workspace.path);
    let data_dir = ctx.workspace.data_dir.clone();
    let mut branch = ctx.workspace.branch.clone();
    let code_graph = ctx.config.code_graph.clone();

    if let Ok(resolved_branch) = resolve_git_branch(&ws_path) {
        if resolved_branch != branch {
            let workspace_id = workspace_hash(&ws_path, &resolved_branch);
            let metrics_branch = resolved_branch.clone();
            let snapshot_branch = resolved_branch.clone();
            let _ = state
                .update_workspace(|ws| {
                    ws.branch = snapshot_branch;
                    ws.workspace_id = workspace_id;
                })
                .await;
            crate::services::metrics::switch_branch(metrics_branch);
            branch = resolved_branch;
        }
    }

    // If indexing is already running, queue a sync to run after it finishes
    // rather than returning an error — callers get a "queued" status (044.004-T).
    if !state.try_start_indexing() {
        // 101.002-T: preserve the queued sync's gate flags across coalescing,
        // and publish them BEFORE set_pending_sync() so a concurrent drain can
        // never observe pending_sync == true alongside a stale companion bit
        // (which would downgrade the request to a routine no-op and strand the
        // sticky bit). The queue decision runs before params are parsed, and the
        // coalesced drain would otherwise pass both gates as false — silently
        // dropping a queued --revalidate-code-graph or --backfill-python-canonical.
        let params_ref = params.as_ref();
        let queued_flag = |name: &str| -> bool {
            params_ref
                .and_then(|p| p.get(name))
                .and_then(Value::as_bool)
                .unwrap_or(false)
        };
        if queued_flag("revalidate_code_graph") {
            state.set_pending_sync_revalidate();
        }
        if queued_flag("backfill_python_canonical") {
            state.set_pending_sync_backfill_python();
        }
        state.set_pending_sync();
        return Ok(
            json!({ "status": "queued", "message": "Sync queued; will run after current indexing completes" }),
        );
    }

    begin_indexing_scan_progress(&state).await;

    // Run the sync logic, ensuring the flag is cleared on all exit paths.
    let result =
        sync_workspace_inner(&state, &ws_path, &data_dir, &branch, code_graph, params).await;
    finalize_indexing_request(&state, &result, false, |state| {
        Box::pin(drain_pending_sync(state))
    })
    .await;
    result
}

/// Inner sync logic separated to guarantee `finish_indexing()` runs.
async fn sync_workspace_inner(
    state: &SharedState,
    ws_path: &std::path::Path,
    data_dir: &std::path::Path,
    branch: &str,
    config: CodeGraphConfig,
    params: Option<Value>,
) -> Result<Value, EngramError> {
    let parsed: SyncWorkspaceParams = serde_json::from_value(params.unwrap_or_else(|| json!({})))
        .map_err(|e| {
        EngramError::System(SystemError::InvalidParams {
            reason: e.to_string(),
        })
    })?;

    let last_completed_at = state
        .scan_progress_snapshot()
        .await
        .and_then(|progress| progress.last_completed_at);
    let (progress_tx, progress_task) = spawn_scan_progress_updater(state.clone());
    let result = {
        let mut progress_callback = move |files_scanned, files_total| {
            let _ = progress_tx.send(running_scan_progress(
                files_scanned,
                files_total,
                last_completed_at.clone(),
            ));
        };
        crate::services::code_graph::sync_workspace_with_progress(
            ws_path,
            data_dir,
            branch,
            &config,
            parsed.backfill_python_canonical,
            parsed.revalidate_code_graph,
            Some(&mut progress_callback),
        )
        .await
    };
    progress_task.await.map_err(|e| {
        EngramError::System(SystemError::DatabaseError {
            reason: format!("scan progress updater failed: {e}"),
        })
    })?;
    let result = result?;

    serde_json::to_value(result).map_err(|e| {
        EngramError::System(SystemError::DatabaseError {
            reason: format!("result serialization failed: {e}"),
        })
    })
}

fn running_scan_progress(
    files_scanned: u64,
    files_total: u64,
    last_completed_at: Option<String>,
) -> ScanProgress {
    ScanProgress {
        running: true,
        files_scanned,
        files_total,
        last_completed_at,
    }
}

fn spawn_scan_progress_updater(
    state: SharedState,
) -> (
    tokio::sync::mpsc::UnboundedSender<ScanProgress>,
    tokio::task::JoinHandle<()>,
) {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let handle = tokio::spawn(async move {
        while let Some(progress) = rx.recv().await {
            state.set_scan_progress(Some(progress)).await;
        }
    });
    (tx, handle)
}

fn indexing_started_progress(last_completed_at: Option<String>) -> ScanProgress {
    running_scan_progress(0, 0, last_completed_at)
}

fn completed_index_scan_progress(result: &Value) -> ScanProgress {
    let total = value_u64(result, "files_parsed") + value_u64(result, "files_skipped");
    completed_scan_progress(total)
}

fn completed_sync_scan_progress(result: &Value) -> ScanProgress {
    let total = value_u64(result, "files_modified")
        + value_u64(result, "files_added")
        + value_u64(result, "files_deleted")
        + value_u64(result, "files_unchanged")
        + value_u64(result, "oversized_files_skipped");
    completed_scan_progress(total)
}

fn completed_scan_progress(total: u64) -> ScanProgress {
    ScanProgress {
        running: false,
        files_scanned: total,
        files_total: total,
        last_completed_at: Some(Utc::now().to_rfc3339()),
    }
}

fn value_u64(result: &Value, field: &str) -> u64 {
    result.get(field).and_then(Value::as_u64).unwrap_or(0)
}

async fn begin_indexing_scan_progress(state: &SharedState) {
    let last_completed_at = state
        .scan_progress_snapshot()
        .await
        .and_then(|progress| progress.last_completed_at);
    state
        .set_scan_progress(Some(indexing_started_progress(last_completed_at)))
        .await;
}

async fn finish_indexing_scan_progress(
    state: &SharedState,
    result: &Result<Value, EngramError>,
    full_index: bool,
) {
    let progress = match result {
        Ok(value) if full_index => completed_index_scan_progress(value),
        Ok(value) => completed_sync_scan_progress(value),
        Err(_) => completed_scan_progress(0),
    };
    state.set_scan_progress(Some(progress)).await;
}

type DrainFuture<'a> = Pin<Box<dyn Future<Output = ()> + Send + 'a>>;

async fn finalize_indexing_request<F>(
    state: &SharedState,
    result: &Result<Value, EngramError>,
    full_index: bool,
    drain: F,
) where
    F: for<'a> FnOnce(&'a SharedState) -> DrainFuture<'a>,
{
    state.finish_indexing().await;
    drain(state).await;
    finish_indexing_scan_progress(state, result, full_index).await;
}

// ── index_git_history (T042) ──────────────────────────────────────────────────

/// Parameters for the `index_git_history` MCP tool.
#[cfg(feature = "git-graph")]
#[derive(serde::Deserialize)]
struct IndexGitHistoryParams {
    /// Number of commits to walk from HEAD (default: 500).
    #[serde(default)]
    depth: Option<u32>,
    /// When true, re-index all commits even if already stored.
    #[serde(default)]
    force: bool,
}

/// Index the workspace's git commit history into the `commit_node` table.
///
/// Requires the `git-graph` feature flag and a workspace that is a valid git
/// repository. Returns a summary of the indexing run.
#[cfg(feature = "git-graph")]
pub async fn index_git_history(
    state: SharedState,
    params: Option<Value>,
) -> Result<Value, EngramError> {
    let ws_path = workspace_path(&state).await?;
    let (data_dir, branch) = {
        let snap = state
            .snapshot_workspace()
            .await
            .ok_or(EngramError::Workspace(WorkspaceError::NotSet))?;
        (snap.data_dir.clone(), snap.branch.clone())
    };

    let parsed: IndexGitHistoryParams = serde_json::from_value(params.unwrap_or_else(|| json!({})))
        .map_err(|e| {
            EngramError::System(SystemError::InvalidParams {
                reason: e.to_string(),
            })
        })?;

    if parsed.depth == Some(0) {
        return Err(EngramError::System(SystemError::InvalidParams {
            reason: "depth must be greater than 0 when provided".to_owned(),
        }));
    }

    let depth = parsed.depth.unwrap_or(0); // None → service uses default 500

    let db = connect_db(&data_dir, &branch).await?;
    let queries = CodeGraphQueries::new(db);

    let summary =
        crate::services::git_graph::index_git_history(&queries, &ws_path, depth, parsed.force)
            .await?;

    serde_json::to_value(&summary).map_err(|e| {
        EngramError::System(SystemError::DatabaseError {
            reason: format!("index_git_history serialization failed: {e}"),
        })
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    use serde_json::json;

    use super::{
        completed_index_scan_progress, completed_sync_scan_progress, finalize_indexing_request,
        indexing_started_progress,
    };
    use crate::server::state::AppState;

    #[test]
    fn indexing_started_progress_sets_running_without_totals() {
        let progress = indexing_started_progress(Some("2026-05-14T00:00:00Z".to_owned()));
        assert!(progress.running, "progress should mark indexing as running");
        assert_eq!(progress.files_scanned, 0);
        assert_eq!(progress.files_total, 0);
        assert_eq!(
            progress.last_completed_at.as_deref(),
            Some("2026-05-14T00:00:00Z")
        );
    }

    #[test]
    fn completed_index_scan_progress_uses_parsed_and_skipped_counts() {
        let progress = completed_index_scan_progress(&json!({
            "files_parsed": 7,
            "files_skipped": 2
        }));
        assert!(
            !progress.running,
            "completed progress should not be running"
        );
        assert_eq!(progress.files_scanned, 9);
        assert_eq!(progress.files_total, 9);
        assert!(progress.last_completed_at.is_some());
    }

    #[test]
    fn completed_sync_scan_progress_uses_all_file_buckets() {
        let progress = completed_sync_scan_progress(&json!({
            "files_modified": 3,
            "files_added": 2,
            "files_deleted": 1,
            "files_unchanged": 4,
            "oversized_files_skipped": 2
        }));
        assert!(!progress.running, "completed sync should not be running");
        assert_eq!(progress.files_scanned, 12);
        assert_eq!(progress.files_total, 12);
        assert!(progress.last_completed_at.is_some());
    }

    #[tokio::test]
    async fn finalize_indexing_request_keeps_progress_running_until_pending_sync_drains() {
        let state = Arc::new(AppState::new(1));
        state
            .set_scan_progress(Some(indexing_started_progress(None)))
            .await;
        assert!(state.try_start_indexing(), "should acquire indexing lock");

        let observed_running = Arc::new(AtomicBool::new(false));
        let observed_running_for_drain = Arc::clone(&observed_running);
        let result = Ok(json!({
            "files_parsed": 2,
            "files_skipped": 1
        }));

        finalize_indexing_request(&state, &result, true, |state| {
            let observed_running = Arc::clone(&observed_running_for_drain);
            Box::pin(async move {
                let progress = state
                    .scan_progress_snapshot()
                    .await
                    .expect("scan progress should be present while draining");
                observed_running.store(progress.running, Ordering::SeqCst);
            })
        })
        .await;

        assert!(
            observed_running.load(Ordering::SeqCst),
            "progress should remain running until pending sync drain completes"
        );
        let final_progress = state
            .scan_progress_snapshot()
            .await
            .expect("completed progress should be recorded");
        assert!(
            !final_progress.running,
            "progress should be marked complete after drain finishes"
        );
        assert_eq!(final_progress.files_scanned, 3);
        assert_eq!(final_progress.files_total, 3);
    }
}
