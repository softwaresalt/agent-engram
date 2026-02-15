use std::path::PathBuf;

use chrono::Utc;
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::config::StaleStrategy;
use crate::db::connect_db;
use crate::db::queries::Queries;
use crate::errors::{SystemError, TMemError, TaskError, WorkspaceError};
use crate::models::context::Context;
use crate::models::task::{Task, TaskStatus, compute_priority_order};
use crate::server::state::SharedState;
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
}

#[derive(Deserialize)]
struct LabelParams {
    task_id: String,
    label: String,
}

async fn workspace_path(state: &SharedState) -> Result<PathBuf, TMemError> {
    if let Some(snapshot) = state.snapshot_workspace().await {
        return Ok(PathBuf::from(snapshot.path));
    }
    Err(TMemError::Workspace(WorkspaceError::NotSet))
}

async fn workspace_id(state: &SharedState) -> Result<String, TMemError> {
    if let Some(snapshot) = state.snapshot_workspace().await {
        return Ok(snapshot.workspace_id);
    }
    Err(TMemError::Workspace(WorkspaceError::NotSet))
}

fn parse_status(raw: &str) -> Result<TaskStatus, TMemError> {
    match raw {
        "todo" => Ok(TaskStatus::Todo),
        "in_progress" => Ok(TaskStatus::InProgress),
        "done" => Ok(TaskStatus::Done),
        "blocked" => Ok(TaskStatus::Blocked),
        _ => Err(TMemError::Task(TaskError::InvalidStatus {
            status: raw.to_string(),
        })),
    }
}

fn validate_transition(from: TaskStatus, to: TaskStatus) -> Result<(), TMemError> {
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
        Err(TMemError::Task(TaskError::InvalidStatus {
            status: format!("{}->{}", from.as_str(), to.as_str()),
        }))
    }
}

pub async fn update_task(state: SharedState, params: Option<Value>) -> Result<Value, TMemError> {
    let workspace_id = workspace_id(&state).await?;
    let workspace_path = workspace_path(&state).await?;
    let parsed: UpdateTaskParams =
        serde_json::from_value(params.unwrap_or_default()).map_err(|e| {
            TMemError::System(SystemError::InvalidParams {
                reason: format!("invalid params: {e}"),
            })
        })?;

    let new_status = parse_status(&parsed.status)?;
    let now = Utc::now();

    let db = connect_db(&workspace_id).await?;
    let queries = Queries::new(db.clone());

    let mut existing = queries.get_task(&parsed.id).await?;
    if existing.is_none() {
        hydration::hydrate_into_db(&workspace_path, &queries).await?;
        existing = queries.get_task(&parsed.id).await?;
    }

    let existing = existing.ok_or_else(|| {
        TMemError::Task(TaskError::NotFound {
            id: parsed.id.clone(),
        })
    })?;

    let previous_status = existing.status;

    validate_transition(previous_status, new_status)?;

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
        issue_type: existing.issue_type,
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

pub async fn add_blocker(state: SharedState, params: Option<Value>) -> Result<Value, TMemError> {
    let workspace_id = workspace_id(&state).await?;
    let workspace_path = workspace_path(&state).await?;
    let parsed: AddBlockerParams =
        serde_json::from_value(params.unwrap_or_default()).map_err(|e| {
            TMemError::System(SystemError::InvalidParams {
                reason: format!("invalid params: {e}"),
            })
        })?;

    let now = Utc::now();
    let db = connect_db(&workspace_id).await?;
    let queries = Queries::new(db.clone());

    let task_id = parsed.task_id.clone();

    let mut task = queries.get_task(&task_id).await?;
    if task.is_none() {
        hydration::hydrate_into_db(&workspace_path, &queries).await?;
        task = queries.get_task(&task_id).await?;
    }

    let task = task.ok_or_else(|| {
        TMemError::Task(TaskError::NotFound {
            id: task_id.clone(),
        })
    })?;

    if task.status == TaskStatus::Blocked {
        return Err(TMemError::Task(TaskError::BlockerExists { id: task_id }));
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
) -> Result<Value, TMemError> {
    let workspace_id = workspace_id(&state).await?;
    let parsed: RegisterDecisionParams = serde_json::from_value(params.unwrap_or_default())
        .map_err(|e| {
            TMemError::System(SystemError::InvalidParams {
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
        "file_path": ".tmem/decisions.md",
        "created_at": now.to_rfc3339(),
        "topic": parsed.topic,
    }))
}

/// Create a new task with `todo` status and optional parent dependency.
pub async fn create_task(state: SharedState, params: Option<Value>) -> Result<Value, TMemError> {
    let workspace_id = workspace_id(&state).await?;
    let parsed: CreateTaskParams =
        serde_json::from_value(params.unwrap_or_default()).map_err(|e| {
            TMemError::System(SystemError::InvalidParams {
                reason: format!("invalid params: {e}"),
            })
        })?;

    let title = parsed.title.trim();
    if title.is_empty() || title.len() > MAX_TITLE_LEN {
        return Err(TMemError::Task(TaskError::TitleEmpty));
    }

    let db = connect_db(&workspace_id).await?;
    let queries = Queries::new(db);

    let task = queries
        .create_task(
            title,
            parsed.description.as_deref().unwrap_or(""),
            parsed.work_item_id.as_deref(),
            parsed.parent_task_id.as_deref(),
        )
        .await?;

    let mut response = json!({
        "task_id": task.id,
        "title": task.title,
        "status": "todo",
        "created_at": task.created_at.to_rfc3339(),
    });

    if let Some(parent) = &parsed.parent_task_id {
        response["parent_task_id"] = json!(parent);
    }

    Ok(response)
}
pub async fn flush_state(state: SharedState, params: Option<Value>) -> Result<Value, TMemError> {
    // T092: Acquire per-workspace write lock for FIFO serialization of concurrent flushes
    let _flush_guard = dehydration::acquire_flush_lock().await;
    let snapshot = state
        .snapshot_workspace()
        .await
        .ok_or(TMemError::Workspace(WorkspaceError::NotSet))?;

    let path = PathBuf::from(&snapshot.path);
    let workspace_id = snapshot.workspace_id.clone();
    let tmem_dir = path.join(".tmem");
    let stale_strategy = state.stale_strategy();
    let mut warnings: Vec<String> = Vec::new();
    let is_stale =
        snapshot.stale_files || hydration::detect_stale_since(&snapshot.file_mtimes, &tmem_dir);

    let _ = params;

    let db = connect_db(&workspace_id).await?;
    let queries = Queries::new(db.clone());

    // Determine staleness action from strategy before touching the DB
    match (is_stale, stale_strategy) {
        (true, StaleStrategy::Fail) => {
            return Err(TMemError::Hydration(
                crate::errors::HydrationError::StaleWorkspace,
            ));
        }
        (true, StaleStrategy::Warn) => {
            warnings.push("2004 StaleWorkspace: .tmem files modified externally".to_string());
        }
        (true, StaleStrategy::Rehydrate) => {
            hydration::hydrate_into_db(&path, &queries).await?;
        }
        (false, _) => {}
    }

    let result = dehydration::dehydrate_workspace(&queries, &path).await?;
    let new_mtimes = hydration::collect_file_mtimes(&tmem_dir);

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

pub async fn add_label(state: SharedState, params: Option<Value>) -> Result<Value, TMemError> {
    let ws_id = workspace_id(&state).await?;
    let parsed: LabelParams =
        serde_json::from_value(params.unwrap_or_else(|| json!({}))).map_err(|e| {
            TMemError::System(SystemError::InvalidParams {
                reason: e.to_string(),
            })
        })?;

    // Validate against allowed_labels if configured
    if let Some(config) = state.workspace_config().await {
        if !config.allowed_labels.is_empty() && !config.allowed_labels.contains(&parsed.label) {
            return Err(TMemError::Task(TaskError::LabelValidation {
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

pub async fn remove_label(state: SharedState, params: Option<Value>) -> Result<Value, TMemError> {
    let ws_id = workspace_id(&state).await?;
    let parsed: LabelParams =
        serde_json::from_value(params.unwrap_or_else(|| json!({}))).map_err(|e| {
            TMemError::System(SystemError::InvalidParams {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_done_to_blocked_transition() {
        let result = validate_transition(TaskStatus::Done, TaskStatus::Blocked);
        assert!(result.is_err());
    }
}
