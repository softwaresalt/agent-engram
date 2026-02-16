use std::path::PathBuf;

use chrono::Utc;
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::config::StaleStrategy;
use crate::db::connect_db;
use crate::db::queries::Queries;
use crate::errors::{EngramError, SystemError, TaskError, WorkspaceError};
use crate::models::context::Context;
use crate::models::graph::DependencyType;
use crate::models::task::{Task, TaskStatus, compute_priority_order};
use crate::server::state::SharedState;
use crate::services::compaction::truncate_at_word_boundary;
use crate::services::connection::create_status_change_note;
use crate::services::dehydration;
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

    let previous_status = existing.status;

    validate_transition(previous_status, new_status)?;

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

    Ok(json!({
        "task_id": parsed.id,
        "previous_status": previous_status.as_str(),
        "new_status": new_status.as_str(),
        "context_id": context_id,
        "updated_at": now.to_rfc3339(),
    }))
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
        "files_written": result.files_written,
        "warnings": warnings,
        "flush_timestamp": result.flush_timestamp,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_done_to_blocked_transition() {
        let result = validate_transition(TaskStatus::Done, TaskStatus::Blocked);
        assert!(result.is_err());
    }
}
