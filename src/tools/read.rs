use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::db::connect_db;
use crate::db::queries::Queries;
use crate::errors::{SystemError, TMemError, TaskError, WorkspaceError};
use crate::models::task::{Task, TaskStatus};
use crate::server::state::SharedState;

#[derive(Deserialize)]
struct TaskGraphParams {
    root_task_id: String,
    #[serde(default = "default_depth")]
    depth: u32,
}

#[derive(Deserialize)]
struct CheckStatusParams {
    work_item_ids: Vec<String>,
}

#[derive(Serialize)]
struct TaskNode {
    id: String,
    status: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    children: Vec<TaskNode>,
}

fn default_depth() -> u32 {
    5
}

async fn ensure_workspace(state: &SharedState) -> Result<(), TMemError> {
    if state.snapshot_workspace().await.is_none() {
        return Err(TMemError::Workspace(WorkspaceError::NotSet));
    }
    Ok(())
}

async fn workspace_id(state: &SharedState) -> Result<String, TMemError> {
    if let Some(snapshot) = state.snapshot_workspace().await {
        return Ok(snapshot.workspace_id);
    }
    Err(TMemError::Workspace(WorkspaceError::NotSet))
}

fn not_implemented(method: &str) -> Result<Value, TMemError> {
    Err(TMemError::System(SystemError::DatabaseError {
        reason: format!("{method} not implemented"),
    }))
}

pub async fn get_task_graph(state: SharedState, params: Option<Value>) -> Result<Value, TMemError> {
    ensure_workspace(&state).await?;

    let parsed: TaskGraphParams =
        serde_json::from_value(params.unwrap_or_default()).map_err(|e| {
            TMemError::System(SystemError::DatabaseError {
                reason: format!("invalid params: {e}"),
            })
        })?;

    let workspace_id = workspace_id(&state).await?;
    let db = connect_db(&workspace_id).await?;
    let queries = Queries::new(db.clone());

    let root = queries
        .get_task(&parsed.root_task_id)
        .await?
        .ok_or_else(|| {
            TMemError::Task(TaskError::NotFound {
                id: parsed.root_task_id.clone(),
            })
        })?;

    let root_node = build_node(&queries, root, parsed.depth).await?;

    Ok(json!({
        "root": root_node,
    }))
}

pub async fn check_status(state: SharedState, params: Option<Value>) -> Result<Value, TMemError> {
    ensure_workspace(&state).await?;

    let parsed: CheckStatusParams =
        serde_json::from_value(params.unwrap_or_default()).map_err(|e| {
            TMemError::System(SystemError::DatabaseError {
                reason: format!("invalid params: {e}"),
            })
        })?;

    let mut statuses = serde_json::Map::new();
    let workspace_id = workspace_id(&state).await?;
    let db = connect_db(&workspace_id).await?;
    let queries = Queries::new(db.clone());

    for id in parsed.work_item_ids {
        let task = queries.task_by_work_item(&id).await?;

        if let Some(task) = task {
            statuses.insert(
                id.clone(),
                json!({
                    "task_id": task.id,
                    "status": format_status(task.status),
                    "updated_at": task.updated_at.to_rfc3339(),
                }),
            );
        }
    }

    Ok(json!({ "statuses": statuses }))
}

pub async fn query_memory(state: SharedState, params: Option<Value>) -> Result<Value, TMemError> {
    ensure_workspace(&state).await?;
    let _ = params;
    not_implemented("query_memory")
}

async fn build_node(queries: &Queries, task: Task, depth: u32) -> Result<TaskNode, TMemError> {
    if depth == 0 {
        return Ok(TaskNode {
            id: task.id,
            status: format_status(task.status),
            children: Vec::new(),
        });
    }

    let edges = queries.dependencies_of(&task.id).await?;
    let mut children = Vec::new();

    for edge in edges {
        if let Some(child_task) = queries.get_task(&edge.to).await? {
            let child = build_node(queries, child_task, depth - 1).await?;
            children.push(child);
        }
    }

    Ok(TaskNode {
        id: task.id,
        status: format_status(task.status),
        children,
    })
}

fn format_status(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Todo => "todo",
        TaskStatus::InProgress => "in_progress",
        TaskStatus::Done => "done",
        TaskStatus::Blocked => "blocked",
    }
}
