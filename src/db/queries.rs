#![allow(dead_code)]

use std::collections::{HashSet, VecDeque};

use chrono::{DateTime, Utc};
use serde::Deserialize;
use surrealdb::sql::Thing;

use crate::db::{Db, map_db_err};
use crate::errors::{TMemError, TaskError};
use crate::models::graph::DependencyType;
use crate::models::task::TaskStatus;
use crate::models::{Context, Task};

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
        let task_owned = task.clone();
        let _: Option<Task> = self
            .db
            .update(("task", task_owned.id.as_str()))
            .content(task_owned)
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
        let record = Thing::from(("task", id));
        let status_str = format_status(status).to_string();
        self.db
            .query("UPDATE $record MERGE { status: $status, updated_at: $updated }")
            .bind(("record", record))
            .bind(("status", status_str))
            .bind(("updated", updated_at))
            .await
            .map_err(map_db_err)?;
        Ok(())
    }

    pub async fn insert_context(&self, ctx: &Context) -> Result<(), TMemError> {
        let ctx_owned = ctx.clone();
        let _: Option<Context> = self
            .db
            .create(("context", ctx_owned.id.as_str()))
            .content(ctx_owned)
            .await
            .map_err(map_db_err)?;
        Ok(())
    }

    pub async fn link_task_context(
        &self,
        task_id: &str,
        context_id: &str,
    ) -> Result<(), TMemError> {
        let from = Thing::from(("task", task_id));
        let to = Thing::from(("context", context_id));
        self.db
            .query("RELATE $from->relates_to->$to SET created_at = time::now();")
            .bind(("from", from))
            .bind(("to", to))
            .await
            .map_err(map_db_err)?;
        Ok(())
    }

    pub async fn link_task_spec(&self, task_id: &str, spec_id: &str) -> Result<(), TMemError> {
        let from = Thing::from(("task", task_id));
        let to = Thing::from(("spec", spec_id));
        self.db
            .query("RELATE $from->implements->$to SET created_at = time::now();")
            .bind(("from", from))
            .bind(("to", to))
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

        let from = Thing::from(("task", dependent));
        let to = Thing::from(("task", blocker));
        self.db
            .query("RELATE $from->depends_on->$to SET type = $kind, created_at = time::now();")
            .bind(("from", from))
            .bind(("to", to))
            .bind(("kind", format_dependency(kind).to_string()))
            .await
            .map_err(map_db_err)?;
        Ok(())
    }

    pub async fn dependencies_of(&self, task_id: &str) -> Result<Vec<DependencyEdge>, TMemError> {
        let record = Thing::from(("task", task_id));
        let rows: Vec<DependsOnRow> = self
            .db
            .query("SELECT out, type FROM depends_on WHERE in = $record")
            .bind(("record", record))
            .await
            .map_err(map_db_err)?
            .take(0)
            .unwrap_or_default();

        let edges = rows
            .into_iter()
            .filter_map(|row| {
                let target = row.out.id.to_raw();
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
        let id_owned = work_item_id.to_string();
        let task: Option<Task> = self
            .db
            .query("SELECT * FROM task WHERE work_item_id = $id LIMIT 1")
            .bind(("id", id_owned))
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

        let ids_owned = ids.to_vec();
        let tasks: Vec<Task> = self
            .db
            .query("SELECT * FROM $ids")
            .bind(("ids", ids_owned))
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
    use chrono::Utc;
    use surrealdb::Surreal;
    use surrealdb::engine::local::SurrealKv;

    use super::*;
    use crate::db::schema;
    use crate::models::task::Task;

    /// Create an isolated in-directory embedded DB for testing.
    async fn test_db(dir: &std::path::Path) -> Queries {
        let db = Surreal::new::<SurrealKv>(dir.to_str().unwrap())
            .await
            .expect("embedded db");
        db.use_ns("test").use_db("cyclic").await.expect("ns/db");
        db.query(schema::DEFINE_TASK).await.expect("schema task");
        db.query(schema::DEFINE_RELATIONSHIPS)
            .await
            .expect("schema rels");
        Queries::new(db)
    }

    fn make_task(id: &str) -> Task {
        let now = Utc::now();
        Task {
            id: id.to_string(),
            title: format!("Task {id}"),
            status: TaskStatus::Todo,
            work_item_id: None,
            description: String::new(),
            context_summary: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[tokio::test]
    async fn self_dependency_rejected() {
        let dir = tempfile::tempdir().expect("tempdir");
        let q = test_db(dir.path()).await;
        q.upsert_task(&make_task("a")).await.expect("insert a");

        let err = q
            .create_dependency("a", "a", DependencyType::HardBlocker)
            .await
            .expect_err("self-dep must fail");

        assert!(
            matches!(err, TMemError::Task(TaskError::CyclicDependency)),
            "expected CyclicDependency, got {err:?}"
        );
    }

    #[tokio::test]
    async fn direct_cycle_rejected() {
        let dir = tempfile::tempdir().expect("tempdir");
        let q = test_db(dir.path()).await;
        q.upsert_task(&make_task("a")).await.expect("insert a");
        q.upsert_task(&make_task("b")).await.expect("insert b");

        // a depends on b
        q.create_dependency("a", "b", DependencyType::HardBlocker)
            .await
            .expect("a->b ok");

        // b depends on a would create a cycle
        let err = q
            .create_dependency("b", "a", DependencyType::HardBlocker)
            .await
            .expect_err("cycle must fail");

        assert!(
            matches!(err, TMemError::Task(TaskError::CyclicDependency)),
            "expected CyclicDependency, got {err:?}"
        );
    }

    #[tokio::test]
    async fn transitive_cycle_rejected() {
        let dir = tempfile::tempdir().expect("tempdir");
        let q = test_db(dir.path()).await;
        q.upsert_task(&make_task("a")).await.expect("insert a");
        q.upsert_task(&make_task("b")).await.expect("insert b");
        q.upsert_task(&make_task("c")).await.expect("insert c");

        // a -> b -> c
        q.create_dependency("a", "b", DependencyType::HardBlocker)
            .await
            .expect("a->b ok");
        q.create_dependency("b", "c", DependencyType::HardBlocker)
            .await
            .expect("b->c ok");

        // c -> a would create a transitive cycle
        let err = q
            .create_dependency("c", "a", DependencyType::HardBlocker)
            .await
            .expect_err("transitive cycle must fail");

        assert!(
            matches!(err, TMemError::Task(TaskError::CyclicDependency)),
            "expected CyclicDependency, got {err:?}"
        );
    }

    #[tokio::test]
    async fn valid_dag_accepted() {
        let dir = tempfile::tempdir().expect("tempdir");
        let q = test_db(dir.path()).await;
        q.upsert_task(&make_task("a")).await.expect("insert a");
        q.upsert_task(&make_task("b")).await.expect("insert b");
        q.upsert_task(&make_task("c")).await.expect("insert c");

        // Valid DAG: a -> b, a -> c, b -> c (diamond, no cycle)
        q.create_dependency("a", "b", DependencyType::HardBlocker)
            .await
            .expect("a->b ok");
        q.create_dependency("a", "c", DependencyType::SoftDependency)
            .await
            .expect("a->c ok");
        q.create_dependency("b", "c", DependencyType::HardBlocker)
            .await
            .expect("b->c ok (diamond, no cycle)");
    }
}
