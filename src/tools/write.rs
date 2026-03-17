use std::path::PathBuf;

use chrono::Utc;
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::config::StaleStrategy;
use crate::db::connect_db;
use crate::db::queries::Queries;
use crate::errors::{CodeGraphError, EngramError, SystemError, TaskError, WorkspaceError};
use crate::models::context::Context;
use crate::models::event::EventKind;
use crate::models::graph::DependencyType;
use crate::models::task::{Task, TaskStatus, compute_priority_order};
use crate::server::state::SharedState;
use crate::services::compaction::truncate_at_word_boundary;
use crate::services::connection::create_status_change_note;
use crate::services::dehydration;
use crate::services::event_ledger;
use crate::services::gate;
use crate::services::hydration;

#[derive(Deserialize)]
struct UpdateTaskParams {
    id: String,
    status: String,
    #[serde(default)]
    notes: Option<String>,
    #[serde(default)]
    priority: Option<String>,
    #[serde(default)]
    issue_type: Option<String>,
}

#[derive(Deserialize)]
struct AddBlockerParams {
    task_id: String,
    reason: String,
}

#[derive(Deserialize)]
struct RegisterDecisionParams {
    topic: String,
    decision: String,
}

const MAX_TITLE_LEN: usize = 200;

#[derive(Deserialize)]
struct CreateTaskParams {
    title: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    parent_task_id: Option<String>,
    #[serde(default)]
    work_item_id: Option<String>,
    #[serde(default)]
    issue_type: Option<String>,
}

#[derive(Deserialize)]
struct LabelParams {
    task_id: String,
    label: String,
}

#[derive(Deserialize)]
struct AddDependencyParams {
    from_task_id: String,
    to_task_id: String,
    dependency_type: DependencyType,
}

async fn workspace_path(state: &SharedState) -> Result<PathBuf, EngramError> {
    if let Some(snapshot) = state.snapshot_workspace().await {
        return Ok(PathBuf::from(snapshot.path));
    }
    Err(EngramError::Workspace(WorkspaceError::NotSet))
}

async fn workspace_id(state: &SharedState) -> Result<String, EngramError> {
    if let Some(snapshot) = state.snapshot_workspace().await {
        return Ok(snapshot.workspace_id);
    }
    Err(EngramError::Workspace(WorkspaceError::NotSet))
}

fn parse_status(raw: &str) -> Result<TaskStatus, EngramError> {
    match raw {
        "todo" => Ok(TaskStatus::Todo),
        "in_progress" => Ok(TaskStatus::InProgress),
        "done" => Ok(TaskStatus::Done),
        "blocked" => Ok(TaskStatus::Blocked),
        _ => Err(EngramError::Task(TaskError::InvalidStatus {
            status: raw.to_string(),
        })),
    }
}

fn validate_transition(from: TaskStatus, to: TaskStatus) -> Result<(), EngramError> {
    if from == to {
        return Ok(());
    }

    let allowed = match from {
        TaskStatus::Todo => matches!(to, TaskStatus::InProgress | TaskStatus::Done),
        TaskStatus::InProgress => matches!(
            to,
            TaskStatus::Done | TaskStatus::Blocked | TaskStatus::Todo
        ),
        TaskStatus::Blocked => matches!(
            to,
            TaskStatus::InProgress | TaskStatus::Todo | TaskStatus::Done
        ),
        TaskStatus::Done => matches!(to, TaskStatus::Todo),
    };

    if allowed {
        Ok(())
    } else {
        Err(EngramError::Task(TaskError::InvalidStatus {
            status: format!("{}->{}", from.as_str(), to.as_str()),
        }))
    }
}

/// Look up a task by ID, hydrating from `.engram/` files if not found in DB.
///
/// This eliminates the repeated get→hydrate→get→error pattern used
/// by many write handlers.
async fn get_task_or_hydrate(
    queries: &Queries,
    task_id: &str,
    workspace_path: &std::path::Path,
) -> Result<Task, EngramError> {
    if let Some(task) = queries.get_task(task_id).await? {
        return Ok(task);
    }
    hydration::hydrate_into_db(workspace_path, queries).await?;
    queries.get_task(task_id).await?.ok_or_else(|| {
        EngramError::Task(TaskError::NotFound {
            id: task_id.to_string(),
        })
    })
}

pub async fn update_task(state: SharedState, params: Option<Value>) -> Result<Value, EngramError> {
    let workspace_id = workspace_id(&state).await?;
    let workspace_path = workspace_path(&state).await?;
    let parsed: UpdateTaskParams =
        serde_json::from_value(params.unwrap_or_default()).map_err(|e| {
            EngramError::System(SystemError::InvalidParams {
                reason: format!("invalid params: {e}"),
            })
        })?;

    let new_status = parse_status(&parsed.status)?;
    let now = Utc::now();

    let db = connect_db(&workspace_id).await?;
    let queries = Queries::new(db.clone());

    let existing = get_task_or_hydrate(&queries, &parsed.id, &workspace_path).await?;

    // Capture previous state before fields are moved into `updated`.
    let previous_snapshot = serde_json::to_value(&existing).ok();
    let previous_status = existing.status;

    validate_transition(previous_status, new_status)?;

    // Gate enforcement: reject in_progress transitions when hard blockers are
    // incomplete (S001–S003, S010–S012).
    let soft_warnings = if new_status == TaskStatus::InProgress {
        let gate_result = gate::evaluate(&parsed.id, &queries).await?;
        if gate_result.is_blocked() {
            return Err(gate::blocked_error(&parsed.id, gate_result));
        }
        gate_result.warnings
    } else {
        Vec::new()
    };

    // Validate issue_type against allowed_types if configured (FR-048)
    let issue_type = if let Some(ref new_type) = parsed.issue_type {
        if let Some(config) = state.workspace_config().await {
            if !config.allowed_types.is_empty() && !config.allowed_types.contains(new_type) {
                return Err(EngramError::Task(TaskError::InvalidIssueType {
                    issue_type: new_type.clone(),
                }));
            }
        }
        new_type.clone()
    } else {
        existing.issue_type.clone()
    };

    // Apply priority change if requested
    let (priority, priority_order) = if let Some(ref new_priority) = parsed.priority {
        let order = compute_priority_order(new_priority);
        (new_priority.clone(), order)
    } else {
        (existing.priority.clone(), existing.priority_order)
    };

    let updated = Task {
        id: parsed.id.clone(),
        title: existing.title,
        status: new_status,
        work_item_id: existing.work_item_id,
        description: existing.description,
        context_summary: existing.context_summary,
        priority,
        priority_order,
        issue_type,
        assignee: existing.assignee,
        defer_until: existing.defer_until,
        pinned: existing.pinned,
        compaction_level: existing.compaction_level,
        compacted_at: existing.compacted_at,
        workflow_state: existing.workflow_state,
        workflow_id: existing.workflow_id,
        created_at: existing.created_at,
        updated_at: now,
    };

    queries.upsert_task(&updated).await?;

    // Record event — fire-and-forget; event failures must not fail the mutation.
    let event_max = state
        .workspace_config()
        .await
        .map_or(500, |c| c.event_ledger_max);
    if let Err(e) = event_ledger::record_event(
        &queries,
        EventKind::TaskUpdated,
        "task",
        &format!("task:{}", parsed.id),
        previous_snapshot,
        serde_json::to_value(&updated).ok(),
        "mcp-tool",
        event_max,
    )
    .await
    {
        tracing::warn!(error = %e, task_id = %parsed.id, "event recording failed for update_task");
    }

    // FR-015: always append a context note on task update
    let context_id = create_status_change_note(
        &queries,
        &parsed.id,
        previous_status,
        new_status,
        parsed.notes.as_deref(),
        now,
    )
    .await?;

    let mut response = json!({
        "task_id": parsed.id,
        "previous_status": previous_status.as_str(),
        "new_status": new_status.as_str(),
        "context_id": context_id,
        "updated_at": now.to_rfc3339(),
    });
    if !soft_warnings.is_empty() {
        response["warnings"] = serde_json::Value::Array(soft_warnings);
    }
    Ok(response)
}

pub async fn add_blocker(state: SharedState, params: Option<Value>) -> Result<Value, EngramError> {
    let workspace_id = workspace_id(&state).await?;
    let workspace_path = workspace_path(&state).await?;
    let parsed: AddBlockerParams =
        serde_json::from_value(params.unwrap_or_default()).map_err(|e| {
            EngramError::System(SystemError::InvalidParams {
                reason: format!("invalid params: {e}"),
            })
        })?;

    let now = Utc::now();
    let db = connect_db(&workspace_id).await?;
    let queries = Queries::new(db.clone());

    let task_id = parsed.task_id.clone();

    let task = get_task_or_hydrate(&queries, &task_id, &workspace_path).await?;

    if task.status == TaskStatus::Blocked {
        return Err(EngramError::Task(TaskError::BlockerExists { id: task_id }));
    }

    validate_transition(task.status, TaskStatus::Blocked)?;

    let ctx_id = format!("context:{}", Uuid::new_v4());
    let ctx = Context {
        id: ctx_id.clone(),
        content: parsed.reason,
        embedding: None,
        source_client: "daemon".into(),
        created_at: now,
    };
    queries.insert_context(&ctx).await?;
    queries.link_task_context(&task.id, &ctx_id).await?;

    let previous_status = task.status;
    queries
        .set_task_status(&task.id, TaskStatus::Blocked, now)
        .await?;

    // FR-015: audit trail for every status transition
    let audit_ctx_id = create_status_change_note(
        &queries,
        &task.id,
        previous_status,
        TaskStatus::Blocked,
        Some(&ctx.content),
        now,
    )
    .await?;

    Ok(json!({
        "task_id": task.id,
        "blocker_context_id": ctx_id,
        "audit_context_id": audit_ctx_id,
        "updated_at": now.to_rfc3339(),
    }))
}

pub async fn register_decision(
    state: SharedState,
    params: Option<Value>,
) -> Result<Value, EngramError> {
    let workspace_id = workspace_id(&state).await?;
    let parsed: RegisterDecisionParams = serde_json::from_value(params.unwrap_or_default())
        .map_err(|e| {
            EngramError::System(SystemError::InvalidParams {
                reason: format!("invalid params: {e}"),
            })
        })?;

    let now = Utc::now();
    let db = connect_db(&workspace_id).await?;
    let queries = Queries::new(db.clone());

    let ctx_id = format!("context:{}", Uuid::new_v4());
    let content = format!("# {}\n{}", parsed.topic, parsed.decision);
    let ctx = Context {
        id: ctx_id.clone(),
        content,
        embedding: None,
        source_client: "daemon".into(),
        created_at: now,
    };
    queries.insert_context(&ctx).await?;

    Ok(json!({
        "decision_id": ctx_id,
        "file_path": ".engram/decisions.md",
        "created_at": now.to_rfc3339(),
        "topic": parsed.topic,
    }))
}

/// Create a new task with `todo` status and optional parent dependency.
pub async fn create_task(state: SharedState, params: Option<Value>) -> Result<Value, EngramError> {
    let workspace_id = workspace_id(&state).await?;
    let parsed: CreateTaskParams =
        serde_json::from_value(params.unwrap_or_default()).map_err(|e| {
            EngramError::System(SystemError::InvalidParams {
                reason: format!("invalid params: {e}"),
            })
        })?;

    let title = parsed.title.trim();
    if title.is_empty() {
        return Err(EngramError::Task(TaskError::TitleEmpty));
    }
    if title.len() > MAX_TITLE_LEN {
        return Err(EngramError::Task(TaskError::TitleTooLong));
    }

    // Validate issue_type against allowed_types if configured (FR-048)
    if let Some(ref issue_type) = parsed.issue_type {
        if let Some(config) = state.workspace_config().await {
            if !config.allowed_types.is_empty() && !config.allowed_types.contains(issue_type) {
                return Err(EngramError::Task(TaskError::InvalidIssueType {
                    issue_type: issue_type.clone(),
                }));
            }
        }
    }

    let db = connect_db(&workspace_id).await?;
    let queries = Queries::new(db);

    let task = queries
        .create_task(
            title,
            parsed.description.as_deref().unwrap_or(""),
            parsed.work_item_id.as_deref(),
            parsed.parent_task_id.as_deref(),
            parsed.issue_type.as_deref(),
        )
        .await?;

    // Record event — fire-and-forget; event failures must not fail the mutation.
    let event_max = state
        .workspace_config()
        .await
        .map_or(500, |c| c.event_ledger_max);
    if let Err(e) = event_ledger::record_event(
        &queries,
        EventKind::TaskCreated,
        "task",
        &format!("task:{}", task.id),
        None,
        serde_json::to_value(&task).ok(),
        "mcp-tool",
        event_max,
    )
    .await
    {
        tracing::warn!(error = %e, task_id = %task.id, "event recording failed for create_task");
    }

    let mut response = json!({
        "task_id": task.id,
        "title": task.title,
        "status": "todo",
        "issue_type": task.issue_type,
        "created_at": task.created_at.to_rfc3339(),
    });

    if let Some(parent) = &parsed.parent_task_id {
        response["parent_task_id"] = json!(parent);
    }

    Ok(response)
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
    let workspace_id = snapshot.workspace_id.clone();
    let engram_dir = path.join(".engram");
    let stale_strategy = state.stale_strategy();
    let mut warnings: Vec<String> = Vec::new();
    let is_stale =
        snapshot.stale_files || hydration::detect_stale_since(&snapshot.file_mtimes, &engram_dir);

    let _ = params;

    let db = connect_db(&workspace_id).await?;
    let queries = Queries::new(db.clone());

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
            hydration::hydrate_into_db(&path, &queries).await?;
        }
        (false, _) => {}
    }

    let result = dehydration::dehydrate_workspace(&queries, &path).await?;

    // Code graph serialization (FR-132, FR-133, FR-134)
    let cg_queries = crate::db::queries::CodeGraphQueries::new(db);
    let cg_result = dehydration::dehydrate_code_graph(&cg_queries, &path).await?;

    let mut all_files = result.files_written.clone();
    all_files.extend(cg_result.files_written);

    let new_mtimes = hydration::collect_file_mtimes(&engram_dir);

    let _ = state
        .update_workspace(|ws| {
            ws.last_flush = Some(result.flush_timestamp.clone());
            ws.stale_files = false;
            ws.file_mtimes = new_mtimes;
            ws.task_count = result.tasks_written as u64;
        })
        .await;

    Ok(json!({
        "files_written": all_files,
        "warnings": warnings,
        "flush_timestamp": result.flush_timestamp,
        "code_graph": {
            "nodes_written": cg_result.nodes_written,
            "edges_written": cg_result.edges_written,
        },
    }))
}

// ── Label operations ────────────────────────────────────────────────────

pub async fn add_label(state: SharedState, params: Option<Value>) -> Result<Value, EngramError> {
    let ws_id = workspace_id(&state).await?;
    let parsed: LabelParams =
        serde_json::from_value(params.unwrap_or_else(|| json!({}))).map_err(|e| {
            EngramError::System(SystemError::InvalidParams {
                reason: e.to_string(),
            })
        })?;

    // Validate against allowed_labels if configured
    if let Some(config) = state.workspace_config().await {
        if !config.allowed_labels.is_empty() && !config.allowed_labels.contains(&parsed.label) {
            return Err(EngramError::Task(TaskError::LabelValidation {
                reason: format!(
                    "label '{}' not in allowed_labels: {:?}",
                    parsed.label, config.allowed_labels
                ),
            }));
        }
    }

    let db = connect_db(&ws_id).await?;
    let queries = Queries::new(db);

    // Strip table prefix if present (e.g., "task:abc" → "abc")
    let task_id = parsed
        .task_id
        .strip_prefix("task:")
        .unwrap_or(&parsed.task_id);

    queries.insert_label(task_id, &parsed.label).await?;
    let label_count = queries.count_labels_for_task(task_id).await?;

    Ok(json!({
        "task_id": task_id,
        "label": parsed.label,
        "label_count": label_count,
    }))
}

pub async fn remove_label(state: SharedState, params: Option<Value>) -> Result<Value, EngramError> {
    let ws_id = workspace_id(&state).await?;
    let parsed: LabelParams =
        serde_json::from_value(params.unwrap_or_else(|| json!({}))).map_err(|e| {
            EngramError::System(SystemError::InvalidParams {
                reason: e.to_string(),
            })
        })?;

    let db = connect_db(&ws_id).await?;
    let queries = Queries::new(db);

    let task_id = parsed
        .task_id
        .strip_prefix("task:")
        .unwrap_or(&parsed.task_id);

    queries.delete_label(task_id, &parsed.label).await?;
    let label_count = queries.count_labels_for_task(task_id).await?;

    Ok(json!({
        "task_id": task_id,
        "label": parsed.label,
        "label_count": label_count,
    }))
}

// ── Dependency operations ───────────────────────────────────────────────

pub async fn add_dependency(
    state: SharedState,
    params: Option<Value>,
) -> Result<Value, EngramError> {
    let ws_id = workspace_id(&state).await?;
    let parsed: AddDependencyParams =
        serde_json::from_value(params.unwrap_or_default()).map_err(|e| {
            EngramError::System(SystemError::InvalidParams {
                reason: format!("invalid params: {e}"),
            })
        })?;

    let db = connect_db(&ws_id).await?;
    let queries = Queries::new(db);

    let from_id = parsed
        .from_task_id
        .strip_prefix("task:")
        .unwrap_or(&parsed.from_task_id);
    let to_id = parsed
        .to_task_id
        .strip_prefix("task:")
        .unwrap_or(&parsed.to_task_id);

    queries
        .create_dependency(from_id, to_id, parsed.dependency_type)
        .await?;

    // Record event — fire-and-forget.
    let event_max = state
        .workspace_config()
        .await
        .map_or(500, |c| c.event_ledger_max);
    if let Err(e) = event_ledger::record_event(
        &queries,
        EventKind::EdgeCreated,
        "depends_on",
        &format!("depends_on:{from_id}_{to_id}"),
        None,
        Some(serde_json::json!({
            "from": from_id,
            "to": to_id,
            "type": parsed.dependency_type,
        })),
        "mcp-tool",
        event_max,
    )
    .await
    {
        tracing::warn!(
            error = %e,
            from = %from_id,
            to = %to_id,
            "event recording failed for add_dependency"
        );
    }

    Ok(json!({
        "from_task_id": from_id,
        "to_task_id": to_id,
        "dependency_type": parsed.dependency_type,
    }))
}

// ── Compaction ───────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct CompactionItem {
    task_id: String,
    summary: String,
}

#[derive(Deserialize)]
struct ApplyCompactionParams {
    compactions: Vec<CompactionItem>,
}

/// Apply agent-generated summaries to completed tasks, replacing their
/// description, incrementing `compaction_level`, and setting `compacted_at`.
pub async fn apply_compaction(
    state: SharedState,
    params: Option<Value>,
) -> Result<Value, EngramError> {
    let ws_id = workspace_id(&state).await?;
    let parsed: ApplyCompactionParams = serde_json::from_value(params.unwrap_or_default())
        .map_err(|e| {
            EngramError::System(SystemError::InvalidParams {
                reason: format!("invalid params: {e}"),
            })
        })?;

    let db = connect_db(&ws_id).await?;
    let queries = Queries::new(db);

    // Read truncation_length from workspace config (default 500)
    let trunc_len = state
        .workspace_config()
        .await
        .map_or(500, |c| c.compaction.truncation_length as usize);

    let mut results = Vec::new();
    for item in &parsed.compactions {
        let task_id = item.task_id.strip_prefix("task:").unwrap_or(&item.task_id);
        let summary = truncate_at_word_boundary(&item.summary, trunc_len);
        let updated = queries.apply_compaction(task_id, &summary).await?;
        results.push(json!({
            "task_id": updated.id,
            "new_compaction_level": updated.compaction_level,
            "compacted_at": updated.compacted_at.map(|d| d.to_rfc3339()),
        }));
    }

    Ok(json!({ "results": results }))
}

// ── Claim/Release ──────────────────────────────────────────────

#[derive(Deserialize)]
struct ClaimTaskParams {
    task_id: String,
    claimant: String,
}

/// Claim a task for a specific agent/user. Rejects if already claimed.
pub async fn claim_task(state: SharedState, params: Option<Value>) -> Result<Value, EngramError> {
    let ws_id = workspace_id(&state).await?;
    let parsed: ClaimTaskParams =
        serde_json::from_value(params.unwrap_or_default()).map_err(|e| {
            EngramError::System(SystemError::InvalidParams {
                reason: format!("invalid params: {e}"),
            })
        })?;

    let task_id = parsed
        .task_id
        .strip_prefix("task:")
        .unwrap_or(&parsed.task_id);

    let db = connect_db(&ws_id).await?;
    let queries = Queries::new(db);

    let updated = queries.claim_task(task_id, &parsed.claimant).await?;

    // Create context note recording the claim
    let now = chrono::Utc::now();
    let ctx_id = format!("context:{}", uuid::Uuid::new_v4());
    let ctx = crate::models::context::Context {
        id: ctx_id.clone(),
        content: format!("Claimed by {}", parsed.claimant),
        embedding: None,
        source_client: "daemon".into(),
        created_at: now,
    };
    queries.insert_context(&ctx).await?;
    queries.link_task_context(task_id, &ctx_id).await?;

    Ok(json!({
        "task_id": updated.id,
        "claimant": parsed.claimant,
        "context_id": ctx_id,
        "claimed_at": now.to_rfc3339(),
    }))
}

#[derive(Deserialize)]
struct ReleaseTaskParams {
    task_id: String,
}

/// Release a claimed task. Any client may release any claim.
pub async fn release_task(state: SharedState, params: Option<Value>) -> Result<Value, EngramError> {
    let ws_id = workspace_id(&state).await?;
    let parsed: ReleaseTaskParams =
        serde_json::from_value(params.unwrap_or_default()).map_err(|e| {
            EngramError::System(SystemError::InvalidParams {
                reason: format!("invalid params: {e}"),
            })
        })?;

    let task_id = parsed
        .task_id
        .strip_prefix("task:")
        .unwrap_or(&parsed.task_id);

    let db = connect_db(&ws_id).await?;
    let queries = Queries::new(db);

    let previous_claimant = queries.release_task(task_id).await?;

    // Create context note recording the release
    let now = chrono::Utc::now();
    let ctx_id = format!("context:{}", uuid::Uuid::new_v4());
    let ctx = crate::models::context::Context {
        id: ctx_id.clone(),
        content: format!("Released, previously claimed by {previous_claimant}"),
        embedding: None,
        source_client: "daemon".into(),
        created_at: now,
    };
    queries.insert_context(&ctx).await?;
    queries.link_task_context(task_id, &ctx_id).await?;

    Ok(json!({
        "task_id": task_id,
        "previous_claimant": previous_claimant,
        "context_id": ctx_id,
        "released_at": now.to_rfc3339(),
    }))
}

// ── Defer/Pin operations ────────────────────────────────────────────────

#[derive(Deserialize)]
struct DeferTaskParams {
    task_id: String,
    until: String,
}

/// Defer a task until a given ISO 8601 datetime.
pub async fn defer_task(state: SharedState, params: Option<Value>) -> Result<Value, EngramError> {
    let ws_id = workspace_id(&state).await?;
    let workspace_path = workspace_path(&state).await?;
    let parsed: DeferTaskParams =
        serde_json::from_value(params.unwrap_or_default()).map_err(|e| {
            EngramError::System(SystemError::InvalidParams {
                reason: format!("invalid params: {e}"),
            })
        })?;

    let task_id = parsed
        .task_id
        .strip_prefix("task:")
        .unwrap_or(&parsed.task_id);

    let defer_until = chrono::DateTime::parse_from_rfc3339(&parsed.until)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| {
            EngramError::System(SystemError::InvalidParams {
                reason: format!("invalid ISO 8601 datetime: {e}"),
            })
        })?;

    let db = connect_db(&ws_id).await?;
    let queries = Queries::new(db);

    let mut task = get_task_or_hydrate(&queries, task_id, &workspace_path).await?;

    let now = Utc::now();
    task.defer_until = Some(defer_until);
    task.updated_at = now;
    queries.upsert_task(&task).await?;

    let ctx_id = format!("context:{}", Uuid::new_v4());
    let ctx = Context {
        id: ctx_id.clone(),
        content: format!("Deferred until {}", parsed.until),
        embedding: None,
        source_client: "daemon".into(),
        created_at: now,
    };
    queries.insert_context(&ctx).await?;
    queries.link_task_context(task_id, &ctx_id).await?;

    Ok(json!({
        "task_id": task_id,
        "defer_until": parsed.until,
        "context_id": ctx_id,
        "updated_at": now.to_rfc3339(),
    }))
}

#[derive(Deserialize)]
struct UnDeferTaskParams {
    task_id: String,
}

/// Clear the defer_until date on a task, making it immediately eligible for ready-work.
pub async fn undefer_task(state: SharedState, params: Option<Value>) -> Result<Value, EngramError> {
    let ws_id = workspace_id(&state).await?;
    let workspace_path = workspace_path(&state).await?;
    let parsed: UnDeferTaskParams =
        serde_json::from_value(params.unwrap_or_default()).map_err(|e| {
            EngramError::System(SystemError::InvalidParams {
                reason: format!("invalid params: {e}"),
            })
        })?;

    let task_id = parsed
        .task_id
        .strip_prefix("task:")
        .unwrap_or(&parsed.task_id);

    let db = connect_db(&ws_id).await?;
    let queries = Queries::new(db);

    let mut task = get_task_or_hydrate(&queries, task_id, &workspace_path).await?;

    let previous_defer = task.defer_until.map(|d| d.to_rfc3339());
    let now = Utc::now();
    task.defer_until = None;
    task.updated_at = now;
    queries.upsert_task(&task).await?;

    let ctx_id = format!("context:{}", Uuid::new_v4());
    let content = if let Some(ref prev) = previous_defer {
        format!("Undeferred (was deferred until {prev})")
    } else {
        "Undeferred (was not deferred)".to_string()
    };
    let ctx = Context {
        id: ctx_id.clone(),
        content,
        embedding: None,
        source_client: "daemon".into(),
        created_at: now,
    };
    queries.insert_context(&ctx).await?;
    queries.link_task_context(task_id, &ctx_id).await?;

    Ok(json!({
        "task_id": task_id,
        "previous_defer_until": previous_defer,
        "context_id": ctx_id,
        "updated_at": now.to_rfc3339(),
    }))
}

#[derive(Deserialize)]
struct PinTaskParams {
    task_id: String,
}

/// Pin a task so it appears first in ready-work results.
pub async fn pin_task(state: SharedState, params: Option<Value>) -> Result<Value, EngramError> {
    let ws_id = workspace_id(&state).await?;
    let workspace_path = workspace_path(&state).await?;
    let parsed: PinTaskParams =
        serde_json::from_value(params.unwrap_or_default()).map_err(|e| {
            EngramError::System(SystemError::InvalidParams {
                reason: format!("invalid params: {e}"),
            })
        })?;

    let task_id = parsed
        .task_id
        .strip_prefix("task:")
        .unwrap_or(&parsed.task_id);

    let db = connect_db(&ws_id).await?;
    let queries = Queries::new(db);

    let mut task = get_task_or_hydrate(&queries, task_id, &workspace_path).await?;

    let now = Utc::now();
    task.pinned = true;
    task.updated_at = now;
    queries.upsert_task(&task).await?;

    let ctx_id = format!("context:{}", Uuid::new_v4());
    let ctx = Context {
        id: ctx_id.clone(),
        content: "Pinned".to_string(),
        embedding: None,
        source_client: "daemon".into(),
        created_at: now,
    };
    queries.insert_context(&ctx).await?;
    queries.link_task_context(task_id, &ctx_id).await?;

    Ok(json!({
        "task_id": task_id,
        "pinned": true,
        "context_id": ctx_id,
        "updated_at": now.to_rfc3339(),
    }))
}

/// Unpin a task, restoring normal priority ordering.
pub async fn unpin_task(state: SharedState, params: Option<Value>) -> Result<Value, EngramError> {
    let ws_id = workspace_id(&state).await?;
    let workspace_path = workspace_path(&state).await?;
    let parsed: PinTaskParams =
        serde_json::from_value(params.unwrap_or_default()).map_err(|e| {
            EngramError::System(SystemError::InvalidParams {
                reason: format!("invalid params: {e}"),
            })
        })?;

    let task_id = parsed
        .task_id
        .strip_prefix("task:")
        .unwrap_or(&parsed.task_id);

    let db = connect_db(&ws_id).await?;
    let queries = Queries::new(db);

    let mut task = get_task_or_hydrate(&queries, task_id, &workspace_path).await?;

    let now = Utc::now();
    task.pinned = false;
    task.updated_at = now;
    queries.upsert_task(&task).await?;

    let ctx_id = format!("context:{}", Uuid::new_v4());
    let ctx = Context {
        id: ctx_id.clone(),
        content: "Unpinned".to_string(),
        embedding: None,
        source_client: "daemon".into(),
        created_at: now,
    };
    queries.insert_context(&ctx).await?;
    queries.link_task_context(task_id, &ctx_id).await?;

    Ok(json!({
        "task_id": task_id,
        "pinned": false,
        "context_id": ctx_id,
        "updated_at": now.to_rfc3339(),
    }))
}

// ── Batch operations ────────────────────────────────────────────────────

#[derive(Deserialize)]
struct BatchUpdateItem {
    id: String,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    notes: Option<String>,
    #[serde(default)]
    priority: Option<String>,
    #[serde(default)]
    issue_type: Option<String>,
}

#[derive(Deserialize)]
struct BatchUpdateParams {
    updates: Vec<BatchUpdateItem>,
}

/// Update multiple tasks in a single call with per-item results (FR-058, FR-059).
///
/// Validates batch size against config max (FR-060, default 100). Each update
/// is attempted independently — one failure does not prevent others from
/// succeeding. Returns `BatchPartialFailure` error when any item fails.
pub async fn batch_update_tasks(
    state: SharedState,
    params: Option<Value>,
) -> Result<Value, EngramError> {
    let ws_id = workspace_id(&state).await?;
    let parsed: BatchUpdateParams =
        serde_json::from_value(params.unwrap_or_default()).map_err(|e| {
            EngramError::System(SystemError::InvalidParams {
                reason: format!("invalid params: {e}"),
            })
        })?;

    // FR-060: enforce batch max_size from config (default: 100)
    let max_size = if let Some(cfg) = state.workspace_config().await {
        cfg.batch.max_size
    } else {
        100
    };
    if parsed.updates.len() > max_size as usize {
        return Err(EngramError::System(SystemError::InvalidParams {
            reason: format!(
                "batch size {} exceeds maximum {}",
                parsed.updates.len(),
                max_size
            ),
        }));
    }

    let db = connect_db(&ws_id).await?;
    let queries = Queries::new(db);

    // Hydrate once for the entire batch so individual lookups succeed
    let workspace_path = workspace_path(&state).await?;
    hydration::hydrate_into_db(&workspace_path, &queries).await?;

    let mut results = Vec::new();
    let mut succeeded: u64 = 0;
    let mut failed: u64 = 0;

    for item in &parsed.updates {
        match apply_single_update(&state, &queries, item).await {
            Ok(result_item) => {
                succeeded += 1;
                results.push(result_item);
            }
            Err(err) => {
                failed += 1;
                let resp = err.to_response();
                results.push(json!({
                    "id": item.id,
                    "success": false,
                    "error": {
                        "code": resp.error.code,
                        "message": resp.error.message,
                    }
                }));
            }
        }
    }

    // FR-059: return partial failure error when any item fails
    if failed > 0 {
        return Err(EngramError::Task(TaskError::BatchPartialFailure {
            succeeded,
            failed,
            results: serde_json::to_value(&results).unwrap_or_default(),
        }));
    }

    Ok(json!({
        "results": results,
        "succeeded": succeeded,
        "failed": failed,
    }))
}

/// Apply a single item update within a batch call.
async fn apply_single_update(
    state: &SharedState,
    queries: &Queries,
    item: &BatchUpdateItem,
) -> Result<Value, EngramError> {
    let task_id = item.id.strip_prefix("task:").unwrap_or(&item.id);

    let existing = queries.get_task(task_id).await?.ok_or_else(|| {
        EngramError::Task(TaskError::NotFound {
            id: task_id.to_string(),
        })
    })?;

    let previous_status = existing.status;

    // Determine new status — default to existing if not provided
    let new_status = if let Some(ref status_str) = item.status {
        parse_status(status_str)?
    } else {
        existing.status
    };

    if item.status.is_some() {
        validate_transition(previous_status, new_status)?;
    }

    // Validate issue_type if provided (FR-048)
    let issue_type = if let Some(ref new_type) = item.issue_type {
        if let Some(config) = state.workspace_config().await {
            if !config.allowed_types.is_empty() && !config.allowed_types.contains(new_type) {
                return Err(EngramError::Task(TaskError::InvalidIssueType {
                    issue_type: new_type.clone(),
                }));
            }
        }
        new_type.clone()
    } else {
        existing.issue_type.clone()
    };

    let (priority, priority_order) = if let Some(ref new_priority) = item.priority {
        let order = compute_priority_order(new_priority);
        (new_priority.clone(), order)
    } else {
        (existing.priority.clone(), existing.priority_order)
    };

    let now = Utc::now();
    let updated = Task {
        id: task_id.to_string(),
        title: existing.title,
        status: new_status,
        work_item_id: existing.work_item_id,
        description: existing.description,
        context_summary: existing.context_summary,
        priority,
        priority_order,
        issue_type,
        assignee: existing.assignee,
        defer_until: existing.defer_until,
        pinned: existing.pinned,
        compaction_level: existing.compaction_level,
        compacted_at: existing.compacted_at,
        workflow_state: existing.workflow_state,
        workflow_id: existing.workflow_id,
        created_at: existing.created_at,
        updated_at: now,
    };

    queries.upsert_task(&updated).await?;

    // FR-015: every update creates a context note
    let _context_id = create_status_change_note(
        queries,
        task_id,
        previous_status,
        new_status,
        item.notes.as_deref(),
        now,
    )
    .await?;

    Ok(json!({
        "id": task_id,
        "success": true,
        "previous_status": previous_status.as_str(),
        "new_status": new_status.as_str(),
    }))
}

// ── Comment operations ──────────────────────────────────────────────────

#[derive(Deserialize)]
struct AddCommentParams {
    task_id: String,
    content: String,
    author: String,
}

/// Add a discussion comment to a task, separate from context notes (FR-061, FR-062).
pub async fn add_comment(state: SharedState, params: Option<Value>) -> Result<Value, EngramError> {
    let ws_id = workspace_id(&state).await?;
    let parsed: AddCommentParams =
        serde_json::from_value(params.unwrap_or_default()).map_err(|e| {
            EngramError::System(SystemError::InvalidParams {
                reason: format!("invalid params: {e}"),
            })
        })?;

    let task_id = parsed
        .task_id
        .strip_prefix("task:")
        .unwrap_or(&parsed.task_id);

    let db = connect_db(&ws_id).await?;
    let queries = Queries::new(db);

    // Validate that the task exists (hydrate if missing from DB)
    let mut task = queries.get_task(task_id).await?;
    if task.is_none() {
        let workspace_path = workspace_path(&state).await?;
        hydration::hydrate_into_db(&workspace_path, &queries).await?;
        task = queries.get_task(task_id).await?;
    }
    if task.is_none() {
        return Err(EngramError::Task(TaskError::NotFound {
            id: task_id.to_string(),
        }));
    }

    let comment_id = queries
        .insert_comment(task_id, &parsed.content, &parsed.author)
        .await?;

    let now = Utc::now();

    Ok(json!({
        "comment_id": comment_id,
        "task_id": task_id,
        "author": parsed.author,
        "created_at": now.to_rfc3339(),
    }))
}

// ── index_workspace ─────────────────────────────────────────────────

#[derive(Deserialize)]
struct IndexWorkspaceParams {
    #[serde(default)]
    force: bool,
}

/// Parse all supported source files and populate the code knowledge graph.
///
/// Returns a structured summary of files parsed, symbols indexed, edges
/// created, and any per-file errors encountered.
pub async fn index_workspace(
    state: SharedState,
    params: Option<Value>,
) -> Result<Value, EngramError> {
    let ws_path = workspace_path(&state).await?;
    let ws_id = workspace_id(&state).await?;

    // Reject if indexing is already running.
    if !state.try_start_indexing() {
        return Err(EngramError::CodeGraph(CodeGraphError::IndexInProgress));
    }

    // Run the indexing logic, ensuring the flag is cleared on all exit paths.
    let result = index_workspace_inner(&state, &ws_path, &ws_id, params).await;
    state.finish_indexing().await;
    result
}

/// Inner indexing logic separated to guarantee `finish_indexing()` runs.
async fn index_workspace_inner(
    state: &SharedState,
    ws_path: &std::path::Path,
    ws_id: &str,
    params: Option<Value>,
) -> Result<Value, EngramError> {
    let parsed: IndexWorkspaceParams = serde_json::from_value(params.unwrap_or_else(|| json!({})))
        .map_err(|e| {
            EngramError::System(SystemError::InvalidParams {
                reason: e.to_string(),
            })
        })?;

    let config = state
        .workspace_config()
        .await
        .map(|c| c.code_graph.clone())
        .unwrap_or_default();

    let result =
        crate::services::code_graph::index_workspace(ws_path, ws_id, &config, parsed.force).await?;

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
    let ws_path = workspace_path(&state).await?;
    let ws_id = workspace_id(&state).await?;

    // Reject if indexing is already running (FR-121 / 7003).
    if !state.try_start_indexing() {
        return Err(EngramError::CodeGraph(CodeGraphError::IndexInProgress));
    }

    // Run the sync logic, ensuring the flag is cleared on all exit paths.
    let result = sync_workspace_inner(&state, &ws_path, &ws_id, params).await;
    state.finish_indexing().await;
    result
}

/// Inner sync logic separated to guarantee `finish_indexing()` runs.
async fn sync_workspace_inner(
    state: &SharedState,
    ws_path: &std::path::Path,
    ws_id: &str,
    params: Option<Value>,
) -> Result<Value, EngramError> {
    let _ = params; // no params for sync_workspace currently

    let config = state
        .workspace_config()
        .await
        .map(|c| c.code_graph.clone())
        .unwrap_or_default();

    let result = crate::services::code_graph::sync_workspace(ws_path, ws_id, &config).await?;

    serde_json::to_value(result).map_err(|e| {
        EngramError::System(SystemError::DatabaseError {
            reason: format!("result serialization failed: {e}"),
        })
    })
}

// ── link_task_to_code (T050) ────────────────────────────────────────

/// Create `concerns` edges between a task and all code symbols matching
/// the given `symbol_name`. Idempotent per FR-152: calling with the same
/// `(task_id, symbol_name)` pair does not create duplicate edges.
pub async fn link_task_to_code(
    state: SharedState,
    params: Option<Value>,
) -> Result<Value, EngramError> {
    #[derive(Debug, Deserialize)]
    struct Params {
        task_id: String,
        symbol_name: String,
        #[serde(default = "default_linked_by")]
        linked_by: String,
    }

    fn default_linked_by() -> String {
        "agent".to_owned()
    }

    let ws_id = workspace_id(&state).await?;

    let parsed: Params =
        serde_json::from_value(params.unwrap_or_else(|| json!({}))).map_err(|e| {
            EngramError::System(SystemError::InvalidParams {
                reason: e.to_string(),
            })
        })?;

    let db = connect_db(&ws_id).await?;
    let cg_queries = crate::db::queries::CodeGraphQueries::new(db.clone());
    let queries = Queries::new(db);

    // Verify task exists.
    let bare_task_id = parsed
        .task_id
        .strip_prefix("task:")
        .unwrap_or(&parsed.task_id);
    let task = queries.get_task(bare_task_id).await?;
    if task.is_none() {
        return Err(EngramError::Task(TaskError::NotFound {
            id: parsed.task_id.clone(),
        }));
    }

    // Find all symbols matching the name.
    let symbols = cg_queries.find_symbols_by_name(&parsed.symbol_name).await?;
    if symbols.is_empty() {
        return Err(EngramError::CodeGraph(CodeGraphError::SymbolNotFound {
            name: parsed.symbol_name,
        }));
    }

    let mut links_created = 0u32;
    let mut links_existing = 0u32;

    for sym in &symbols {
        // Idempotency check (FR-152).
        let exists = cg_queries
            .concerns_edge_exists(bare_task_id, &sym.table, &sym.id)
            .await?;
        if exists {
            links_existing += 1;
            continue;
        }

        cg_queries
            .create_concerns_edge(bare_task_id, &sym.table, &sym.id, &parsed.linked_by)
            .await?;
        links_created += 1;
    }

    Ok(json!({
        "task_id": bare_task_id,
        "symbol_name": parsed.symbol_name,
        "links_created": links_created,
        "links_existing": links_existing,
        "total_matches": symbols.len(),
    }))
}

// ── unlink_task_from_code (T051) ────────────────────────────────────

/// Remove `concerns` edges between a task and all code symbols matching
/// the given `symbol_name`.
pub async fn unlink_task_from_code(
    state: SharedState,
    params: Option<Value>,
) -> Result<Value, EngramError> {
    #[derive(Debug, Deserialize)]
    struct Params {
        task_id: String,
        symbol_name: String,
    }

    let ws_id = workspace_id(&state).await?;

    let parsed: Params =
        serde_json::from_value(params.unwrap_or_else(|| json!({}))).map_err(|e| {
            EngramError::System(SystemError::InvalidParams {
                reason: e.to_string(),
            })
        })?;

    let db = connect_db(&ws_id).await?;
    let cg_queries = crate::db::queries::CodeGraphQueries::new(db.clone());
    let queries = Queries::new(db);

    // Verify task exists.
    let task = queries.get_task(&parsed.task_id).await?;
    if task.is_none() {
        return Err(EngramError::Task(TaskError::NotFound {
            id: parsed.task_id,
        }));
    }

    let deleted = cg_queries
        .delete_concerns_by_task_and_symbol_name(&parsed.task_id, &parsed.symbol_name)
        .await?;

    Ok(json!({
        "task_id": parsed.task_id,
        "symbol_name": parsed.symbol_name,
        "links_removed": deleted,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_done_to_blocked_transition() {
        let result = validate_transition(TaskStatus::Done, TaskStatus::Blocked);
        assert!(result.is_err());
    }
}

// ── Rollback ─────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct RollbackParams {
    event_id: String,
}

/// Revert workspace state to the point immediately after a specific event.
///
/// Requires `allow_agent_rollback = true` in workspace config (disabled by
/// default for safety). Returns the count of events reversed and the ID of
/// the new `rollback_applied` event added to the ledger.
pub async fn rollback_to_event(
    state: SharedState,
    params: Option<Value>,
) -> Result<Value, EngramError> {
    let ws_id = workspace_id(&state).await?;
    let parsed: RollbackParams =
        serde_json::from_value(params.unwrap_or_default()).map_err(|e| {
            EngramError::System(SystemError::InvalidParams {
                reason: format!("invalid params: {e}"),
            })
        })?;

    let (allow_rollback, max_events) = state
        .workspace_config()
        .await
        .map_or((false, 500_usize), |c| {
            (c.allow_agent_rollback, c.event_ledger_max)
        });

    let db = connect_db(&ws_id).await?;
    let queries = Queries::new(db);

    let events = event_ledger::prepare_rollback(&queries, &parsed.event_id, allow_rollback).await?;
    let (reversed_count, rollback_event_id) =
        event_ledger::apply_rollback(&queries, events, &parsed.event_id, "mcp-tool", max_events)
            .await?;

    Ok(json!({
        "events_reversed": reversed_count,
        "rollback_event_id": rollback_event_id,
        "target_event_id": parsed.event_id,
    }))
}

// ── Collection Write Tools (T086–T088) ────────────────────────────────────────

#[derive(Deserialize)]
struct CreateCollectionParams {
    name: String,
    #[serde(default)]
    description: Option<String>,
}

#[derive(Deserialize)]
struct AddToCollectionParams {
    collection_id: String,
    member_ids: Vec<String>,
}

#[derive(Deserialize)]
struct RemoveFromCollectionParams {
    collection_id: String,
    member_ids: Vec<String>,
}

/// Create a named collection for grouping tasks and sub-collections.
///
/// Returns `COLLECTION_EXISTS` (3030) if a collection with the same name
/// already exists in this workspace.
#[tracing::instrument(name = "tool.create_collection", skip(state, params))]
pub async fn create_collection(
    state: SharedState,
    params: Option<Value>,
) -> Result<Value, EngramError> {
    let ws_id = workspace_id(&state).await?;
    let parsed: CreateCollectionParams = serde_json::from_value(params.unwrap_or_default())
        .map_err(|e| {
            EngramError::System(SystemError::InvalidParams {
                reason: e.to_string(),
            })
        })?;

    if parsed.name.trim().is_empty() {
        return Err(EngramError::System(SystemError::InvalidParams {
            reason: "name must not be empty".to_string(),
        }));
    }

    let db = connect_db(&ws_id).await?;
    let queries = Queries::new(db);
    let collection = queries
        .create_collection(&parsed.name, parsed.description.as_deref())
        .await?;

    let max_events = state
        .workspace_config()
        .await
        .map_or(500, |c| c.event_ledger_max);
    if let Err(e) = event_ledger::record_event(
        &queries,
        EventKind::CollectionCreated,
        "collection",
        &collection.id,
        None,
        serde_json::to_value(&collection).ok(),
        "mcp-tool",
        max_events,
    )
    .await
    {
        tracing::warn!("event recording failed: {e}");
    }

    Ok(json!({
        "collection_id": collection.id,
        "name": collection.name,
        "description": collection.description,
        "created_at": collection.created_at.to_rfc3339(),
    }))
}

/// Add tasks or sub-collections to an existing collection.
///
/// Checks cycle safety before creating any `contains` edges. Returns
/// `CYCLIC_COLLECTION` (3032) if adding would create a cycle.
#[tracing::instrument(name = "tool.add_to_collection", skip(state, params))]
pub async fn add_to_collection(
    state: SharedState,
    params: Option<Value>,
) -> Result<Value, EngramError> {
    let ws_id = workspace_id(&state).await?;
    let parsed: AddToCollectionParams = serde_json::from_value(params.unwrap_or_default())
        .map_err(|e| {
            EngramError::System(SystemError::InvalidParams {
                reason: e.to_string(),
            })
        })?;

    let db = connect_db(&ws_id).await?;
    let queries = Queries::new(db);

    // Verify the target collection exists.
    queries
        .get_collection_by_id(&parsed.collection_id)
        .await?
        .ok_or_else(|| {
            EngramError::Collection(crate::errors::CollectionError::NotFound {
                name: parsed.collection_id.clone(),
            })
        })?;

    // Guard against cyclic nesting (collection members only).
    for member_id in &parsed.member_ids {
        if member_id.starts_with("collection:")
            && queries
                .check_collection_cycle(&parsed.collection_id, member_id)
                .await?
        {
            return Err(EngramError::Collection(
                crate::errors::CollectionError::CyclicCollection {
                    name: member_id.clone(),
                },
            ));
        }
    }

    let (added, already) = queries
        .add_collection_members(&parsed.collection_id, &parsed.member_ids)
        .await?;

    Ok(json!({
        "collection_id": parsed.collection_id,
        "added": added,
        "already_members": already,
    }))
}

/// Remove tasks or sub-collections from a collection.
///
/// Non-member IDs are counted in `not_members` and silently skipped.
#[tracing::instrument(name = "tool.remove_from_collection", skip(state, params))]
pub async fn remove_from_collection(
    state: SharedState,
    params: Option<Value>,
) -> Result<Value, EngramError> {
    let ws_id = workspace_id(&state).await?;
    let parsed: RemoveFromCollectionParams = serde_json::from_value(params.unwrap_or_default())
        .map_err(|e| {
            EngramError::System(SystemError::InvalidParams {
                reason: e.to_string(),
            })
        })?;

    let db = connect_db(&ws_id).await?;
    let queries = Queries::new(db);

    let (removed, not_member) = queries
        .remove_collection_members(&parsed.collection_id, &parsed.member_ids)
        .await?;

    Ok(json!({
        "collection_id": parsed.collection_id,
        "removed": removed,
        "not_members": not_member,
    }))
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
    let ws_id = workspace_id(&state).await?;

    let parsed: IndexGitHistoryParams = serde_json::from_value(params.unwrap_or_else(|| json!({})))
        .map_err(|e| {
            EngramError::System(SystemError::InvalidParams {
                reason: e.to_string(),
            })
        })?;

    let depth = parsed.depth.unwrap_or(0); // 0 → service uses default 500

    let db = connect_db(&ws_id).await?;
    let queries = Queries::new(db);

    let summary =
        crate::services::git_graph::index_git_history(&queries, &ws_path, depth, parsed.force)
            .await?;

    serde_json::to_value(&summary).map_err(|e| {
        EngramError::System(SystemError::DatabaseError {
            reason: format!("index_git_history serialization failed: {e}"),
        })
    })
}
