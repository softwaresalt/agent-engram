#![allow(dead_code)]

use std::collections::{HashSet, VecDeque};

use chrono::{DateTime, Utc};
use serde::Deserialize;
use surrealdb::sql::Thing;

use crate::db::{Db, map_db_err};
use crate::errors::{TMemError, TaskError};
use crate::models::graph::DependencyType;
use crate::models::task::TaskStatus;
use crate::models::{Context, Spec, Task};

/// Relationship edge carrying normalized task IDs and dependency type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyEdge {
    pub from: String,
    pub to: String,
    pub kind: DependencyType,
}

#[derive(Deserialize)]
struct DependsOnRow {
    out: Thing,
    #[serde(default)]
    r#type: Option<String>,
}

/// Query helper wrapping SurrealDB handle.
#[derive(Clone)]
pub struct Queries {
    db: Db,
}

impl Queries {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    pub async fn upsert_task(&self, task: &Task) -> Result<(), TMemError> {
        self.db
            .update(("task", task.id.as_str()))
            .content(task)
            .await
            .map_err(map_db_err)?;
        Ok(())
    }

    pub async fn get_task(&self, id: &str) -> Result<Option<Task>, TMemError> {
        let task: Option<Task> = self.db.select(("task", id)).await.map_err(map_db_err)?;
        Ok(task)
    }

    pub async fn set_task_status(
        &self,
        id: &str,
        status: TaskStatus,
        updated_at: DateTime<Utc>,
    ) -> Result<(), TMemError> {
        self.db
            .query(
                "UPDATE type::thing('task', $id) MERGE { status: $status, updated_at: $updated }",
            )
            .bind(("id", id))
            .bind(("status", format_status(status)))
            .bind(("updated", updated_at))
            .await
            .map_err(map_db_err)?;
        Ok(())
    }

    pub async fn insert_context(&self, ctx: &Context) -> Result<(), TMemError> {
        self.db
            .create(("context", ctx.id.as_str()))
            .content(ctx)
            .await
            .map_err(map_db_err)?;
        Ok(())
    }

    pub async fn link_task_context(
        &self,
        task_id: &str,
        context_id: &str,
    ) -> Result<(), TMemError> {
        self.db
            .query("RELATE type::thing('task', $task)->relates_to->type::thing('context', $ctx) SET created_at = time::now();")
            .bind(("task", task_id))
            .bind(("ctx", context_id))
            .await
            .map_err(map_db_err)?;
        Ok(())
    }

    pub async fn link_task_spec(&self, task_id: &str, spec_id: &str) -> Result<(), TMemError> {
        self.db
            .query("RELATE type::thing('task', $task)->implements->type::thing('spec', $spec) SET created_at = time::now();")
            .bind(("task", task_id))
            .bind(("spec", spec_id))
            .await
            .map_err(map_db_err)?;
        Ok(())
    }

    pub async fn create_dependency(
        &self,
        dependent: &str,
        blocker: &str,
        kind: DependencyType,
    ) -> Result<(), TMemError> {
        if dependent == blocker {
            return Err(TMemError::Task(TaskError::CyclicDependency));
        }

        if self.detect_cycle(blocker, dependent).await? {
            return Err(TMemError::Task(TaskError::CyclicDependency));
        }

        self.db
            .query(
                "RELATE type::thing('task', $dependent)->depends_on->type::thing('task', $blocker) \
                 SET type = $kind, created_at = time::now();",
            )
            .bind(("dependent", dependent))
            .bind(("blocker", blocker))
            .bind(("kind", format_dependency(kind)))
            .await
            .map_err(map_db_err)?;
        Ok(())
    }

    pub async fn dependencies_of(&self, task_id: &str) -> Result<Vec<DependencyEdge>, TMemError> {
        let rows: Vec<DependsOnRow> = self
            .db
            .query("SELECT out, type FROM depends_on WHERE in = $id")
            .bind(("id", task_id))
            .await
            .map_err(map_db_err)?
            .take(0)
            .unwrap_or_default();

        let edges = rows
            .into_iter()
            .filter_map(|row| {
                let target = row.out.to_string();
                let kind = row
                    .r#type
                    .map(parse_dependency_type)
                    .unwrap_or(DependencyType::HardBlocker);
                Some(DependencyEdge {
                    from: task_id.to_string(),
                    to: target,
                    kind,
                })
            })
            .collect();

        Ok(edges)
    }

    pub async fn task_by_work_item(&self, work_item_id: &str) -> Result<Option<Task>, TMemError> {
        let task: Option<Task> = self
            .db
            .query("SELECT * FROM task WHERE work_item_id = $id LIMIT 1")
            .bind(("id", work_item_id))
            .await
            .map_err(map_db_err)?
            .take(0)
            .unwrap_or_default();
        Ok(task)
    }

    pub async fn all_tasks(&self) -> Result<Vec<Task>, TMemError> {
        let tasks: Vec<Task> = self
            .db
            .query("SELECT * FROM task")
            .await
            .map_err(map_db_err)?
            .take(0)
            .unwrap_or_default();
        Ok(tasks)
    }

    pub async fn tasks_by_ids(&self, ids: &[String]) -> Result<Vec<Task>, TMemError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let tasks: Vec<Task> = self
            .db
            .query("SELECT * FROM $ids")
            .bind(("ids", ids))
            .await
            .map_err(map_db_err)?
            .take(0)
            .unwrap_or_default();
        Ok(tasks)
    }

    async fn detect_cycle(&self, start: &str, target: &str) -> Result<bool, TMemError> {
        let mut visited: HashSet<String> = HashSet::new();
        let mut queue: VecDeque<String> = VecDeque::from([start.to_string()]);

        while let Some(node) = queue.pop_front() {
            if !visited.insert(node.clone()) {
                continue;
            }
            if node == target {
                return Ok(true);
            }

            let edges = self.dependencies_of(&node).await?;
            for edge in edges {
                if !visited.contains(&edge.to) {
                    queue.push_back(edge.to);
                }
            }
        }

        Ok(false)
    }
}

fn format_status(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Todo => "todo",
        TaskStatus::InProgress => "in_progress",
        TaskStatus::Done => "done",
        TaskStatus::Blocked => "blocked",
    }
}

fn format_dependency(kind: DependencyType) -> &'static str {
    match kind {
        DependencyType::HardBlocker => "hard_blocker",
        DependencyType::SoftDependency => "soft_dependency",
    }
}

fn parse_dependency_type(raw: String) -> DependencyType {
    match raw.as_str() {
        "soft_dependency" => DependencyType::SoftDependency,
        _ => DependencyType::HardBlocker,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn cyclic_dependency_detection_placeholder() {
        todo!("cyclic dependency detection test not implemented yet");
    }
}
