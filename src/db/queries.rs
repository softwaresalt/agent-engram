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

/// Task-to-Spec implementation edge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImplementsEdge {
    pub task_id: String,
    pub spec_id: String,
}

/// Task-to-Context relation edge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelatesToEdge {
    pub task_id: String,
    pub context_id: String,
}

#[derive(Deserialize)]
struct DependsOnRow {
    r#in: Thing,
    out: Thing,
    #[serde(default)]
    r#type: Option<String>,
}

#[derive(Deserialize)]
struct RelationRow {
    r#in: Thing,
    out: Thing,
}

/// Internal row type for deserializing tasks from SurrealDB.
///
/// SurrealDB v2 returns `id` as a `Thing` (not `String`), so we
/// deserialize into this struct then convert to the public `Task`.
#[derive(Deserialize)]
struct TaskRow {
    id: Thing,
    title: String,
    status: TaskStatus,
    #[serde(default)]
    work_item_id: Option<String>,
    #[serde(default)]
    description: String,
    #[serde(default)]
    context_summary: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TaskRow {
    fn into_task(self) -> Task {
        Task {
            id: self.id.id.to_raw(),
            title: self.title,
            status: self.status,
            work_item_id: self.work_item_id,
            description: self.description,
            context_summary: self.context_summary,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

/// Internal row type for deserializing contexts from SurrealDB.
#[derive(Deserialize)]
struct ContextRow {
    id: Thing,
    content: String,
    #[serde(default)]
    embedding: Option<Vec<f32>>,
    source_client: String,
    created_at: DateTime<Utc>,
}

impl ContextRow {
    fn into_context(self) -> Context {
        Context {
            id: self.id.id.to_raw(),
            content: self.content,
            embedding: self.embedding,
            source_client: self.source_client,
            created_at: self.created_at,
        }
    }
}

/// Internal row type for deserializing specs from SurrealDB.
#[derive(Deserialize)]
struct SpecRow {
    id: Thing,
    title: String,
    content: String,
    #[serde(default)]
    embedding: Option<Vec<f32>>,
    file_path: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl SpecRow {
    fn into_spec(self) -> Spec {
        Spec {
            id: self.id.id.to_raw(),
            title: self.title,
            content: self.content,
            embedding: self.embedding,
            file_path: self.file_path,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
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

    /// Insert or update a task record using last-write-wins semantics (US5/T093).
    ///
    /// Under concurrent access, SurrealDB serializes writes internally.
    /// The last `UPSERT` to execute wins, and its `updated_at` timestamp
    /// reflects the final state. Callers should always set `updated_at`
    /// to `Utc::now()` before calling this method.
    pub async fn upsert_task(&self, task: &Task) -> Result<(), TMemError> {
        let record = Thing::from(("task", task.id.as_str()));
        let status_str = task.status.as_str().to_string();
        let created = task.created_at.to_rfc3339();
        let updated = task.updated_at.to_rfc3339();
        self.db
            .query(
                "UPSERT $record SET \
                    title = $title, \
                    status = $status, \
                    work_item_id = $wid, \
                    description = $desc, \
                    context_summary = $ctx_summary, \
                    created_at = <datetime>$created, \
                    updated_at = <datetime>$updated",
            )
            .bind(("record", record))
            .bind(("title", task.title.clone()))
            .bind(("status", status_str))
            .bind(("wid", task.work_item_id.clone()))
            .bind(("desc", task.description.clone()))
            .bind(("ctx_summary", task.context_summary.clone()))
            .bind(("created", created))
            .bind(("updated", updated))
            .await
            .map_err(map_db_err)?;
        Ok(())
    }

    /// Create a new task with generated UUID, `todo` status, and optional parent.
    ///
    /// When `parent_id` is `Some`, a `depends_on` hard-blocker edge is created
    /// from the new task to the parent after cyclic-dependency validation.
    pub async fn create_task(
        &self,
        title: &str,
        description: &str,
        work_item_id: Option<&str>,
        parent_id: Option<&str>,
    ) -> Result<Task, TMemError> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();
        let task = Task {
            id: id.clone(),
            title: title.to_string(),
            status: TaskStatus::Todo,
            work_item_id: work_item_id.map(String::from),
            description: description.to_string(),
            context_summary: None,
            created_at: now,
            updated_at: now,
        };
        self.upsert_task(&task).await?;

        if let Some(parent) = parent_id {
            self.create_dependency(&id, parent, DependencyType::HardBlocker)
                .await?;
        }

        Ok(task)
    }

    pub async fn get_task(&self, id: &str) -> Result<Option<Task>, TMemError> {
        let record = Thing::from(("task", id));
        let rows: Vec<TaskRow> = self
            .db
            .query("SELECT * FROM $record")
            .bind(("record", record))
            .await
            .map_err(map_db_err)?
            .take(0)
            .unwrap_or_default();
        Ok(rows.into_iter().next().map(TaskRow::into_task))
    }

    pub async fn set_task_status(
        &self,
        id: &str,
        status: TaskStatus,
        updated_at: DateTime<Utc>,
    ) -> Result<(), TMemError> {
        let record = Thing::from(("task", id));
        let status_str = status.as_str().to_string();
        self.db
            .query("UPDATE $record MERGE { status: $status, updated_at: $updated }")
            .bind(("record", record))
            .bind(("status", status_str))
            .bind(("updated", updated_at))
            .await
            .map_err(map_db_err)?;
        Ok(())
    }

    /// Insert a new context record with append-only semantics (US5/T094).
    ///
    /// Uses `CREATE` (not `UPSERT`) so existing records are never overwritten.
    /// Each context has a unique UUID, ensuring concurrent insertions from
    /// multiple clients never collide or lose data.
    pub async fn insert_context(&self, ctx: &Context) -> Result<(), TMemError> {
        let record = Thing::from(("context", ctx.id.as_str()));
        let created = ctx.created_at.to_rfc3339();
        self.db
            .query(
                "CREATE $record SET \
                    content = $content, \
                    embedding = $embedding, \
                    source_client = $source, \
                    created_at = <datetime>$created",
            )
            .bind(("record", record))
            .bind(("content", ctx.content.clone()))
            .bind(("embedding", ctx.embedding.clone()))
            .bind(("source", ctx.source_client.clone()))
            .bind(("created", created))
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
            .query("SELECT in, out, type FROM depends_on WHERE in = $record")
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
        let rows: Vec<TaskRow> = self
            .db
            .query("SELECT * FROM task WHERE work_item_id = $id LIMIT 1")
            .bind(("id", id_owned))
            .await
            .map_err(map_db_err)?
            .take(0)
            .unwrap_or_default();
        Ok(rows.into_iter().next().map(TaskRow::into_task))
    }

    pub async fn all_tasks(&self) -> Result<Vec<Task>, TMemError> {
        let rows: Vec<TaskRow> = self
            .db
            .query("SELECT * FROM task")
            .await
            .map_err(map_db_err)?
            .take(0)
            .unwrap_or_default();
        Ok(rows.into_iter().map(TaskRow::into_task).collect())
    }

    pub async fn all_contexts(&self) -> Result<Vec<Context>, TMemError> {
        let rows: Vec<ContextRow> = self
            .db
            .query("SELECT * FROM context")
            .await
            .map_err(map_db_err)?
            .take(0)
            .unwrap_or_default();
        Ok(rows.into_iter().map(ContextRow::into_context).collect())
    }

    /// Return all specs in the workspace.
    pub async fn all_specs(&self) -> Result<Vec<Spec>, TMemError> {
        let rows: Vec<SpecRow> = self
            .db
            .query("SELECT * FROM spec")
            .await
            .map_err(map_db_err)?
            .take(0)
            .unwrap_or_default();
        Ok(rows.into_iter().map(SpecRow::into_spec).collect())
    }

    /// Insert or update a spec record.
    pub async fn upsert_spec(&self, spec: &Spec) -> Result<(), TMemError> {
        let record = Thing::from(("spec", spec.id.as_str()));
        let created = spec.created_at.to_rfc3339();
        let updated = spec.updated_at.to_rfc3339();
        self.db
            .query(
                "UPSERT $record SET \
                    title = $title, \
                    content = $content, \
                    embedding = $embedding, \
                    file_path = $file_path, \
                    created_at = <datetime>$created, \
                    updated_at = <datetime>$updated",
            )
            .bind(("record", record))
            .bind(("title", spec.title.clone()))
            .bind(("content", spec.content.clone()))
            .bind(("embedding", spec.embedding.clone()))
            .bind(("file_path", spec.file_path.clone()))
            .bind(("created", created))
            .bind(("updated", updated))
            .await
            .map_err(map_db_err)?;
        Ok(())
    }

    pub async fn all_dependency_edges(&self) -> Result<Vec<DependencyEdge>, TMemError> {
        let rows: Vec<DependsOnRow> = self
            .db
            .query("SELECT in, out, type FROM depends_on")
            .await
            .map_err(map_db_err)?
            .take(0)
            .unwrap_or_default();

        let edges = rows
            .into_iter()
            .map(|row| {
                let from = row.r#in.id.to_raw();
                let to = row.out.id.to_raw();
                let kind = row
                    .r#type
                    .map(parse_dependency_type)
                    .unwrap_or(DependencyType::HardBlocker);
                DependencyEdge { from, to, kind }
            })
            .collect();

        Ok(edges)
    }

    pub async fn all_implements_edges(&self) -> Result<Vec<ImplementsEdge>, TMemError> {
        let rows: Vec<RelationRow> = self
            .db
            .query("SELECT in, out FROM implements")
            .await
            .map_err(map_db_err)?
            .take(0)
            .unwrap_or_default();

        let edges = rows
            .into_iter()
            .map(|row| ImplementsEdge {
                task_id: row.r#in.id.to_raw(),
                spec_id: row.out.id.to_raw(),
            })
            .collect();

        Ok(edges)
    }

    pub async fn all_relates_to_edges(&self) -> Result<Vec<RelatesToEdge>, TMemError> {
        let rows: Vec<RelationRow> = self
            .db
            .query("SELECT in, out FROM relates_to")
            .await
            .map_err(map_db_err)?
            .take(0)
            .unwrap_or_default();

        let edges = rows
            .into_iter()
            .map(|row| RelatesToEdge {
                task_id: row.r#in.id.to_raw(),
                context_id: row.out.id.to_raw(),
            })
            .collect();

        Ok(edges)
    }

    /// Update the embedding vector on a context record.
    pub async fn set_context_embedding(
        &self,
        id: &str,
        embedding: Vec<f32>,
    ) -> Result<(), TMemError> {
        let record = Thing::from(("context", id));
        self.db
            .query("UPDATE $record SET embedding = $embedding")
            .bind(("record", record))
            .bind(("embedding", embedding))
            .await
            .map_err(map_db_err)?;
        Ok(())
    }

    /// Clear all data from the database (used for corruption recovery).
    pub async fn clear_all_data(&self) -> Result<(), TMemError> {
        self.db
            .query("DELETE task; DELETE context; DELETE spec; DELETE depends_on; DELETE implements; DELETE relates_to;")
            .await
            .map_err(map_db_err)?;
        Ok(())
    }

    pub async fn tasks_by_ids(&self, ids: &[String]) -> Result<Vec<Task>, TMemError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let things: Vec<Thing> = ids
            .iter()
            .map(|id| Thing::from(("task", id.as_str())))
            .collect();
        let rows: Vec<TaskRow> = self
            .db
            .query("SELECT * FROM $ids")
            .bind(("ids", things))
            .await
            .map_err(map_db_err)?
            .take(0)
            .unwrap_or_default();
        Ok(rows.into_iter().map(TaskRow::into_task).collect())
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
