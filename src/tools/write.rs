use std::fs;
use std::path::PathBuf;

use chrono::Utc;
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::db::connect_db;
use crate::db::queries::Queries;
use crate::errors::{SystemError, TMemError, TaskError, WorkspaceError};
use crate::models::context::Context;
use crate::models::task::{Task, TaskStatus};
use crate::server::state::SharedState;
use crate::services::connection::create_status_change_note;

#[derive(Deserialize)]
struct UpdateTaskParams {
    id: String,
    status: String,
    #[serde(default)]
    notes: Option<String>,
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

pub async fn update_task(state: SharedState, params: Option<Value>) -> Result<Value, TMemError> {
    let workspace_id = workspace_id(&state).await?;
    let parsed: UpdateTaskParams =
        serde_json::from_value(params.unwrap_or_default()).map_err(|e| {
            TMemError::System(SystemError::DatabaseError {
                reason: format!("invalid params: {e}"),
            })
        })?;

    let new_status = parse_status(&parsed.status)?;
    let now = Utc::now();

    let db = connect_db(&workspace_id).await?;
    let queries = Queries::new(db.clone());

    let existing = queries.get_task(&parsed.id).await?.ok_or_else(|| {
        TMemError::Task(TaskError::NotFound {
            id: parsed.id.clone(),
        })
    })?;

    let previous_status = existing.status;

    let updated = Task {
        id: parsed.id.clone(),
        title: existing.title,
        status: new_status,
        work_item_id: existing.work_item_id,
        description: existing.description,
        context_summary: existing.context_summary,
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
        "previous_status": format_status(previous_status),
        "new_status": format_status(new_status),
        "context_id": context_id,
        "updated_at": now.to_rfc3339(),
    }))
}

pub async fn add_blocker(state: SharedState, params: Option<Value>) -> Result<Value, TMemError> {
    let workspace_id = workspace_id(&state).await?;
    let parsed: AddBlockerParams =
        serde_json::from_value(params.unwrap_or_default()).map_err(|e| {
            TMemError::System(SystemError::DatabaseError {
                reason: format!("invalid params: {e}"),
            })
        })?;

    let now = Utc::now();
    let db = connect_db(&workspace_id).await?;
    let queries = Queries::new(db.clone());

    let task_id = parsed.task_id.clone();

    let task = queries.get_task(&task_id).await?.ok_or_else(|| {
        TMemError::Task(TaskError::NotFound {
            id: task_id.clone(),
        })
    })?;

    if task.status == TaskStatus::Blocked {
        return Err(TMemError::Task(TaskError::BlockerExists { id: task_id }));
    }

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

    queries
        .set_task_status(&task.id, TaskStatus::Blocked, now)
        .await?;

    Ok(json!({
        "task_id": task.id,
        "blocker_context_id": ctx_id,
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
            TMemError::System(SystemError::DatabaseError {
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

pub async fn flush_state(state: SharedState, params: Option<Value>) -> Result<Value, TMemError> {
    let path = workspace_path(&state).await?;
    let workspace_id = workspace_id(&state).await?;
    let _ = params;

    let tmem_dir = path.join(".tmem");
    fs::create_dir_all(&tmem_dir).map_err(|_| {
        TMemError::System(SystemError::FlushFailed {
            path: tmem_dir.display().to_string(),
        })
    })?;

    let flush_ts = Utc::now().to_rfc3339();
    let lastflush_path = tmem_dir.join(".lastflush");
    fs::write(&lastflush_path, &flush_ts).map_err(|_| {
        TMemError::System(SystemError::FlushFailed {
            path: lastflush_path.display().to_string(),
        })
    })?;

    // Serialize tasks to tasks.md
    let db = connect_db(&workspace_id).await?;
    let queries = Queries::new(db.clone());
    let tasks: Vec<Task> = queries.all_tasks().await?;

    let mut content = String::from("# Tasks\n\n");
    for task in tasks {
        content.push_str(&format!(
            "## {}\n---\nstatus: {}\nupdated_at: {}\n---\n\n{}\n\n",
            task.id,
            format_status(task.status),
            task.updated_at.to_rfc3339(),
            task.description
        ));
    }
    let tasks_path = tmem_dir.join("tasks.md");
    fs::write(&tasks_path, content).map_err(|_| {
        TMemError::System(SystemError::FlushFailed {
            path: tasks_path.display().to_string(),
        })
    })?;

    Ok(json!({
        "files_written": [".tmem/.lastflush", ".tmem/tasks.md"],
        "warnings": [],
        "flush_timestamp": flush_ts,
    }))
}

fn format_status(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Todo => "todo",
        TaskStatus::InProgress => "in_progress",
        TaskStatus::Done => "done",
        TaskStatus::Blocked => "blocked",
    }
}
