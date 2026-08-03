#[cfg(test)]
use std::future::Future;
use std::path::{Path, PathBuf};
#[cfg(test)]
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
use crate::server::state::{
    AppState, CompletionOutcome, CoordinatorCell, DispatchSnapshot, DriverTaskGuard, OwnerKind,
    OwnerPermit, RequestOutcome, SharedState, WorkMask,
};
use crate::services::dehydration;
use crate::services::hydration;

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
    let (admission, ctx) = state
        .guarded_dispatch_context()
        .await
        .ok_or(EngramError::Workspace(WorkspaceError::NotSet))?;
    let mut permit =
        match CoordinatorCell::request(admission, WorkMask::from_bits(0b111), OwnerKind::Index) {
            Ok(RequestOutcome::Acquired(permit)) => permit,
            Ok(RequestOutcome::Waiting(_) | RequestOutcome::Enqueued) => {
                return Err(EngramError::CodeGraph(CodeGraphError::IndexInProgress));
            }
            Ok(RequestOutcome::Stale) => {
                return Err(EngramError::System(SystemError::DatabaseError {
                    reason: "index admission became stale during workspace rebind".to_owned(),
                }));
            }
            Err(error) => {
                return Err(EngramError::System(SystemError::DatabaseError {
                    reason: format!("index coordinator admission failed: {error}"),
                }));
            }
        };
    let work_bits = permit.work_bits();

    let operation = async {
        let ws_path = PathBuf::from(&ctx.workspace.path);
        begin_indexing_scan_progress(&state).await;
        let result = run_workspace_write(
            &state,
            &ws_path,
            &ctx.workspace.data_dir,
            &ctx.workspace.branch,
            ctx.config.code_graph.clone(),
            params,
            true,
            work_bits,
        )
        .await;
        finish_workspace_write_scan_progress(&state, &result, true).await;
        result
    };
    let Some(result) = permit.run_until_cancelled(operation).await else {
        return Err(EngramError::System(SystemError::DatabaseError {
            reason: "index cancelled by workspace rebind".to_owned(),
        }));
    };
    let result = result?;
    if result.unfulfilled_work_bits != 0 {
        // Armed Drop atomically republishes the complete owner mask together
        // with any concurrent pending work. A structured per-file error remains
        // part of the successful response contract, but its heavy intent must
        // stay coordinator-owned for a later retry.
        drop(permit);
        return Ok(result.value);
    }
    let _ = state.set_hydration_ready_for_permit(&permit);
    let value = result.value;
    if let CompletionOutcome::Transferred(successor) = CoordinatorCell::complete(permit) {
        drive_transferred_sync(&state, successor, &ctx).await;
    }
    Ok(value)
}

struct WorkspaceWriteOutcome {
    value: Value,
    unfulfilled_work_bits: u8,
}

async fn run_workspace_write(
    state: &SharedState,
    ws_path: &std::path::Path,
    data_dir: &std::path::Path,
    branch: &str,
    config: CodeGraphConfig,
    params: Option<Value>,
    full_index: bool,
    required_work_bits: u8,
) -> Result<WorkspaceWriteOutcome, EngramError> {
    let last_completed_at = state
        .scan_progress_snapshot()
        .await
        .and_then(|progress| progress.last_completed_at);
    let (progress_tx, progress_task) = spawn_scan_progress_updater(
        state.clone(),
        #[cfg(test)]
        None,
    );
    let mut progress_driver = DriverTaskGuard {
        task: Some(progress_task),
    };

    let (value, unfulfilled_work_bits) = if full_index {
        let parsed: IndexWorkspaceParams =
            serde_json::from_value(params.unwrap_or_else(|| json!({}))).map_err(|error| {
                EngramError::System(SystemError::InvalidParams {
                    reason: error.to_string(),
                })
            })?;
        // A full Index owner claims all three work bits. Heavy bits mean every
        // indexed file must be re-extracted; a hash-skipping index cannot
        // certify either durable migration marker.
        let force = parsed.force || required_work_bits & 0b110 != 0;
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
                force,
                Some(&mut progress_callback),
            )
            .await
        }?;
        let unfulfilled_work_bits = unfulfilled_work_bits(&result.errors, required_work_bits);
        (serde_json::to_value(result), unfulfilled_work_bits)
    } else {
        let mut parsed: SyncWorkspaceParams =
            serde_json::from_value(params.unwrap_or_else(|| json!({}))).map_err(|error| {
                EngramError::System(SystemError::InvalidParams {
                    reason: error.to_string(),
                })
            })?;
        parsed.revalidate_code_graph |= required_work_bits & 0b010 != 0;
        parsed.backfill_python_canonical |= required_work_bits & 0b100 != 0;
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
        }?;
        let unfulfilled_work_bits = unfulfilled_work_bits(&result.errors, required_work_bits);
        (serde_json::to_value(result), unfulfilled_work_bits)
    };
    let value = value.map_err(|error| {
        EngramError::System(SystemError::DatabaseError {
            reason: format!("result serialization failed: {error}"),
        })
    })?;

    let progress_result = match progress_driver.task.as_mut() {
        Some(task) => task.await,
        None => Ok(()),
    };
    let _ = progress_driver.task.take();
    progress_result.map_err(|error| {
        EngramError::System(SystemError::DatabaseError {
            reason: format!("scan progress updater failed: {error}"),
        })
    })?;
    Ok(WorkspaceWriteOutcome {
        value,
        unfulfilled_work_bits,
    })
}

fn unfulfilled_work_bits(
    errors: &[crate::services::code_graph::FileError],
    required_work_bits: u8,
) -> u8 {
    let revalidation = if required_work_bits & 0b010 != 0 && !errors.is_empty() {
        0b010
    } else {
        0
    };
    let python_backfill = if required_work_bits & 0b100 != 0
        && errors.iter().any(|error| {
            Path::new(&error.file)
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("py"))
        }) {
        0b100
    } else {
        0
    };
    revalidation | python_backfill
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
    let params_ref = params.as_ref();
    let requested = WorkMask::from_bits(
        0b001
            | if params_ref
                .and_then(|value| value.get("revalidate_code_graph"))
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                0b010
            } else {
                0
            }
            | if params_ref
                .and_then(|value| value.get("backfill_python_canonical"))
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                0b100
            } else {
                0
            },
    );
    let mut guarded = state
        .guarded_dispatch_context()
        .await
        .ok_or(EngramError::Workspace(WorkspaceError::NotSet))?;
    let mut metrics_branch = None;
    loop {
        let (admission, ctx) = guarded;
        let mut permit = match CoordinatorCell::request(admission, requested, OwnerKind::Sync) {
            Ok(RequestOutcome::Acquired(permit)) => permit,
            Ok(RequestOutcome::Enqueued) => {
                return Ok(
                    json!({ "status": "queued", "message": "Sync queued; will run after current indexing completes" }),
                );
            }
            Ok(RequestOutcome::Waiting(_)) => {
                return Err(EngramError::System(SystemError::DatabaseError {
                    reason: "non-empty sync request unexpectedly entered waiting admission"
                        .to_owned(),
                }));
            }
            Ok(RequestOutcome::Stale) => {
                return Err(EngramError::System(SystemError::DatabaseError {
                    reason: "sync admission became stale during workspace rebind".to_owned(),
                }));
            }
            Err(error) => {
                return Err(EngramError::System(SystemError::DatabaseError {
                    reason: format!("sync coordinator admission failed: {error}"),
                }));
            }
        };
        if let Some(branch) = metrics_branch.take() {
            crate::services::metrics::switch_branch(branch);
        }
        let work_bits = permit.work_bits();

        enum SyncAttempt {
            BranchChanged(String),
            Finished(Result<WorkspaceWriteOutcome, EngramError>),
        }

        let operation = async {
            let ws_path = PathBuf::from(&ctx.workspace.path);
            if let Ok(resolved_branch) = resolve_git_branch(&ws_path) {
                if resolved_branch != ctx.workspace.branch {
                    return SyncAttempt::BranchChanged(resolved_branch);
                }
            }

            begin_indexing_scan_progress(&state).await;
            let result = run_workspace_write(
                &state,
                &ws_path,
                &ctx.workspace.data_dir,
                &ctx.workspace.branch,
                ctx.config.code_graph.clone(),
                params.clone(),
                false,
                work_bits,
            )
            .await;
            finish_workspace_write_scan_progress(&state, &result, false).await;
            SyncAttempt::Finished(result)
        };
        let Some(attempt) = permit.run_until_cancelled(operation).await else {
            return Err(EngramError::System(SystemError::DatabaseError {
                reason: "sync cancelled by workspace rebind".to_owned(),
            }));
        };

        match attempt {
            SyncAttempt::BranchChanged(branch) => {
                let mut workspace = ctx.workspace;
                workspace.workspace_id = workspace_hash(Path::new(&workspace.path), &branch);
                workspace.branch = branch.clone();
                let (_generation, admission) = state
                    .publish_workspace_generation(workspace.clone(), Some(ctx.config.clone()))
                    .await
                    .map_err(|error| {
                        EngramError::System(SystemError::DatabaseError {
                            reason: format!("sync branch publication failed: {error}"),
                        })
                    })?;
                match CoordinatorCell::request(
                    admission,
                    WorkMask::from_bits(work_bits),
                    OwnerKind::Sync,
                )
                .map_err(|error| {
                    EngramError::System(SystemError::DatabaseError {
                        reason: format!("sync branch reissue failed: {error}"),
                    })
                })? {
                    RequestOutcome::Enqueued => {}
                    RequestOutcome::Acquired(_)
                    | RequestOutcome::Waiting(_)
                    | RequestOutcome::Stale => {
                        return Err(EngramError::System(SystemError::DatabaseError {
                            reason:
                                "sync branch reissue was not retained by the retirement barrier"
                                    .to_owned(),
                        }));
                    }
                }
                if !matches!(
                    CoordinatorCell::complete(permit),
                    CompletionOutcome::RetirementAcknowledged
                ) {
                    return Err(EngramError::System(SystemError::DatabaseError {
                        reason: "sync branch publication lost retirement acknowledgment".to_owned(),
                    }));
                }
                metrics_branch = Some(branch);
                let next_guarded = state.guarded_dispatch_context().await.ok_or_else(|| {
                    EngramError::System(SystemError::DatabaseError {
                        reason: "sync branch publication lost its dispatch context".to_owned(),
                    })
                })?;
                if next_guarded.1.workspace.workspace_uuid != workspace.workspace_uuid
                    || next_guarded.1.workspace.workspace_id != workspace.workspace_id
                {
                    return Err(EngramError::System(SystemError::DatabaseError {
                        reason: "sync branch publication was superseded by a distinct binding"
                            .to_owned(),
                    }));
                }
                guarded = next_guarded;
            }
            SyncAttempt::Finished(result) => {
                let result = result?;
                if result.unfulfilled_work_bits != 0 {
                    drop(permit);
                    return Ok(result.value);
                }
                let _ = state.set_hydration_ready_for_permit(&permit);
                let value = result.value;
                if let CompletionOutcome::Transferred(successor) = CoordinatorCell::complete(permit)
                {
                    drive_transferred_sync(&state, successor, &ctx).await;
                }
                return Ok(value);
            }
        }
    }
}

/// Re-resolve and, when needed, coherently publish a transferred Sync's branch.
///
/// Branch publication deliberately transfers zero old-binding bits. The saved
/// mask is reissued with the returned new-generation admission while the old
/// permit is still behind its retirement barrier, then the old permit
/// acknowledges quiescence. This is a new-binding request, not an implicit
/// cross-binding transfer.
pub(crate) async fn prepare_transferred_sync(
    state: &AppState,
    permit: OwnerPermit,
    ctx: DispatchSnapshot,
) -> Result<Option<(OwnerPermit, DispatchSnapshot)>, EngramError> {
    let workspace_path = PathBuf::from(&ctx.workspace.path);
    let Ok(resolved_branch) = resolve_git_branch(&workspace_path) else {
        // Non-Git workspaces retain their published default/synthetic branch.
        return Ok(Some((permit, ctx)));
    };
    if resolved_branch == ctx.workspace.branch {
        return Ok(Some((permit, ctx)));
    }

    let work_bits = permit.work_bits();
    let mut workspace = ctx.workspace;
    workspace.workspace_id = workspace_hash(Path::new(&workspace.path), &resolved_branch);
    workspace.branch = resolved_branch;
    let (_generation, admission) = state
        .publish_workspace_generation(workspace.clone(), Some(ctx.config))
        .await
        .map_err(|error| {
            EngramError::System(SystemError::DatabaseError {
                reason: format!("transferred sync branch publication failed: {error}"),
            })
        })?;

    match CoordinatorCell::request(admission, WorkMask::from_bits(work_bits), OwnerKind::Sync)
        .map_err(|error| {
            EngramError::System(SystemError::DatabaseError {
                reason: format!("transferred sync branch reissue failed: {error}"),
            })
        })? {
        RequestOutcome::Enqueued => {}
        RequestOutcome::Acquired(_) | RequestOutcome::Waiting(_) | RequestOutcome::Stale => {
            return Err(EngramError::System(SystemError::DatabaseError {
                reason:
                    "transferred sync branch reissue was not retained by the retirement barrier"
                        .to_owned(),
            }));
        }
    }

    if !matches!(
        CoordinatorCell::complete(permit),
        CompletionOutcome::RetirementAcknowledged
    ) {
        return Err(EngramError::System(SystemError::DatabaseError {
            reason: "transferred sync branch publication lost retirement acknowledgment".to_owned(),
        }));
    }

    let Some((admission, current_ctx)) = state.guarded_dispatch_context().await else {
        return Err(EngramError::System(SystemError::DatabaseError {
            reason: "transferred sync branch publication lost its dispatch context".to_owned(),
        }));
    };
    if current_ctx.workspace.workspace_uuid != workspace.workspace_uuid
        || current_ctx.workspace.workspace_id != workspace.workspace_id
    {
        // A later distinct publication correctly discarded this superseded
        // target's pending bits. Never carry them into an unrelated binding.
        return Ok(None);
    }

    match CoordinatorCell::request(admission, WorkMask::from_bits(work_bits), OwnerKind::Sync)
        .map_err(|error| {
            EngramError::System(SystemError::DatabaseError {
                reason: format!("transferred sync branch reacquisition failed: {error}"),
            })
        })? {
        RequestOutcome::Acquired(permit) => Ok(Some((permit, current_ctx))),
        RequestOutcome::Enqueued | RequestOutcome::Stale => Ok(None),
        RequestOutcome::Waiting(_) => Err(EngramError::System(SystemError::DatabaseError {
            reason: "non-empty transferred sync unexpectedly entered waiting admission".to_owned(),
        })),
    }
}

async fn drive_transferred_sync(state: &SharedState, permit: OwnerPermit, ctx: &DispatchSnapshot) {
    let mut permit = permit;
    let mut current_ctx = ctx.clone();
    loop {
        let prepared = prepare_transferred_sync(state, permit, current_ctx).await;
        let Some((prepared_permit, prepared_ctx)) = (match prepared {
            Ok(prepared) => prepared,
            Err(error) => {
                tracing::warn!(%error, "transferred sync branch preparation failed");
                return;
            }
        }) else {
            return;
        };
        permit = prepared_permit;
        current_ctx = prepared_ctx;
        let work_bits = permit.work_bits();
        let operation = async {
            let ws_path = PathBuf::from(&current_ctx.workspace.path);
            begin_indexing_scan_progress(state).await;
            let result = run_workspace_write(
                state,
                &ws_path,
                &current_ctx.workspace.data_dir,
                &current_ctx.workspace.branch,
                current_ctx.config.code_graph.clone(),
                Some(json!({
                    "revalidate_code_graph": work_bits & 0b010 != 0,
                    "backfill_python_canonical": work_bits & 0b100 != 0
                })),
                false,
                work_bits,
            )
            .await;
            finish_workspace_write_scan_progress(state, &result, false).await;
            result
        };

        match permit.run_until_cancelled(operation).await {
            Some(Ok(result)) => {
                if result.unfulfilled_work_bits != 0 {
                    return;
                }
                let _ = state.set_hydration_ready_for_permit(&permit);
                match CoordinatorCell::complete(permit) {
                    CompletionOutcome::Transferred(successor) => permit = successor,
                    CompletionOutcome::Released
                    | CompletionOutcome::RetirementAcknowledged
                    | CompletionOutcome::SequenceExhausted(_)
                    | CompletionOutcome::Stale => return,
                }
            }
            Some(Err(error)) => {
                tracing::warn!(%error, "transferred sync failed");
                return;
            }
            None => return,
        }
    }
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
    #[cfg(test)] mut probe: Option<ProgressProbe>,
) -> (
    tokio::sync::mpsc::UnboundedSender<ScanProgress>,
    tokio::task::JoinHandle<()>,
) {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let handle = tokio::spawn(async move {
        while let Some(progress) = rx.recv().await {
            #[cfg(test)]
            if let Some(probe) = probe.as_mut() {
                if let Some(entered) = probe.entered.take() {
                    let _ = entered.send(());
                }
                if let Some(release) = probe.release.take() {
                    let _ = release.await;
                }
                probe
                    .writes
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }
            state.set_scan_progress(Some(progress)).await;
        }
    });
    (tx, handle)
}

#[cfg(test)]
struct ProgressProbe {
    entered: Option<tokio::sync::oneshot::Sender<()>>,
    release: Option<tokio::sync::oneshot::Receiver<()>>,
    writes: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    terminated: Option<tokio::sync::oneshot::Sender<()>>,
}

#[cfg(test)]
impl Drop for ProgressProbe {
    fn drop(&mut self) {
        if let Some(terminated) = self.terminated.take() {
            let _ = terminated.send(());
        }
    }
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

async fn finish_workspace_write_scan_progress(
    state: &SharedState,
    result: &Result<WorkspaceWriteOutcome, EngramError>,
    full_index: bool,
) {
    let progress = match result {
        Ok(result) if full_index => completed_index_scan_progress(&result.value),
        Ok(result) => completed_sync_scan_progress(&result.value),
        Err(_) => completed_scan_progress(0),
    };
    state.set_scan_progress(Some(progress)).await;
}

#[cfg(test)]
type DrainFuture<'a> = Pin<Box<dyn Future<Output = ()> + Send + 'a>>;

#[cfg(test)]
async fn finalize_indexing_request<F>(
    state: &SharedState,
    permit: OwnerPermit,
    result: &Result<Value, EngramError>,
    full_index: bool,
    drain: F,
) where
    F: for<'a> FnOnce(&'a SharedState) -> DrainFuture<'a>,
{
    assert!(
        matches!(
            CoordinatorCell::complete(permit),
            CompletionOutcome::Released
        ),
        "fixture owner must complete without pending work"
    );
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
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    use serde_json::json;
    use tokio::sync::oneshot;

    use super::{
        ProgressProbe, completed_index_scan_progress, completed_sync_scan_progress,
        drive_transferred_sync, finalize_indexing_request, index_workspace,
        indexing_started_progress, running_scan_progress, spawn_scan_progress_updater,
        sync_workspace,
    };
    use crate::db::connect_db;
    use crate::db::queries::CodeGraphQueries;
    use crate::models::config::{CodeGraphConfig, WorkspaceConfig};
    use crate::server::state::{
        AppState, CompletionOutcome, CoordinatorCell, DispatchSnapshot, DriverTaskGuard, OwnerKind,
        OwnerPermit, RequestOutcome, WorkMask, WorkspaceSnapshot,
    };
    use crate::services::code_graph;

    #[derive(Clone, Copy)]
    enum WriteExit {
        EarlyReturn,
        EarlyError,
        Cancellation,
        CallerAbort,
    }

    struct DriverCounts {
        active: Arc<AtomicUsize>,
        max_active: Arc<AtomicUsize>,
    }

    fn snapshot(
        _name: &str,
        workspace_uuid: &str,
        workspace_id: &str,
        path: &std::path::Path,
        data_dir: std::path::PathBuf,
    ) -> WorkspaceSnapshot {
        WorkspaceSnapshot {
            workspace_id: workspace_id.to_owned(),
            workspace_uuid: workspace_uuid.to_owned(),
            branch: "main".to_owned(),
            data_dir,
            path: path.to_string_lossy().into_owned(),
            last_flush: None,
            stale_files: false,
            connection_count: 0,
            file_mtimes: HashMap::new(),
        }
    }

    async fn publish(state: &AppState, snapshot: WorkspaceSnapshot) {
        if let Err(error) = state
            .publish_workspace_generation(snapshot, Some(WorkspaceConfig::default()))
            .await
        {
            panic!("unexpected workspace publication error: {error}");
        }
    }

    fn acquired(outcome: RequestOutcome) -> OwnerPermit {
        match outcome {
            RequestOutcome::Acquired(permit) => permit,
            RequestOutcome::Waiting(_) => panic!("expected acquired permit, got waiting"),
            RequestOutcome::Enqueued => panic!("expected acquired permit, got enqueued"),
            RequestOutcome::Stale => panic!("expected acquired permit, got stale"),
        }
    }

    fn request(state: &AppState, mask: WorkMask, kind: OwnerKind) -> RequestOutcome {
        match CoordinatorCell::request(state.coordinator.admission(), mask, kind) {
            Ok(outcome) => outcome,
            Err(error) => panic!("unexpected coordinator request error: {error}"),
        }
    }

    async fn run_guarded_write_probe(
        state: Arc<AppState>,
        mut permit: OwnerPermit,
        exit: WriteExit,
        progress_probe: ProgressProbe,
        driver_entered: oneshot::Sender<()>,
        release_driver: oneshot::Receiver<()>,
        driver_counts: DriverCounts,
    ) -> Result<(), ()> {
        struct ActiveDriver(Arc<AtomicUsize>);

        impl Drop for ActiveDriver {
            fn drop(&mut self) {
                self.0.fetch_sub(1, Ordering::SeqCst);
            }
        }

        let (progress_tx, progress_task) =
            spawn_scan_progress_updater(Arc::clone(&state), Some(progress_probe));
        let progress_driver = DriverTaskGuard {
            task: Some(progress_task),
        };
        let _ = progress_tx.send(running_scan_progress(1, 1, None));
        let operation = async move {
            let active = driver_counts.active.fetch_add(1, Ordering::SeqCst) + 1;
            driver_counts.max_active.fetch_max(active, Ordering::SeqCst);
            let _active = ActiveDriver(driver_counts.active);
            let _ = driver_entered.send(());
            match exit {
                WriteExit::EarlyReturn | WriteExit::EarlyError => {
                    let _ = release_driver.await;
                }
                WriteExit::Cancellation | WriteExit::CallerAbort => {
                    std::future::pending::<()>().await;
                }
            }
            if matches!(exit, WriteExit::EarlyError) {
                Err(())
            } else {
                Ok(())
            }
        };

        let outcome = permit.run_until_cancelled(operation).await;
        drop(progress_tx);
        drop(progress_driver);
        if let Some(result) = outcome {
            result?;
        }
        if matches!(exit, WriteExit::EarlyReturn) {
            let _ = CoordinatorCell::complete(permit);
        }
        Ok(())
    }

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
        let permit = acquired(request(&state, WorkMask::default(), OwnerKind::Index));

        let observed_running = Arc::new(AtomicBool::new(false));
        let observed_running_for_drain = Arc::clone(&observed_running);
        let result = Ok(json!({
            "files_parsed": 2,
            "files_skipped": 1
        }));

        finalize_indexing_request(&state, permit, &result, true, |state| {
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

    #[tokio::test]
    async fn write_owner_rebind_and_exit_matrix_quiesces_driver_and_progress_before_ack() {
        for kind in [OwnerKind::Index, OwnerKind::Sync] {
            for same_binding in [true, false] {
                for exit in [
                    WriteExit::EarlyReturn,
                    WriteExit::EarlyError,
                    WriteExit::Cancellation,
                    WriteExit::CallerAbort,
                ] {
                    let temp = tempfile::tempdir().expect("tempdir");
                    let state = Arc::new(AppState::new(2));
                    publish(
                        &state,
                        snapshot(
                            "old",
                            "uuid-old",
                            "id-old",
                            temp.path(),
                            temp.path().join("old-data"),
                        ),
                    )
                    .await;
                    let permit = acquired(request(&state, WorkMask::from_bits(0b111), kind));
                    let (progress_entered_tx, progress_entered_rx) = oneshot::channel();
                    let (release_progress_tx, release_progress_rx) = oneshot::channel();
                    let (progress_terminated_tx, progress_terminated_rx) = oneshot::channel();
                    let progress_writes = Arc::new(AtomicUsize::new(0));
                    let progress_probe = ProgressProbe {
                        entered: Some(progress_entered_tx),
                        release: Some(release_progress_rx),
                        writes: Arc::clone(&progress_writes),
                        terminated: Some(progress_terminated_tx),
                    };
                    let (driver_entered_tx, driver_entered_rx) = oneshot::channel();
                    let (release_driver_tx, release_driver_rx) = oneshot::channel();
                    let active_db_drivers = Arc::new(AtomicUsize::new(0));
                    let max_active_db_drivers = Arc::new(AtomicUsize::new(0));
                    let caller = tokio::spawn(run_guarded_write_probe(
                        Arc::clone(&state),
                        permit,
                        exit,
                        progress_probe,
                        driver_entered_tx,
                        release_driver_rx,
                        DriverCounts {
                            active: Arc::clone(&active_db_drivers),
                            max_active: Arc::clone(&max_active_db_drivers),
                        },
                    ));
                    progress_entered_rx
                        .await
                        .expect("progress child should enter its blocked update");
                    driver_entered_rx
                        .await
                        .expect("DB-capable operation should enter");

                    if matches!(exit, WriteExit::EarlyReturn | WriteExit::EarlyError) {
                        let _ = release_driver_tx.send(());
                        let result = caller.await.expect("caller task should join");
                        assert_eq!(result.is_err(), matches!(exit, WriteExit::EarlyError));
                    } else {
                        let target = if same_binding {
                            snapshot(
                                "same",
                                "uuid-old",
                                "id-old",
                                temp.path(),
                                temp.path().join("same-data"),
                            )
                        } else {
                            snapshot(
                                "new",
                                "uuid-new",
                                "id-new",
                                temp.path(),
                                temp.path().join("new-data"),
                            )
                        };
                        publish(&state, target).await;
                        if matches!(exit, WriteExit::CallerAbort) {
                            caller.abort();
                            assert!(
                                caller
                                    .await
                                    .expect_err("caller abort should cancel")
                                    .is_cancelled()
                            );
                        } else {
                            assert!(
                                caller.await.expect("caller task should join").is_ok(),
                                "cancellation should be handled"
                            );
                        }
                    }

                    assert_eq!(active_db_drivers.load(Ordering::SeqCst), 0);
                    assert_eq!(max_active_db_drivers.load(Ordering::SeqCst), 1);
                    assert!(state.coordinator.test_is_idle());
                    if matches!(exit, WriteExit::Cancellation | WriteExit::CallerAbort) {
                        assert_eq!(state.coordinator.test_notification_calls(), 1);
                        assert_eq!(
                            state.coordinator.test_pending_bits(),
                            if same_binding { 0b111 } else { 0 }
                        );
                    }

                    let _ = release_progress_tx.send(());
                    progress_terminated_rx
                        .await
                        .expect("progress child should terminate");
                    assert_eq!(
                        progress_writes.load(Ordering::SeqCst),
                        0,
                        "old progress must not write after permit terminal"
                    );
                }
            }
        }
    }

    #[tokio::test]
    async fn busy_sync_publishes_full_mask_before_exact_queued_response() {
        let temp = tempfile::tempdir().expect("tempdir");
        let invalid_data_dir = temp.path().join("not-a-directory");
        std::fs::write(&invalid_data_dir, b"file blocks database directory")
            .expect("create invalid data path");
        let state = Arc::new(AppState::new(1));
        publish(
            &state,
            snapshot(
                "busy",
                "uuid-busy",
                "id-busy",
                temp.path(),
                invalid_data_dir,
            ),
        )
        .await;
        let owner = acquired(request(
            &state,
            WorkMask::from_bits(0b001),
            OwnerKind::Index,
        ));

        let response = sync_workspace(
            Arc::clone(&state),
            Some(json!({
                "revalidate_code_graph": true,
                "backfill_python_canonical": true
            })),
        )
        .await
        .expect("busy sync should queue without touching the database");

        assert_eq!(
            response,
            json!({
                "status": "queued",
                "message": "Sync queued; will run after current indexing completes"
            })
        );
        assert_eq!(state.coordinator.test_pending_bits(), 0b111);
        drop(owner);
    }

    #[tokio::test]
    async fn direct_sync_executes_recovered_backfill_mask_not_only_request_params() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = temp.path().join("workspace");
        let data_dir = temp.path().join("data");
        std::fs::create_dir_all(&workspace).expect("create workspace");
        std::fs::write(
            workspace.join("app.py"),
            "from helper import compute\n\n\ndef run():\n    compute()\n",
        )
        .expect("write app.py");
        std::fs::write(
            workspace.join("helper.py"),
            "def compute():\n    return 1\n",
        )
        .expect("write helper.py");

        let config = CodeGraphConfig::default();
        code_graph::index_workspace(&workspace, &data_dir, "main", &config, false)
            .await
            .expect("seed index");
        {
            let db = connect_db(&data_dir, "main")
                .await
                .expect("connect seed DB");
            let queries = CodeGraphQueries::new(db);
            queries
                .retract_all_calls_resolved_canonical_edges()
                .await
                .expect("retract canonical edges");
            queries
                .set_python_extraction_version("0")
                .await
                .expect("reset extraction marker");
        }

        let state = Arc::new(AppState::new(1));
        publish(
            &state,
            snapshot(
                "recovered",
                "uuid-recovered",
                "id-recovered",
                &workspace,
                data_dir.clone(),
            ),
        )
        .await;
        let abandoned = acquired(request(&state, WorkMask::from_bits(0b101), OwnerKind::Sync));
        drop(abandoned);
        assert_eq!(state.coordinator.test_pending_bits(), 0b101);

        sync_workspace(Arc::clone(&state), None)
            .await
            .expect("direct recovery sync");

        let db = connect_db(&data_dir, "main")
            .await
            .expect("reconnect recovered DB");
        let queries = CodeGraphQueries::new(db);
        assert_eq!(
            queries.python_extraction_version().expect("read marker"),
            Some("1".to_owned()),
            "the acquired permit's recovered backfill bit must augment the direct request"
        );
        assert_eq!(state.coordinator.test_pending_bits(), 0);
    }

    #[tokio::test]
    async fn sync_branch_refresh_rebinds_coordinator_before_writing_new_branch() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = temp.path().join("workspace");
        let data_dir = temp.path().join("data");
        std::fs::create_dir_all(workspace.join(".git")).expect("create git metadata");
        std::fs::write(
            workspace.join(".git").join("HEAD"),
            "ref: refs/heads/main\n",
        )
        .expect("write git HEAD");
        let state = Arc::new(AppState::new(1));
        let mut stale = snapshot(
            "branch-refresh",
            "uuid-branch-refresh",
            "id-stale-branch",
            &workspace,
            data_dir,
        );
        stale.branch = "stale-branch".to_owned();
        publish(&state, stale).await;
        let old_admission = state.coordinator.admission();
        state.set_hydration_ready();
        let recovered = acquired(request(&state, WorkMask::from_bits(0b111), OwnerKind::Sync));
        drop(recovered);

        sync_workspace(Arc::clone(&state), None)
            .await
            .expect("branch-refresh sync");

        let active = state
            .snapshot_workspace()
            .await
            .expect("active branch snapshot");
        assert_eq!(active.branch, "main");
        assert_eq!(
            active.workspace_id,
            super::workspace_hash(&workspace, "main")
        );
        assert!(matches!(
            CoordinatorCell::request(old_admission, WorkMask::from_bits(0b001), OwnerKind::Sync,),
            Ok(RequestOutcome::Stale)
        ));
        assert!(state.coordinator.test_is_idle());
        assert_eq!(state.coordinator.test_pending_bits(), 0);
        assert!(
            state.is_hydration_ready(),
            "successful branch initialization must restore readiness"
        );

        let db = connect_db(&active.data_dir, "main")
            .await
            .expect("connect refreshed branch DB");
        let queries = CodeGraphQueries::new(db);
        assert_eq!(
            queries
                .python_extraction_version()
                .expect("read Python extraction marker"),
            Some("1".to_owned()),
            "recovered backfill intent must be reissued for the new branch"
        );
        assert_eq!(
            queries
                .code_graph_extraction_generation()
                .expect("read code-graph generation"),
            Some("1".to_owned()),
            "recovered revalidation intent must be reissued for the new branch"
        );
    }

    #[tokio::test]
    async fn full_index_fulfills_recovered_heavy_work_before_success() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = temp.path().join("workspace");
        let data_dir = temp.path().join("data");
        std::fs::create_dir_all(&workspace).expect("create workspace");
        std::fs::write(workspace.join("app.py"), "def run():\n    return 1\n")
            .expect("write source");

        let config = CodeGraphConfig::default();
        code_graph::index_workspace(&workspace, &data_dir, "main", &config, false)
            .await
            .expect("seed index");
        {
            let db = connect_db(&data_dir, "main")
                .await
                .expect("connect seed DB");
            let queries = CodeGraphQueries::new(db);
            queries
                .set_python_extraction_version("0")
                .await
                .expect("reset Python marker");
            queries
                .set_code_graph_extraction_generation("0")
                .await
                .expect("reset code-graph marker");
        }

        let state = Arc::new(AppState::new(1));
        publish(
            &state,
            snapshot(
                "full-heavy",
                "uuid-full-heavy",
                "id-full-heavy",
                &workspace,
                data_dir.clone(),
            ),
        )
        .await;
        let recovered = acquired(request(
            &state,
            WorkMask::from_bits(0b111),
            OwnerKind::Index,
        ));
        drop(recovered);

        index_workspace(Arc::clone(&state), None)
            .await
            .expect("full index with recovered heavy work");

        let db = connect_db(&data_dir, "main")
            .await
            .expect("reconnect indexed DB");
        let queries = CodeGraphQueries::new(db);
        assert_eq!(
            queries
                .python_extraction_version()
                .expect("read Python marker"),
            Some("1".to_owned())
        );
        assert_eq!(
            queries
                .code_graph_extraction_generation()
                .expect("read code-graph marker"),
            Some("1".to_owned())
        );
        assert_eq!(state.coordinator.test_pending_bits(), 0);
    }

    #[tokio::test]
    async fn full_index_retains_heavy_work_when_file_errors_prevent_fulfillment() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = temp.path().join("workspace");
        let data_dir = temp.path().join("data");
        std::fs::create_dir_all(&workspace).expect("create workspace");
        std::fs::write(workspace.join("invalid.py"), [0xff]).expect("write invalid UTF-8 source");

        let state = Arc::new(AppState::new(1));
        publish(
            &state,
            snapshot(
                "full-error",
                "uuid-full-error",
                "id-full-error",
                &workspace,
                data_dir,
            ),
        )
        .await;

        let response = index_workspace(Arc::clone(&state), None)
            .await
            .expect("per-file failures remain structured index results");
        assert!(
            response["errors"]
                .as_array()
                .is_some_and(|errors| !errors.is_empty()),
            "fixture must exercise a non-fatal file error"
        );
        assert_eq!(
            state.coordinator.test_pending_bits(),
            0b111,
            "unfulfilled claimed heavy work must remain coordinator-owned"
        );
    }

    #[tokio::test]
    async fn transferred_sync_refreshes_branch_before_any_database_write() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = temp.path().join("workspace");
        let data_dir = temp.path().join("data");
        std::fs::create_dir_all(workspace.join(".git")).expect("create git metadata");
        std::fs::write(
            workspace.join(".git").join("HEAD"),
            "ref: refs/heads/main\n",
        )
        .expect("write git HEAD");

        let state = Arc::new(AppState::new(1));
        let mut stale = snapshot(
            "transferred-branch",
            "uuid-transferred-branch",
            "id-transferred-stale",
            &workspace,
            data_dir.clone(),
        );
        stale.branch = "stale-branch".to_owned();
        publish(&state, stale.clone()).await;
        let owner = acquired(request(&state, WorkMask::default(), OwnerKind::Index));
        assert!(matches!(
            request(&state, WorkMask::from_bits(0b111), OwnerKind::Sync),
            RequestOutcome::Enqueued
        ));
        let successor = match CoordinatorCell::complete(owner) {
            CompletionOutcome::Transferred(successor) => successor,
            CompletionOutcome::Released
            | CompletionOutcome::RetirementAcknowledged
            | CompletionOutcome::SequenceExhausted(_)
            | CompletionOutcome::Stale => panic!("queued full mask must transfer"),
        };
        let context = DispatchSnapshot {
            workspace: stale,
            config: WorkspaceConfig::default(),
        };

        drive_transferred_sync(&state, successor, &context).await;

        let active = state
            .snapshot_workspace()
            .await
            .expect("active branch snapshot");
        assert_eq!(active.branch, "main");
        assert_eq!(
            active.workspace_id,
            super::workspace_hash(&workspace, "main")
        );
        assert!(
            !data_dir
                .join("cozo")
                .join("stale-branch")
                .join("engram.db")
                .exists(),
            "predecessor branch DB must remain untouched"
        );
        assert!(
            data_dir
                .join("cozo")
                .join("main")
                .join("engram.db")
                .exists(),
            "transferred work must initialize the resolved branch DB"
        );
        assert!(state.is_hydration_ready());
    }

    #[tokio::test]
    async fn failed_branch_initialization_does_not_restore_readiness() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = temp.path().join("workspace");
        std::fs::create_dir_all(workspace.join(".git")).expect("create git metadata");
        std::fs::write(
            workspace.join(".git").join("HEAD"),
            "ref: refs/heads/main\n",
        )
        .expect("write git HEAD");
        let invalid_data_dir = temp.path().join("not-a-directory");
        std::fs::write(&invalid_data_dir, b"blocks database directory")
            .expect("write invalid data path");

        let state = Arc::new(AppState::new(1));
        let mut stale = snapshot(
            "failed-branch",
            "uuid-failed-branch",
            "id-failed-stale",
            &workspace,
            invalid_data_dir,
        );
        stale.branch = "stale-branch".to_owned();
        publish(&state, stale).await;
        state.set_hydration_ready();

        assert!(sync_workspace(Arc::clone(&state), None).await.is_err());
        assert!(
            !state.is_hydration_ready(),
            "failed branch initialization must remain in starting state"
        );
    }
}
