use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::db::connect_db;
use crate::db::queries::{Queries, ReadyWorkParams};
use crate::errors::{SystemError, TMemError, TaskError, WorkspaceError};
use crate::models::task::Task;
use crate::server::state::SharedState;
use crate::services::embedding;
use crate::services::search::{SearchCandidate, hybrid_search};

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
    children: Vec<EdgeNode>,
}

#[derive(Serialize)]
struct EdgeNode {
    dependency_type: String,
    #[serde(flatten)]
    node: TaskNode,
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

pub async fn get_task_graph(state: SharedState, params: Option<Value>) -> Result<Value, TMemError> {
    ensure_workspace(&state).await?;

    let parsed: TaskGraphParams =
        serde_json::from_value(params.unwrap_or_default()).map_err(|e| {
            TMemError::System(SystemError::InvalidParams {
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
            TMemError::System(SystemError::InvalidParams {
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
                    "status": task.status.as_str().to_string(),
                    "updated_at": task.updated_at.to_rfc3339(),
                }),
            );
        }
    }

    Ok(json!({ "statuses": statuses }))
}

// ── get_ready_work ────────────────────────────────────────────────

#[derive(Deserialize)]
struct GetReadyWorkParams {
    #[serde(default = "default_ready_limit")]
    limit: u32,
    #[serde(default)]
    label: Option<Vec<String>>,
    #[serde(default)]
    priority: Option<String>,
    #[serde(default)]
    issue_type: Option<String>,
    #[serde(default)]
    assignee: Option<String>,
    #[serde(default)]
    brief: bool,
    #[serde(default)]
    fields: Option<Vec<String>>,
}

fn default_ready_limit() -> u32 {
    10
}

/// Get prioritized list of actionable tasks.
pub async fn get_ready_work(state: SharedState, params: Option<Value>) -> Result<Value, TMemError> {
    ensure_workspace(&state).await?;

    let parsed: GetReadyWorkParams =
        serde_json::from_value(params.unwrap_or_default()).map_err(|e| {
            TMemError::System(SystemError::InvalidParams {
                reason: format!("invalid params: {e}"),
            })
        })?;

    let workspace_id = workspace_id(&state).await?;
    let db = connect_db(&workspace_id).await?;
    let queries = Queries::new(db.clone());

    let query_params = ReadyWorkParams {
        limit: parsed.limit,
        labels: parsed.label.unwrap_or_default(),
        priority: parsed.priority,
        issue_type: parsed.issue_type,
        assignee: parsed.assignee,
    };

    let result = queries.get_ready_work(&query_params).await?;

    let task_values: Vec<Value> = result
        .tasks
        .into_iter()
        .map(|t| serialize_task(&t, parsed.brief, parsed.fields.as_deref()))
        .collect();

    Ok(json!({
        "tasks": task_values,
        "total_eligible": result.total_eligible,
    }))
}

/// Serialize a task to JSON, optionally applying brief mode or field filtering.
fn serialize_task(task: &Task, brief: bool, fields: Option<&[String]>) -> Value {
    if brief {
        return json!({
            "id": task.id,
            "title": task.title,
            "status": task.status.as_str(),
            "priority": task.priority,
            "assignee": task.assignee,
        });
    }

    let full = json!({
        "id": task.id,
        "title": task.title,
        "status": task.status.as_str(),
        "priority": task.priority,
        "priority_order": task.priority_order,
        "issue_type": task.issue_type,
        "assignee": task.assignee,
        "description": task.description,
        "context_summary": task.context_summary,
        "pinned": task.pinned,
        "defer_until": task.defer_until.map(|d| d.to_rfc3339()),
        "compaction_level": task.compaction_level,
        "created_at": task.created_at.to_rfc3339(),
        "updated_at": task.updated_at.to_rfc3339(),
    });

    if let Some(fields) = fields {
        let obj = full.as_object().unwrap();
        let filtered: serde_json::Map<String, Value> = obj
            .iter()
            .filter(|(k, _)| fields.iter().any(|f| f == *k))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        Value::Object(filtered)
    } else {
        full
    }
}

#[derive(Deserialize)]
struct QueryMemoryParams {
    query: String,
    #[serde(default = "default_limit")]
    limit: usize,
}

fn default_limit() -> usize {
    10
}

pub async fn query_memory(state: SharedState, params: Option<Value>) -> Result<Value, TMemError> {
    ensure_workspace(&state).await?;

    let parsed: QueryMemoryParams =
        serde_json::from_value(params.unwrap_or_default()).map_err(|e| {
            TMemError::System(SystemError::InvalidParams {
                reason: format!("invalid params: {e}"),
            })
        })?;

    // Validate query length before any DB or model work.
    embedding::validate_query_length(&parsed.query)?;

    let workspace_id = workspace_id(&state).await?;
    let db = connect_db(&workspace_id).await?;
    let queries = Queries::new(db.clone());

    // Gather candidates from specs, tasks, and contexts.
    let mut candidates: Vec<SearchCandidate> = Vec::new();

    let specs = queries.all_specs().await?;
    for spec in specs {
        candidates.push(SearchCandidate {
            id: format!("spec:{}", spec.id),
            source_type: "spec".to_string(),
            content: format!("{}\n{}", spec.title, spec.content),
            embedding: spec.embedding,
        });
    }

    let tasks = queries.all_tasks().await?;
    for task in tasks {
        let text = format!(
            "{}\n{}{}",
            task.title,
            task.description,
            task.context_summary
                .as_deref()
                .map_or_else(String::new, |s| format!("\n{s}"))
        );
        candidates.push(SearchCandidate {
            id: format!("task:{}", task.id),
            source_type: "task".to_string(),
            content: text,
            embedding: None,
        });
    }

    let contexts = queries.all_contexts().await?;
    for ctx in contexts {
        candidates.push(SearchCandidate {
            id: format!("context:{}", ctx.id),
            source_type: "context".to_string(),
            content: ctx.content,
            embedding: ctx.embedding,
        });
    }

    let results = hybrid_search(&parsed.query, &candidates, parsed.limit)?;

    Ok(json!({ "results": results }))
}

fn build_node(
    queries: &Queries,
    task: Task,
    depth: u32,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<TaskNode, TMemError>> + Send + '_>> {
    Box::pin(async move {
        if depth == 0 {
            return Ok(TaskNode {
                id: task.id,
                status: task.status.as_str().to_string(),
                children: Vec::new(),
            });
        }

        let edges = queries.dependencies_of(&task.id).await?;
        let mut children = Vec::new();

        for edge in edges {
            if let Some(child_task) = queries.get_task(&edge.to).await? {
                let child = build_node(queries, child_task, depth - 1).await?;
                children.push(EdgeNode {
                    dependency_type: serde_json::to_value(edge.kind)
                        .ok()
                        .and_then(|v| v.as_str().map(String::from))
                        .unwrap_or_else(|| "unknown".to_string()),
                    node: child,
                });
            }
        }

        Ok(TaskNode {
            id: task.id,
            status: task.status.as_str().to_string(),
            children,
        })
    })
}
