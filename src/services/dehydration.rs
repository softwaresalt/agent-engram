//! Dehydration: serialize workspace state from SurrealDB to `.engram/` files.
//!
//! Produces human-readable Markdown (`tasks.md`) and SurrealQL (`graph.surql`)
//! files that can be committed to Git. User-added comments in existing
//! files are preserved across flushes via diff-based merging (FR-012).

use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::Path;

use chrono::Utc;
use tokio::sync::{Mutex, MutexGuard};

use crate::db::queries::{DependencyEdge, ImplementsEdge, Queries, RelatesToEdge};
use crate::db::{self};
use crate::errors::{EngramError, SystemError};
use crate::models::comment::Comment;
use crate::models::graph::DependencyType;
use crate::models::task::{Task, TaskStatus};
use crate::server::state::SharedState;

/// Global flush lock for serializing concurrent dehydration operations (ADR-0002).
static FLUSH_LOCK: Mutex<()> = Mutex::const_new(());

/// Acquire the per-process flush lock for concurrent dehydration serialization.
///
/// Returns a guard that releases the lock when dropped.
pub async fn acquire_flush_lock() -> MutexGuard<'static, ()> {
    FLUSH_LOCK.lock().await
}

/// Flush all active workspaces to `.engram/` files (FR-006 graceful shutdown).
///
/// Acquires the flush lock and dehydrates the active workspace, if any.
pub async fn flush_all_workspaces(state: &SharedState) -> Result<(), EngramError> {
    let _guard = acquire_flush_lock().await;
    let Some(snapshot) = state.snapshot_workspace().await else {
        return Ok(());
    };

    let workspace_path = Path::new(&snapshot.path);
    let db = db::connect_db(&snapshot.workspace_id).await?;
    let queries = Queries::new(db);

    dehydrate_workspace(&queries, workspace_path).await?;
    Ok(())
}

/// Schema version written to `.engram/.version`.
pub const SCHEMA_VERSION: &str = "1.0.0";

/// Result of a dehydration (flush) operation.
#[derive(Debug, Clone)]
pub struct DehydrationResult {
    pub files_written: Vec<String>,
    pub tasks_written: usize,
    pub edges_written: usize,
    pub comments_preserved: usize,
    pub flush_timestamp: String,
}

/// Dehydrate full workspace state from the database into `.engram/` files.
///
/// Writes `tasks.md`, `graph.surql`, `.version`, and `.lastflush`.
/// Preserves user-added comments in existing `tasks.md` (FR-012).
pub async fn dehydrate_workspace(
    queries: &Queries,
    workspace_path: &Path,
) -> Result<DehydrationResult, EngramError> {
    let engram_dir = workspace_path.join(".engram");
    fs::create_dir_all(&engram_dir).map_err(|_| flush_err(&engram_dir))?;

    let tasks = queries.all_tasks().await?;
    let dep_edges = queries.all_dependency_edges().await?;
    let impl_edges = queries.all_implements_edges().await?;
    let rel_edges = queries.all_relates_to_edges().await?;

    // Read existing tasks.md for comment preservation (FR-012)
    let tasks_path = engram_dir.join("tasks.md");
    let old_content = fs::read_to_string(&tasks_path).unwrap_or_default();
    let old_blocks = parse_task_blocks(&old_content);
    let old_bodies: HashMap<String, String> = old_blocks
        .into_iter()
        .map(|b| (b.task_id, b.body))
        .collect();
    let comments_preserved = old_bodies.values().filter(|b| !b.trim().is_empty()).count();

    // Collect labels for each task
    let mut task_labels: HashMap<String, Vec<String>> = HashMap::new();
    for task in &tasks {
        let labels = queries.get_labels_for_task(&task.id).await?;
        if !labels.is_empty() {
            task_labels.insert(task.id.clone(), labels);
        }
    }

    // Serialize tasks.md preserving user comments
    let tasks_content = serialize_tasks_md(&tasks, &old_bodies, &old_content, &task_labels);
    atomic_write(&tasks_path, &tasks_content)?;

    // Serialize graph.surql
    let graph_path = engram_dir.join("graph.surql");
    let total_edges = dep_edges.len() + impl_edges.len() + rel_edges.len();
    let graph_content = serialize_graph_surql(&dep_edges, &impl_edges, &rel_edges);
    atomic_write(&graph_path, &graph_content)?;

    // Serialize comments.md (FR-063b)
    let all_comments = queries.all_comments().await?;
    let comments_path = engram_dir.join("comments.md");
    if !all_comments.is_empty() {
        let comments_content = serialize_comments_md(&all_comments);
        atomic_write(&comments_path, &comments_content)?;
    } else if comments_path.exists() {
        // Remove stale comments.md when all comments have been deleted
        let _ = std::fs::remove_file(&comments_path);
    }

    // Write version
    let version_path = engram_dir.join(".version");
    atomic_write(&version_path, SCHEMA_VERSION)?;

    // Write lastflush timestamp
    let flush_ts = Utc::now().to_rfc3339();
    let lastflush_path = engram_dir.join(".lastflush");
    atomic_write(&lastflush_path, &flush_ts)?;

    let mut files_written = vec![
        ".engram/tasks.md".to_string(),
        ".engram/graph.surql".to_string(),
        ".engram/.version".to_string(),
        ".engram/.lastflush".to_string(),
    ];

    if !all_comments.is_empty() {
        files_written.push(".engram/comments.md".to_string());
    }

    Ok(DehydrationResult {
        files_written,
        tasks_written: tasks.len(),
        edges_written: total_edges,
        comments_preserved,
        flush_timestamp: flush_ts,
    })
}

/// Result of a code graph dehydration operation.
#[derive(Debug, Clone)]
pub struct CodeGraphDehydrationResult {
    /// Files written during code graph serialization.
    pub files_written: Vec<String>,
    /// Total nodes written (code_files + functions + classes + interfaces).
    pub nodes_written: usize,
    /// Total edges written.
    pub edges_written: usize,
}

/// Dehydrate code graph state to `.engram/code-graph/` JSONL files (FR-132, FR-133, FR-134).
///
/// Writes `nodes.jsonl` and `edges.jsonl` using atomic temp-file-then-rename.
/// Only writes files when there is data; removes stale files when the graph is empty.
pub async fn dehydrate_code_graph(
    cg_queries: &crate::db::queries::CodeGraphQueries,
    workspace_path: &Path,
) -> Result<CodeGraphDehydrationResult, EngramError> {
    let code_graph_dir = workspace_path.join(".engram").join("code-graph");
    fs::create_dir_all(&code_graph_dir).map_err(|_| flush_err(&code_graph_dir))?;

    let code_files = cg_queries.list_code_files().await?;
    let functions = cg_queries.all_functions().await?;
    let classes = cg_queries.all_classes().await?;
    let interfaces = cg_queries.all_interfaces().await?;
    let edges = cg_queries.all_code_edges().await?;

    let total_nodes = code_files.len() + functions.len() + classes.len() + interfaces.len();
    let total_edges = edges.len();
    let mut files_written = Vec::new();

    // Serialize nodes.jsonl
    let nodes_path = code_graph_dir.join("nodes.jsonl");
    if total_nodes > 0 {
        let nodes_content = serialize_nodes_jsonl(&code_files, &functions, &classes, &interfaces);
        atomic_write(&nodes_path, &nodes_content)?;
        files_written.push(".engram/code-graph/nodes.jsonl".to_string());
    } else if nodes_path.exists() {
        let _ = fs::remove_file(&nodes_path);
    }

    // Serialize edges.jsonl
    let edges_path = code_graph_dir.join("edges.jsonl");
    if total_edges > 0 {
        let edges_content = serialize_edges_jsonl(&edges);
        atomic_write(&edges_path, &edges_content)?;
        files_written.push(".engram/code-graph/edges.jsonl".to_string());
    } else if edges_path.exists() {
        let _ = fs::remove_file(&edges_path);
    }

    Ok(CodeGraphDehydrationResult {
        files_written,
        nodes_written: total_nodes,
        edges_written: total_edges,
    })
}

/// Serialize tasks to the canonical `tasks.md` format.
///
/// Preserves file-level header and per-task user comments from the old file.
pub fn serialize_tasks_md(
    tasks: &[Task],
    old_bodies: &HashMap<String, String>,
    old_content: &str,
    task_labels: &HashMap<String, Vec<String>>,
) -> String {
    let mut out = String::new();

    // Preserve file header (content before first ## task:)
    let header = extract_file_header(old_content);
    if header.is_empty() {
        out.push_str("# Tasks\n\n");
    } else {
        out.push_str(&header);
        if !header.ends_with('\n') {
            out.push('\n');
        }
    }

    for task in tasks {
        // Use task: prefix in heading and YAML id per data-model spec.
        // task.id may be bare ("t1") or prefixed ("task:t1") depending
        // on whether it came from DB insert or DB read-back.
        let display_id = if task.id.starts_with("task:") {
            task.id.clone()
        } else {
            format!("task:{}", task.id)
        };
        out.push_str(&format!("## {display_id}\n\n"));
        out.push_str("---\n");
        out.push_str(&format!("id: {display_id}\n"));
        out.push_str(&format!("title: {}\n", task.title));
        out.push_str(&format!("status: {}\n", format_status(task.status)));
        if let Some(ref wid) = task.work_item_id {
            out.push_str(&format!("work_item_id: {wid}\n"));
        }
        out.push_str(&format!("created_at: {}\n", task.created_at.to_rfc3339()));
        out.push_str(&format!("updated_at: {}\n", task.updated_at.to_rfc3339()));
        out.push_str(&format!("priority: {}\n", task.priority));
        out.push_str(&format!("issue_type: {}\n", task.issue_type));
        if let Some(ref a) = task.assignee {
            out.push_str(&format!("assignee: {a}\n"));
        }
        if task.pinned {
            out.push_str("pinned: true\n");
        }
        if let Some(ref du) = task.defer_until {
            out.push_str(&format!("defer_until: {}\n", du.to_rfc3339()));
        }
        if task.compaction_level > 0 {
            out.push_str(&format!("compaction_level: {}\n", task.compaction_level));
        }
        if let Some(ref ca) = task.compacted_at {
            out.push_str(&format!("compacted_at: {}\n", ca.to_rfc3339()));
        }
        if let Some(labels) = task_labels.get(&task.id) {
            if !labels.is_empty() {
                out.push_str(&format!("labels: {}\n", labels.join(", ")));
            }
        }
        out.push_str("---\n\n");

        // Merge description with preserved user comments.
        // old_bodies keys are "task:id" (from file parsing), so use display_id.
        if let Some(old_body) = old_bodies.get(&display_id) {
            let merged = merge_body_with_comments(&task.description, old_body);
            if !merged.is_empty() {
                out.push_str(&merged);
                if !merged.ends_with('\n') {
                    out.push('\n');
                }
            }
        } else if !task.description.is_empty() {
            out.push_str(&task.description);
            if !task.description.ends_with('\n') {
                out.push('\n');
            }
        }

        out.push('\n');
    }

    out
}

/// Merge new description with preserved user content from old body.
///
/// Only HTML comment blocks (`<!-- ... -->`) in the old body are treated
/// as user-added content that must be preserved across flushes (FR-012).
/// All other lines are controlled by the daemon and may be replaced.
pub fn merge_body_with_comments(new_description: &str, old_body: &str) -> String {
    let new_desc_trimmed = new_description.trim();
    let old_body_trimmed = old_body.trim();

    if old_body_trimmed.is_empty() {
        return new_desc_trimmed.to_string();
    }

    if new_desc_trimmed == old_body_trimmed {
        return new_desc_trimmed.to_string();
    }

    // Extract HTML comment blocks from the old body
    let mut user_blocks: Vec<String> = Vec::new();
    let mut in_comment = false;
    let mut block = String::new();

    for line in old_body_trimmed.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("<!--") {
            in_comment = true;
            block.clear();
            block.push_str(line);
            block.push('\n');
            if trimmed.ends_with("-->") {
                in_comment = false;
                user_blocks.push(block.clone());
                block.clear();
            }
        } else if in_comment {
            block.push_str(line);
            block.push('\n');
            if trimmed.ends_with("-->") {
                in_comment = false;
                user_blocks.push(block.clone());
                block.clear();
            }
        }
    }

    let mut result = new_desc_trimmed.to_string();
    if !user_blocks.is_empty() {
        result.push_str("\n\n");
        for blk in user_blocks {
            result.push_str(&blk);
        }
    }

    result
}

/// Serialize graph edges to SurrealQL format.
pub fn serialize_graph_surql(
    deps: &[DependencyEdge],
    impls: &[ImplementsEdge],
    rels: &[RelatesToEdge],
) -> String {
    let mut out = String::new();
    let now = Utc::now().to_rfc3339();

    out.push_str("-- Generated by Engram. Do not edit manually.\n");
    out.push_str(&format!("-- Schema version: {SCHEMA_VERSION}\n"));
    out.push_str(&format!("-- Generated at: {now}\n\n"));

    if !deps.is_empty() {
        out.push_str("-- Dependencies\n");
        for edge in deps {
            out.push_str(&format!(
                "RELATE task:{}->depends_on->task:{} SET type = '{}';\n",
                edge.from,
                edge.to,
                format_dependency(edge.kind),
            ));
        }
        out.push('\n');
    }

    if !impls.is_empty() {
        out.push_str("-- Implementations\n");
        for edge in impls {
            out.push_str(&format!(
                "RELATE task:{}->implements->spec:{};\n",
                edge.task_id, edge.spec_id,
            ));
        }
        out.push('\n');
    }

    if !rels.is_empty() {
        out.push_str("-- Context Relations\n");
        for edge in rels {
            out.push_str(&format!(
                "RELATE task:{}->relates_to->context:{};\n",
                edge.task_id, edge.context_id,
            ));
        }
        out.push('\n');
    }

    out
}

/// Serialize comments to the canonical `comments.md` format.
///
/// Groups comments by task ID and writes each comment with a timestamp—author
/// heading, matching the format expected by `parse_comments_md` in hydration.
pub fn serialize_comments_md(comments: &[Comment]) -> String {
    use std::collections::BTreeMap;

    // Group comments by task_id, preserving insertion order within each group
    let mut groups: BTreeMap<&str, Vec<&Comment>> = BTreeMap::new();
    for comment in comments {
        groups.entry(&comment.task_id).or_default().push(comment);
    }

    let mut out = String::new();
    out.push_str("# Comments\n\n");

    for (task_id, task_comments) in &groups {
        // Ensure task: prefix in heading
        if task_id.starts_with("task:") {
            out.push_str(&format!("## {task_id}\n\n"));
        } else {
            out.push_str(&format!("## task:{task_id}\n\n"));
        }

        for comment in task_comments {
            let ts = comment.created_at.to_rfc3339();
            out.push_str(&format!("### {ts} — {}\n\n", comment.author));
            out.push_str(&comment.content);
            if !comment.content.ends_with('\n') {
                out.push('\n');
            }
            out.push('\n');
        }
    }

    out
}

/// Extract the file header (everything before the first `## task:` heading).
fn extract_file_header(content: &str) -> String {
    let mut header = String::new();
    for line in content.lines() {
        if line.starts_with("## task:") {
            break;
        }
        header.push_str(line);
        header.push('\n');
    }
    header
}

/// A parsed task block from the raw markdown.
pub struct TaskBlock {
    pub task_id: String,
    pub body: String,
}

/// Parse tasks.md content into task blocks for comment extraction.
pub fn parse_task_blocks(content: &str) -> Vec<TaskBlock> {
    let mut blocks: Vec<TaskBlock> = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        if lines[i].starts_with("## task:") {
            let task_id = lines[i].trim_start_matches("## ").trim().to_string();
            i += 1;

            // Skip blank lines
            while i < lines.len() && lines[i].trim().is_empty() {
                i += 1;
            }

            // Skip frontmatter (between --- delimiters)
            if i < lines.len() && lines[i].trim() == "---" {
                i += 1;
                while i < lines.len() && lines[i].trim() != "---" {
                    i += 1;
                }
                if i < lines.len() {
                    i += 1; // skip closing ---
                }
                // Skip blank lines after frontmatter
                while i < lines.len() && lines[i].trim().is_empty() {
                    i += 1;
                }
            }

            // Capture body until next ## or EOF
            let mut body = String::new();
            while i < lines.len() && !lines[i].starts_with("## ") {
                body.push_str(lines[i]);
                body.push('\n');
                i += 1;
            }

            blocks.push(TaskBlock {
                task_id,
                body: body.trim_end().to_string(),
            });
        } else {
            i += 1;
        }
    }

    blocks
}

/// Write content atomically using temp file + rename pattern.
///
/// Creates a temporary `.tmp` file alongside the target, writes content,
/// then renames atomically to prevent partial writes on crash.
pub fn atomic_write(path: &Path, content: &str) -> Result<(), EngramError> {
    let tmp_path = path.with_extension("tmp");
    let mut file = fs::File::create(&tmp_path).map_err(|_| flush_err(path))?;
    file.write_all(content.as_bytes())
        .map_err(|_| flush_err(path))?;
    file.sync_all().map_err(|_| flush_err(path))?;
    drop(file);

    fs::rename(&tmp_path, path).map_err(|_| flush_err(path))?;

    Ok(())
}

fn flush_err(path: &Path) -> EngramError {
    EngramError::System(SystemError::FlushFailed {
        path: path.display().to_string(),
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

// ── Code graph JSONL serialization (FR-132, FR-133, FR-134) ───────────────

use crate::models::{Class, CodeEdge, CodeFile, Function, Interface};

/// Intermediate struct for serializing a code graph node to one JSONL line.
///
/// Bodies are stripped (FR-133, FR-134) — only `body_hash` is persisted.
#[derive(Debug, serde::Serialize)]
struct NodeLine {
    id: String,
    #[serde(rename = "type")]
    node_type: String,
    name: String,
    file_path: String,
    line_start: u32,
    line_end: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    signature: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    docstring: Option<String>,
    body_hash: String,
    token_count: u32,
    embed_type: String,
    embedding: Vec<f32>,
    summary: String,
}

/// Intermediate struct for serializing a code edge to one JSONL line.
#[derive(Debug, serde::Serialize)]
struct EdgeLine {
    #[serde(rename = "type")]
    edge_type: String,
    from: String,
    to: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    import_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    linked_by: Option<String>,
    created_at: String,
}

/// Intermediate struct for serializing a `CodeFile` to one JSONL line.
#[derive(Debug, serde::Serialize)]
struct FileLine {
    id: String,
    #[serde(rename = "type")]
    node_type: String,
    path: String,
    language: String,
    size_bytes: u64,
    content_hash: String,
    last_indexed_at: String,
}

/// Serialize code graph nodes (code_files + functions + classes + interfaces) to JSONL.
///
/// Each line is a self-contained JSON object sorted by `id`.
/// Bodies are excluded; only `body_hash` is persisted (FR-133).
pub fn serialize_nodes_jsonl(
    code_files: &[CodeFile],
    functions: &[Function],
    classes: &[Class],
    interfaces: &[Interface],
) -> String {
    let mut lines: Vec<String> = Vec::new();

    // Code files
    for cf in code_files {
        let fl = FileLine {
            id: cf.id.clone(),
            node_type: "code_file".to_string(),
            path: cf.path.clone(),
            language: cf.language.clone(),
            size_bytes: cf.size_bytes,
            content_hash: cf.content_hash.clone(),
            last_indexed_at: cf.last_indexed_at.clone(),
        };
        if let Ok(json) = serde_json::to_string(&fl) {
            lines.push(json);
        }
    }

    // Functions
    for f in functions {
        let nl = NodeLine {
            id: f.id.clone(),
            node_type: "function".to_string(),
            name: f.name.clone(),
            file_path: f.file_path.clone(),
            line_start: f.line_start,
            line_end: f.line_end,
            signature: Some(f.signature.clone()),
            docstring: f.docstring.clone(),
            body_hash: f.body_hash.clone(),
            token_count: f.token_count,
            embed_type: f.embed_type.clone(),
            embedding: f.embedding.clone(),
            summary: f.summary.clone(),
        };
        if let Ok(json) = serde_json::to_string(&nl) {
            lines.push(json);
        }
    }

    // Classes
    for c in classes {
        let nl = NodeLine {
            id: c.id.clone(),
            node_type: "class".to_string(),
            name: c.name.clone(),
            file_path: c.file_path.clone(),
            line_start: c.line_start,
            line_end: c.line_end,
            signature: None,
            docstring: c.docstring.clone(),
            body_hash: c.body_hash.clone(),
            token_count: c.token_count,
            embed_type: c.embed_type.clone(),
            embedding: c.embedding.clone(),
            summary: c.summary.clone(),
        };
        if let Ok(json) = serde_json::to_string(&nl) {
            lines.push(json);
        }
    }

    // Interfaces
    for i in interfaces {
        let nl = NodeLine {
            id: i.id.clone(),
            node_type: "interface".to_string(),
            name: i.name.clone(),
            file_path: i.file_path.clone(),
            line_start: i.line_start,
            line_end: i.line_end,
            signature: None,
            docstring: i.docstring.clone(),
            body_hash: i.body_hash.clone(),
            token_count: i.token_count,
            embed_type: i.embed_type.clone(),
            embedding: i.embedding.clone(),
            summary: i.summary.clone(),
        };
        if let Ok(json) = serde_json::to_string(&nl) {
            lines.push(json);
        }
    }

    // Sort all lines by id for deterministic output
    lines.sort();

    if lines.is_empty() {
        String::new()
    } else {
        let mut out = lines.join("\n");
        out.push('\n');
        out
    }
}

/// Serialize code edges to JSONL sorted by (type, from, to).
pub fn serialize_edges_jsonl(edges: &[CodeEdge]) -> String {
    let mut lines: Vec<(String, String, String, String)> = Vec::new();

    for edge in edges {
        let el = EdgeLine {
            edge_type: edge.edge_type.as_str().to_string(),
            from: edge.from.clone(),
            to: edge.to.clone(),
            import_path: edge.import_path.clone(),
            linked_by: edge.linked_by.clone(),
            created_at: edge.created_at.clone(),
        };
        if let Ok(json) = serde_json::to_string(&el) {
            lines.push((
                edge.edge_type.as_str().to_string(),
                edge.from.clone(),
                edge.to.clone(),
                json,
            ));
        }
    }

    // Sort by (type, from, to)
    lines.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));

    if lines.is_empty() {
        String::new()
    } else {
        let sorted: Vec<String> = lines.into_iter().map(|(_, _, _, json)| json).collect();
        let mut out = sorted.join("\n");
        out.push('\n');
        out
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;

    fn make_task(id: &str, desc: &str) -> Task {
        let now = Utc::now();
        Task {
            id: id.to_string(),
            title: format!("Task {id}"),
            status: TaskStatus::Todo,
            work_item_id: None,
            description: desc.to_string(),
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

    #[test]
    fn serialize_tasks_md_empty() {
        let out = serialize_tasks_md(&[], &HashMap::new(), "", &HashMap::new());
        assert!(out.starts_with("# Tasks\n"));
    }

    #[test]
    fn serialize_tasks_md_round_trip_fields() {
        let tasks = vec![make_task("task:abc", "Do something")];
        let out = serialize_tasks_md(&tasks, &HashMap::new(), "", &HashMap::new());
        assert!(out.contains("## task:abc"));
        assert!(out.contains("id: task:abc"));
        assert!(out.contains("title: Task task:abc"));
        assert!(out.contains("status: todo"));
        assert!(out.contains("Do something"));
    }

    #[test]
    fn merge_preserves_html_comments() {
        let desc = "My description";
        let old_body = "My description\n\n<!-- User note: important -->";
        let merged = merge_body_with_comments(desc, old_body);
        assert!(merged.contains("My description"));
        assert!(merged.contains("<!-- User note: important -->"));
    }

    #[test]
    fn merge_no_old_body() {
        let merged = merge_body_with_comments("New desc", "");
        assert_eq!(merged, "New desc");
    }

    #[test]
    fn merge_identical_no_duplication() {
        let merged = merge_body_with_comments("Same text", "Same text");
        assert_eq!(merged, "Same text");
    }

    #[test]
    fn serialize_graph_surql_empty() {
        let out = serialize_graph_surql(&[], &[], &[]);
        assert!(out.contains("Generated by Engram"));
        assert!(out.contains(&format!("Schema version: {SCHEMA_VERSION}")));
    }

    #[test]
    fn serialize_graph_surql_with_edges() {
        let deps = vec![DependencyEdge {
            from: "a".into(),
            to: "b".into(),
            kind: DependencyType::HardBlocker,
        }];
        let impls = vec![ImplementsEdge {
            task_id: "a".into(),
            spec_id: "s1".into(),
        }];
        let rels = vec![RelatesToEdge {
            task_id: "a".into(),
            context_id: "c1".into(),
        }];
        let out = serialize_graph_surql(&deps, &impls, &rels);
        assert!(out.contains("RELATE task:a->depends_on->task:b SET type = 'hard_blocker';"));
        assert!(out.contains("RELATE task:a->implements->spec:s1;"));
        assert!(out.contains("RELATE task:a->relates_to->context:c1;"));
    }

    #[test]
    fn atomic_write_creates_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("test.txt");
        atomic_write(&path, "hello").expect("write");
        let content = fs::read_to_string(&path).expect("read");
        assert_eq!(content, "hello");
    }

    #[test]
    fn parse_task_blocks_extracts_body() {
        let content = r#"# Tasks

## task:abc

---
id: task:abc
title: Test
status: todo
---

Description here.

<!-- User comment -->

## task:def

---
id: task:def
title: Other
status: done
---

Other description.
"#;
        let blocks = parse_task_blocks(content);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].task_id, "task:abc");
        assert!(blocks[0].body.contains("Description here."));
        assert!(blocks[0].body.contains("<!-- User comment -->"));
        assert_eq!(blocks[1].task_id, "task:def");
        assert!(blocks[1].body.contains("Other description."));
    }

    #[test]
    fn file_header_preserved() {
        let old_content = "# Tasks\n\n<!-- Global project notes -->\n\n## task:abc\n";
        let header = extract_file_header(old_content);
        assert!(header.contains("# Tasks"));
        assert!(header.contains("<!-- Global project notes -->"));
    }

    #[test]
    fn comment_preservation_across_flush() {
        let tasks = vec![make_task("task:abc", "My description")];
        let mut old_bodies = HashMap::new();
        old_bodies.insert(
            "task:abc".to_string(),
            "My description\n\n<!-- Important user note -->".to_string(),
        );
        let old_content = "# Tasks\n\n<!-- Header comment -->\n\n## task:abc\n";
        let out = serialize_tasks_md(&tasks, &old_bodies, old_content, &HashMap::new());
        assert!(out.contains("<!-- Header comment -->"));
        assert!(out.contains("<!-- Important user note -->"));
        assert!(out.contains("My description"));
    }

    #[test]
    fn serialize_comments_md_empty() {
        let out = serialize_comments_md(&[]);
        assert!(out.starts_with("# Comments\n"));
        assert!(!out.contains("## task:"));
    }

    #[test]
    fn serialize_comments_md_groups_by_task() {
        let now = Utc::now();
        let comments = vec![
            Comment {
                id: "comment:c1".to_string(),
                task_id: "task:abc".to_string(),
                content: "First comment".to_string(),
                author: "agent-1".to_string(),
                created_at: now,
            },
            Comment {
                id: "comment:c2".to_string(),
                task_id: "task:abc".to_string(),
                content: "Second comment".to_string(),
                author: "agent-2".to_string(),
                created_at: now,
            },
            Comment {
                id: "comment:c3".to_string(),
                task_id: "task:def".to_string(),
                content: "Other task comment".to_string(),
                author: "user-1".to_string(),
                created_at: now,
            },
        ];
        let out = serialize_comments_md(&comments);
        assert!(out.contains("## task:abc"));
        assert!(out.contains("## task:def"));
        assert!(out.contains("First comment"));
        assert!(out.contains("Second comment"));
        assert!(out.contains("Other task comment"));
        // Verify heading format includes author
        assert!(out.contains("— agent-1"));
        assert!(out.contains("— agent-2"));
        assert!(out.contains("— user-1"));
    }

    #[test]
    fn serialize_comments_md_bare_task_id_gets_prefix() {
        let now = Utc::now();
        let comments = vec![Comment {
            id: "comment:c1".to_string(),
            task_id: "bare_id".to_string(),
            content: "Body text".to_string(),
            author: "tester".to_string(),
            created_at: now,
        }];
        let out = serialize_comments_md(&comments);
        assert!(out.contains("## task:bare_id"));
    }

    // ── JSONL serialization tests (Phase 9) ─────────────────────────

    #[test]
    fn serialize_nodes_jsonl_empty() {
        let out = serialize_nodes_jsonl(&[], &[], &[], &[]);
        assert_eq!(out, "");
    }

    #[test]
    fn serialize_nodes_jsonl_function_excludes_body() {
        let f = Function {
            id: "function:abc".to_string(),
            name: "my_fn".to_string(),
            file_path: "src/lib.rs".to_string(),
            line_start: 1,
            line_end: 10,
            signature: "fn my_fn()".to_string(),
            docstring: None,
            body: "fn my_fn() { /* secret */ }".to_string(),
            body_hash: "abcdef".to_string(),
            token_count: 6,
            embed_type: "explicit_code".to_string(),
            embedding: vec![0.1, 0.2],
            summary: "A test function".to_string(),
        };
        let out = serialize_nodes_jsonl(&[], &[f], &[], &[]);
        // Must contain body_hash but NOT the raw body
        assert!(out.contains("\"body_hash\":\"abcdef\""));
        assert!(!out.contains("secret"));
        assert!(out.contains("\"type\":\"function\""));
        assert!(out.contains("\"signature\":\"fn my_fn()\""));
    }

    #[test]
    fn serialize_nodes_jsonl_sorted_by_id() {
        let f1 = Function {
            id: "function:zzz".to_string(),
            name: "z".to_string(),
            file_path: "a.rs".to_string(),
            line_start: 1,
            line_end: 1,
            signature: String::new(),
            docstring: None,
            body: String::new(),
            body_hash: String::new(),
            token_count: 0,
            embed_type: String::new(),
            embedding: vec![],
            summary: String::new(),
        };
        let f2 = Function {
            id: "function:aaa".to_string(),
            ..f1.clone()
        };
        let out = serialize_nodes_jsonl(&[], &[f1, f2], &[], &[]);
        let lines: Vec<&str> = out.trim().lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("\"id\":\"function:aaa\""));
        assert!(lines[1].contains("\"id\":\"function:zzz\""));
    }

    #[test]
    fn serialize_edges_jsonl_empty() {
        let out = serialize_edges_jsonl(&[]);
        assert_eq!(out, "");
    }

    #[test]
    fn serialize_edges_jsonl_sorted_by_type_from_to() {
        use crate::models::code_edge::{CodeEdge, CodeEdgeType};
        let edges = vec![
            CodeEdge {
                edge_type: CodeEdgeType::Imports,
                from: "code_file:b".to_string(),
                to: "code_file:a".to_string(),
                import_path: Some("crate::a".to_string()),
                linked_by: None,
                created_at: String::new(),
            },
            CodeEdge {
                edge_type: CodeEdgeType::Calls,
                from: "function:x".to_string(),
                to: "function:y".to_string(),
                import_path: None,
                linked_by: None,
                created_at: String::new(),
            },
        ];
        let out = serialize_edges_jsonl(&edges);
        let lines: Vec<&str> = out.trim().lines().collect();
        assert_eq!(lines.len(), 2);
        // calls < imports alphabetically
        assert!(lines[0].contains("\"type\":\"calls\""));
        assert!(lines[1].contains("\"type\":\"imports\""));
        assert!(lines[1].contains("\"import_path\":\"crate::a\""));
    }

    #[test]
    fn serialize_nodes_jsonl_code_file() {
        let cf = CodeFile {
            id: "code_file:f1".to_string(),
            path: "src/main.rs".to_string(),
            language: "rust".to_string(),
            size_bytes: 1024,
            content_hash: "hash123".to_string(),
            last_indexed_at: "2026-01-01T00:00:00Z".to_string(),
        };
        let out = serialize_nodes_jsonl(&[cf], &[], &[], &[]);
        assert!(out.contains("\"type\":\"code_file\""));
        assert!(out.contains("\"path\":\"src/main.rs\""));
        assert!(out.contains("\"size_bytes\":1024"));
    }
}
