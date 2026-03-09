//! Hydration: load workspace state from `.engram/` files into SurrealDB.
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
use crate::errors::{EngramError, HydrationError};
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

/// Result of loading `.engram/` files into the database.
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

/// Load workspace state summary from `.engram/` files (lightweight).
///
/// Creates the `.engram/` directory if missing. Does NOT load data into the
/// database; use [`hydrate_into_db`] for that.
pub async fn hydrate_workspace(path: &Path) -> Result<HydrationSummary, EngramError> {
    let engram_dir = path.join(".engram");

    if !engram_dir.exists() {
        tokio::fs::create_dir_all(&engram_dir).await.map_err(|e| {
            EngramError::Hydration(HydrationError::Failed {
                reason: format!("failed to create .engram directory: {e}"),
            })
        })?;
        return Ok(HydrationSummary::default());
    }

    let tasks_path = engram_dir.join("tasks.md");
    let task_count = if tokio::fs::try_exists(&tasks_path).await.map_err(|e| {
        EngramError::Hydration(HydrationError::Failed {
            reason: format!("failed to check tasks.md existence: {e}"),
        })
    })? {
        count_tasks(&tasks_path).await?
    } else {
        0
    };

    let context_count = 0;

    let file_mtimes = collect_file_mtimes(&engram_dir);
    let (last_flush, stale_files) = last_flush_state(&engram_dir, &file_mtimes);

    // Validate schema version if present
    let version_path = engram_dir.join(".version");
    if tokio::fs::try_exists(&version_path).await.map_err(|e| {
        EngramError::Hydration(HydrationError::Failed {
            reason: format!("failed to check .version existence: {e}"),
        })
    })? {
        let version = tokio::fs::read_to_string(&version_path)
            .await
            .map_err(|e| {
                EngramError::Hydration(HydrationError::Failed {
                    reason: format!("failed to read .version file: {e}"),
                })
            })?;
        let version = version.trim();
        if !version.is_empty() && version != SCHEMA_VERSION {
            return Err(EngramError::Hydration(HydrationError::SchemaMismatch {
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

/// Parse `.engram/` files and load all entities into the database.
///
/// Parses `tasks.md` for task data and `graph.surql` for relationship
/// edges. Upserts tasks (idempotent) and recreates edges.
pub async fn hydrate_into_db(
    path: &Path,
    queries: &Queries,
) -> Result<HydrationResult, EngramError> {
    let engram_dir = path.join(".engram");
    let mut tasks_loaded = 0;
    let mut edges_loaded = 0;

    // Parse and load tasks from tasks.md
    let tasks_path = engram_dir.join("tasks.md");
    if tokio::fs::try_exists(&tasks_path).await.unwrap_or(false) {
        let content = tokio::fs::read_to_string(&tasks_path).await.map_err(|e| {
            EngramError::Hydration(HydrationError::Failed {
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
    let graph_path = engram_dir.join("graph.surql");
    if tokio::fs::try_exists(&graph_path).await.unwrap_or(false) {
        let content = tokio::fs::read_to_string(&graph_path).await.map_err(|e| {
            EngramError::Hydration(HydrationError::Failed {
                reason: format!("failed to read graph.surql: {e}"),
            })
        })?;
        let relations = parse_graph_surql(&content);
        for rel in &relations {
            apply_relation(queries, rel).await?;
            edges_loaded += 1;
        }
    }

    // Parse and load comments from comments.md (FR-063b)
    let comments_path = engram_dir.join("comments.md");
    if tokio::fs::try_exists(&comments_path).await.unwrap_or(false) {
        let content = tokio::fs::read_to_string(&comments_path)
            .await
            .map_err(|e| {
                EngramError::Hydration(HydrationError::Failed {
                    reason: format!("failed to read comments.md: {e}"),
                })
            })?;
        let comments = parse_comments_md(&content);
        for comment in &comments {
            // Strip task: prefix to match internal bare ID convention
            let bare_task_id = comment
                .task_id
                .strip_prefix("task:")
                .unwrap_or(&comment.task_id);
            // Idempotent: ignore duplicate insert errors during rehydration
            let _ = queries
                .insert_comment(bare_task_id, &comment.content, &comment.author)
                .await;
        }
    }

    Ok(HydrationResult {
        tasks_loaded,
        edges_loaded,
    })
}

// ── Collection hydration (T092) ───────────────────────────────────────────────

/// Load collections from `.engram/collections.md` into the database.
///
/// Parses the Markdown format written by
/// [`dehydrate_collections`](crate::services::dehydration::dehydrate_collections)
/// and upserts each collection. Missing collection IDs cause the collection to
/// be skipped (corrupt line). Returns the number of collections successfully
/// loaded.
///
/// Returns `Ok(0)` when the file does not exist (first run or no collections).
pub async fn hydrate_collections(path: &Path, queries: &Queries) -> Result<usize, EngramError> {
    let collections_path = path.join(".engram").join("collections.md");

    if !tokio::fs::try_exists(&collections_path)
        .await
        .unwrap_or(false)
    {
        return Ok(0);
    }

    let content = tokio::fs::read_to_string(&collections_path)
        .await
        .map_err(|e| {
            EngramError::Hydration(HydrationError::Failed {
                reason: format!("failed to read collections.md: {e}"),
            })
        })?;

    let parsed = parse_collections_md(&content);
    let mut loaded = 0usize;

    for pc in &parsed {
        // Use UPSERT so re-hydration is idempotent.
        let result = upsert_collection(queries, pc).await;
        match result {
            Ok(()) => loaded += 1,
            Err(e) => {
                tracing::warn!(
                    collection_name = %pc.name,
                    "skipping corrupt collection during hydration: {e}"
                );
            }
        }
    }

    Ok(loaded)
}

/// A collection parsed from `collections.md`.
#[derive(Debug, Clone)]
struct ParsedCollection {
    name: String,
    id: Option<String>,
    description: Option<String>,
    created_at: Option<DateTime<Utc>>,
    updated_at: Option<DateTime<Utc>>,
}

/// Parse the `collections.md` format into a list of [`ParsedCollection`]s.
fn parse_collections_md(content: &str) -> Vec<ParsedCollection> {
    let mut collections: Vec<ParsedCollection> = Vec::new();
    let mut current: Option<ParsedCollection> = None;

    for line in content.lines() {
        let line = line.trim();

        // `## Name` starts a new collection section.
        if let Some(name) = line.strip_prefix("## ") {
            if let Some(c) = current.take() {
                collections.push(c);
            }
            current = Some(ParsedCollection {
                name: name.trim().to_string(),
                id: None,
                description: None,
                created_at: None,
                updated_at: None,
            });
            continue;
        }

        // Parse `- **key**: value` list items.
        if let Some(rest) = line.strip_prefix("- **") {
            if let Some((key, value)) = rest.split_once("**: ") {
                if let Some(ref mut c) = current {
                    match key {
                        "id" => c.id = Some(value.trim().to_string()),
                        "description" => c.description = Some(value.trim().to_string()),
                        "created_at" => {
                            c.created_at = value.trim().parse::<DateTime<Utc>>().ok();
                        }
                        "updated_at" => {
                            c.updated_at = value.trim().parse::<DateTime<Utc>>().ok();
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    if let Some(c) = current {
        collections.push(c);
    }

    collections
}

/// Upsert a single parsed collection into the database.
async fn upsert_collection(queries: &Queries, pc: &ParsedCollection) -> Result<(), EngramError> {
    let Some(ref full_id) = pc.id else {
        return Err(EngramError::Hydration(HydrationError::Failed {
            reason: format!("collection '{}' has no id field", pc.name),
        }));
    };

    let now = chrono::Utc::now();
    let collection = crate::models::Collection {
        id: full_id.clone(),
        name: pc.name.clone(),
        description: pc.description.clone(),
        created_at: pc.created_at.unwrap_or(now),
        updated_at: pc.updated_at.unwrap_or(now),
    };

    queries.upsert_collection(&collection).await
}

/// Perform corruption recovery: delete DB state and re-hydrate from files.
///
/// Called when DB is suspected corrupt. Clears all task and edge data,
/// then re-hydrates from the canonical `.engram/` files.
pub async fn recover_from_corruption(
    path: &Path,
    queries: &Queries,
) -> Result<HydrationResult, EngramError> {
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

/// Result of loading code graph JSONL files into the database.
#[derive(Debug, Clone, Default)]
pub struct CodeGraphHydrationResult {
    /// Total nodes loaded (code_files + functions + classes + interfaces).
    pub nodes_loaded: usize,
    /// Total edges loaded.
    pub edges_loaded: usize,
    /// Lines skipped due to parse errors (FR-135).
    pub lines_skipped: usize,
}

/// Hydrate code graph from `.engram/code-graph/` JSONL files (FR-132, FR-135).
///
/// Parses `nodes.jsonl` and `edges.jsonl`, upserting into SurrealDB.
/// Corrupt lines are logged and skipped (FR-135: graceful degradation).
pub async fn hydrate_code_graph(
    path: &Path,
    cg_queries: &crate::db::queries::CodeGraphQueries,
) -> Result<CodeGraphHydrationResult, EngramError> {
    let code_graph_dir = path.join(".engram").join("code-graph");
    let mut result = CodeGraphHydrationResult::default();

    // Parse nodes.jsonl
    let nodes_path = code_graph_dir.join("nodes.jsonl");
    if tokio::fs::try_exists(&nodes_path).await.unwrap_or(false) {
        let content = tokio::fs::read_to_string(&nodes_path).await.map_err(|e| {
            EngramError::Hydration(HydrationError::Failed {
                reason: format!("failed to read nodes.jsonl: {e}"),
            })
        })?;

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Ok(node) = parse_node_line(line) {
                if upsert_node(cg_queries, node, path).await {
                    result.nodes_loaded += 1;
                } else {
                    result.lines_skipped += 1;
                }
            } else {
                tracing::warn!(
                    line_preview = &line[..line.len().min(80)],
                    "skipping corrupt nodes.jsonl line (FR-135)"
                );
                result.lines_skipped += 1;
            }
        }
    }

    // Parse edges.jsonl
    let edges_path = code_graph_dir.join("edges.jsonl");
    if tokio::fs::try_exists(&edges_path).await.unwrap_or(false) {
        let content = tokio::fs::read_to_string(&edges_path).await.map_err(|e| {
            EngramError::Hydration(HydrationError::Failed {
                reason: format!("failed to read edges.jsonl: {e}"),
            })
        })?;

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Ok(edge) = parse_edge_line(line) {
                if upsert_edge(cg_queries, &edge).await {
                    result.edges_loaded += 1;
                } else {
                    result.lines_skipped += 1;
                }
            } else {
                tracing::warn!(
                    line_preview = &line[..line.len().min(80)],
                    "skipping corrupt edges.jsonl line (FR-135)"
                );
                result.lines_skipped += 1;
            }
        }
    }

    Ok(result)
}

/// Intermediate node representation parsed from a JSONL line.
#[derive(Debug, serde::Deserialize)]
struct ParsedNode {
    id: String,
    #[serde(rename = "type")]
    node_type: String,
    // code_file fields
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    language: Option<String>,
    #[serde(default)]
    size_bytes: Option<u64>,
    #[serde(default)]
    content_hash: Option<String>,
    #[serde(default)]
    last_indexed_at: Option<String>,
    // symbol fields
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    file_path: Option<String>,
    #[serde(default)]
    line_start: Option<u32>,
    #[serde(default)]
    line_end: Option<u32>,
    #[serde(default)]
    signature: Option<String>,
    #[serde(default)]
    docstring: Option<String>,
    #[serde(default)]
    body_hash: Option<String>,
    #[serde(default)]
    token_count: Option<u32>,
    #[serde(default)]
    embed_type: Option<String>,
    #[serde(default)]
    embedding: Option<Vec<f32>>,
    #[serde(default)]
    summary: Option<String>,
}

/// Intermediate edge representation parsed from a JSONL line.
#[derive(Debug, serde::Deserialize)]
struct ParsedEdge {
    #[serde(rename = "type")]
    edge_type: String,
    from: String,
    to: String,
    #[serde(default)]
    import_path: Option<String>,
    #[serde(default)]
    linked_by: Option<String>,
    #[allow(dead_code)]
    #[serde(default)]
    created_at: Option<String>,
}

fn parse_node_line(line: &str) -> Result<ParsedNode, serde_json::Error> {
    serde_json::from_str(line)
}

fn parse_edge_line(line: &str) -> Result<ParsedEdge, serde_json::Error> {
    serde_json::from_str(line)
}

/// Read the source body for a symbol from its file on disk.
///
/// Extracts lines `line_start..=line_end` (1-based, inclusive) from `file_path`
/// (workspace-relative). Returns an empty `String` if the file cannot be read or
/// the line range is out of bounds; logs a warning in both cases.
///
/// Body re-derivation is best-effort — a missing body is a degraded but non-fatal
/// condition; the node is still hydrated and searchable by name and hash.
async fn read_body_lines(
    workspace: &Path,
    file_path: &str,
    line_start: u32,
    line_end: u32,
) -> String {
    if file_path.is_empty() || line_start == 0 {
        return String::new();
    }
    let abs = workspace.join(file_path);
    let content = match tokio::fs::read_to_string(&abs).await {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(
                file = %abs.display(),
                error = %e,
                "hydration: cannot read source file for body re-derivation"
            );
            return String::new();
        }
    };
    let lines: Vec<&str> = content.lines().collect();
    // Convert 1-based inclusive range to 0-based half-open slice indices.
    let start = (line_start as usize).saturating_sub(1);
    let end = (line_end as usize).min(lines.len());
    if start >= lines.len() || start >= end {
        tracing::warn!(
            file = %abs.display(),
            line_start,
            line_end,
            total_lines = lines.len(),
            "hydration: line range out of bounds during body re-derivation"
        );
        return String::new();
    }
    lines[start..end].join("\n")
}

/// Upsert a parsed node into the database. Returns `true` on success.
async fn upsert_node(
    cg_queries: &crate::db::queries::CodeGraphQueries,
    node: ParsedNode,
    workspace: &Path,
) -> bool {
    use crate::models::{Class, CodeFile, Function, Interface};

    match node.node_type.as_str() {
        "code_file" => {
            let cf = CodeFile {
                id: node.id,
                path: node.path.unwrap_or_default(),
                language: node.language.unwrap_or_default(),
                size_bytes: node.size_bytes.unwrap_or(0),
                content_hash: node.content_hash.unwrap_or_default(),
                last_indexed_at: node.last_indexed_at.unwrap_or_default(),
            };
            cg_queries.upsert_code_file(&cf).await.is_ok()
        }
        "function" => {
            let file_path = node.file_path.unwrap_or_default();
            let line_start = node.line_start.unwrap_or(0);
            let line_end = node.line_end.unwrap_or(0);
            let body = read_body_lines(workspace, &file_path, line_start, line_end).await;
            let f = Function {
                id: node.id,
                name: node.name.unwrap_or_default(),
                file_path,
                line_start,
                line_end,
                signature: node.signature.unwrap_or_default(),
                docstring: node.docstring,
                body,
                body_hash: node.body_hash.unwrap_or_default(),
                token_count: node.token_count.unwrap_or(0),
                embed_type: node.embed_type.unwrap_or_default(),
                embedding: node.embedding.unwrap_or_default(),
                summary: node.summary.unwrap_or_default(),
            };
            cg_queries.upsert_function(&f).await.is_ok()
        }
        "class" => {
            let file_path = node.file_path.unwrap_or_default();
            let line_start = node.line_start.unwrap_or(0);
            let line_end = node.line_end.unwrap_or(0);
            let body = read_body_lines(workspace, &file_path, line_start, line_end).await;
            let c = Class {
                id: node.id,
                name: node.name.unwrap_or_default(),
                file_path,
                line_start,
                line_end,
                docstring: node.docstring,
                body,
                body_hash: node.body_hash.unwrap_or_default(),
                token_count: node.token_count.unwrap_or(0),
                embed_type: node.embed_type.unwrap_or_default(),
                embedding: node.embedding.unwrap_or_default(),
                summary: node.summary.unwrap_or_default(),
            };
            cg_queries.upsert_class(&c).await.is_ok()
        }
        "interface" => {
            let file_path = node.file_path.unwrap_or_default();
            let line_start = node.line_start.unwrap_or(0);
            let line_end = node.line_end.unwrap_or(0);
            let body = read_body_lines(workspace, &file_path, line_start, line_end).await;
            let i = Interface {
                id: node.id,
                name: node.name.unwrap_or_default(),
                file_path,
                line_start,
                line_end,
                docstring: node.docstring,
                body,
                body_hash: node.body_hash.unwrap_or_default(),
                token_count: node.token_count.unwrap_or(0),
                embed_type: node.embed_type.unwrap_or_default(),
                embedding: node.embedding.unwrap_or_default(),
                summary: node.summary.unwrap_or_default(),
            };
            cg_queries.upsert_interface(&i).await.is_ok()
        }
        _ => {
            tracing::warn!(
                node_type = node.node_type,
                "unknown node type in nodes.jsonl"
            );
            false
        }
    }
}

/// Upsert a parsed edge into the database. Returns `true` on success.
async fn upsert_edge(cg_queries: &crate::db::queries::CodeGraphQueries, edge: &ParsedEdge) -> bool {
    match edge.edge_type.as_str() {
        "calls" => cg_queries
            .create_calls_edge(&edge.from, &edge.to)
            .await
            .is_ok(),
        "imports" => {
            let import_path = edge.import_path.as_deref().unwrap_or("");
            cg_queries
                .create_imports_edge(&edge.from, &edge.to, import_path)
                .await
                .is_ok()
        }
        "defines" => {
            // create_defines_edge(file_id, symbol_table, symbol_id)
            let (to_table, to_id) = split_thing_id(&edge.to);
            cg_queries
                .create_defines_edge(&edge.from, to_table, to_id)
                .await
                .is_ok()
        }
        "inherits_from" => {
            // create_inherits_edge(child_table, child_id, parent_table, parent_id)
            let (from_table, from_id) = split_thing_id(&edge.from);
            let (to_table, to_id) = split_thing_id(&edge.to);
            cg_queries
                .create_inherits_edge(from_table, from_id, to_table, to_id)
                .await
                .is_ok()
        }
        "concerns" => {
            // create_concerns_edge(task_id, symbol_table, symbol_id, linked_by)
            let (to_table, to_id) = split_thing_id(&edge.to);
            let linked_by = edge.linked_by.as_deref().unwrap_or("hydration");
            cg_queries
                .create_concerns_edge(&edge.from, to_table, to_id, linked_by)
                .await
                .is_ok()
        }
        _ => {
            tracing::warn!(
                edge_type = edge.edge_type,
                "unknown edge type in edges.jsonl"
            );
            false
        }
    }
}

/// Split a `"table:id"` string into `("table", "id")`.
/// Falls back to `("unknown", full_id)` if no colon is found.
fn split_thing_id(id: &str) -> (&str, &str) {
    id.split_once(':').unwrap_or(("unknown", id))
}

/// Detect whether `.engram/` files have been modified since the last flush.
pub fn detect_stale(engram_dir: &Path) -> bool {
    let file_mtimes = collect_file_mtimes(engram_dir);
    let (_, stale) = last_flush_state(engram_dir, &file_mtimes);
    stale
}

/// Read the `.engram/.version` file contents.
pub fn read_version(engram_dir: &Path) -> Option<String> {
    let path = engram_dir.join(".version");
    fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Read the `.engram/.lastflush` timestamp.
pub fn read_lastflush(engram_dir: &Path) -> Option<DateTime<Utc>> {
    let path = engram_dir.join(".lastflush");
    fs::read_to_string(path)
        .ok()
        .and_then(|s| DateTime::parse_from_rfc3339(s.trim()).ok())
        .map(|dt| dt.with_timezone(&Utc))
}

/// Capture modification times for known `.engram/` files to support stale detection.
pub fn collect_file_mtimes(engram_dir: &Path) -> HashMap<String, FileFingerprint> {
    let mut mtimes = HashMap::new();

    let candidates = [
        ("tasks.md", engram_dir.join("tasks.md")),
        ("graph.surql", engram_dir.join("graph.surql")),
        (".version", engram_dir.join(".version")),
        (".lastflush", engram_dir.join(".lastflush")),
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

/// Determine if any tracked `.engram/` file has changed since the recorded mtimes.
pub fn detect_stale_since(recorded: &HashMap<String, FileFingerprint>, engram_dir: &Path) -> bool {
    if recorded.is_empty() {
        return false;
    }

    let current = collect_file_mtimes(engram_dir);

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
            let defer_until = frontmatter
                .get("defer_until")
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&Utc));
            let compaction_level = frontmatter
                .get("compaction_level")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            let compacted_at = frontmatter
                .get("compacted_at")
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&Utc));
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
                    defer_until,
                    pinned,
                    compaction_level,
                    compacted_at,
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

/// A comment parsed from `comments.md`.
#[derive(Debug, Clone)]
pub struct ParsedComment {
    pub task_id: String,
    pub author: String,
    pub content: String,
}

/// Parse `.engram/comments.md` into a list of comments.
///
/// Expected format:
/// ```text
/// ## task:abc123
///
/// ### 2026-02-14T12:00:00+00:00 — agent-1
///
/// Comment body text here.
///
/// ### 2026-02-14T13:00:00+00:00 — agent-2
///
/// Another comment.
///
/// ## task:def456
/// ...
/// ```
pub fn parse_comments_md(content: &str) -> Vec<ParsedComment> {
    let mut comments = Vec::new();
    let mut current_task: Option<String> = None;
    let mut current_author: Option<String> = None;
    let mut current_body = String::new();

    for line in content.lines() {
        if let Some(task_heading) = line.strip_prefix("## ") {
            // Flush any pending comment
            if let (Some(tid), Some(auth)) = (&current_task, &current_author) {
                let body = current_body.trim().to_string();
                if !body.is_empty() {
                    comments.push(ParsedComment {
                        task_id: tid.clone(),
                        author: auth.clone(),
                        content: body,
                    });
                }
            }
            current_task = Some(task_heading.trim().to_string());
            current_author = None;
            current_body.clear();
        } else if let Some(comment_heading) = line.strip_prefix("### ") {
            // Flush previous comment in same task section
            if let (Some(tid), Some(auth)) = (&current_task, &current_author) {
                let body = current_body.trim().to_string();
                if !body.is_empty() {
                    comments.push(ParsedComment {
                        task_id: tid.clone(),
                        author: auth.clone(),
                        content: body,
                    });
                }
            }
            // Parse "timestamp — author" or "timestamp -- author"
            let heading = comment_heading.trim();
            let author = heading
                .split(" — ")
                .nth(1)
                .or_else(|| heading.split(" -- ").nth(1))
                .unwrap_or("unknown")
                .trim()
                .to_string();
            current_author = Some(author);
            current_body.clear();
        } else if current_author.is_some() {
            current_body.push_str(line);
            current_body.push('\n');
        }
    }

    // Flush final comment
    if let (Some(tid), Some(auth)) = (&current_task, &current_author) {
        let body = current_body.trim().to_string();
        if !body.is_empty() {
            comments.push(ParsedComment {
                task_id: tid.clone(),
                author: auth.clone(),
                content: body,
            });
        }
    }

    comments
}

/// Apply a parsed RELATE statement to the database.
async fn apply_relation(queries: &Queries, rel: &ParsedRelation) -> Result<(), EngramError> {
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

async fn count_tasks(path: &Path) -> Result<u64, EngramError> {
    let contents = tokio::fs::read_to_string(path).await.map_err(|e| {
        EngramError::Hydration(HydrationError::Failed {
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
    engram_dir: &Path,
    file_mtimes: &HashMap<String, FileFingerprint>,
) -> (Option<String>, bool) {
    let last_flush_path = engram_dir.join(".lastflush");

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
        let content = r#"-- Generated by Engram
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
        fs::write(dir.path().join(".version"), format!("{SCHEMA_VERSION}\n")).expect("write");
        assert_eq!(read_version(dir.path()), Some(SCHEMA_VERSION.to_string()));
    }

    #[test]
    fn read_version_missing_returns_none() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert_eq!(read_version(dir.path()), None);
    }

    // ── GAP-001: read_body_lines unit tests ─────────────────────────

    #[tokio::test]
    async fn read_body_lines_extracts_correct_lines() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let content = "line1\nline2\nline3\nline4\n";
        tokio::fs::write(tmp.path().join("src.rs"), content)
            .await
            .expect("write");
        let body = read_body_lines(tmp.path(), "src.rs", 2, 3).await;
        assert_eq!(body, "line2\nline3");
    }

    #[tokio::test]
    async fn read_body_lines_single_line() {
        let tmp = tempfile::tempdir().expect("tempdir");
        tokio::fs::write(tmp.path().join("f.rs"), "alpha\nbeta\ngamma\n")
            .await
            .expect("write");
        let body = read_body_lines(tmp.path(), "f.rs", 2, 2).await;
        assert_eq!(body, "beta");
    }

    #[tokio::test]
    async fn read_body_lines_missing_file_returns_empty() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let body = read_body_lines(tmp.path(), "nonexistent.rs", 1, 5).await;
        assert!(body.is_empty());
    }

    #[tokio::test]
    async fn read_body_lines_out_of_bounds_returns_empty() {
        let tmp = tempfile::tempdir().expect("tempdir");
        tokio::fs::write(tmp.path().join("f.rs"), "only one line\n")
            .await
            .expect("write");
        let body = read_body_lines(tmp.path(), "f.rs", 99, 100).await;
        assert!(body.is_empty());
    }

    #[tokio::test]
    async fn read_body_lines_empty_file_path_returns_empty() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let body = read_body_lines(tmp.path(), "", 1, 5).await;
        assert!(body.is_empty());
    }

    #[tokio::test]
    async fn read_body_lines_zero_line_start_returns_empty() {
        let tmp = tempfile::tempdir().expect("tempdir");
        tokio::fs::write(tmp.path().join("f.rs"), "content\n")
            .await
            .expect("write");
        let body = read_body_lines(tmp.path(), "f.rs", 0, 1).await;
        assert!(body.is_empty());
    }

    #[test]
    fn null_embedding_in_jsonl_deserializes_to_none() {
        // When embedding is absent from JSONL, ParsedNode.embedding is None.
        let line = r#"{"id":"function:x","type":"function","name":"x","file_path":"src/lib.rs","line_start":1,"line_end":2,"body_hash":"abc","token_count":0,"embed_type":"summary_pointer","summary":"fn x()"}"#;
        let node: ParsedNode = serde_json::from_str(line).expect("parse");
        // embedding absent (null) → unwrap_or_default in upsert_node → empty vec
        assert!(node.embedding.is_none());
    }
}
