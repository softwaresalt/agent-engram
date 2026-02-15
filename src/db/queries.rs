use std::collections::{HashSet, VecDeque};

use chrono::{DateTime, Utc};
use serde::Deserialize;
use surrealdb::sql::Thing;

use crate::db::{Db, map_db_err};
use crate::errors::{TMemError, TaskError};
use crate::models::graph::DependencyType;
use crate::models::task::{Task, TaskStatus, compute_priority_order};
use crate::models::{Context, Spec};

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
    #[serde(default = "default_priority")]
    priority: String,
    #[serde(default = "default_priority_order")]
    priority_order: u32,
    #[serde(default = "default_issue_type")]
    issue_type: String,
    #[serde(default)]
    assignee: Option<String>,
    #[serde(default)]
    defer_until: Option<DateTime<Utc>>,
    #[serde(default)]
    pinned: bool,
    #[serde(default)]
    compaction_level: u32,
    #[serde(default)]
    compacted_at: Option<DateTime<Utc>>,
    #[serde(default)]
    workflow_state: Option<String>,
    #[serde(default)]
    workflow_id: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

fn default_priority() -> String {
    "p2".to_owned()
}

const fn default_priority_order() -> u32 {
    2
}

fn default_issue_type() -> String {
    "task".to_owned()
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
            priority: self.priority,
            priority_order: self.priority_order,
            issue_type: self.issue_type,
            assignee: self.assignee,
            defer_until: self.defer_until,
            pinned: self.pinned,
            compaction_level: self.compaction_level,
            compacted_at: self.compacted_at,
            workflow_state: self.workflow_state,
            workflow_id: self.workflow_id,
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

/// Row type for COUNT() aggregate queries.
#[derive(Deserialize)]
struct CountRow {
    count: u64,
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

/// Parameters for the ready-work query.
#[derive(Debug, Default)]
pub struct ReadyWorkParams {
    /// Maximum tasks to return (default: 10).
    pub limit: u32,
    /// Filter: task must have ALL listed labels (AND logic).
    pub labels: Vec<String>,
    /// Maximum priority threshold (e.g., "p2" returns p0, p1, p2).
    pub priority: Option<String>,
    /// Filter by issue type (e.g., "bug").
    pub issue_type: Option<String>,
    /// Filter by assignee identity.
    pub assignee: Option<String>,
}

/// Result from the ready-work query.
#[derive(Debug)]
pub struct ReadyWorkResult {
    /// Tasks matching the criteria, ordered and limited.
    pub tasks: Vec<Task>,
    /// Total number of eligible tasks before applying limit.
    pub total_eligible: u32,
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
                    priority = $priority, \
                    priority_order = $priority_order, \
                    issue_type = $issue_type, \
                    assignee = $assignee, \
                    defer_until = IF $defer_until != NONE THEN <datetime>$defer_until END, \
                    pinned = $pinned, \
                    compaction_level = $compaction_level, \
                    compacted_at = IF $compacted_at != NONE THEN <datetime>$compacted_at END, \
                    workflow_state = $workflow_state, \
                    workflow_id = $workflow_id, \
                    created_at = <datetime>$created, \
                    updated_at = <datetime>$updated",
            )
            .bind(("record", record))
            .bind(("title", task.title.clone()))
            .bind(("status", status_str))
            .bind(("wid", task.work_item_id.clone()))
            .bind(("desc", task.description.clone()))
            .bind(("ctx_summary", task.context_summary.clone()))
            .bind(("priority", task.priority.clone()))
            .bind(("priority_order", task.priority_order))
            .bind(("issue_type", task.issue_type.clone()))
            .bind(("assignee", task.assignee.clone()))
            .bind(("defer_until", task.defer_until.map(|d| d.to_rfc3339())))
            .bind(("pinned", task.pinned))
            .bind(("compaction_level", task.compaction_level))
            .bind(("compacted_at", task.compacted_at.map(|d| d.to_rfc3339())))
            .bind(("workflow_state", task.workflow_state.clone()))
            .bind(("workflow_id", task.workflow_id.clone()))
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
            priority: "p2".to_owned(),
            priority_order: 2,
            issue_type: "task".to_owned(),
            assignee: None,
            defer_until: None,
            pinned: false,
            compaction_level: 0,
            compacted_at: None,
            workflow_state: None,
            workflow_id: None,
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

    /// Return prioritized actionable tasks: unblocked, undeferred,
    /// not done, sorted by pinned → priority → creation date.
    ///
    /// Blocking logic: tasks with incoming `hard_blocker` or `blocked_by`
    /// edges where the blocker's status != done are excluded. Tasks with
    /// `duplicate_of` outgoing edges are also excluded.
    pub async fn get_ready_work(
        &self,
        params: &ReadyWorkParams,
    ) -> Result<ReadyWorkResult, TMemError> {
        // Step 1: Get all non-done, non-blocked tasks.
        let rows: Vec<TaskRow> = self
            .db
            .query(
                "SELECT * FROM task WHERE status NOT IN ['done', 'blocked'] \
                 ORDER BY pinned DESC, priority_order ASC, created_at ASC",
            )
            .await
            .map_err(map_db_err)?
            .take(0)
            .unwrap_or_default();

        let mut candidates: Vec<Task> = rows.into_iter().map(TaskRow::into_task).collect();

        // Step 2: Filter out deferred tasks (defer_until in the future).
        let now = Utc::now();
        candidates.retain(|t| t.defer_until.is_none_or(|defer| defer <= now));

        // Step 3: Find tasks blocked by unresolved hard_blocker/blocked_by deps.
        let blocked_ids = self.find_blocked_task_ids().await?;
        candidates.retain(|t| !blocked_ids.contains(&t.id));

        // Step 4: Find tasks that are duplicates (have outgoing duplicate_of edge).
        let duplicate_ids = self.find_duplicate_task_ids().await?;
        candidates.retain(|t| !duplicate_ids.contains(&t.id));

        // Step 5: Apply optional filters.
        // Priority threshold filter.
        if let Some(ref threshold) = params.priority {
            let max_order = compute_priority_order(threshold);
            candidates.retain(|t| t.priority_order <= max_order);
        }

        // Issue type filter.
        if let Some(ref issue_type) = params.issue_type {
            candidates.retain(|t| t.issue_type == *issue_type);
        }

        // Assignee filter.
        if let Some(ref assignee) = params.assignee {
            candidates.retain(|t| t.assignee.as_deref() == Some(assignee.as_str()));
        }

        // Label filter (AND logic — task must have ALL listed labels).
        if !params.labels.is_empty() {
            let mut label_matched = Vec::new();
            for task in &candidates {
                if self.task_has_all_labels(&task.id, &params.labels).await? {
                    label_matched.push(task.id.clone());
                }
            }
            candidates.retain(|t| label_matched.contains(&t.id));
        }

        let total_eligible = u32::try_from(candidates.len()).unwrap_or(u32::MAX);

        // Step 6: Apply limit.
        let limit = if params.limit == 0 { 10 } else { params.limit };
        candidates.truncate(limit as usize);

        Ok(ReadyWorkResult {
            tasks: candidates,
            total_eligible,
        })
    }

    /// Find task IDs that are blocked by unresolved hard_blocker or blocked_by edges.
    async fn find_blocked_task_ids(&self) -> Result<HashSet<String>, TMemError> {
        // Get all hard_blocker and blocked_by edges.
        // `type` is a reserved keyword in SurrealQL, requires backtick escaping.
        let rows: Vec<DependsOnRow> = self
            .db
            .query(
                "SELECT in, out, type FROM depends_on \
                 WHERE `type` IN ['hard_blocker', 'blocked_by']",
            )
            .await
            .map_err(map_db_err)?
            .take(0)
            .unwrap_or_default();

        let mut blocked = HashSet::new();

        for row in rows {
            let blocker_id = row.out.id.to_raw();
            let dependent_id = row.r#in.id.to_raw();

            // Check if the blocker task is NOT done.
            if let Some(blocker) = self.get_task(&blocker_id).await? {
                if blocker.status != TaskStatus::Done {
                    blocked.insert(dependent_id);
                }
            }
        }

        Ok(blocked)
    }

    /// Find task IDs that have an outgoing duplicate_of edge.
    async fn find_duplicate_task_ids(&self) -> Result<HashSet<String>, TMemError> {
        let rows: Vec<DependsOnRow> = self
            .db
            .query(
                "SELECT in, out, type FROM depends_on \
                 WHERE `type` = 'duplicate_of'",
            )
            .await
            .map_err(map_db_err)?
            .take(0)
            .unwrap_or_default();

        let ids = rows.into_iter().map(|row| row.r#in.id.to_raw()).collect();

        Ok(ids)
    }

    // ── Label CRUD ──────────────────────────────────────────────────────

    /// Insert a label for a task. Returns error if the label already exists
    /// (UNIQUE constraint on `label_task_name` index).
    pub async fn insert_label(&self, task_id: &str, name: &str) -> Result<(), TMemError> {
        // Check for duplicate first (UNIQUE index enforcement)
        let existing: Vec<CountRow> = self
            .db
            .query(
                "SELECT count() AS count FROM label \
                 WHERE task_id = $task_id AND name = $name GROUP ALL",
            )
            .bind(("task_id", task_id.to_string()))
            .bind(("name", name.to_string()))
            .await
            .map_err(map_db_err)?
            .take(0)
            .unwrap_or_default();

        if existing.first().map_or(0, |r| r.count) > 0 {
            return Err(TMemError::Task(TaskError::DuplicateLabel {
                task_id: task_id.to_string(),
                label: name.to_string(),
            }));
        }

        self.db
            .query(
                "INSERT INTO label { \
                    task_id: $task_id, \
                    name: $name, \
                    created_at: time::now() \
                 }",
            )
            .bind(("task_id", task_id.to_string()))
            .bind(("name", name.to_string()))
            .await
            .map_err(map_db_err)?;

        Ok(())
    }

    /// Delete a label from a task.
    pub async fn delete_label(&self, task_id: &str, name: &str) -> Result<(), TMemError> {
        self.db
            .query("DELETE FROM label WHERE task_id = $task_id AND name = $name")
            .bind(("task_id", task_id.to_string()))
            .bind(("name", name.to_string()))
            .await
            .map_err(map_db_err)?;
        Ok(())
    }

    /// Get all labels for a task.
    pub async fn get_labels_for_task(&self, task_id: &str) -> Result<Vec<String>, TMemError> {
        #[derive(Deserialize)]
        struct LabelRow {
            name: String,
        }
        let rows: Vec<LabelRow> = self
            .db
            .query("SELECT name FROM label WHERE task_id = $task_id ORDER BY name ASC")
            .bind(("task_id", task_id.to_string()))
            .await
            .map_err(map_db_err)?
            .take(0)
            .unwrap_or_default();

        Ok(rows.into_iter().map(|r| r.name).collect())
    }

    /// Count labels for a task.
    pub async fn count_labels_for_task(&self, task_id: &str) -> Result<u32, TMemError> {
        let rows: Vec<CountRow> = self
            .db
            .query(
                "SELECT count() AS count FROM label \
                 WHERE task_id = $task_id GROUP ALL",
            )
            .bind(("task_id", task_id.to_string()))
            .await
            .map_err(map_db_err)?
            .take(0)
            .unwrap_or_default();

        Ok(u32::try_from(rows.first().map_or(0, |r| r.count)).unwrap_or(u32::MAX))
    }

    /// Check if a task has ALL specified labels.
    async fn task_has_all_labels(
        &self,
        task_id: &str,
        labels: &[String],
    ) -> Result<bool, TMemError> {
        for label in labels {
            let count: Vec<CountRow> = self
                .db
                .query(
                    "SELECT count() AS count FROM label \
                     WHERE task_id = $task_id AND name = $name GROUP ALL",
                )
                .bind(("task_id", task_id.to_string()))
                .bind(("name", label.clone()))
                .await
                .map_err(map_db_err)?
                .take(0)
                .unwrap_or_default();

            let found = count.first().map_or(0, |r| r.count);
            if found == 0 {
                return Ok(false);
            }
        }
        Ok(true)
    }

    // ── Compaction queries ───────────────────────────────────────────────────

    /// Return done, non-pinned tasks older than `threshold_days`, ordered by
    /// `updated_at` ascending, limited to `max_candidates`.
    pub async fn get_compaction_candidates(
        &self,
        threshold_days: u32,
        max_candidates: u32,
    ) -> Result<Vec<Task>, TMemError> {
        let rows: Vec<TaskRow> = self
            .db
            .query(
                "SELECT * FROM task \
                 WHERE status = 'done' \
                   AND pinned = false \
                   AND updated_at < time::now() - type::duration($threshold) \
                 ORDER BY updated_at ASC \
                 LIMIT $max_limit",
            )
            .bind(("threshold", format!("{threshold_days}d")))
            .bind(("max_limit", max_candidates))
            .await
            .map_err(map_db_err)?
            .take(0)
            .unwrap_or_default();
        Ok(rows.into_iter().map(TaskRow::into_task).collect())
    }

    /// Apply compaction to a single task: replace description, increment
    /// `compaction_level`, and set `compacted_at` to now.
    pub async fn apply_compaction(&self, task_id: &str, summary: &str) -> Result<Task, TMemError> {
        let record = Thing::from(("task", task_id));
        let now = Utc::now().to_rfc3339();
        let rows: Vec<TaskRow> = self
            .db
            .query(
                "UPDATE $record SET \
                    description = $summary, \
                    compaction_level = compaction_level + 1, \
                    compacted_at = <datetime>$now, \
                    updated_at = <datetime>$now \
                 RETURN AFTER",
            )
            .bind(("record", record))
            .bind(("summary", summary.to_string()))
            .bind(("now", now))
            .await
            .map_err(map_db_err)?
            .take(0)
            .unwrap_or_default();

        rows.into_iter()
            .next()
            .map(TaskRow::into_task)
            .ok_or_else(|| {
                TMemError::Task(TaskError::CompactionFailed {
                    id: task_id.to_string(),
                    reason: "task not found".to_string(),
                })
            })
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
        DependencyType::ChildOf => "child_of",
        DependencyType::BlockedBy => "blocked_by",
        DependencyType::DuplicateOf => "duplicate_of",
        DependencyType::RelatedTo => "related_to",
        DependencyType::Predecessor => "predecessor",
        DependencyType::Successor => "successor",
    }
}

fn parse_dependency_type(raw: String) -> DependencyType {
    match raw.as_str() {
        "soft_dependency" => DependencyType::SoftDependency,
        "child_of" => DependencyType::ChildOf,
        "blocked_by" => DependencyType::BlockedBy,
        "duplicate_of" => DependencyType::DuplicateOf,
        "related_to" => DependencyType::RelatedTo,
        "predecessor" => DependencyType::Predecessor,
        "successor" => DependencyType::Successor,
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
            priority: "p2".to_owned(),
            priority_order: 2,
            issue_type: "task".to_owned(),
            assignee: None,
            defer_until: None,
            pinned: false,
            compaction_level: 0,
            compacted_at: None,
            workflow_state: None,
            workflow_id: None,
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
