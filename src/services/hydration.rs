//! Hydration: load workspace state from `.tmem/` files into SurrealDB.
//!
//! Parses human-readable Markdown (`tasks.md`) and SurrealQL (`graph.surql`)
//! files, populating the database for runtime queries. Supports stale file
//! detection and corruption recovery by re-hydrating from canonical files.

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::SystemTime;

use chrono::{DateTime, Utc};

use crate::db::queries::Queries;
use crate::errors::{HydrationError, TMemError};
use crate::models::graph::DependencyType;
use crate::models::task::{Task, TaskStatus, compute_priority_order};
use crate::services::dehydration::SCHEMA_VERSION;

#[derive(Debug, Clone, Default)]
pub struct HydrationSummary {
    pub task_count: u64,
    pub context_count: u64,
    pub last_flush: Option<String>,
    pub stale_files: bool,
    pub file_mtimes: HashMap<String, FileFingerprint>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileFingerprint {
    pub modified: SystemTime,
    pub len: u64,
}

/// Result of loading `.tmem/` files into the database.
#[derive(Debug, Clone, Default)]
pub struct HydrationResult {
    pub tasks_loaded: usize,
    pub edges_loaded: usize,
}

/// A task parsed from `tasks.md` with frontmatter fields.
#[derive(Debug, Clone)]
pub struct ParsedTask {
    pub task: Task,
    /// Labels parsed from the `labels:` frontmatter field.
    pub labels: Vec<String>,
}

/// A RELATE statement parsed from `graph.surql`.
#[derive(Debug, Clone)]
pub struct ParsedRelation {
    pub from: String,
    pub edge_type: String,
    pub to: String,
    pub properties: Vec<(String, String)>,
}

/// Load workspace state summary from `.tmem/` files (lightweight).
///
/// Creates the `.tmem/` directory if missing. Does NOT load data into the
/// database; use [`hydrate_into_db`] for that.
pub async fn hydrate_workspace(path: &Path) -> Result<HydrationSummary, TMemError> {
    let tmem_dir = path.join(".tmem");

    if !tmem_dir.exists() {
        fs::create_dir_all(&tmem_dir).map_err(|e| {
            TMemError::Hydration(HydrationError::Failed {
                reason: format!("failed to create .tmem directory: {e}"),
            })
        })?;
        return Ok(HydrationSummary::default());
    }

    let tasks_path = tmem_dir.join("tasks.md");
    let task_count = if tasks_path.exists() {
        count_tasks(&tasks_path)?
    } else {
        0
    };

    let context_count = 0;

    let file_mtimes = collect_file_mtimes(&tmem_dir);
    let (last_flush, stale_files) = last_flush_state(&tmem_dir, &file_mtimes);

    // Validate schema version if present
    let version_path = tmem_dir.join(".version");
    if version_path.exists() {
        let version = fs::read_to_string(&version_path).unwrap_or_default();
        let version = version.trim();
        if !version.is_empty() && version != SCHEMA_VERSION {
            return Err(TMemError::Hydration(HydrationError::SchemaMismatch {
                expected: SCHEMA_VERSION.to_string(),
                found: version.to_string(),
            }));
        }
    }

    Ok(HydrationSummary {
        task_count,
        context_count,
        last_flush,
        stale_files,
        file_mtimes,
    })
}

/// Parse `.tmem/` files and load all entities into the database.
///
/// Parses `tasks.md` for task data and `graph.surql` for relationship
/// edges. Upserts tasks (idempotent) and recreates edges.
pub async fn hydrate_into_db(path: &Path, queries: &Queries) -> Result<HydrationResult, TMemError> {
    let tmem_dir = path.join(".tmem");
    let mut tasks_loaded = 0;
    let mut edges_loaded = 0;

    // Parse and load tasks from tasks.md
    let tasks_path = tmem_dir.join("tasks.md");
    if tasks_path.exists() {
        let content = fs::read_to_string(&tasks_path).map_err(|e| {
            TMemError::Hydration(HydrationError::Failed {
                reason: format!("failed to read tasks.md: {e}"),
            })
        })?;
        let parsed_tasks = parse_tasks_md(&content);
        for pt in &parsed_tasks {
            queries.upsert_task(&pt.task).await?;
            for label in &pt.labels {
                // Ignore duplicate errors during rehydration (idempotent)
                let _ = queries.insert_label(&pt.task.id, label).await;
            }
            tasks_loaded += 1;
        }
    }

    // Parse and load edges from graph.surql
    let graph_path = tmem_dir.join("graph.surql");
    if graph_path.exists() {
        let content = fs::read_to_string(&graph_path).map_err(|e| {
            TMemError::Hydration(HydrationError::Failed {
                reason: format!("failed to read graph.surql: {e}"),
            })
        })?;
        let relations = parse_graph_surql(&content);
        for rel in &relations {
            apply_relation(queries, rel).await?;
            edges_loaded += 1;
        }
    }

    Ok(HydrationResult {
        tasks_loaded,
        edges_loaded,
    })
}

/// Perform corruption recovery: delete DB state and re-hydrate from files.
///
/// Called when DB is suspected corrupt. Clears all task and edge data,
/// then re-hydrates from the canonical `.tmem/` files.
pub async fn recover_from_corruption(
    path: &Path,
    queries: &Queries,
) -> Result<HydrationResult, TMemError> {
    // Clear existing DB data
    queries.clear_all_data().await?;

    // Re-hydrate from files
    hydrate_into_db(path, queries).await
}

/// Attempt to generate embeddings for specs and contexts that lack them.
///
/// Called after hydration to backfill missing embedding vectors. Failures
/// are logged but do not prevent the hydration from succeeding — the
/// `embeddings` feature may be disabled or the model unavailable.
pub async fn backfill_embeddings(queries: &Queries) {
    // Backfill specs missing embeddings
    if let Ok(specs) = queries.all_specs().await {
        let needs_embed: Vec<_> = specs.iter().filter(|s| s.embedding.is_none()).collect();
        if !needs_embed.is_empty() {
            let texts: Vec<String> = needs_embed
                .iter()
                .map(|s| format!("{}\n{}", s.title, s.content))
                .collect();
            if let Ok(embeddings) = crate::services::embedding::embed_texts(&texts) {
                for (spec, emb) in needs_embed.iter().zip(embeddings) {
                    let mut updated = (*spec).clone();
                    updated.embedding = Some(emb);
                    let _ = queries.upsert_spec(&updated).await;
                }
            }
        }
    }

    // Backfill contexts missing embeddings
    if let Ok(contexts) = queries.all_contexts().await {
        let needs_embed: Vec<_> = contexts.iter().filter(|c| c.embedding.is_none()).collect();
        if !needs_embed.is_empty() {
            let texts: Vec<String> = needs_embed.iter().map(|c| c.content.clone()).collect();
            if let Ok(embeddings) = crate::services::embedding::embed_texts(&texts) {
                for (ctx, emb) in needs_embed.iter().zip(embeddings) {
                    let _ = queries.set_context_embedding(&ctx.id, emb).await;
                }
            }
        }
    }
}
/// Detect whether `.tmem/` files have been modified since the last flush.
pub fn detect_stale(tmem_dir: &Path) -> bool {
    let file_mtimes = collect_file_mtimes(tmem_dir);
    let (_, stale) = last_flush_state(tmem_dir, &file_mtimes);
    stale
}

/// Read the `.tmem/.version` file contents.
pub fn read_version(tmem_dir: &Path) -> Option<String> {
    let path = tmem_dir.join(".version");
    fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Read the `.tmem/.lastflush` timestamp.
pub fn read_lastflush(tmem_dir: &Path) -> Option<DateTime<Utc>> {
    let path = tmem_dir.join(".lastflush");
    fs::read_to_string(path)
        .ok()
        .and_then(|s| DateTime::parse_from_rfc3339(s.trim()).ok())
        .map(|dt| dt.with_timezone(&Utc))
}

/// Capture modification times for known `.tmem/` files to support stale detection.
pub fn collect_file_mtimes(tmem_dir: &Path) -> HashMap<String, FileFingerprint> {
    let mut mtimes = HashMap::new();

    let candidates = [
        ("tasks.md", tmem_dir.join("tasks.md")),
        ("graph.surql", tmem_dir.join("graph.surql")),
        (".version", tmem_dir.join(".version")),
        (".lastflush", tmem_dir.join(".lastflush")),
    ];

    for (name, path) in candidates {
        if let Ok(meta) = fs::metadata(&path) {
            if let Ok(modified) = meta.modified() {
                mtimes.insert(
                    name.to_string(),
                    FileFingerprint {
                        modified,
                        len: meta.len(),
                    },
                );
            }
        }
    }

    mtimes
}

/// Determine if any tracked `.tmem/` file has changed since the recorded mtimes.
pub fn detect_stale_since(recorded: &HashMap<String, FileFingerprint>, tmem_dir: &Path) -> bool {
    if recorded.is_empty() {
        return false;
    }

    let current = collect_file_mtimes(tmem_dir);

    // Changed or missing files compared to baseline
    for (name, recorded_time) in recorded {
        match current.get(name) {
            Some(current_time) => {
                if current_time.modified > recorded_time.modified
                    || current_time.len != recorded_time.len
                {
                    return true;
                }
            }
            None => return true, // file was removed
        }
    }

    // New files not in baseline
    for name in current.keys() {
        if !recorded.contains_key(name) {
            return true;
        }
    }

    false
}

// ─── Parsing ────────────────────────────────────────────────────────────────

/// Parse `tasks.md` content into structured task records.
///
/// Extracts YAML frontmatter fields and body description from each
/// `## task:*` heading block. Uses `pulldown-cmark` for heading detection
/// and manual parsing for frontmatter extraction.
pub fn parse_tasks_md(content: &str) -> Vec<ParsedTask> {
    let lines: Vec<&str> = content.lines().collect();
    let mut tasks = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        if lines[i].starts_with("## task:") {
            let raw_heading = lines[i].trim_start_matches("## ").trim().to_string();
            let task_id = strip_table_prefix(&raw_heading);
            i += 1;

            // Skip blank lines
            while i < lines.len() && lines[i].trim().is_empty() {
                i += 1;
            }

            // Parse YAML frontmatter
            let mut frontmatter: HashMap<String, String> = HashMap::new();
            if i < lines.len() && lines[i].trim() == "---" {
                i += 1;
                while i < lines.len() && lines[i].trim() != "---" {
                    if let Some((key, value)) = lines[i].split_once(':') {
                        frontmatter.insert(key.trim().to_string(), value.trim().to_string());
                    }
                    i += 1;
                }
                if i < lines.len() {
                    i += 1; // skip closing ---
                }
            }

            // Skip blank lines after frontmatter
            while i < lines.len() && lines[i].trim().is_empty() {
                i += 1;
            }

            // Capture description (body text until next ## or EOF)
            let mut description = String::new();
            while i < lines.len() && !lines[i].starts_with("## ") {
                description.push_str(lines[i]);
                description.push('\n');
                i += 1;
            }
            let description = description.trim().to_string();

            // Build Task from frontmatter + body
            let title = frontmatter
                .get("title")
                .cloned()
                .unwrap_or_else(|| task_id.clone());
            let status = frontmatter
                .get("status")
                .and_then(|s| parse_status(s))
                .unwrap_or(TaskStatus::Todo);
            let work_item_id = frontmatter.get("work_item_id").cloned();
            let priority = frontmatter
                .get("priority")
                .cloned()
                .unwrap_or_else(|| "p2".to_string());
            let priority_order = frontmatter
                .get("priority_order")
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(|| compute_priority_order(&priority));
            let issue_type = frontmatter
                .get("issue_type")
                .cloned()
                .unwrap_or_else(|| "task".to_string());
            let assignee = frontmatter.get("assignee").cloned();
            let pinned = frontmatter.get("pinned").is_some_and(|s| s == "true");
            let labels: Vec<String> = frontmatter
                .get("labels")
                .map(|s| {
                    s.split(',')
                        .map(|l| l.trim().to_string())
                        .filter(|l| !l.is_empty())
                        .collect()
                })
                .unwrap_or_default();
            let created_at = frontmatter
                .get("created_at")
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now);
            let updated_at = frontmatter
                .get("updated_at")
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now);

            tasks.push(ParsedTask {
                task: Task {
                    id: task_id,
                    title,
                    status,
                    work_item_id,
                    description,
                    context_summary: None,
                    priority,
                    priority_order,
                    issue_type,
                    assignee,
                    defer_until: None,
                    pinned,
                    compaction_level: 0,
                    compacted_at: None,
                    workflow_state: None,
                    workflow_id: None,
                    created_at,
                    updated_at,
                },
                labels,
            });
        } else {
            i += 1;
        }
    }

    tasks
}

/// Parse `graph.surql` content into relation records.
///
/// Extracts RELATE statements, ignoring comments and unknown lines.
pub fn parse_graph_surql(content: &str) -> Vec<ParsedRelation> {
    let mut relations = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("--") || trimmed.is_empty() {
            continue;
        }
        if let Some(rel) = parse_relate_line(trimmed) {
            relations.push(rel);
        }
    }

    relations
}

/// Parse a single RELATE statement.
///
/// Format: `RELATE from->edge_type->to SET key = 'value';`
fn parse_relate_line(line: &str) -> Option<ParsedRelation> {
    let line = line.strip_prefix("RELATE ")?.strip_suffix(';')?;

    // Split on " SET " first to separate properties
    let (relate_part, set_part) = if let Some(idx) = line.find(" SET ") {
        (&line[..idx], Some(&line[idx + 5..]))
    } else {
        (line, None)
    };

    // Split relate_part on "->"
    let parts: Vec<&str> = relate_part.split("->").collect();
    if parts.len() < 3 {
        return None;
    }

    let from = parts[0].trim().to_string();
    let edge_type = parts[1].trim().to_string();
    let to = parts[2..].join("->").trim().to_string();

    let properties = set_part.map(parse_set_clauses).unwrap_or_default();

    Some(ParsedRelation {
        from,
        edge_type,
        to,
        properties,
    })
}

/// Parse SET clause key-value pairs.
fn parse_set_clauses(s: &str) -> Vec<(String, String)> {
    let mut clauses = Vec::new();
    for pair in s.split(',') {
        let pair = pair.trim();
        if let Some(eq_pos) = pair.find('=') {
            let key = pair[..eq_pos].trim().to_string();
            let value = pair[eq_pos + 1..].trim().trim_matches('\'').to_string();
            clauses.push((key, value));
        }
    }
    clauses
}

/// Apply a parsed RELATE statement to the database.
async fn apply_relation(queries: &Queries, rel: &ParsedRelation) -> Result<(), TMemError> {
    match rel.edge_type.as_str() {
        "depends_on" => {
            let from_id = strip_table_prefix(&rel.from);
            let to_id = strip_table_prefix(&rel.to);
            let kind = rel
                .properties
                .iter()
                .find(|(k, _)| k == "type")
                .map(|(_, v)| match v.as_str() {
                    "soft_dependency" => DependencyType::SoftDependency,
                    "child_of" => DependencyType::ChildOf,
                    "blocked_by" => DependencyType::BlockedBy,
                    "duplicate_of" => DependencyType::DuplicateOf,
                    "related_to" => DependencyType::RelatedTo,
                    "predecessor" => DependencyType::Predecessor,
                    "successor" => DependencyType::Successor,
                    _ => DependencyType::HardBlocker,
                })
                .unwrap_or(DependencyType::HardBlocker);
            queries.create_dependency(&from_id, &to_id, kind).await?;
        }
        "implements" => {
            let task_id = strip_table_prefix(&rel.from);
            let spec_id = strip_table_prefix(&rel.to);
            queries.link_task_spec(&task_id, &spec_id).await?;
        }
        "relates_to" => {
            let task_id = strip_table_prefix(&rel.from);
            let ctx_id = strip_table_prefix(&rel.to);
            queries.link_task_context(&task_id, &ctx_id).await?;
        }
        _ => {
            // Unknown edge type — skip silently per parsing rules
        }
    }
    Ok(())
}

/// Strip the table prefix from a record ID (e.g., "task:abc" → "abc").
fn strip_table_prefix(record: &str) -> String {
    record
        .split_once(':')
        .map(|(_, id)| id.to_string())
        .unwrap_or_else(|| record.to_string())
}

// ─── Internal helpers ───────────────────────────────────────────────────────

fn count_tasks(path: &Path) -> Result<u64, TMemError> {
    let contents = fs::read_to_string(path).map_err(|e| {
        TMemError::Hydration(HydrationError::Failed {
            reason: format!("failed to read tasks.md: {e}"),
        })
    })?;

    let count = contents
        .lines()
        .filter(|line| line.trim_start().starts_with("## task:"))
        .count();

    Ok(count as u64)
}

pub fn last_flush_state(
    tmem_dir: &Path,
    file_mtimes: &HashMap<String, FileFingerprint>,
) -> (Option<String>, bool) {
    let last_flush_path = tmem_dir.join(".lastflush");

    let last_flush_str = fs::read_to_string(&last_flush_path).ok();
    let last_flush = last_flush_str
        .as_deref()
        .and_then(|s| DateTime::parse_from_rfc3339(s.trim()).ok())
        .map(|dt| dt.with_timezone(&Utc).to_rfc3339());

    let mut stale_files = false;
    if let Some(ref flush) = last_flush {
        stale_files = file_mtimes
            .values()
            .any(|fingerprint| is_newer(&fingerprint.modified, flush));
    }

    (last_flush, stale_files)
}

fn is_newer(modified: &SystemTime, flush: &str) -> bool {
    if let Ok(flush_time) = DateTime::parse_from_rfc3339(flush) {
        if let Ok(duration) = modified.duration_since(SystemTime::UNIX_EPOCH) {
            let modified_time = DateTime::<Utc>::from(SystemTime::UNIX_EPOCH + duration);
            return modified_time > flush_time.with_timezone(&Utc);
        }
    }
    false
}

fn parse_status(raw: &str) -> Option<TaskStatus> {
    match raw {
        "todo" => Some(TaskStatus::Todo),
        "in_progress" => Some(TaskStatus::InProgress),
        "done" => Some(TaskStatus::Done),
        "blocked" => Some(TaskStatus::Blocked),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tasks_md_extracts_fields() {
        let content = r#"# Tasks

## task:abc123

---
id: task:abc123
title: Implement auth
status: in_progress
work_item_id: AB#12345
created_at: 2026-02-05T10:00:00+00:00
updated_at: 2026-02-05T14:30:00+00:00
---

Detailed description of the task.
"#;
        let tasks = parse_tasks_md(content);
        assert_eq!(tasks.len(), 1);
        let t = &tasks[0].task;
        assert_eq!(t.id, "abc123");
        assert_eq!(t.title, "Implement auth");
        assert_eq!(t.status, TaskStatus::InProgress);
        assert_eq!(t.work_item_id.as_deref(), Some("AB#12345"));
        assert!(t.description.contains("Detailed description"));
    }

    #[test]
    fn parse_tasks_md_multiple_tasks() {
        let content = r#"# Tasks

## task:a

---
id: task:a
title: First
status: todo
---

Task A desc.

## task:b

---
id: task:b
title: Second
status: done
---

Task B desc.
"#;
        let tasks = parse_tasks_md(content);
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].task.id, "a");
        assert_eq!(tasks[1].task.id, "b");
        assert_eq!(tasks[1].task.status, TaskStatus::Done);
    }

    #[test]
    fn parse_graph_surql_extracts_relations() {
        let content = r#"-- Generated by T-Mem
-- Schema version: 1.0.0

-- Dependencies
RELATE task:abc->depends_on->task:def SET type = 'hard_blocker';
RELATE task:ghi->depends_on->task:abc SET type = 'soft_dependency';

-- Implementations
RELATE task:abc->implements->spec:auth;

-- Context Relations
RELATE task:abc->relates_to->context:note1;
"#;
        let rels = parse_graph_surql(content);
        assert_eq!(rels.len(), 4);
        assert_eq!(rels[0].from, "task:abc");
        assert_eq!(rels[0].edge_type, "depends_on");
        assert_eq!(rels[0].to, "task:def");
        assert_eq!(
            rels[0].properties,
            vec![("type".into(), "hard_blocker".into())]
        );
        assert_eq!(rels[2].edge_type, "implements");
        assert_eq!(rels[3].edge_type, "relates_to");
    }

    #[test]
    fn parse_relate_line_basic() {
        let rel = parse_relate_line("RELATE task:a->depends_on->task:b SET type = 'hard_blocker';");
        assert!(rel.is_some());
        let rel = rel.unwrap();
        assert_eq!(rel.from, "task:a");
        assert_eq!(rel.edge_type, "depends_on");
        assert_eq!(rel.to, "task:b");
    }

    #[test]
    fn parse_relate_line_no_set() {
        let rel = parse_relate_line("RELATE task:x->implements->spec:y;");
        assert!(rel.is_some());
        let rel = rel.unwrap();
        assert_eq!(rel.from, "task:x");
        assert_eq!(rel.to, "spec:y");
        assert!(rel.properties.is_empty());
    }

    #[test]
    fn strip_table_prefix_works() {
        assert_eq!(strip_table_prefix("task:abc"), "abc");
        assert_eq!(strip_table_prefix("spec:xyz"), "xyz");
        assert_eq!(strip_table_prefix("nocolon"), "nocolon");
    }

    #[test]
    fn stale_detection_no_lastflush() {
        let dir = tempfile::tempdir().expect("tempdir");
        let tasks = dir.path().join("tasks.md");
        fs::write(&tasks, "# Tasks").expect("write");
        let mtimes = collect_file_mtimes(dir.path());
        let (flush, stale) = last_flush_state(dir.path(), &mtimes);
        assert!(flush.is_none());
        assert!(!stale, "no lastflush means not stale");
    }

    #[test]
    fn stale_detection_fresh() {
        let dir = tempfile::tempdir().expect("tempdir");
        let tasks = dir.path().join("tasks.md");
        fs::write(&tasks, "# Tasks").expect("write tasks");
        // Write lastflush with a future timestamp so tasks.md appears older
        let future = Utc::now() + chrono::Duration::hours(1);
        fs::write(dir.path().join(".lastflush"), future.to_rfc3339()).expect("write flush");
        let mtimes = collect_file_mtimes(dir.path());
        let (flush, stale) = last_flush_state(dir.path(), &mtimes);
        assert!(flush.is_some());
        assert!(!stale, "tasks.md older than lastflush is not stale");
    }

    #[test]
    fn read_version_returns_content() {
        let dir = tempfile::tempdir().expect("tempdir");
        fs::write(dir.path().join(".version"), "1.0.0\n").expect("write");
        assert_eq!(read_version(dir.path()), Some("1.0.0".to_string()));
    }

    #[test]
    fn read_version_missing_returns_none() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert_eq!(read_version(dir.path()), None);
    }
}
