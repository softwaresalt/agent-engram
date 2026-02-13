//! Dehydration: serialize workspace state from SurrealDB to `.tmem/` files.
//!
//! Produces human-readable Markdown (`tasks.md`) and SurrealQL (`graph.surql`)
//! files that can be committed to Git. User-added comments in existing
//! files are preserved across flushes via diff-based merging (FR-012).

use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::Utc;
use similar::{ChangeTag, TextDiff};

use crate::db::queries::{DependencyEdge, ImplementsEdge, Queries, RelatesToEdge};
use crate::errors::{SystemError, TMemError};
use crate::models::graph::DependencyType;
use crate::models::task::{Task, TaskStatus};

use crate::db::connect_db;
use crate::server::state::AppState;

/// Schema version written to `.tmem/.version`.
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

/// Dehydrate full workspace state from the database into `.tmem/` files.
///
/// Writes `tasks.md`, `graph.surql`, `.version`, and `.lastflush`.
/// Preserves user-added comments in existing `tasks.md` (FR-012).
pub async fn dehydrate_workspace(
    queries: &Queries,
    workspace_path: &Path,
) -> Result<DehydrationResult, TMemError> {
    let tmem_dir = workspace_path.join(".tmem");
    fs::create_dir_all(&tmem_dir).map_err(|_| flush_err(&tmem_dir))?;

    let mut tasks = queries.all_tasks().await?;

    // Safety net: if the DB is empty (e.g., after a rehydrate into a fresh store),
    // fall back to parsing the on-disk tasks.md so we do not drop user edits.
    if tasks.is_empty() {
        let tasks_path = tmem_dir.join("tasks.md");
        if let Ok(content) = fs::read_to_string(&tasks_path) {
            let parsed = crate::services::hydration::parse_tasks_md(&content);
            tasks = parsed.into_iter().map(|p| p.task).collect();
        }
    }
    let dep_edges = queries.all_dependency_edges().await?;
    let impl_edges = queries.all_implements_edges().await?;
    let rel_edges = queries.all_relates_to_edges().await?;

    // Read existing tasks.md for comment preservation (FR-012)
    let tasks_path = tmem_dir.join("tasks.md");
    let old_content = fs::read_to_string(&tasks_path).unwrap_or_default();
    let old_blocks = parse_task_blocks(&old_content);
    let old_bodies: HashMap<String, String> = old_blocks
        .into_iter()
        .map(|b| (b.task_id, b.body))
        .collect();
    let comments_preserved = old_bodies.values().filter(|b| !b.trim().is_empty()).count();

    // Serialize tasks.md preserving user comments
    let tasks_content = serialize_tasks_md(&tasks, &old_bodies, &old_content);
    atomic_write(&tasks_path, &tasks_content)?;

    // Serialize graph.surql
    let graph_path = tmem_dir.join("graph.surql");
    let total_edges = dep_edges.len() + impl_edges.len() + rel_edges.len();
    let graph_content = serialize_graph_surql(&dep_edges, &impl_edges, &rel_edges);
    atomic_write(&graph_path, &graph_content)?;

    // Write version
    let version_path = tmem_dir.join(".version");
    atomic_write(&version_path, SCHEMA_VERSION)?;

    // Write lastflush timestamp
    let flush_ts = Utc::now().to_rfc3339();
    let lastflush_path = tmem_dir.join(".lastflush");
    atomic_write(&lastflush_path, &flush_ts)?;

    let files_written = vec![
        ".tmem/tasks.md".to_string(),
        ".tmem/graph.surql".to_string(),
        ".tmem/.version".to_string(),
        ".tmem/.lastflush".to_string(),
    ];

    Ok(DehydrationResult {
        files_written,
        tasks_written: tasks.len(),
        edges_written: total_edges,
        comments_preserved,
        flush_timestamp: flush_ts,
    })
}

/// Flush all active workspaces to `.tmem/` files (FR-006).
///
/// Called during graceful shutdown (SIGTERM/SIGINT) to ensure in-memory
/// state is persisted before the daemon exits. Returns `Ok(())` when
/// no workspace is active.
pub async fn flush_all_workspaces(state: &AppState) -> Result<(), TMemError> {
    let Some(snapshot) = state.snapshot_workspace().await else {
        return Ok(());
    };

    let workspace_path = PathBuf::from(&snapshot.path);
    let db = connect_db(&snapshot.workspace_id).await?;
    let queries = Queries::new(db);

    tracing::info!(path = %snapshot.path, "shutdown: flushing workspace");
    dehydrate_workspace(&queries, &workspace_path).await?;
    tracing::info!(path = %snapshot.path, "shutdown: flush complete");

    Ok(())
}

/// Serialize tasks to the canonical `tasks.md` format.
///
/// Preserves file-level header and per-task user comments from the old file.
pub fn serialize_tasks_md(
    tasks: &[Task],
    old_bodies: &HashMap<String, String>,
    old_content: &str,
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
/// Uses `similar::TextDiff` to identify lines in the old body that
/// are not part of the new description (= user-added content per FR-012).
pub fn merge_body_with_comments(new_description: &str, old_body: &str) -> String {
    let new_desc_trimmed = new_description.trim();
    let old_body_trimmed = old_body.trim();

    if old_body_trimmed.is_empty() {
        return new_desc_trimmed.to_string();
    }

    if new_desc_trimmed == old_body_trimmed {
        return new_desc_trimmed.to_string();
    }

    // Find user-added lines: present in old body but not in new description
    let diff = TextDiff::from_lines(new_desc_trimmed, old_body_trimmed);
    let mut user_lines: Vec<String> = Vec::new();

    for change in diff.iter_all_changes() {
        if change.tag() == ChangeTag::Insert {
            let line = change.value().to_string();
            user_lines.push(line);
        }
    }

    let mut result = new_desc_trimmed.to_string();
    if !user_lines.is_empty() {
        result.push_str("\n\n");
        for line in user_lines {
            result.push_str(&line);
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

    out.push_str("-- Generated by T-Mem. Do not edit manually.\n");
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
pub fn atomic_write(path: &Path, content: &str) -> Result<(), TMemError> {
    let tmp_path = path.with_extension("tmp");
    let mut file = fs::File::create(&tmp_path).map_err(|_| flush_err(path))?;
    file.write_all(content.as_bytes())
        .map_err(|_| flush_err(path))?;
    file.sync_all().map_err(|_| flush_err(path))?;
    drop(file);

    fs::rename(&tmp_path, path).map_err(|_| flush_err(path))?;

    Ok(())
}

fn flush_err(path: &Path) -> TMemError {
    TMemError::System(SystemError::FlushFailed {
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
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn serialize_tasks_md_empty() {
        let out = serialize_tasks_md(&[], &HashMap::new(), "");
        assert!(out.starts_with("# Tasks\n"));
    }

    #[test]
    fn serialize_tasks_md_round_trip_fields() {
        let tasks = vec![make_task("task:abc", "Do something")];
        let out = serialize_tasks_md(&tasks, &HashMap::new(), "");
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
        assert!(out.contains("Generated by T-Mem"));
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
        let out = serialize_tasks_md(&tasks, &old_bodies, old_content);
        assert!(out.contains("<!-- Header comment -->"));
        assert!(out.contains("<!-- Important user note -->"));
        assert!(out.contains("My description"));
    }
}
