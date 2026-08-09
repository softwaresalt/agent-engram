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
use crate::errors::{CodeGraphError, EngramError, MetricsError, SystemError, WorkspaceError};
use crate::models::config::CodeGraphConfig;
use crate::models::health::ScanProgress;
use crate::server::state::{
    ClaimOutcome, CompletionOutcome, CoordinatorCell, DispatchSnapshot, DriverTaskGuard, OwnerKind,
    OwnerPermit, OwnerProgressScope, RequestOutcome, SharedState, WorkMask,
    WorkspacePublicationGuard,
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

#[derive(Clone, Copy, Deserialize)]
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
#[derive(Clone, Copy, Deserialize, Default)]
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
    let parsed: IndexWorkspaceParams = serde_json::from_value(params.unwrap_or_else(|| json!({})))
        .map_err(|error| {
            EngramError::System(SystemError::InvalidParams {
                reason: error.to_string(),
            })
        })?;
    let (admission, ctx) = state
        .guarded_dispatch_context()
        .await
        .ok_or(EngramError::Workspace(WorkspaceError::NotSet))?;
    let requested = WorkMask::from_bits(if parsed.force { 0b111 } else { 0b001 });
    let permit = match CoordinatorCell::request(admission, requested, OwnerKind::Index) {
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
    let mut metrics_control = crate::services::metrics::writer_control_token(
        Path::new(&ctx.workspace.path),
        &ctx.workspace.branch,
    )?;
    let Some((mut permit, ctx)) = prepare_branch_owner_with_control(
        &state,
        permit,
        ctx,
        OwnerKind::Index,
        &mut metrics_control,
    )
    .await?
    else {
        return Err(EngramError::System(SystemError::DatabaseError {
            reason: "index branch preparation was superseded".to_owned(),
        }));
    };
    let work_bits = permit.work_bits();
    let ws_path = PathBuf::from(&ctx.workspace.path);
    let Some(result) = run_guarded_workspace_write(
        &state,
        &mut permit,
        WorkspaceWriteTarget {
            path: &ws_path,
            data_dir: &ctx.workspace.data_dir,
            branch: &ctx.workspace.branch,
            config: &ctx.config.code_graph,
        },
        Some(json!({ "force": parsed.force })),
        true,
        work_bits,
    )
    .await
    else {
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
        drive_transferred_sync(&state, successor, &ctx, metrics_control).await;
    }
    Ok(value)
}

struct WorkspaceWriteOutcome {
    value: Value,
    unfulfilled_work_bits: u8,
}

struct WorkspaceWriteTarget<'a> {
    path: &'a Path,
    data_dir: &'a Path,
    branch: &'a str,
    config: &'a CodeGraphConfig,
}

enum SyncAttempt {
    BranchChanged(String),
    Finished(Result<WorkspaceWriteOutcome, EngramError>),
}

async fn run_workspace_write(
    target: WorkspaceWriteTarget<'_>,
    params: Option<Value>,
    full_index: bool,
    required_work_bits: u8,
    last_completed_at: Option<String>,
    progress_tx: tokio::sync::mpsc::UnboundedSender<ScanProgress>,
) -> Result<WorkspaceWriteOutcome, EngramError> {
    let (value, unfulfilled_work_bits) = if full_index {
        let parsed: IndexWorkspaceParams =
            serde_json::from_value(params.unwrap_or_else(|| json!({}))).map_err(|error| {
                EngramError::System(SystemError::InvalidParams {
                    reason: error.to_string(),
                })
            })?;
        // Recovered heavy bits mean every indexed file must be re-extracted; a
        // hash-skipping index cannot certify either durable migration marker.
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
                target.path,
                target.data_dir,
                target.branch,
                target.config,
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
                target.path,
                target.data_dir,
                target.branch,
                target.config,
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

    Ok(WorkspaceWriteOutcome {
        value,
        unfulfilled_work_bits,
    })
}

async fn run_guarded_workspace_write(
    state: &SharedState,
    permit: &mut OwnerPermit,
    target: WorkspaceWriteTarget<'_>,
    params: Option<Value>,
    full_index: bool,
    required_work_bits: u8,
) -> Option<Result<WorkspaceWriteOutcome, EngramError>> {
    let last_completed_at = state
        .scan_progress_snapshot()
        .await
        .and_then(|progress| progress.last_completed_at);
    let progress_scope = permit.progress_scope()?;
    if !begin_indexing_scan_progress(state, &progress_scope, last_completed_at.clone()).await {
        return None;
    }
    let (progress_tx, progress_task) = spawn_scan_progress_updater(
        state.clone(),
        progress_scope.clone(),
        #[cfg(test)]
        None,
    );
    let progress_driver = DriverTaskGuard {
        task: Some(progress_task),
    };
    let operation = run_workspace_write(
        target,
        params,
        full_index,
        required_work_bits,
        last_completed_at,
        progress_tx,
    );

    let result = permit.run_until_cancelled(operation).await;
    let Some(result) = result else {
        if let Err(error) = progress_driver.abort_and_join().await {
            if !error.is_cancelled() {
                tracing::warn!(%error, "scan progress updater cancellation failed");
            }
        }
        return None;
    };
    if let Err(error) = progress_driver.join().await {
        return Some(Err(EngramError::System(SystemError::DatabaseError {
            reason: format!("scan progress updater failed: {error}"),
        })));
    }
    if !finish_workspace_write_scan_progress(state, &progress_scope, &result, full_index).await {
        return None;
    }
    Some(result)
}

pub(crate) fn unfulfilled_work_bits(
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

const DETACHED_BRANCH_ROLLBACK_ATTEMPTS: usize = 3;
const DETACHED_BRANCH_ROLLBACK_RETRY_DELAY: std::time::Duration =
    std::time::Duration::from_millis(100);
const BRANCH_ROLLBACK_DIAGNOSTIC_CHARS: usize = 512;

fn bounded_branch_rollback_diagnostic(error: &impl std::fmt::Display) -> String {
    error
        .to_string()
        .chars()
        .take(BRANCH_ROLLBACK_DIAGNOSTIC_CHARS)
        .collect()
}

/// Owned rollback state kept armed across the awaited metrics switch.
struct BranchRefreshRollback {
    state: SharedState,
    publication: WorkspacePublicationGuard,
    prior: DispatchSnapshot,
    metrics_control: crate::services::metrics::WriterControlToken,
    permit: Option<OwnerPermit>,
    prior_hydration_ready: bool,
    restored_generation: Option<u64>,
}

impl BranchRefreshRollback {
    async fn run(&mut self) -> Result<(), EngramError> {
        let (restored_generation, restored_admission) = self
            .state
            .rollback_workspace_generation_guarded(
                &self.publication,
                self.prior.workspace.clone(),
                Some(self.prior.config.clone()),
            )
            .await
            .map_err(|error| {
                EngramError::System(SystemError::DatabaseError {
                    reason: format!("branch refresh rollback publication failed: {error}"),
                })
            })?;
        drop(restored_admission);
        self.restored_generation = Some(restored_generation);

        crate::services::metrics::switch_branch_for(
            &mut self.metrics_control,
            self.prior.workspace.branch.clone(),
        )
        .await?;
        if self.prior_hydration_ready {
            let _ = self
                .state
                .set_hydration_ready_for_generation(restored_generation);
        }
        let permit = self.permit.take().ok_or_else(|| {
            EngramError::System(SystemError::DatabaseError {
                reason: "branch refresh rollback owner was already consumed".to_owned(),
            })
        })?;
        // Abandoning the failed-refresh owner acknowledges its retirement and
        // atomically requeues the barrier's exact deferred mask.
        drop(permit);
        Ok(())
    }

    async fn recover_terminal(mut self, attempts: usize, last_error: &str) {
        let publication_error = match self
            .state
            .rollback_workspace_generation_guarded(
                &self.publication,
                self.prior.workspace.clone(),
                Some(self.prior.config.clone()),
            )
            .await
        {
            Ok((generation, admission)) => {
                drop(admission);
                self.restored_generation = Some(generation);
                None
            }
            Err(error) => Some(bounded_branch_rollback_diagnostic(&error)),
        };

        let (metrics_outcome, metrics_error) = if self.restored_generation.is_some() {
            match crate::services::metrics::initialize(
                Path::new(&self.prior.workspace.path),
                &self.prior.workspace.branch,
                &self.prior.config.metrics,
            )
            .await
            {
                Ok(()) => ("prior_writer_reinitialized", None),
                Err(error) => {
                    let diagnostic = bounded_branch_rollback_diagnostic(&error);
                    if let Err(unavailable_error) =
                        crate::services::metrics::mark_writer_unavailable()
                    {
                        tracing::error!(
                            error = %bounded_branch_rollback_diagnostic(&unavailable_error),
                            "terminal branch rollback could not mark metrics unavailable"
                        );
                    }
                    ("unavailable", Some(diagnostic))
                }
            }
        } else {
            let diagnostic = match crate::services::metrics::mark_writer_unavailable() {
                Ok(()) => {
                    "prior workspace was not republished; metrics marked unavailable".to_owned()
                }
                Err(error) => bounded_branch_rollback_diagnostic(&error),
            };
            ("unavailable", Some(diagnostic))
        };

        if self.prior_hydration_ready {
            if let Some(generation) = self.restored_generation {
                let _ = self.state.set_hydration_ready_for_generation(generation);
            }
        }

        let owner_reissued = self.permit.is_some();
        drop(self.permit.take());
        tracing::error!(
            attempts,
            last_error,
            publication_error = publication_error.as_deref().unwrap_or("none"),
            metrics_outcome,
            metrics_error = metrics_error.as_deref().unwrap_or("none"),
            owner_reissued,
            "detached branch refresh rollback exhausted retries; terminal recovery completed"
        );
        drop(self.publication);
    }
}

struct BranchRefreshTransaction {
    rollback: Option<BranchRefreshRollback>,
}

impl BranchRefreshTransaction {
    fn new(
        state: &SharedState,
        publication: WorkspacePublicationGuard,
        prior: DispatchSnapshot,
        metrics_control: &crate::services::metrics::WriterControlToken,
        permit: OwnerPermit,
        prior_hydration_ready: bool,
    ) -> Self {
        Self {
            rollback: Some(BranchRefreshRollback {
                state: std::sync::Arc::clone(state),
                publication,
                prior,
                metrics_control: metrics_control.clone(),
                permit: Some(permit),
                prior_hydration_ready,
                restored_generation: None,
            }),
        }
    }

    async fn switch_and_commit(
        mut self,
        metrics_control: &mut crate::services::metrics::WriterControlToken,
        branch: String,
    ) -> Result<OwnerPermit, EngramError> {
        if let Err(switch_error) =
            crate::services::metrics::switch_branch_for(metrics_control, branch).await
        {
            let rollback = self.rollback.as_mut().ok_or_else(|| {
                EngramError::System(SystemError::DatabaseError {
                    reason: "branch refresh rollback state was already consumed".to_owned(),
                })
            })?;
            if let Err(rollback_error) = rollback.run().await {
                return Err(EngramError::Metrics(MetricsError::WriteFailed {
                    reason: format!(
                        "branch metrics switch failed ({switch_error}); transactional rollback \
                         failed ({rollback_error})"
                    ),
                }));
            }
            let _ = self.rollback.take();
            return Err(switch_error);
        }

        let mut rollback = self.rollback.take().ok_or_else(|| {
            EngramError::System(SystemError::DatabaseError {
                reason: "branch refresh transaction was already consumed".to_owned(),
            })
        })?;
        let permit = rollback.permit.take().ok_or_else(|| {
            EngramError::System(SystemError::DatabaseError {
                reason: "branch refresh transaction owner was already consumed".to_owned(),
            })
        })?;
        drop(rollback.publication);
        Ok(permit)
    }
}

impl Drop for BranchRefreshTransaction {
    fn drop(&mut self) {
        let Some(mut rollback) = self.rollback.take() else {
            return;
        };
        let Ok(runtime) = tokio::runtime::Handle::try_current() else {
            tracing::error!(
                "cancelled branch refresh lost its runtime before transactional rollback"
            );
            return;
        };
        let _rollback = runtime.spawn(async move {
            // Keep the publication guard and exact owner private until metrics
            // is coherent; competing writes remain busy/queued in the barrier.
            let mut last_error = String::new();
            for attempt in 1..=DETACHED_BRANCH_ROLLBACK_ATTEMPTS {
                match rollback.run().await {
                    Ok(()) => return,
                    Err(error) => {
                        last_error = bounded_branch_rollback_diagnostic(&error);
                        tracing::error!(
                            attempt,
                            max_attempts = DETACHED_BRANCH_ROLLBACK_ATTEMPTS,
                            error = %last_error,
                            "detached branch refresh rollback retry failed"
                        );
                        if rollback.permit.is_none() {
                            return;
                        }
                    }
                }
                if attempt < DETACHED_BRANCH_ROLLBACK_ATTEMPTS {
                    tokio::time::sleep(DETACHED_BRANCH_ROLLBACK_RETRY_DELAY).await;
                }
            }
            rollback
                .recover_terminal(DETACHED_BRANCH_ROLLBACK_ATTEMPTS, &last_error)
                .await;
        });
    }
}

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
    let parsed: SyncWorkspaceParams = serde_json::from_value(params.unwrap_or_else(|| json!({})))
        .map_err(|error| {
        EngramError::System(SystemError::InvalidParams {
            reason: error.to_string(),
        })
    })?;
    let requested = WorkMask::from_bits(
        0b001
            | if parsed.revalidate_code_graph {
                0b010
            } else {
                0
            }
            | if parsed.backfill_python_canonical {
                0b100
            } else {
                0
            },
    );
    let write_params = Some(json!({
        "revalidate_code_graph": parsed.revalidate_code_graph,
        "backfill_python_canonical": parsed.backfill_python_canonical
    }));
    let mut next_owner = None;
    loop {
        let (mut permit, ctx, mut metrics_control) = if let Some(owner) = next_owner.take() {
            owner
        } else {
            let (admission, ctx) = state
                .guarded_dispatch_context()
                .await
                .ok_or(EngramError::Workspace(WorkspaceError::NotSet))?;
            let permit = match CoordinatorCell::request(admission, requested, OwnerKind::Sync) {
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
            let metrics_control = crate::services::metrics::writer_control_token(
                Path::new(&ctx.workspace.path),
                &ctx.workspace.branch,
            )?;
            (permit, ctx, metrics_control)
        };
        let work_bits = permit.work_bits();
        let ws_path = PathBuf::from(&ctx.workspace.path);
        let Some(branch_change) = permit
            .run_until_cancelled(async {
                resolve_git_branch(&ws_path)
                    .ok()
                    .filter(|branch| branch != &ctx.workspace.branch)
            })
            .await
        else {
            return Err(EngramError::System(SystemError::DatabaseError {
                reason: "sync cancelled by workspace rebind".to_owned(),
            }));
        };
        let attempt = if let Some(branch) = branch_change {
            SyncAttempt::BranchChanged(branch)
        } else {
            crate::services::metrics::switch_branch_for(
                &mut metrics_control,
                ctx.workspace.branch.clone(),
            )
            .await?;
            let Some(result) = run_guarded_workspace_write(
                &state,
                &mut permit,
                WorkspaceWriteTarget {
                    path: &ws_path,
                    data_dir: &ctx.workspace.data_dir,
                    branch: &ctx.workspace.branch,
                    config: &ctx.config.code_graph,
                },
                write_params.clone(),
                false,
                work_bits,
            )
            .await
            else {
                return Err(EngramError::System(SystemError::DatabaseError {
                    reason: "sync cancelled by workspace rebind".to_owned(),
                }));
            };
            SyncAttempt::Finished(result)
        };

        match attempt {
            SyncAttempt::BranchChanged(branch) => {
                let prior = ctx.clone();
                let prior_hydration_ready = state.is_hydration_ready();
                let previous_branch = ctx.workspace.branch.clone();
                let mut workspace = ctx.workspace;
                workspace.workspace_id = workspace_hash(Path::new(&workspace.path), &branch);
                workspace.branch = branch.clone();
                let publication = state.acquire_workspace_publication().await;
                metrics_control = crate::services::metrics::writer_control_token(
                    Path::new(&workspace.path),
                    &previous_branch,
                )?;
                let (_, publication_admission) = state
                    .publish_workspace_generation_with_reissue_guarded(
                        &publication,
                        workspace.clone(),
                        Some(ctx.config.clone()),
                        WorkMask::from_bits(work_bits),
                    )
                    .await
                    .map_err(|error| {
                        EngramError::System(SystemError::DatabaseError {
                            reason: format!("sync branch publication failed: {error}"),
                        })
                    })?;
                match CoordinatorCell::acknowledge_and_claim_reissued(
                    permit,
                    publication_admission,
                    OwnerKind::Sync,
                )
                .map_err(|error| {
                    EngramError::System(SystemError::DatabaseError {
                        reason: format!("sync atomic branch claim failed: {error}"),
                    })
                })? {
                    ClaimOutcome::Acquired(next) => {
                        let next = BranchRefreshTransaction::new(
                            &state,
                            publication,
                            prior,
                            &metrics_control,
                            next,
                            prior_hydration_ready,
                        )
                        .switch_and_commit(&mut metrics_control, workspace.branch.clone())
                        .await?;
                        next_owner = Some((
                            next,
                            DispatchSnapshot {
                                workspace,
                                config: ctx.config,
                            },
                            metrics_control,
                        ));
                    }
                    ClaimOutcome::Retained => {
                        return Ok(
                            json!({ "status": "queued", "message": "Sync queued; will run after current indexing completes" }),
                        );
                    }
                    ClaimOutcome::Missing => {
                        return Err(EngramError::System(SystemError::DatabaseError {
                            reason: "sync branch reissue was superseded before acquisition"
                                .to_owned(),
                        }));
                    }
                    ClaimOutcome::Stale => {
                        return Err(EngramError::System(SystemError::DatabaseError {
                            reason: "sync branch publication was superseded by a distinct binding"
                                .to_owned(),
                        }));
                    }
                }
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
                    drive_transferred_sync(&state, successor, &ctx, metrics_control).await;
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
    state: &SharedState,
    permit: OwnerPermit,
    ctx: DispatchSnapshot,
) -> Result<Option<(OwnerPermit, DispatchSnapshot)>, EngramError> {
    prepare_branch_owner(state, permit, ctx, OwnerKind::Sync).await
}

/// Refresh HEAD and reacquire one owner before any branch-scoped mutation.
///
/// Retries only while the same workspace path and UUID remain active. Sync
/// owners atomically qualify their saved mask to each newly published branch;
/// empty Startup/Watcher owners publish no inherited work.
pub(crate) async fn prepare_branch_owner(
    state: &SharedState,
    permit: OwnerPermit,
    ctx: DispatchSnapshot,
    kind: OwnerKind,
) -> Result<Option<(OwnerPermit, DispatchSnapshot)>, EngramError> {
    let mut metrics_control = crate::services::metrics::writer_control_token(
        Path::new(&ctx.workspace.path),
        &ctx.workspace.branch,
    )?;
    prepare_branch_owner_with_control(state, permit, ctx, kind, &mut metrics_control).await
}

#[cfg(test)]
pub(crate) async fn prepare_branch_owner_with_existing_metrics_control(
    state: &SharedState,
    permit: OwnerPermit,
    ctx: DispatchSnapshot,
    kind: OwnerKind,
    mut metrics_control: crate::services::metrics::WriterControlToken,
) -> Result<Option<(OwnerPermit, DispatchSnapshot)>, EngramError> {
    prepare_branch_owner_with_control(state, permit, ctx, kind, &mut metrics_control).await
}

async fn prepare_branch_owner_with_control(
    state: &SharedState,
    mut permit: OwnerPermit,
    mut ctx: DispatchSnapshot,
    kind: OwnerKind,
    metrics_control: &mut crate::services::metrics::WriterControlToken,
) -> Result<Option<(OwnerPermit, DispatchSnapshot)>, EngramError> {
    let workspace_path = ctx.workspace.path.clone();
    let workspace_uuid = ctx.workspace.workspace_uuid.clone();

    loop {
        if ctx.workspace.path != workspace_path || ctx.workspace.workspace_uuid != workspace_uuid {
            return Ok(None);
        }
        let path = PathBuf::from(&ctx.workspace.path);
        let Ok(resolved_branch) = resolve_git_branch(&path) else {
            // Non-Git workspaces retain their published default/synthetic branch.
            return Ok(Some((permit, ctx)));
        };
        if resolved_branch == ctx.workspace.branch {
            crate::services::metrics::switch_branch_for(
                metrics_control,
                ctx.workspace.branch.clone(),
            )
            .await?;
            return Ok(Some((permit, ctx)));
        }

        let work_bits = permit.work_bits();
        let prior = ctx.clone();
        let prior_hydration_ready = state.is_hydration_ready();
        let previous_branch = ctx.workspace.branch.clone();
        let mut workspace = ctx.workspace;
        workspace.workspace_id = workspace_hash(Path::new(&workspace.path), &resolved_branch);
        workspace.branch = resolved_branch;
        let published_ctx = DispatchSnapshot {
            workspace: workspace.clone(),
            config: ctx.config.clone(),
        };
        let publication_guard = state.acquire_workspace_publication().await;
        *metrics_control = crate::services::metrics::writer_control_token(
            Path::new(&published_ctx.workspace.path),
            &previous_branch,
        )?;
        let publication = if work_bits == 0 {
            state
                .publish_workspace_generation_guarded(
                    &publication_guard,
                    workspace,
                    Some(ctx.config),
                )
                .await
        } else {
            state
                .publish_workspace_generation_with_reissue_guarded(
                    &publication_guard,
                    workspace,
                    Some(ctx.config),
                    WorkMask::from_bits(work_bits),
                )
                .await
        };
        let (_, publication_admission) = publication.map_err(|error| {
            EngramError::System(SystemError::DatabaseError {
                reason: format!("branch publication failed before {kind:?} mutation: {error}"),
            })
        })?;

        if work_bits != 0 {
            return match CoordinatorCell::acknowledge_and_claim_reissued(
                permit,
                publication_admission,
                kind,
            )
            .map_err(|error| {
                EngramError::System(SystemError::DatabaseError {
                    reason: format!("{kind:?} atomic branch reacquisition failed: {error}"),
                })
            })? {
                ClaimOutcome::Acquired(next) => {
                    let next = BranchRefreshTransaction::new(
                        state,
                        publication_guard,
                        prior,
                        metrics_control,
                        next,
                        prior_hydration_ready,
                    )
                    .switch_and_commit(metrics_control, published_ctx.workspace.branch.clone())
                    .await?;
                    Ok(Some((next, published_ctx)))
                }
                ClaimOutcome::Retained | ClaimOutcome::Missing | ClaimOutcome::Stale => Ok(None),
            };
        }
        drop(publication_admission);
        permit = BranchRefreshTransaction::new(
            state,
            publication_guard,
            prior,
            metrics_control,
            permit,
            prior_hydration_ready,
        )
        .switch_and_commit(metrics_control, published_ctx.workspace.branch.clone())
        .await?;
        if !matches!(
            CoordinatorCell::complete(permit),
            CompletionOutcome::RetirementAcknowledged
        ) {
            return Err(EngramError::System(SystemError::DatabaseError {
                reason: format!("branch publication lost {kind:?} retirement acknowledgment"),
            }));
        }

        loop {
            let Some((admission, current_ctx)) = state.guarded_dispatch_context().await else {
                return Err(EngramError::System(SystemError::DatabaseError {
                    reason: format!("branch publication lost {kind:?} dispatch context"),
                }));
            };
            if current_ctx.workspace.path != workspace_path
                || current_ctx.workspace.workspace_uuid != workspace_uuid
            {
                return Ok(None);
            }

            if let Some(next) = admission.acquire_background(kind).await.map_err(|error| {
                EngramError::System(SystemError::DatabaseError {
                    reason: format!("{kind:?} branch reacquisition failed: {error}"),
                })
            })? {
                permit = next;
                ctx = current_ctx;
                break;
            }
        }
    }
}

async fn drive_transferred_sync(
    state: &SharedState,
    permit: OwnerPermit,
    ctx: &DispatchSnapshot,
    mut metrics_control: crate::services::metrics::WriterControlToken,
) {
    let mut permit = permit;
    let mut current_ctx = ctx.clone();
    loop {
        let prepared = prepare_branch_owner_with_control(
            state,
            permit,
            current_ctx,
            OwnerKind::Sync,
            &mut metrics_control,
        )
        .await;
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
        let ws_path = PathBuf::from(&current_ctx.workspace.path);
        match run_guarded_workspace_write(
            state,
            &mut permit,
            WorkspaceWriteTarget {
                path: &ws_path,
                data_dir: &current_ctx.workspace.data_dir,
                branch: &current_ctx.workspace.branch,
                config: &current_ctx.config.code_graph,
            },
            Some(json!({
                "revalidate_code_graph": work_bits & 0b010 != 0,
                "backfill_python_canonical": work_bits & 0b100 != 0
            })),
            false,
            work_bits,
        )
        .await
        {
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
    progress_scope: OwnerProgressScope,
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
            let _ = state
                .set_scan_progress_for_owner(&progress_scope, Some(progress))
                .await;
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

async fn begin_indexing_scan_progress(
    state: &SharedState,
    progress_scope: &OwnerProgressScope,
    last_completed_at: Option<String>,
) -> bool {
    state
        .set_scan_progress_for_owner(
            progress_scope,
            Some(indexing_started_progress(last_completed_at)),
        )
        .await
}

#[cfg(test)]
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
    progress_scope: &OwnerProgressScope,
    result: &Result<WorkspaceWriteOutcome, EngramError>,
    full_index: bool,
) -> bool {
    let progress = match result {
        Ok(result) if full_index => completed_index_scan_progress(&result.value),
        Ok(result) => completed_sync_scan_progress(&result.value),
        Err(_) => completed_scan_progress(0),
    };
    state
        .set_scan_progress_for_owner(progress_scope, Some(progress))
        .await
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
        ProgressProbe, WorkspaceWriteOutcome, begin_indexing_scan_progress,
        completed_index_scan_progress, completed_sync_scan_progress, drive_transferred_sync,
        finalize_indexing_request, finish_workspace_write_scan_progress, index_workspace,
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

    async fn publish_with_disabled_metrics(
        state: &AppState,
        snapshot: WorkspaceSnapshot,
        metrics_guard: &tokio::sync::MutexGuard<'static, ()>,
    ) {
        crate::services::metrics::configure_test_disabled_writer(
            metrics_guard,
            std::path::Path::new(&snapshot.path),
            &snapshot.branch,
        )
        .await
        .expect("configure disabled metrics writer");
        publish(state, snapshot).await;
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

    async fn reset_metrics_writer() -> tokio::sync::MutexGuard<'static, ()> {
        let guard = crate::services::metrics::test_writer_guard().await;
        crate::services::metrics::shutdown()
            .await
            .expect("reset metrics writer");
        guard
    }

    async fn wait_for_published_branch(state: &AppState, expected_branch: &str) {
        for _ in 0..1_000 {
            if state
                .snapshot_workspace()
                .await
                .is_some_and(|snapshot| snapshot.branch == expected_branch)
            {
                return;
            }
            tokio::task::yield_now().await;
        }
        panic!("branch '{expected_branch}' was not published");
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

        let progress_scope = permit.progress_scope().expect("active progress scope");
        let (progress_tx, progress_task) =
            spawn_scan_progress_updater(Arc::clone(&state), progress_scope, Some(progress_probe));
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
    async fn delayed_progress_update_is_rejected_after_owner_terminal() {
        let state = Arc::new(AppState::new(1));
        let permit = acquired(request(
            &state,
            WorkMask::from_bits(0b001),
            OwnerKind::Index,
        ));
        let (progress_entered_tx, progress_entered_rx) = oneshot::channel();
        let (release_progress_tx, release_progress_rx) = oneshot::channel();
        let progress_writes = Arc::new(AtomicUsize::new(0));
        let progress_probe = ProgressProbe {
            entered: Some(progress_entered_tx),
            release: Some(release_progress_rx),
            writes: Arc::clone(&progress_writes),
            terminated: None,
        };
        let progress_scope = permit.progress_scope().expect("active progress scope");
        let (progress_tx, progress_task) =
            spawn_scan_progress_updater(Arc::clone(&state), progress_scope, Some(progress_probe));
        progress_tx
            .send(running_scan_progress(1, 1, None))
            .expect("queue progress update");
        progress_entered_rx
            .await
            .expect("progress child should block before its write");

        drop(permit);
        release_progress_tx
            .send(())
            .expect("release stale progress child");
        drop(progress_tx);
        progress_task.await.expect("progress child should join");

        assert_eq!(progress_writes.load(Ordering::SeqCst), 1);
        assert!(
            state.scan_progress_snapshot().await.is_none(),
            "a child released after its owner terminal must not mutate progress"
        );
    }

    #[tokio::test]
    async fn parent_progress_updates_are_rejected_after_owner_terminal() {
        let state = Arc::new(AppState::new(1));
        let permit = acquired(request(
            &state,
            WorkMask::from_bits(0b001),
            OwnerKind::Index,
        ));
        let progress_scope = permit.progress_scope().expect("active progress scope");
        drop(permit);

        assert!(
            !begin_indexing_scan_progress(&state, &progress_scope, None).await,
            "stale initial progress publication must be rejected"
        );
        assert!(
            state.scan_progress_snapshot().await.is_none(),
            "stale owner must not publish initial progress"
        );

        let result = Ok(WorkspaceWriteOutcome {
            value: json!({ "files_parsed": 1, "files_skipped": 0 }),
            unfulfilled_work_bits: 0,
        });
        assert!(
            !finish_workspace_write_scan_progress(&state, &progress_scope, &result, true).await,
            "stale final progress publication must be rejected"
        );
        assert!(
            state.scan_progress_snapshot().await.is_none(),
            "stale owner must not publish final progress"
        );
    }

    #[tokio::test]
    async fn busy_sync_publishes_full_mask_before_exact_queued_response() {
        let metrics_guard = reset_metrics_writer().await;
        let temp = tempfile::tempdir().expect("tempdir");
        let invalid_data_dir = temp.path().join("not-a-directory");
        std::fs::write(&invalid_data_dir, b"file blocks database directory")
            .expect("create invalid data path");
        let state = Arc::new(AppState::new(1));
        publish_with_disabled_metrics(
            &state,
            snapshot(
                "busy",
                "uuid-busy",
                "id-busy",
                temp.path(),
                invalid_data_dir,
            ),
            &metrics_guard,
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

    #[tokio::test(start_paused = true)]
    async fn branch_refreshing_index_reports_exact_busy_while_metrics_control_is_pending() {
        let metrics_guard = reset_metrics_writer().await;
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = temp.path().join("workspace");
        let data_dir = temp.path().join("data");
        std::fs::create_dir_all(workspace.join(".git")).expect("create git metadata");
        std::fs::write(
            workspace.join(".git").join("HEAD"),
            "ref: refs/heads/main\n",
        )
        .expect("write git HEAD");
        std::fs::write(workspace.join("app.py"), "def run():\n    return 1\n")
            .expect("write source");
        let state = Arc::new(AppState::new(1));
        let mut stale_snapshot = snapshot(
            "pending-index-branch",
            "uuid-pending-index-branch",
            "id-pending-index-branch",
            &workspace,
            data_dir,
        );
        stale_snapshot.branch = "stale-branch".to_owned();
        publish(&state, stale_snapshot).await;
        let mut saturated_writer = crate::services::metrics::configure_test_saturated_writer(
            &metrics_guard,
            &workspace,
            "stale-branch",
        )
        .expect("configure saturated metrics writer");

        let owner_state = Arc::clone(&state);
        let owner = tokio::spawn(async move {
            index_workspace(owner_state, Some(json!({ "force": true }))).await
        });
        wait_for_published_branch(&state, "main").await;
        tokio::time::timeout(
            std::time::Duration::from_secs(1),
            saturated_writer.wait_for_pending_branch_control("main"),
        )
        .await
        .expect("owner must publish pending metrics control")
        .expect("pending metrics control must target the published branch");
        let owner_was_pending = !owner.is_finished();

        let competitor = index_workspace(Arc::clone(&state), None).await;
        let owner_still_pending = !owner.is_finished();

        drop(saturated_writer);
        let owner_result = owner.await.expect("branch-refreshing index task");
        crate::services::metrics::shutdown()
            .await
            .expect("clean saturated metrics writer");

        assert!(
            owner_was_pending && owner_still_pending,
            "owner must remain pending on metrics acknowledgment during competitor admission"
        );
        assert!(
            matches!(
                competitor,
                Err(crate::errors::EngramError::CodeGraph(
                    crate::errors::CodeGraphError::IndexInProgress
                ))
            ),
            "competitor must receive the exact INDEX_IN_PROGRESS result"
        );
        assert!(
            owner_result.is_err(),
            "closing the saturated writer must fail the owner after the assertion window"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn branch_refreshing_sync_reports_exact_queued_while_metrics_control_is_pending() {
        let metrics_guard = reset_metrics_writer().await;
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
        let mut stale_snapshot = snapshot(
            "pending-sync-branch",
            "uuid-pending-sync-branch",
            "id-pending-sync-branch",
            &workspace,
            data_dir,
        );
        stale_snapshot.branch = "stale-branch".to_owned();
        publish(&state, stale_snapshot).await;
        let mut saturated_writer = crate::services::metrics::configure_test_saturated_writer(
            &metrics_guard,
            &workspace,
            "stale-branch",
        )
        .expect("configure saturated metrics writer");

        let owner_state = Arc::clone(&state);
        let owner = tokio::spawn(async move { sync_workspace(owner_state, None).await });
        wait_for_published_branch(&state, "main").await;
        tokio::time::timeout(
            std::time::Duration::from_secs(1),
            saturated_writer.wait_for_pending_branch_control("main"),
        )
        .await
        .expect("owner must publish pending metrics control")
        .expect("pending metrics control must target the published branch");
        let owner_was_pending = !owner.is_finished();

        let competitor = sync_workspace(
            Arc::clone(&state),
            Some(json!({
                "revalidate_code_graph": true,
                "backfill_python_canonical": true
            })),
        )
        .await;
        let owner_still_pending = !owner.is_finished();

        drop(saturated_writer);
        let owner_result = owner.await.expect("branch-refreshing Sync task");
        crate::services::metrics::shutdown()
            .await
            .expect("clean saturated metrics writer");

        assert!(
            owner_was_pending && owner_still_pending,
            "owner must remain pending on metrics acknowledgment during competitor admission"
        );
        assert_eq!(
            competitor.expect("competing Sync must queue"),
            json!({
                "status": "queued",
                "message": "Sync queued; will run after current indexing completes"
            })
        );
        assert!(
            owner_result.is_err(),
            "closing the saturated writer must fail the owner after the assertion window"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn timed_out_index_branch_refresh_rolls_back_and_retries_full_work() {
        let metrics_guard = reset_metrics_writer().await;
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = temp.path().join("workspace");
        let data_dir = temp.path().join("data");
        std::fs::create_dir_all(workspace.join(".git")).expect("create git metadata");
        std::fs::write(
            workspace.join(".git").join("HEAD"),
            "ref: refs/heads/main\n",
        )
        .expect("write git HEAD");
        std::fs::write(workspace.join("app.py"), "def run():\n    return 1\n")
            .expect("write source");
        let state = Arc::new(AppState::new(1));
        let mut stale_snapshot = snapshot(
            "timeout-index-branch",
            "uuid-timeout-index-branch",
            "id-timeout-index-branch",
            &workspace,
            data_dir.clone(),
        );
        stale_snapshot.branch = "stale-branch".to_owned();
        publish(&state, stale_snapshot).await;
        let mut saturated_writer = crate::services::metrics::configure_test_saturated_writer(
            &metrics_guard,
            &workspace,
            "stale-branch",
        )
        .expect("configure saturated metrics writer");
        let predecessor_admission = state.coordinator.admission();

        let owner_state = Arc::clone(&state);
        let owner = tokio::spawn(async move { index_workspace(owner_state, None).await });
        wait_for_published_branch(&state, "main").await;
        let transient_admission = state.coordinator.admission();
        let queued = sync_workspace(
            Arc::clone(&state),
            Some(json!({
                "revalidate_code_graph": true,
                "backfill_python_canonical": true
            })),
        )
        .await
        .expect("competing heavy Sync must queue");
        assert_eq!(
            queued,
            json!({
                "status": "queued",
                "message": "Sync queued; will run after current indexing completes"
            })
        );
        tokio::task::yield_now().await;
        tokio::time::advance(std::time::Duration::from_secs(1)).await;
        let owner_error = owner
            .await
            .expect("timed-out index task")
            .expect_err("saturated metrics control must time out");
        assert!(
            owner_error.to_string().contains("timed out"),
            "unexpected branch-control error: {owner_error}"
        );

        let rolled_back = state
            .snapshot_workspace()
            .await
            .expect("rolled-back workspace");
        assert_eq!(rolled_back.branch, "stale-branch");
        assert_eq!(
            state.coordinator.test_pending_bits(),
            0b111,
            "failed forced index must retain its complete routine/heavy mask"
        );
        assert!(state.coordinator.test_is_idle());
        assert!(matches!(
            CoordinatorCell::request(
                predecessor_admission,
                WorkMask::from_bits(0b001),
                OwnerKind::Sync,
            ),
            Ok(RequestOutcome::Stale)
        ));
        assert!(matches!(
            CoordinatorCell::request(
                transient_admission,
                WorkMask::from_bits(0b001),
                OwnerKind::Sync,
            ),
            Ok(RequestOutcome::Stale)
        ));
        crate::services::metrics::writer_control_token(&workspace, "stale-branch")
            .expect("metrics identity must match the rolled-back workspace");

        saturated_writer
            .release_blocking_event()
            .await
            .expect("release saturated metrics channel");
        let retry_state = Arc::clone(&state);
        let retry = tokio::spawn(async move { index_workspace(retry_state, None).await });
        saturated_writer
            .acknowledge_branch_control("main")
            .await
            .expect("acknowledge retry branch control");
        retry
            .await
            .expect("retry index task")
            .expect("retry index must succeed");

        assert_eq!(state.coordinator.test_pending_bits(), 0);
        assert!(state.coordinator.test_is_idle());
        let db = connect_db(&data_dir, "main")
            .await
            .expect("connect retry branch DB");
        let queries = CodeGraphQueries::new(db);
        assert_eq!(
            queries
                .python_extraction_version()
                .expect("read Python extraction marker"),
            Some("1".to_owned())
        );
        assert_eq!(
            queries
                .code_graph_extraction_generation()
                .expect("read code-graph generation"),
            Some("1".to_owned())
        );
        drop(saturated_writer);
        crate::services::metrics::shutdown()
            .await
            .expect("clean saturated metrics writer");
    }

    #[tokio::test(start_paused = true)]
    async fn cancelled_direct_sync_branch_refresh_rolls_back_and_retries_exact_work() {
        let metrics_guard = reset_metrics_writer().await;
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = temp.path().join("workspace");
        let data_dir = temp.path().join("data");
        std::fs::create_dir_all(workspace.join(".git")).expect("create git metadata");
        std::fs::write(
            workspace.join(".git").join("HEAD"),
            "ref: refs/heads/main\n",
        )
        .expect("write git HEAD");
        std::fs::write(workspace.join("app.py"), "def run():\n    return 1\n")
            .expect("write source");
        let state = Arc::new(AppState::new(1));
        let mut stale_snapshot = snapshot(
            "cancel-sync-branch",
            "uuid-cancel-sync-branch",
            "id-cancel-sync-branch",
            &workspace,
            data_dir.clone(),
        );
        stale_snapshot.branch = "stale-branch".to_owned();
        publish(&state, stale_snapshot).await;
        let mut saturated_writer = crate::services::metrics::configure_test_saturated_writer(
            &metrics_guard,
            &workspace,
            "stale-branch",
        )
        .expect("configure saturated metrics writer");

        let owner_state = Arc::clone(&state);
        let owner = tokio::spawn(async move {
            sync_workspace(
                owner_state,
                Some(json!({
                    "revalidate_code_graph": true,
                    "backfill_python_canonical": true
                })),
            )
            .await
        });
        wait_for_published_branch(&state, "main").await;
        let transient_admission = state.coordinator.admission();
        tokio::task::yield_now().await;
        owner.abort();
        assert!(
            owner
                .await
                .expect_err("cancelled Sync task must not join successfully")
                .is_cancelled()
        );
        wait_for_published_branch(&state, "stale-branch").await;

        assert_eq!(
            state.coordinator.test_pending_bits(),
            0b111,
            "cancelled direct Sync must retain its complete routine/heavy mask"
        );
        assert!(state.coordinator.test_is_idle());
        assert!(matches!(
            CoordinatorCell::request(
                transient_admission,
                WorkMask::from_bits(0b001),
                OwnerKind::Sync,
            ),
            Ok(RequestOutcome::Stale)
        ));
        crate::services::metrics::writer_control_token(&workspace, "stale-branch")
            .expect("metrics identity must match the rolled-back workspace");

        saturated_writer
            .release_blocking_event()
            .await
            .expect("release saturated metrics channel");
        let retry_state = Arc::clone(&state);
        let retry = tokio::spawn(async move { sync_workspace(retry_state, None).await });
        saturated_writer
            .acknowledge_branch_control("main")
            .await
            .expect("acknowledge retry branch control");
        let retry_value = retry
            .await
            .expect("retry Sync task")
            .expect("retry Sync must succeed");
        assert_ne!(
            retry_value
                .get("status")
                .and_then(serde_json::Value::as_str),
            Some("queued"),
            "retry must own and execute the preserved work"
        );

        assert_eq!(state.coordinator.test_pending_bits(), 0);
        assert!(state.coordinator.test_is_idle());
        let db = connect_db(&data_dir, "main")
            .await
            .expect("connect retry branch DB");
        let queries = CodeGraphQueries::new(db);
        assert_eq!(
            queries
                .python_extraction_version()
                .expect("read Python extraction marker"),
            Some("1".to_owned())
        );
        assert_eq!(
            queries
                .code_graph_extraction_generation()
                .expect("read code-graph generation"),
            Some("1".to_owned())
        );
        drop(saturated_writer);
        crate::services::metrics::shutdown()
            .await
            .expect("clean saturated metrics writer");
    }

    #[tokio::test(start_paused = true)]
    async fn cancelled_branch_refresh_with_closed_metrics_has_bounded_terminal_recovery() {
        let metrics_guard = reset_metrics_writer().await;
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = temp.path().join("workspace");
        let data_dir = temp.path().join("data");
        std::fs::create_dir_all(workspace.join(".git")).expect("create git metadata");
        std::fs::write(
            workspace.join(".git").join("HEAD"),
            "ref: refs/heads/main\n",
        )
        .expect("write git HEAD");
        std::fs::write(workspace.join("app.py"), "def run():\n    return 1\n")
            .expect("write source");
        let state = Arc::new(AppState::new(1));
        let mut stale_snapshot = snapshot(
            "closed-metrics-branch",
            "uuid-closed-metrics-branch",
            "id-closed-metrics-branch",
            &workspace,
            data_dir,
        );
        stale_snapshot.branch = "stale-branch".to_owned();
        publish(&state, stale_snapshot).await;
        let mut saturated_writer = crate::services::metrics::configure_test_saturated_writer(
            &metrics_guard,
            &workspace,
            "stale-branch",
        )
        .expect("configure saturated metrics writer");
        let notifications_before = state.coordinator.test_notification_calls();

        let owner_state = Arc::clone(&state);
        let owner = tokio::spawn(async move {
            sync_workspace(
                owner_state,
                Some(json!({
                    "revalidate_code_graph": true,
                    "backfill_python_canonical": true
                })),
            )
            .await
        });
        wait_for_published_branch(&state, "main").await;
        saturated_writer
            .wait_for_pending_branch_control("main")
            .await
            .expect("branch refresh must await metrics acknowledgment");
        let queued = sync_workspace(
            Arc::clone(&state),
            Some(json!({
                "revalidate_code_graph": true,
                "backfill_python_canonical": true
            })),
        )
        .await
        .expect("competing heavy Sync must queue");
        assert_eq!(
            queued,
            json!({
                "status": "queued",
                "message": "Sync queued; will run after current indexing completes"
            })
        );

        owner.abort();
        assert!(
            owner
                .await
                .expect_err("cancelled Sync task must not join successfully")
                .is_cancelled()
        );
        drop(saturated_writer);

        let publication = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            state.acquire_workspace_publication(),
        )
        .await
        .expect("detached rollback must terminate and release publication within its retry bound");
        drop(publication);

        let rolled_back = state
            .snapshot_workspace()
            .await
            .expect("terminally recovered workspace");
        assert_eq!(rolled_back.branch, "stale-branch");
        assert!(state.coordinator.test_is_idle());
        assert_eq!(
            state.coordinator.test_pending_bits(),
            0b111,
            "terminal recovery must reissue the exact routine/heavy work mask once"
        );
        assert_eq!(
            state.coordinator.test_notification_calls(),
            notifications_before + 1,
            "terminal recovery must retire the exact owner once"
        );
        crate::services::metrics::writer_control_token(&workspace, "stale-branch")
            .expect("safe terminal recovery must reinitialize the prior metrics writer");

        let retry = sync_workspace(Arc::clone(&state), None)
            .await
            .expect("next Sync must have a coherent outcome after terminal recovery");
        assert_ne!(
            retry.get("status").and_then(serde_json::Value::as_str),
            Some("queued"),
            "next Sync must claim and execute the reissued work"
        );
        assert_eq!(state.coordinator.test_pending_bits(), 0);
        assert!(state.coordinator.test_is_idle());
        assert_eq!(
            state
                .snapshot_workspace()
                .await
                .expect("workspace after retry")
                .branch,
            "main"
        );
        crate::services::metrics::writer_control_token(&workspace, "main")
            .expect("metrics identity must follow the successful retry");

        crate::services::metrics::shutdown()
            .await
            .expect("clean recovered metrics writer");
        let weak_state = Arc::downgrade(&state);
        drop(state);
        tokio::task::yield_now().await;
        assert!(
            weak_state.upgrade().is_none(),
            "bounded rollback must not leave an orphan task retaining state"
        );
    }

    #[tokio::test]
    async fn direct_sync_executes_recovered_backfill_mask_not_only_request_params() {
        let metrics_guard = reset_metrics_writer().await;
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
        publish_with_disabled_metrics(
            &state,
            snapshot(
                "recovered",
                "uuid-recovered",
                "id-recovered",
                &workspace,
                data_dir.clone(),
            ),
            &metrics_guard,
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
        let metrics_guard = reset_metrics_writer().await;
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
        let mut stale_snapshot = snapshot(
            "branch-refresh",
            "uuid-branch-refresh",
            "id-stale-branch",
            &workspace,
            data_dir,
        );
        stale_snapshot.branch = "stale-branch".to_owned();
        publish_with_disabled_metrics(&state, stale_snapshot, &metrics_guard).await;
        let old_admission = state.coordinator.admission();
        state.set_hydration_ready();
        let recovered = acquired(request(&state, WorkMask::from_bits(0b111), OwnerKind::Sync));
        drop(recovered);
        let notifications_before_sync = state.coordinator.test_notification_calls();

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
        assert_eq!(
            state.coordinator.test_notification_calls(),
            notifications_before_sync + 1,
            "branch refresh must install the reissued owner without notifying waiters; only the \
             final owner completion may notify"
        );
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
    async fn index_branch_refresh_rebinds_coordinator_and_preserves_full_work() {
        let _metrics_guard = reset_metrics_writer().await;
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = temp.path().join("workspace");
        let data_dir = temp.path().join("data");
        std::fs::create_dir_all(workspace.join(".git")).expect("create git metadata");
        std::fs::write(
            workspace.join(".git").join("HEAD"),
            "ref: refs/heads/main\n",
        )
        .expect("write git HEAD");
        std::fs::write(workspace.join("app.py"), "def run():\n    return 1\n")
            .expect("write source");
        let state = Arc::new(AppState::new(1));
        let mut stale_snapshot = snapshot(
            "index-branch-refresh",
            "uuid-index-branch-refresh",
            "id-index-stale-branch",
            &workspace,
            data_dir,
        );
        stale_snapshot.branch = "stale-branch".to_owned();
        publish(&state, stale_snapshot).await;
        let old_admission = state.coordinator.admission();
        let metrics_config = crate::models::metrics::MetricsConfig::default();
        crate::services::metrics::initialize(&workspace, "stale-branch", &metrics_config)
            .await
            .expect("initialize stale-branch metrics");

        let _response = index_workspace(Arc::clone(&state), Some(json!({ "force": true })))
            .await
            .expect("branch-refresh index");

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
        let db = connect_db(&active.data_dir, "main")
            .await
            .expect("connect refreshed branch DB");
        let queries = CodeGraphQueries::new(db);
        assert_eq!(
            queries
                .python_extraction_version()
                .expect("read Python extraction marker"),
            Some("1".to_owned()),
            "forced index must preserve its canonical backfill work"
        );
        assert_eq!(
            queries
                .code_graph_extraction_generation()
                .expect("read code-graph generation"),
            Some("1".to_owned()),
            "forced index must preserve its revalidation work"
        );
        crate::services::metrics::record(crate::models::metrics::UsageEvent {
            tool_name: "branch-probe".to_owned(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            ..crate::models::metrics::UsageEvent::default()
        });
        crate::services::metrics::shutdown()
            .await
            .expect("flush metrics writer");
        let main_usage =
            crate::services::metrics::resolve_usage_path(&workspace, "main", &metrics_config)
                .expect("resolve main metrics path");
        assert!(
            main_usage.is_file(),
            "post-refresh usage metrics must follow the active branch"
        );
        let stale_usage = crate::services::metrics::resolve_usage_path(
            &workspace,
            "stale-branch",
            &metrics_config,
        )
        .expect("resolve stale metrics path");
        assert!(
            !stale_usage.exists(),
            "no post-refresh event may be written to the stale branch"
        );
    }

    #[tokio::test]
    async fn plain_full_index_preserves_hash_skip_without_pending_heavy_work() {
        let metrics_guard = reset_metrics_writer().await;
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = temp.path().join("workspace");
        let data_dir = temp.path().join("data");
        std::fs::create_dir_all(&workspace).expect("create workspace");
        std::fs::write(workspace.join("lib.rs"), "pub fn unchanged() {}\n").expect("write source");

        let state = Arc::new(AppState::new(1));
        publish_with_disabled_metrics(
            &state,
            snapshot(
                "plain-full",
                "uuid-plain-full",
                "id-plain-full",
                &workspace,
                data_dir,
            ),
            &metrics_guard,
        )
        .await;

        index_workspace(Arc::clone(&state), None)
            .await
            .expect("seed full index");
        let result = index_workspace(Arc::clone(&state), None)
            .await
            .expect("repeat plain full index");

        assert_eq!(result["files_parsed"], 0);
        assert_eq!(
            result["files_skipped"], 1,
            "a plain full index must preserve unchanged-file hash skipping"
        );
    }

    #[tokio::test]
    async fn full_index_fulfills_recovered_heavy_work_before_success() {
        let metrics_guard = reset_metrics_writer().await;
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
        publish_with_disabled_metrics(
            &state,
            snapshot(
                "full-heavy",
                "uuid-full-heavy",
                "id-full-heavy",
                &workspace,
                data_dir.clone(),
            ),
            &metrics_guard,
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
    async fn plain_full_index_does_not_invent_heavy_work_for_file_errors() {
        let metrics_guard = reset_metrics_writer().await;
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = temp.path().join("workspace");
        let data_dir = temp.path().join("data");
        std::fs::create_dir_all(&workspace).expect("create workspace");
        std::fs::write(workspace.join("invalid.py"), [0xff]).expect("write invalid UTF-8 source");

        let state = Arc::new(AppState::new(1));
        publish_with_disabled_metrics(
            &state,
            snapshot(
                "full-error",
                "uuid-full-error",
                "id-full-error",
                &workspace,
                data_dir,
            ),
            &metrics_guard,
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
            0,
            "a plain full index must not invent durable migration work"
        );
    }

    #[tokio::test]
    async fn forced_full_index_retains_heavy_work_for_file_errors() {
        let metrics_guard = reset_metrics_writer().await;
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = temp.path().join("workspace");
        let data_dir = temp.path().join("data");
        std::fs::create_dir_all(&workspace).expect("create workspace");
        std::fs::write(workspace.join("invalid.py"), [0xff]).expect("write invalid UTF-8 source");

        let state = Arc::new(AppState::new(1));
        publish_with_disabled_metrics(
            &state,
            snapshot(
                "forced-error",
                "uuid-forced-error",
                "id-forced-error",
                &workspace,
                data_dir,
            ),
            &metrics_guard,
        )
        .await;

        let response = index_workspace(Arc::clone(&state), Some(json!({ "force": true })))
            .await
            .expect("per-file failures remain structured forced-index results");
        assert!(
            response["errors"]
                .as_array()
                .is_some_and(|errors| !errors.is_empty()),
            "fixture must exercise a non-fatal file error"
        );
        assert_eq!(
            state.coordinator.test_pending_bits(),
            0b111,
            "a forced full index must retain unfulfilled migration work"
        );
    }

    #[tokio::test]
    async fn transferred_sync_refreshes_branch_before_any_database_write() {
        let metrics_guard = reset_metrics_writer().await;
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
        let mut stale_snapshot = snapshot(
            "transferred-branch",
            "uuid-transferred-branch",
            "id-transferred-stale",
            &workspace,
            data_dir.clone(),
        );
        stale_snapshot.branch = "stale-branch".to_owned();
        publish_with_disabled_metrics(&state, stale_snapshot.clone(), &metrics_guard).await;
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
            workspace: stale_snapshot,
            config: WorkspaceConfig::default(),
        };
        let metrics_control = crate::services::metrics::writer_control_token(
            workspace.as_path(),
            &context.workspace.branch,
        )
        .expect("capture transferred writer");

        drive_transferred_sync(&state, successor, &context, metrics_control).await;

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
        let metrics_guard = reset_metrics_writer().await;
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
        let mut stale_snapshot = snapshot(
            "failed-branch",
            "uuid-failed-branch",
            "id-failed-stale",
            &workspace,
            invalid_data_dir,
        );
        stale_snapshot.branch = "stale-branch".to_owned();
        publish_with_disabled_metrics(&state, stale_snapshot, &metrics_guard).await;
        state.set_hydration_ready();

        assert!(sync_workspace(Arc::clone(&state), None).await.is_err());
        assert!(
            !state.is_hydration_ready(),
            "failed branch initialization must remain in starting state"
        );
    }
}
