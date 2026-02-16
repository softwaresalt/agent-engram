use std::collections::{HashMap, HashSet, VecDeque};

use chrono::{DateTime, Utc};
use serde::Deserialize;
use surrealdb::sql::Thing;

use crate::db::{Db, map_db_err};
use crate::errors::{EngramError, TaskError};
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

/// Internal row type for deserializing comments from SurrealDB.
#[derive(Deserialize)]
struct CommentRow {
    id: Thing,
    task_id: String,
    content: String,
    author: String,
    created_at: DateTime<Utc>,
}

impl CommentRow {
    fn into_comment(self) -> crate::models::Comment {
        crate::models::Comment {
            id: format!("comment:{}", self.id.id.to_raw()),
            task_id: self.task_id,
            content: self.content,
            author: self.author,
            created_at: self.created_at,
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

/// Aggregate workspace statistics.
#[derive(Debug)]
pub struct WorkspaceStatistics {
    pub total_tasks: u64,
    pub by_status: HashMap<String, u64>,
    pub by_priority: HashMap<String, u64>,
    pub by_type: HashMap<String, u64>,
    pub by_label: HashMap<String, u64>,
    pub deferred_count: u64,
    pub pinned_count: u64,
    pub claimed_count: u64,
    pub compacted_count: u64,
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
    pub async fn upsert_task(&self, task: &Task) -> Result<(), EngramError> {
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
        issue_type: Option<&str>,
    ) -> Result<Task, EngramError> {
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
            issue_type: issue_type.unwrap_or("task").to_owned(),
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

    pub async fn get_task(&self, id: &str) -> Result<Option<Task>, EngramError> {
        let record = Thing::from(("task", id));
        let rows: Vec<TaskRow> = self
            .db
            .query("SELECT * FROM $record")
            .bind(("record", record))
            .await
            .map_err(map_db_err)?
            .take(0)
            .map_err(map_db_err)?;
        Ok(rows.into_iter().next().map(TaskRow::into_task))
    }

    pub async fn set_task_status(
        &self,
        id: &str,
        status: TaskStatus,
        updated_at: DateTime<Utc>,
    ) -> Result<(), EngramError> {
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
    pub async fn insert_context(&self, ctx: &Context) -> Result<(), EngramError> {
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
    ) -> Result<(), EngramError> {
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

    pub async fn link_task_spec(&self, task_id: &str, spec_id: &str) -> Result<(), EngramError> {
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
    ) -> Result<(), EngramError> {
        if dependent == blocker {
            return Err(EngramError::Task(TaskError::CyclicDependency));
        }

        if self.detect_cycle(blocker, dependent).await? {
            return Err(EngramError::Task(TaskError::CyclicDependency));
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

    pub async fn dependencies_of(&self, task_id: &str) -> Result<Vec<DependencyEdge>, EngramError> {
        let record = Thing::from(("task", task_id));
        let rows: Vec<DependsOnRow> = self
            .db
            .query("SELECT in, out, type FROM depends_on WHERE in = $record")
            .bind(("record", record))
            .await
            .map_err(map_db_err)?
            .take(0)
            .map_err(map_db_err)?;

        let edges = rows
            .into_iter()
            .map(|row| {
                let target = row.out.id.to_raw();
                let kind = row
                    .r#type
                    .map(parse_dependency_type)
                    .unwrap_or(DependencyType::HardBlocker);
                DependencyEdge {
                    from: task_id.to_string(),
                    to: target,
                    kind,
                }
            })
            .collect();

        Ok(edges)
    }

    pub async fn task_by_work_item(&self, work_item_id: &str) -> Result<Option<Task>, EngramError> {
        let id_owned = work_item_id.to_string();
        let rows: Vec<TaskRow> = self
            .db
            .query("SELECT * FROM task WHERE work_item_id = $id LIMIT 1")
            .bind(("id", id_owned))
            .await
            .map_err(map_db_err)?
            .take(0)
            .map_err(map_db_err)?;
        Ok(rows.into_iter().next().map(TaskRow::into_task))
    }

    pub async fn all_tasks(&self) -> Result<Vec<Task>, EngramError> {
        let rows: Vec<TaskRow> = self
            .db
            .query("SELECT * FROM task")
            .await
            .map_err(map_db_err)?
            .take(0)
            .map_err(map_db_err)?;
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
    ) -> Result<ReadyWorkResult, EngramError> {
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
            .map_err(map_db_err)?;

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
    async fn find_blocked_task_ids(&self) -> Result<HashSet<String>, EngramError> {
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
            .map_err(map_db_err)?;

        // Collect unique blocker IDs for a single batch fetch
        let blocker_ids: Vec<String> = rows.iter().map(|r| r.out.id.to_raw()).collect();
        let blocker_tasks = self.tasks_by_ids(&blocker_ids).await?;
        let undone_blockers: HashSet<String> = blocker_tasks
            .into_iter()
            .filter(|t| t.status != TaskStatus::Done)
            .map(|t| t.id)
            .collect();

        let mut blocked = HashSet::new();
        for row in rows {
            let blocker_id = row.out.id.to_raw();
            let dependent_id = row.r#in.id.to_raw();

            if undone_blockers.contains(&blocker_id) {
                blocked.insert(dependent_id);
            }
        }

        Ok(blocked)
    }

    /// Find task IDs that have an outgoing duplicate_of edge.
    async fn find_duplicate_task_ids(&self) -> Result<HashSet<String>, EngramError> {
        let rows: Vec<DependsOnRow> = self
            .db
            .query(
                "SELECT in, out, type FROM depends_on \
                 WHERE `type` = 'duplicate_of'",
            )
            .await
            .map_err(map_db_err)?
            .take(0)
            .map_err(map_db_err)?;

        let ids = rows.into_iter().map(|row| row.r#in.id.to_raw()).collect();

        Ok(ids)
    }

    // ── Label CRUD ──────────────────────────────────────────────────────

    /// Insert a label for a task. Returns error if the label already exists
    /// (UNIQUE constraint on `label_task_name` index).
    pub async fn insert_label(&self, task_id: &str, name: &str) -> Result<(), EngramError> {
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
            .map_err(map_db_err)?;

        if existing.first().map_or(0, |r| r.count) > 0 {
            return Err(EngramError::Task(TaskError::DuplicateLabel {
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
    pub async fn delete_label(&self, task_id: &str, name: &str) -> Result<(), EngramError> {
        self.db
            .query("DELETE FROM label WHERE task_id = $task_id AND name = $name")
            .bind(("task_id", task_id.to_string()))
            .bind(("name", name.to_string()))
            .await
            .map_err(map_db_err)?;
        Ok(())
    }

    /// Get all labels for a task.
    pub async fn get_labels_for_task(&self, task_id: &str) -> Result<Vec<String>, EngramError> {
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
            .map_err(map_db_err)?;

        Ok(rows.into_iter().map(|r| r.name).collect())
    }

    /// Count labels for a task.
    pub async fn count_labels_for_task(&self, task_id: &str) -> Result<u32, EngramError> {
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
            .map_err(map_db_err)?;

        Ok(u32::try_from(rows.first().map_or(0, |r| r.count)).unwrap_or(u32::MAX))
    }

    /// Check if a task has ALL specified labels using a single query.
    async fn task_has_all_labels(
        &self,
        task_id: &str,
        labels: &[String],
    ) -> Result<bool, EngramError> {
        if labels.is_empty() {
            return Ok(true);
        }
        let count: Vec<CountRow> = self
            .db
            .query(
                "SELECT count() AS count FROM label \
                 WHERE task_id = $task_id AND name IN $names GROUP ALL",
            )
            .bind(("task_id", task_id.to_string()))
            .bind(("names", labels.to_vec()))
            .await
            .map_err(map_db_err)?
            .take(0)
            .map_err(map_db_err)?;

        let found = count.first().map_or(0_u64, |r| r.count);
        Ok(usize::try_from(found).unwrap_or(usize::MAX) >= labels.len())
    }

    // ── Comment queries ─────────────────────────────────────────────────────

    /// Insert a comment for a task. Returns the generated `comment:uuid` ID.
    pub async fn insert_comment(
        &self,
        task_id: &str,
        content: &str,
        author: &str,
    ) -> Result<String, EngramError> {
        let comment_id = format!("comment:{}", uuid::Uuid::new_v4());

        self.db
            .query(
                "INSERT INTO comment { \
                     id: type::thing('comment', $cid), \
                     task_id: $task_id, \
                     content: $content, \
                     author: $author \
                 }",
            )
            .bind((
                "cid",
                comment_id
                    .strip_prefix("comment:")
                    .unwrap_or(&comment_id)
                    .to_string(),
            ))
            .bind(("task_id", task_id.to_string()))
            .bind(("content", content.to_string()))
            .bind(("author", author.to_string()))
            .await
            .map_err(map_db_err)?;

        Ok(comment_id)
    }

    /// Retrieve all comments for a task, ordered by `created_at` ascending.
    pub async fn get_comments_for_task(
        &self,
        task_id: &str,
    ) -> Result<Vec<crate::models::Comment>, EngramError> {
        let rows: Vec<CommentRow> = self
            .db
            .query(
                "SELECT * FROM comment \
                 WHERE task_id = $task_id \
                 ORDER BY created_at ASC",
            )
            .bind(("task_id", task_id.to_string()))
            .await
            .map_err(map_db_err)?
            .take(0)
            .map_err(map_db_err)?;

        Ok(rows.into_iter().map(CommentRow::into_comment).collect())
    }

    /// Retrieve ALL comments in the workspace, ordered by task_id then created_at.
    pub async fn all_comments(&self) -> Result<Vec<crate::models::Comment>, EngramError> {
        let rows: Vec<CommentRow> = self
            .db
            .query("SELECT * FROM comment ORDER BY task_id ASC, created_at ASC")
            .await
            .map_err(map_db_err)?
            .take(0)
            .map_err(map_db_err)?;

        Ok(rows.into_iter().map(CommentRow::into_comment).collect())
    }

    // ── Compaction queries ───────────────────────────────────────────────────

    /// Return done, non-pinned tasks older than `threshold_days`, ordered by
    /// `updated_at` ascending, limited to `max_candidates`.
    pub async fn get_compaction_candidates(
        &self,
        threshold_days: u32,
        max_candidates: u32,
    ) -> Result<Vec<Task>, EngramError> {
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
            .map_err(map_db_err)?;
        Ok(rows.into_iter().map(TaskRow::into_task).collect())
    }

    /// Apply compaction to a single task: replace description, increment
    /// `compaction_level`, and set `compacted_at` to now.
    pub async fn apply_compaction(
        &self,
        task_id: &str,
        summary: &str,
    ) -> Result<Task, EngramError> {
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
            .map_err(map_db_err)?;

        rows.into_iter()
            .next()
            .map(TaskRow::into_task)
            .ok_or_else(|| {
                EngramError::Task(TaskError::CompactionFailed {
                    id: task_id.to_string(),
                    reason: "task not found".to_string(),
                })
            })
    }

    /// Atomically claim a task for a claimant.
    ///
    /// Uses a conditional `UPDATE ... WHERE assignee = NONE` to prevent
    /// TOCTOU races. Returns `Ok(task)` if the claim succeeds.
    /// Returns `AlreadyClaimed` if someone else holds the claim.
    /// Returns `TaskNotFound` if the task does not exist.
    pub async fn claim_task(&self, task_id: &str, claimant: &str) -> Result<Task, EngramError> {
        let record = Thing::from(("task", task_id));
        let now = Utc::now().to_rfc3339();

        // Atomic conditional update: only succeeds when assignee is NONE
        let rows: Vec<TaskRow> = self
            .db
            .query(
                "UPDATE $record SET \
                    assignee = $claimant, \
                    updated_at = <datetime>$now \
                 WHERE assignee = NONE \
                 RETURN AFTER",
            )
            .bind(("record", record.clone()))
            .bind(("claimant", claimant.to_string()))
            .bind(("now", now))
            .await
            .map_err(map_db_err)?
            .take(0)
            .map_err(map_db_err)?;

        if let Some(task) = rows.into_iter().next().map(TaskRow::into_task) {
            return Ok(task);
        }

        // UPDATE returned no rows: either task doesn't exist or already claimed
        let task = self.get_task(task_id).await?.ok_or_else(|| {
            EngramError::Task(TaskError::NotFound {
                id: task_id.to_string(),
            })
        })?;

        Err(EngramError::Task(TaskError::AlreadyClaimed {
            id: task_id.to_string(),
            assignee: task.assignee.unwrap_or_default(),
        }))
    }

    /// Release a claimed task, clearing the assignee.
    ///
    /// Returns `Ok(previous_claimant)` on success.
    /// Returns `NotClaimable` if the task has no current assignee.
    /// Returns `TaskNotFound` if the task does not exist.
    pub async fn release_task(&self, task_id: &str) -> Result<String, EngramError> {
        let task = self.get_task(task_id).await?.ok_or_else(|| {
            EngramError::Task(TaskError::NotFound {
                id: task_id.to_string(),
            })
        })?;

        let previous = task.assignee.ok_or_else(|| {
            EngramError::Task(TaskError::NotClaimable {
                id: task_id.to_string(),
                status: "not claimed".to_string(),
            })
        })?;

        let record = Thing::from(("task", task_id));
        let now = Utc::now().to_rfc3339();
        self.db
            .query(
                "UPDATE $record SET \
                    assignee = NONE, \
                    updated_at = <datetime>$now \
                 RETURN AFTER",
            )
            .bind(("record", record))
            .bind(("now", now))
            .await
            .map_err(map_db_err)?;

        Ok(previous)
    }

    pub async fn all_contexts(&self) -> Result<Vec<Context>, EngramError> {
        let rows: Vec<ContextRow> = self
            .db
            .query("SELECT * FROM context")
            .await
            .map_err(map_db_err)?
            .take(0)
            .map_err(map_db_err)?;
        Ok(rows.into_iter().map(ContextRow::into_context).collect())
    }

    /// Return all specs in the workspace.
    pub async fn all_specs(&self) -> Result<Vec<Spec>, EngramError> {
        let rows: Vec<SpecRow> = self
            .db
            .query("SELECT * FROM spec")
            .await
            .map_err(map_db_err)?
            .take(0)
            .map_err(map_db_err)?;
        Ok(rows.into_iter().map(SpecRow::into_spec).collect())
    }

    /// Insert or update a spec record.
    pub async fn upsert_spec(&self, spec: &Spec) -> Result<(), EngramError> {
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

    pub async fn all_dependency_edges(&self) -> Result<Vec<DependencyEdge>, EngramError> {
        let rows: Vec<DependsOnRow> = self
            .db
            .query("SELECT in, out, type FROM depends_on")
            .await
            .map_err(map_db_err)?
            .take(0)
            .map_err(map_db_err)?;

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

    pub async fn all_implements_edges(&self) -> Result<Vec<ImplementsEdge>, EngramError> {
        let rows: Vec<RelationRow> = self
            .db
            .query("SELECT in, out FROM implements")
            .await
            .map_err(map_db_err)?
            .take(0)
            .map_err(map_db_err)?;

        let edges = rows
            .into_iter()
            .map(|row| ImplementsEdge {
                task_id: row.r#in.id.to_raw(),
                spec_id: row.out.id.to_raw(),
            })
            .collect();

        Ok(edges)
    }

    pub async fn all_relates_to_edges(&self) -> Result<Vec<RelatesToEdge>, EngramError> {
        let rows: Vec<RelationRow> = self
            .db
            .query("SELECT in, out FROM relates_to")
            .await
            .map_err(map_db_err)?
            .take(0)
            .map_err(map_db_err)?;

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
    ) -> Result<(), EngramError> {
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
    pub async fn clear_all_data(&self) -> Result<(), EngramError> {
        self.db
            .query("DELETE task; DELETE context; DELETE spec; DELETE depends_on; DELETE implements; DELETE relates_to; DELETE label; DELETE comment;")
            .await
            .map_err(map_db_err)?;
        Ok(())
    }

    pub async fn tasks_by_ids(&self, ids: &[String]) -> Result<Vec<Task>, EngramError> {
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
            .map_err(map_db_err)?;
        Ok(rows.into_iter().map(TaskRow::into_task).collect())
    }

    async fn detect_cycle(&self, start: &str, target: &str) -> Result<bool, EngramError> {
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

    /// Compute aggregate workspace statistics.
    ///
    /// Returns grouped counts by status, priority, issue type, plus
    /// deferred, pinned, claimed, and compaction metrics.
    pub async fn get_workspace_statistics(&self) -> Result<WorkspaceStatistics, EngramError> {
        // SurrealDB v2 GROUP BY requires SELECT fields to match GROUP BY columns
        // exactly — aliasing with AS breaks grouping. Use per-field structs.

        #[derive(Deserialize)]
        struct StatusGroup {
            #[serde(default)]
            status: Option<String>,
            count: u64,
        }

        #[derive(Deserialize)]
        struct PriorityGroup {
            #[serde(default)]
            priority: Option<String>,
            count: u64,
        }

        #[derive(Deserialize)]
        struct TypeGroup {
            #[serde(default)]
            issue_type: Option<String>,
            count: u64,
        }

        #[derive(Deserialize)]
        struct LabelGroupRow {
            name: String,
            count: u64,
        }

        let by_status: Vec<StatusGroup> = self
            .db
            .query("SELECT status, count() AS count FROM task GROUP BY status")
            .await
            .map_err(map_db_err)?
            .take(0)
            .map_err(map_db_err)?;

        let by_priority: Vec<PriorityGroup> = self
            .db
            .query("SELECT priority, count() AS count FROM task GROUP BY priority")
            .await
            .map_err(map_db_err)?
            .take(0)
            .map_err(map_db_err)?;

        let by_type: Vec<TypeGroup> = self
            .db
            .query("SELECT issue_type, count() AS count FROM task GROUP BY issue_type")
            .await
            .map_err(map_db_err)?
            .take(0)
            .map_err(map_db_err)?;

        // Label counts via label table
        let by_label: Vec<LabelGroupRow> = self
            .db
            .query("SELECT name, count() AS count FROM label GROUP BY name")
            .await
            .map_err(map_db_err)?
            .take(0)
            .map_err(map_db_err)?;

        // Scalar counts
        let deferred_rows: Vec<CountRow> = self
            .db
            .query(
                "SELECT count() AS count FROM task \
                 WHERE defer_until != NONE AND defer_until > time::now() GROUP ALL",
            )
            .await
            .map_err(map_db_err)?
            .take(0)
            .map_err(map_db_err)?;

        let pinned_rows: Vec<CountRow> = self
            .db
            .query("SELECT count() AS count FROM task WHERE pinned = true GROUP ALL")
            .await
            .map_err(map_db_err)?
            .take(0)
            .map_err(map_db_err)?;

        let claimed_rows: Vec<CountRow> = self
            .db
            .query("SELECT count() AS count FROM task WHERE assignee != NONE GROUP ALL")
            .await
            .map_err(map_db_err)?
            .take(0)
            .map_err(map_db_err)?;

        let compacted_rows: Vec<CountRow> = self
            .db
            .query("SELECT count() AS count FROM task WHERE compaction_level > 0 GROUP ALL")
            .await
            .map_err(map_db_err)?
            .take(0)
            .map_err(map_db_err)?;

        let total_rows: Vec<CountRow> = self
            .db
            .query("SELECT count() AS count FROM task GROUP ALL")
            .await
            .map_err(map_db_err)?
            .take(0)
            .map_err(map_db_err)?;

        let status_map: HashMap<String, u64> = by_status
            .into_iter()
            .filter_map(|r| r.status.map(|s| (s, r.count)))
            .collect();
        let priority_map: HashMap<String, u64> = by_priority
            .into_iter()
            .filter_map(|r| r.priority.map(|p| (p, r.count)))
            .collect();
        let type_map: HashMap<String, u64> = by_type
            .into_iter()
            .filter_map(|r| r.issue_type.map(|t| (t, r.count)))
            .collect();

        Ok(WorkspaceStatistics {
            total_tasks: total_rows.first().map_or(0, |r| r.count),
            by_status: status_map,
            by_priority: priority_map,
            by_type: type_map,
            by_label: by_label.into_iter().map(|r| (r.name, r.count)).collect(),
            deferred_count: deferred_rows.first().map_or(0, |r| r.count),
            pinned_count: pinned_rows.first().map_or(0, |r| r.count),
            claimed_count: claimed_rows.first().map_or(0, |r| r.count),
            compacted_count: compacted_rows.first().map_or(0, |r| r.count),
        })
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

// ── Code Graph Row Types ───────────────────────────────────────────────

/// Internal row type for deserializing code_file records from SurrealDB.
#[derive(Deserialize)]
struct CodeFileRow {
    id: Thing,
    path: String,
    language: String,
    size_bytes: u64,
    content_hash: String,
    last_indexed_at: DateTime<Utc>,
}

impl CodeFileRow {
    fn into_code_file(self) -> crate::models::CodeFile {
        crate::models::CodeFile {
            id: format!("code_file:{}", self.id.id.to_raw()),
            path: self.path,
            language: self.language,
            size_bytes: self.size_bytes,
            content_hash: self.content_hash,
            last_indexed_at: self.last_indexed_at.to_rfc3339(),
        }
    }
}

/// Internal row type for deserializing function records from SurrealDB.
#[derive(Deserialize)]
struct FunctionRow {
    id: Thing,
    name: String,
    file_path: String,
    line_start: u32,
    line_end: u32,
    #[serde(default)]
    signature: String,
    #[serde(default)]
    docstring: Option<String>,
    body_hash: String,
    token_count: u32,
    embed_type: String,
    #[serde(default)]
    embedding: Vec<f32>,
    #[serde(default)]
    summary: String,
}

impl FunctionRow {
    fn into_function(self) -> crate::models::Function {
        crate::models::Function {
            id: format!("function:{}", self.id.id.to_raw()),
            name: self.name,
            file_path: self.file_path,
            line_start: self.line_start,
            line_end: self.line_end,
            signature: self.signature,
            docstring: self.docstring,
            body: String::new(), // body populated at runtime from source
            body_hash: self.body_hash,
            token_count: self.token_count,
            embed_type: self.embed_type,
            embedding: self.embedding,
            summary: self.summary,
        }
    }
}

/// Internal row type for deserializing class records from SurrealDB.
#[derive(Deserialize)]
struct ClassRow {
    id: Thing,
    name: String,
    file_path: String,
    line_start: u32,
    line_end: u32,
    #[serde(default)]
    docstring: Option<String>,
    body_hash: String,
    token_count: u32,
    embed_type: String,
    #[serde(default)]
    embedding: Vec<f32>,
    #[serde(default)]
    summary: String,
}

impl ClassRow {
    fn into_class(self) -> crate::models::Class {
        crate::models::Class {
            id: format!("class:{}", self.id.id.to_raw()),
            name: self.name,
            file_path: self.file_path,
            line_start: self.line_start,
            line_end: self.line_end,
            docstring: self.docstring,
            body: String::new(),
            body_hash: self.body_hash,
            token_count: self.token_count,
            embed_type: self.embed_type,
            embedding: self.embedding,
            summary: self.summary,
        }
    }
}

/// Internal row type for deserializing interface records from SurrealDB.
#[derive(Deserialize)]
struct InterfaceRow {
    id: Thing,
    name: String,
    file_path: String,
    line_start: u32,
    line_end: u32,
    #[serde(default)]
    docstring: Option<String>,
    body_hash: String,
    token_count: u32,
    embed_type: String,
    #[serde(default)]
    embedding: Vec<f32>,
    #[serde(default)]
    summary: String,
}

impl InterfaceRow {
    fn into_interface(self) -> crate::models::Interface {
        crate::models::Interface {
            id: format!("interface:{}", self.id.id.to_raw()),
            name: self.name,
            file_path: self.file_path,
            line_start: self.line_start,
            line_end: self.line_end,
            docstring: self.docstring,
            body: String::new(),
            body_hash: self.body_hash,
            token_count: self.token_count,
            embed_type: self.embed_type,
            embedding: self.embedding,
            summary: self.summary,
        }
    }
}

/// Internal row type for deserializing code edge records from SurrealDB.
#[derive(Deserialize)]
#[allow(dead_code)]
struct CodeEdgeRow {
    r#in: Thing,
    out: Thing,
    #[serde(default)]
    import_path: Option<String>,
    #[serde(default)]
    linked_by: Option<String>,
    #[serde(default)]
    created_at: Option<DateTime<Utc>>,
}

// ── Code Graph Queries ─────────────────────────────────────────────────

/// Query helper for code graph CRUD operations.
///
/// Follows the same pattern as [`Queries`] — wraps a cloneable SurrealDB
/// handle and provides typed, validated methods for all code graph tables.
#[derive(Clone)]
pub struct CodeGraphQueries {
    db: Db,
}

impl CodeGraphQueries {
    /// Create a new `CodeGraphQueries` instance wrapping the given DB handle.
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    // ── code_file CRUD ──────────────────────────────────────────────

    /// Insert or update a code file record.
    pub async fn upsert_code_file(
        &self,
        file: &crate::models::CodeFile,
    ) -> Result<(), EngramError> {
        let id_raw = file.id.strip_prefix("code_file:").unwrap_or(&file.id);
        let record = Thing::from(("code_file", id_raw));
        #[allow(clippy::cast_possible_wrap)]
        let size_i64 = file.size_bytes as i64;
        self.db
            .query("UPSERT $id SET path = $path, language = $lang, size_bytes = $size, content_hash = $hash, last_indexed_at = time::now()")
            .bind(("id", record))
            .bind(("path", file.path.clone()))
            .bind(("lang", file.language.clone()))
            .bind(("size", size_i64))
            .bind(("hash", file.content_hash.clone()))
            .await
            .map_err(map_db_err)?;
        Ok(())
    }

    /// Look up a code file by its workspace-relative path.
    pub async fn get_code_file_by_path(
        &self,
        path: &str,
    ) -> Result<Option<crate::models::CodeFile>, EngramError> {
        let mut response = self
            .db
            .query("SELECT * FROM code_file WHERE path = $path LIMIT 1")
            .bind(("path", path.to_owned()))
            .await
            .map_err(map_db_err)?;
        let rows: Vec<CodeFileRow> = response.take(0).map_err(map_db_err)?;
        Ok(rows.into_iter().next().map(CodeFileRow::into_code_file))
    }

    /// Delete a code file record and all its `defines` edges.
    pub async fn delete_code_file(&self, path: &str) -> Result<(), EngramError> {
        self.db
            .query("DELETE FROM code_file WHERE path = $path")
            .bind(("path", path.to_owned()))
            .await
            .map_err(map_db_err)?;
        Ok(())
    }

    /// List all indexed code files.
    pub async fn list_code_files(&self) -> Result<Vec<crate::models::CodeFile>, EngramError> {
        let mut response = self
            .db
            .query("SELECT * FROM code_file ORDER BY path ASC")
            .await
            .map_err(map_db_err)?;
        let rows: Vec<CodeFileRow> = response.take(0).map_err(map_db_err)?;
        Ok(rows.into_iter().map(CodeFileRow::into_code_file).collect())
    }

    // ── function CRUD ───────────────────────────────────────────────

    /// Insert or update a function record.
    pub async fn upsert_function(&self, func: &crate::models::Function) -> Result<(), EngramError> {
        let id_raw = func.id.strip_prefix("function:").unwrap_or(&func.id);
        let record = Thing::from(("function", id_raw));
        self.db
            .query("UPSERT $id SET name = $name, file_path = $fp, line_start = $ls, line_end = $le, signature = $sig, docstring = $doc, body_hash = $bh, token_count = $tc, embed_type = $et, embedding = $emb, summary = $sum")
            .bind(("id", record))
            .bind(("name", func.name.clone()))
            .bind(("fp", func.file_path.clone()))
            .bind(("ls", i64::from(func.line_start)))
            .bind(("le", i64::from(func.line_end)))
            .bind(("sig", func.signature.clone()))
            .bind(("doc", func.docstring.clone()))
            .bind(("bh", func.body_hash.clone()))
            .bind(("tc", i64::from(func.token_count)))
            .bind(("et", func.embed_type.clone()))
            .bind(("emb", func.embedding.clone()))
            .bind(("sum", func.summary.clone()))
            .await
            .map_err(map_db_err)?;
        Ok(())
    }

    /// Look up a function by name.
    pub async fn get_function_by_name(
        &self,
        name: &str,
    ) -> Result<Option<crate::models::Function>, EngramError> {
        let mut response = self
            .db
            .query("SELECT * FROM function WHERE name = $name LIMIT 1")
            .bind(("name", name.to_owned()))
            .await
            .map_err(map_db_err)?;
        let rows: Vec<FunctionRow> = response.take(0).map_err(map_db_err)?;
        Ok(rows.into_iter().next().map(FunctionRow::into_function))
    }

    /// List all functions in a given file.
    pub async fn get_functions_by_file(
        &self,
        file_path: &str,
    ) -> Result<Vec<crate::models::Function>, EngramError> {
        let mut response = self
            .db
            .query("SELECT * FROM function WHERE file_path = $fp ORDER BY line_start ASC")
            .bind(("fp", file_path.to_owned()))
            .await
            .map_err(map_db_err)?;
        let rows: Vec<FunctionRow> = response.take(0).map_err(map_db_err)?;
        Ok(rows.into_iter().map(FunctionRow::into_function).collect())
    }

    /// Delete all functions in a given file.
    pub async fn delete_functions_by_file(&self, file_path: &str) -> Result<(), EngramError> {
        self.db
            .query("DELETE FROM function WHERE file_path = $fp")
            .bind(("fp", file_path.to_owned()))
            .await
            .map_err(map_db_err)?;
        Ok(())
    }

    // ── class CRUD ──────────────────────────────────────────────────

    /// Insert or update a class record.
    pub async fn upsert_class(&self, class: &crate::models::Class) -> Result<(), EngramError> {
        let id_raw = class.id.strip_prefix("class:").unwrap_or(&class.id);
        let record = Thing::from(("class", id_raw));
        self.db
            .query("UPSERT $id SET name = $name, file_path = $fp, line_start = $ls, line_end = $le, docstring = $doc, body_hash = $bh, token_count = $tc, embed_type = $et, embedding = $emb, summary = $sum")
            .bind(("id", record))
            .bind(("name", class.name.clone()))
            .bind(("fp", class.file_path.clone()))
            .bind(("ls", i64::from(class.line_start)))
            .bind(("le", i64::from(class.line_end)))
            .bind(("doc", class.docstring.clone()))
            .bind(("bh", class.body_hash.clone()))
            .bind(("tc", i64::from(class.token_count)))
            .bind(("et", class.embed_type.clone()))
            .bind(("emb", class.embedding.clone()))
            .bind(("sum", class.summary.clone()))
            .await
            .map_err(map_db_err)?;
        Ok(())
    }

    /// Look up a class by name.
    pub async fn get_class_by_name(
        &self,
        name: &str,
    ) -> Result<Option<crate::models::Class>, EngramError> {
        let mut response = self
            .db
            .query("SELECT * FROM class WHERE name = $name LIMIT 1")
            .bind(("name", name.to_owned()))
            .await
            .map_err(map_db_err)?;
        let rows: Vec<ClassRow> = response.take(0).map_err(map_db_err)?;
        Ok(rows.into_iter().next().map(ClassRow::into_class))
    }

    /// Delete all classes in a given file.
    pub async fn delete_classes_by_file(&self, file_path: &str) -> Result<(), EngramError> {
        self.db
            .query("DELETE FROM class WHERE file_path = $fp")
            .bind(("fp", file_path.to_owned()))
            .await
            .map_err(map_db_err)?;
        Ok(())
    }

    // ── interface CRUD ──────────────────────────────────────────────

    /// Insert or update an interface record.
    pub async fn upsert_interface(
        &self,
        iface: &crate::models::Interface,
    ) -> Result<(), EngramError> {
        let id_raw = iface.id.strip_prefix("interface:").unwrap_or(&iface.id);
        let record = Thing::from(("interface", id_raw));
        self.db
            .query("UPSERT $id SET name = $name, file_path = $fp, line_start = $ls, line_end = $le, docstring = $doc, body_hash = $bh, token_count = $tc, embed_type = $et, embedding = $emb, summary = $sum")
            .bind(("id", record))
            .bind(("name", iface.name.clone()))
            .bind(("fp", iface.file_path.clone()))
            .bind(("ls", i64::from(iface.line_start)))
            .bind(("le", i64::from(iface.line_end)))
            .bind(("doc", iface.docstring.clone()))
            .bind(("bh", iface.body_hash.clone()))
            .bind(("tc", i64::from(iface.token_count)))
            .bind(("et", iface.embed_type.clone()))
            .bind(("emb", iface.embedding.clone()))
            .bind(("sum", iface.summary.clone()))
            .await
            .map_err(map_db_err)?;
        Ok(())
    }

    /// Look up an interface by name.
    pub async fn get_interface_by_name(
        &self,
        name: &str,
    ) -> Result<Option<crate::models::Interface>, EngramError> {
        let mut response = self
            .db
            .query("SELECT * FROM interface WHERE name = $name LIMIT 1")
            .bind(("name", name.to_owned()))
            .await
            .map_err(map_db_err)?;
        let rows: Vec<InterfaceRow> = response.take(0).map_err(map_db_err)?;
        Ok(rows.into_iter().next().map(InterfaceRow::into_interface))
    }

    /// Delete all interfaces in a given file.
    pub async fn delete_interfaces_by_file(&self, file_path: &str) -> Result<(), EngramError> {
        self.db
            .query("DELETE FROM interface WHERE file_path = $fp")
            .bind(("fp", file_path.to_owned()))
            .await
            .map_err(map_db_err)?;
        Ok(())
    }

    // ── Edge CRUD ───────────────────────────────────────────────────

    /// Create a `calls` edge between two functions.
    #[allow(clippy::similar_names)]
    pub async fn create_calls_edge(
        &self,
        caller_id: &str,
        callee_id: &str,
    ) -> Result<(), EngramError> {
        let from = Thing::from((
            "function",
            caller_id.strip_prefix("function:").unwrap_or(caller_id),
        ));
        let to = Thing::from((
            "function",
            callee_id.strip_prefix("function:").unwrap_or(callee_id),
        ));
        self.db
            .query("RELATE $from->calls->$to")
            .bind(("from", from))
            .bind(("to", to))
            .await
            .map_err(map_db_err)?;
        Ok(())
    }

    /// Create an `imports` edge between two code files.
    #[allow(clippy::similar_names)]
    pub async fn create_imports_edge(
        &self,
        importer_id: &str,
        imported_id: &str,
        import_path: &str,
    ) -> Result<(), EngramError> {
        let from = Thing::from((
            "code_file",
            importer_id
                .strip_prefix("code_file:")
                .unwrap_or(importer_id),
        ));
        let to = Thing::from((
            "code_file",
            imported_id
                .strip_prefix("code_file:")
                .unwrap_or(imported_id),
        ));
        self.db
            .query("RELATE $from->imports->$to SET import_path = $path")
            .bind(("from", from))
            .bind(("to", to))
            .bind(("path", import_path.to_owned()))
            .await
            .map_err(map_db_err)?;
        Ok(())
    }

    /// Create a `defines` edge from a code file to a symbol.
    pub async fn create_defines_edge(
        &self,
        file_id: &str,
        symbol_table: &str,
        symbol_id: &str,
    ) -> Result<(), EngramError> {
        let from = Thing::from((
            "code_file",
            file_id.strip_prefix("code_file:").unwrap_or(file_id),
        ));
        let prefix = format!("{symbol_table}:");
        let to = Thing::from((
            symbol_table,
            symbol_id.strip_prefix(&prefix).unwrap_or(symbol_id),
        ));
        self.db
            .query("RELATE $from->defines->$to")
            .bind(("from", from))
            .bind(("to", to))
            .await
            .map_err(map_db_err)?;
        Ok(())
    }

    /// Create an `inherits_from` edge from class to class/interface.
    pub async fn create_inherits_edge(
        &self,
        child_table: &str,
        child_id: &str,
        parent_table: &str,
        parent_id: &str,
    ) -> Result<(), EngramError> {
        let child_prefix = format!("{child_table}:");
        let parent_prefix = format!("{parent_table}:");
        let from = Thing::from((
            child_table,
            child_id.strip_prefix(&child_prefix).unwrap_or(child_id),
        ));
        let to = Thing::from((
            parent_table,
            parent_id.strip_prefix(&parent_prefix).unwrap_or(parent_id),
        ));
        self.db
            .query("RELATE $from->inherits_from->$to")
            .bind(("from", from))
            .bind(("to", to))
            .await
            .map_err(map_db_err)?;
        Ok(())
    }

    /// Create a `concerns` edge from a task to a code symbol.
    pub async fn create_concerns_edge(
        &self,
        task_id: &str,
        symbol_table: &str,
        symbol_id: &str,
        linked_by: &str,
    ) -> Result<(), EngramError> {
        let sym_prefix = format!("{symbol_table}:");
        let from = Thing::from(("task", task_id.strip_prefix("task:").unwrap_or(task_id)));
        let to = Thing::from((
            symbol_table,
            symbol_id.strip_prefix(&sym_prefix).unwrap_or(symbol_id),
        ));
        self.db
            .query("RELATE $from->concerns->$to SET linked_by = $by")
            .bind(("from", from))
            .bind(("to", to))
            .bind(("by", linked_by.to_owned()))
            .await
            .map_err(map_db_err)?;
        Ok(())
    }

    /// Delete all edges of a given type originating from a code file.
    pub async fn delete_edges_from_file(
        &self,
        edge_table: &str,
        file_id: &str,
    ) -> Result<(), EngramError> {
        let from = Thing::from((
            "code_file",
            file_id.strip_prefix("code_file:").unwrap_or(file_id),
        ));
        let query = format!("DELETE FROM {edge_table} WHERE in = $from");
        self.db
            .query(&query)
            .bind(("from", from))
            .await
            .map_err(map_db_err)?;
        Ok(())
    }

    /// Delete all symbols and edges for a file (used during re-indexing).
    pub async fn clear_file_graph(&self, file_path: &str) -> Result<(), EngramError> {
        self.delete_functions_by_file(file_path).await?;
        self.delete_classes_by_file(file_path).await?;
        self.delete_interfaces_by_file(file_path).await?;
        Ok(())
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
            matches!(err, EngramError::Task(TaskError::CyclicDependency)),
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
            matches!(err, EngramError::Task(TaskError::CyclicDependency)),
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
            matches!(err, EngramError::Task(TaskError::CyclicDependency)),
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
