use std::collections::HashSet;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::db::connect_db;
use crate::db::queries::{CodeGraphQueries, Queries, ReadyWorkParams, SymbolFilter};
use crate::errors::{
    CodeGraphError, EngramError, QueryError, SystemError, TaskError, WorkspaceError,
};
use crate::models::config::CompactionConfig;
use crate::models::task::Task;
use crate::server::state::SharedState;
use crate::services::embedding;
use crate::services::output::{self, filter_value};
use crate::services::search::{SearchCandidate, hybrid_search};
use crate::services::search::{
    SearchRegion, UnifiedSearchResult, cosine_similarity, merge_unified_results,
};

#[derive(Deserialize)]
struct TaskGraphParams {
    root_task_id: String,
    #[serde(default = "default_depth")]
    depth: u32,
    /// Accepted for API consistency; graph nodes are already compact.
    #[serde(default)]
    #[allow(dead_code)]
    brief: bool,
    /// Accepted for API consistency; graph nodes are already compact.
    #[serde(default)]
    #[allow(dead_code)]
    fields: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct CheckStatusParams {
    work_item_ids: Vec<String>,
    #[serde(default)]
    brief: bool,
    #[serde(default)]
    fields: Option<Vec<String>>,
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

async fn ensure_workspace(state: &SharedState) -> Result<(), EngramError> {
    if state.snapshot_workspace().await.is_none() {
        return Err(EngramError::Workspace(WorkspaceError::NotSet));
    }
    Ok(())
}

async fn workspace_id(state: &SharedState) -> Result<String, EngramError> {
    if let Some(snapshot) = state.snapshot_workspace().await {
        return Ok(snapshot.workspace_id);
    }
    Err(EngramError::Workspace(WorkspaceError::NotSet))
}

pub async fn get_task_graph(
    state: SharedState,
    params: Option<Value>,
) -> Result<Value, EngramError> {
    ensure_workspace(&state).await?;

    let parsed: TaskGraphParams =
        serde_json::from_value(params.unwrap_or_default()).map_err(|e| {
            EngramError::System(SystemError::InvalidParams {
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
            EngramError::Task(TaskError::NotFound {
                id: parsed.root_task_id.clone(),
            })
        })?;

    // TaskGraphParams accepts brief/fields for API consistency, but graph
    // nodes are already compact (id + status + children), so we only forward
    // the structural depth parameter.
    let visited = Arc::new(tokio::sync::Mutex::new(HashSet::new()));
    let root_node = build_node(&queries, root, parsed.depth, &visited).await?;

    Ok(json!({
        "root": root_node,
    }))
}

pub async fn check_status(state: SharedState, params: Option<Value>) -> Result<Value, EngramError> {
    ensure_workspace(&state).await?;

    let parsed: CheckStatusParams =
        serde_json::from_value(params.unwrap_or_default()).map_err(|e| {
            EngramError::System(SystemError::InvalidParams {
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
            let task_value = json!({
                "task_id": task.id,
                "status": task.status.as_str().to_string(),
                "updated_at": task.updated_at.to_rfc3339(),
            });
            let filtered = filter_value(task_value, parsed.brief, parsed.fields.as_deref());
            statuses.insert(id.clone(), filtered);
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
pub async fn get_ready_work(
    state: SharedState,
    params: Option<Value>,
) -> Result<Value, EngramError> {
    ensure_workspace(&state).await?;

    let parsed: GetReadyWorkParams =
        serde_json::from_value(params.unwrap_or_default()).map_err(|e| {
            EngramError::System(SystemError::InvalidParams {
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
        .map(|t| output::serialize_task(&t, parsed.brief, parsed.fields.as_deref()))
        .collect();

    Ok(json!({
        "tasks": task_values,
        "total_eligible": result.total_eligible,
    }))
}

// ── Workspace statistics ─────────────────────────────────────────────────

/// Return aggregate counts by status, priority, type, and label.
pub async fn get_workspace_statistics(
    state: SharedState,
    _params: Option<Value>,
) -> Result<Value, EngramError> {
    ensure_workspace(&state).await?;

    let ws_id = workspace_id(&state).await?;
    let db = connect_db(&ws_id).await?;
    let queries = Queries::new(db.clone());

    let statistics = queries.get_workspace_statistics().await?;

    Ok(json!({
        "total_tasks": statistics.total_tasks,
        "by_status": statistics.by_status,
        "by_priority": statistics.by_priority,
        "by_type": statistics.by_type,
        "by_label": statistics.by_label,
        "deferred_count": statistics.deferred_count,
        "pinned_count": statistics.pinned_count,
        "claimed_count": statistics.claimed_count,
        "compacted_count": statistics.compacted_count,
    }))
}

// ── Compaction candidates ────────────────────────────────────────────────

#[derive(Deserialize)]
struct GetCompactionCandidatesParams {
    #[serde(default)]
    threshold_days: Option<u32>,
    #[serde(default)]
    max_candidates: Option<u32>,
}

/// Return done, non-pinned tasks older than `threshold_days`.
pub async fn get_compaction_candidates(
    state: SharedState,
    params: Option<Value>,
) -> Result<Value, EngramError> {
    ensure_workspace(&state).await?;

    let parsed: GetCompactionCandidatesParams = serde_json::from_value(params.unwrap_or_default())
        .map_err(|e| {
            EngramError::System(SystemError::InvalidParams {
                reason: format!("invalid params: {e}"),
            })
        })?;

    // Read compaction config from workspace or use defaults
    let config = state
        .workspace_config()
        .await
        .map_or_else(CompactionConfig::default, |c| c.compaction);

    let threshold = parsed.threshold_days.unwrap_or(config.threshold_days);
    let max = parsed.max_candidates.unwrap_or(config.max_candidates);

    let workspace_id = workspace_id(&state).await?;
    let db = connect_db(&workspace_id).await?;
    let queries = Queries::new(db.clone());

    let candidates = queries.get_compaction_candidates(threshold, max).await?;

    let now = chrono::Utc::now();
    let candidate_values: Vec<Value> = candidates
        .into_iter()
        .map(|t| {
            let age_days = (now - t.updated_at).num_days();
            json!({
                "task_id": t.id,
                "title": t.title,
                "description": t.description,
                "compaction_level": t.compaction_level,
                "age_days": age_days,
            })
        })
        .collect();

    Ok(json!({ "candidates": candidate_values }))
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

pub async fn query_memory(state: SharedState, params: Option<Value>) -> Result<Value, EngramError> {
    ensure_workspace(&state).await?;

    let parsed: QueryMemoryParams =
        serde_json::from_value(params.unwrap_or_default()).map_err(|e| {
            EngramError::System(SystemError::InvalidParams {
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

// ── map_code (T039) ─────────────────────────────────────────────────

#[derive(Deserialize)]
struct MapCodeParams {
    symbol_name: String,
    #[serde(default = "default_map_depth")]
    depth: usize,
    #[serde(default = "default_map_max_nodes")]
    max_nodes: usize,
}

const fn default_map_depth() -> usize {
    1
}

const fn default_map_max_nodes() -> usize {
    50
}

/// Retrieve a code symbol's definition plus its graph neighborhood.
///
/// Falls back to vector search when the exact symbol name is not found.
/// Returns full source bodies for all nodes (FR-148).
pub async fn map_code(state: SharedState, params: Option<Value>) -> Result<Value, EngramError> {
    ensure_workspace(&state).await?;

    if state.is_indexing() {
        return Err(EngramError::CodeGraph(CodeGraphError::IndexInProgress));
    }

    let parsed: MapCodeParams =
        serde_json::from_value(params.unwrap_or_default()).map_err(|e| {
            EngramError::System(SystemError::InvalidParams {
                reason: format!("invalid params: {e}"),
            })
        })?;

    // Clamp depth and max_nodes to config limits (FR-149)
    let config = state.workspace_config().await.unwrap_or_default();
    let effective_depth = parsed.depth.clamp(1, config.code_graph.max_traversal_depth);
    let effective_max_nodes = parsed.max_nodes.min(config.code_graph.max_traversal_nodes);

    let workspace_id = workspace_id(&state).await?;
    let db = connect_db(&workspace_id).await?;
    let cg_queries = CodeGraphQueries::new(db);

    // Exact-name lookup across all symbol tables
    let matches = cg_queries.find_symbols_by_name(&parsed.symbol_name).await?;

    if matches.is_empty() {
        // Fall back to vector search (FR-130)
        let Ok(query_embedding) = embedding::embed_text(&parsed.symbol_name) else {
            // No embedding model available — return empty fallback result
            return Ok(json!({
                "root": null,
                "neighbors": [],
                "edges": [],
                "truncated": false,
                "fallback_used": true,
                "matches": [],
                "effective_depth": effective_depth,
                "effective_max_nodes": effective_max_nodes,
            }));
        };

        let vector_matches = cg_queries
            .vector_search_symbols(&query_embedding, effective_max_nodes)
            .await?;

        let match_nodes: Vec<Value> = vector_matches.iter().map(symbol_match_to_json).collect();

        return Ok(json!({
            "root": null,
            "neighbors": [],
            "edges": [],
            "truncated": false,
            "fallback_used": true,
            "matches": match_nodes,
            "effective_depth": effective_depth,
            "effective_max_nodes": effective_max_nodes,
        }));
    }

    if matches.len() == 1 {
        // Single match: return root + BFS neighborhood
        let root = &matches[0];
        let bfs = cg_queries
            .bfs_neighborhood(&root.id, effective_depth, effective_max_nodes)
            .await?;

        let root_json = symbol_match_to_json(root);
        let neighbor_json: Vec<Value> = bfs.neighbors.iter().map(symbol_match_to_json).collect();
        let edge_json: Vec<Value> = bfs
            .edges
            .iter()
            .map(|e| {
                json!({
                    "type": e.edge_type,
                    "from": e.from,
                    "to": e.to,
                })
            })
            .collect();

        return Ok(json!({
            "root": root_json,
            "neighbors": neighbor_json,
            "edges": edge_json,
            "truncated": bfs.truncated,
            "fallback_used": false,
            "matches": null,
            "effective_depth": effective_depth,
            "effective_max_nodes": effective_max_nodes,
        }));
    }

    // Multiple matches: return disambiguation array; caller must qualify with file_path.
    let match_nodes: Vec<Value> = matches.iter().map(symbol_match_to_json).collect();

    Ok(json!({
        "root": null,
        "neighbors": [],
        "edges": [],
        "truncated": false,
        "fallback_used": false,
        "matches": match_nodes,
        "effective_depth": effective_depth,
        "effective_max_nodes": effective_max_nodes,
    }))
}

/// Convert a `SymbolMatch` to a JSON `CodeNode` object.
fn symbol_match_to_json(m: &crate::db::queries::SymbolMatch) -> Value {
    json!({
        "id": m.id,
        "type": m.table,
        "name": m.name,
        "file_path": m.file_path,
        "line_start": m.line_start,
        "line_end": m.line_end,
        "signature": m.signature,
        "body": m.body,
        "embed_type": m.embed_type,
        "summary": m.summary,
    })
}

// ── list_symbols (T040) ─────────────────────────────────────────────

#[derive(Deserialize)]
struct ListSymbolsParams {
    #[serde(default)]
    file_path: Option<String>,
    #[serde(default)]
    node_type: Option<String>,
    #[serde(default)]
    name_prefix: Option<String>,
    #[serde(default = "default_list_limit")]
    limit: usize,
    #[serde(default)]
    offset: usize,
}

const fn default_list_limit() -> usize {
    50
}

/// Return a paginated list of indexed code symbols (FR-150).
///
/// Enables agents to discover valid symbol names before invoking
/// `map_code`, `link_task_to_code`, or `impact_analysis`.
pub async fn list_symbols(state: SharedState, params: Option<Value>) -> Result<Value, EngramError> {
    ensure_workspace(&state).await?;

    // Reject while indexing — graph state is not yet consistent
    if state.is_indexing() {
        return Err(EngramError::CodeGraph(CodeGraphError::IndexInProgress));
    }

    let parsed: ListSymbolsParams =
        serde_json::from_value(params.unwrap_or_default()).map_err(|e| {
            EngramError::System(SystemError::InvalidParams {
                reason: format!("invalid params: {e}"),
            })
        })?;

    // Clamp limit
    let limit = parsed.limit.clamp(1, 500);

    let workspace_id = workspace_id(&state).await?;
    let db = connect_db(&workspace_id).await?;
    let cg_queries = CodeGraphQueries::new(db);

    let filter = SymbolFilter {
        file_path: parsed.file_path,
        node_type: parsed.node_type,
        name_prefix: parsed.name_prefix,
        limit,
        offset: parsed.offset,
    };

    let result = cg_queries.list_symbols(&filter).await?;

    // Return 7004 only when a name_prefix filter produced no results.
    if result.total_count == 0 {
        if let Some(ref prefix) = filter.name_prefix {
            return Err(EngramError::CodeGraph(CodeGraphError::SymbolNotFound {
                name: prefix.clone(),
            }));
        }
    }

    Ok(json!({
        "symbols": result.symbols,
        "total_count": result.total_count,
        "has_more": result.has_more,
    }))
}

fn build_node<'a>(
    queries: &'a Queries,
    task: Task,
    depth: u32,
    visited: &'a Arc<tokio::sync::Mutex<HashSet<String>>>,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<TaskNode, EngramError>> + Send + 'a>>
{
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
            // Skip already-visited nodes to avoid exponential traversal on diamonds
            {
                let mut v = visited.lock().await;
                if !v.insert(edge.to.clone()) {
                    continue;
                }
            }
            if let Some(child_task) = queries.get_task(&edge.to).await? {
                let child = build_node(queries, child_task, depth - 1, visited).await?;
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

// ── get_active_context (T052) ───────────────────────────────────────

/// Return all in-progress tasks. For the highest-priority task,
/// expand full code neighborhoods (source bodies) of all `concerns`-linked
/// symbols. For remaining tasks, return task metadata and linked
/// symbol names only (FR-127).
pub async fn get_active_context(
    state: SharedState,
    _params: Option<Value>,
) -> Result<Value, EngramError> {
    ensure_workspace(&state).await?;
    let ws_id = workspace_id(&state).await?;

    let db = connect_db(&ws_id).await?;
    let cg_queries = CodeGraphQueries::new(db.clone());
    let queries = Queries::new(db);

    // Get all in_progress tasks ordered by priority.
    let in_progress = queries.get_in_progress_tasks().await?;

    if in_progress.is_empty() {
        return Ok(json!({
            "primary_task": null,
            "other_tasks": [],
        }));
    }

    // First task = highest priority (lowest priority_order, oldest if tied).
    let primary = &in_progress[0];
    let primary_links = cg_queries.list_concerns_for_task(&primary.id).await?;

    // Expand neighborhoods for primary task's linked symbols.
    let mut primary_code_context = Vec::new();
    for link in &primary_links {
        // Get the symbol details.
        if let Some(sym) = cg_queries.resolve_symbol(&link.symbol_id).await? {
            // Get 1-hop neighborhood.
            let neighborhood = cg_queries.bfs_neighborhood(&link.symbol_id, 1, 20).await?;
            primary_code_context.push(json!({
                "symbol": {
                    "table": sym.table,
                    "id": sym.id,
                    "name": sym.name,
                    "file_path": sym.file_path,
                    "line_start": sym.line_start,
                    "line_end": sym.line_end,
                    "signature": sym.signature,
                    "body": sym.body,
                },
                "neighbors": neighborhood.neighbors.iter().map(|n| json!({
                    "table": n.table,
                    "id": n.id,
                    "name": n.name,
                    "file_path": n.file_path,
                    "line_start": n.line_start,
                    "line_end": n.line_end,
                    "signature": n.signature,
                    "body": n.body,
                })).collect::<Vec<_>>(),
                "edges": neighborhood.edges.iter().map(|e| json!({
                    "type": e.edge_type,
                    "from": e.from,
                    "to": e.to,
                })).collect::<Vec<_>>(),
                "truncated": neighborhood.truncated,
            }));
        }
    }

    let primary_json = json!({
        "task": {
            "id": primary.id,
            "title": primary.title,
            "status": primary.status.as_str(),
            "priority": primary.priority,
            "description": primary.description,
            "context_summary": primary.context_summary,
        },
        "linked_symbols": primary_links.iter().map(|l| &l.symbol_name).collect::<Vec<_>>(),
        "code_context": primary_code_context,
    });

    // Batch-load concerns for remaining in-progress tasks.
    let other_task_ids: Vec<String> = in_progress[1..].iter().map(|t| t.id.clone()).collect();
    let other_concerns = cg_queries.list_concerns_for_tasks(&other_task_ids).await?;

    // Other in-progress tasks: metadata + symbol names only.
    let mut other_tasks = Vec::new();
    for task in &in_progress[1..] {
        let empty_links = Vec::new();
        let links = other_concerns.get(&task.id).unwrap_or(&empty_links);
        other_tasks.push(json!({
            "task": {
                "id": task.id,
                "title": task.title,
                "status": task.status.as_str(),
                "priority": task.priority,
            },
            "linked_symbols": links.iter().map(|l| &l.symbol_name).collect::<Vec<_>>(),
        }));
    }

    Ok(json!({
        "primary_task": primary_json,
        "other_tasks": other_tasks,
    }))
}

// ── unified_search (T057 — Phase 7) ────────────────────────────────────────

#[derive(Deserialize)]
struct UnifiedSearchParams {
    query: String,
    #[serde(default = "default_unified_region")]
    region: String,
    #[serde(default = "default_unified_limit")]
    limit: usize,
}

fn default_unified_region() -> String {
    "all".to_string()
}

const fn default_unified_limit() -> usize {
    10
}

/// Unified semantic search across code and task regions (FR-128/FR-131).
///
/// Scoring: raw cosine similarity on embedding vectors. Results from code
/// and task regions are merged into a single list sorted by descending score.
/// No cross-region normalization or boosting in v0.
///
/// Returns summary text only, not full bodies (FR-148 exemption).
///
/// # Errors
/// - `QueryEmpty` (4001) for empty or whitespace-only queries (FR-157).
/// - `SystemError::DatabaseError` (5001) if embedding generation fails.
/// - `WorkspaceError::NotSet` (1003) if workspace not bound.
pub async fn unified_search(
    state: SharedState,
    params: Option<Value>,
) -> Result<Value, EngramError> {
    ensure_workspace(&state).await?;

    let parsed: UnifiedSearchParams =
        serde_json::from_value(params.unwrap_or_default()).map_err(|e| {
            EngramError::System(SystemError::InvalidParams {
                reason: format!("invalid params: {e}"),
            })
        })?;

    // FR-157: reject empty queries after whitespace trimming.
    let trimmed = parsed.query.trim();
    if trimmed.is_empty() {
        return Err(EngramError::Query(QueryError::QueryEmpty));
    }

    // Validate query length.
    embedding::validate_query_length(trimmed)?;

    // Validate region parameter.
    let search_code = parsed.region == "code" || parsed.region == "all";
    let search_task = parsed.region == "task" || parsed.region == "all";
    if !search_code && !search_task {
        return Err(EngramError::System(SystemError::InvalidParams {
            reason: format!(
                "invalid region '{}': expected code, task, or all",
                parsed.region
            ),
        }));
    }

    // Clamp limit to [1, 50].
    let limit = parsed.limit.clamp(1, 50);

    // Embed the query. FR-157: if embedding fails, return 5001.
    let query_embedding = embedding::embed_text(trimmed).map_err(|e| {
        EngramError::System(SystemError::DatabaseError {
            reason: format!("embedding generation failed: {e}"),
        })
    })?;

    let workspace_id = workspace_id(&state).await?;
    let db = connect_db(&workspace_id).await?;
    let cg_queries = CodeGraphQueries::new(db.clone());
    let queries = Queries::new(db);

    // ── Code region search ──────────────────────────────────────────
    let code_results = if search_code {
        let symbols = cg_queries
            .vector_search_symbols(&query_embedding, limit)
            .await?;
        symbols
            .into_iter()
            .map(|s| {
                let score = cosine_similarity(&query_embedding, &s.embedding);
                let line_range = match (s.line_start, s.line_end) {
                    (Some(start), Some(end)) => Some(format!("L{start}-L{end}")),
                    (Some(start), None) => Some(format!("L{start}")),
                    _ => None,
                };
                UnifiedSearchResult {
                    region: SearchRegion::Code,
                    score,
                    node_type: s.table,
                    id: s.id,
                    title: Some(s.name),
                    file_path: Some(s.file_path),
                    line_range,
                    summary: s.summary,
                    status: None,
                    linked_symbols: None,
                }
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    // ── Task region search ──────────────────────────────────────────
    let task_results = if search_task {
        let mut results: Vec<UnifiedSearchResult> = Vec::new();

        // Search specs (have embeddings).
        let specs = queries.all_specs().await?;
        for spec in specs {
            if let Some(ref emb) = spec.embedding {
                let score = cosine_similarity(&query_embedding, emb);
                if score > 0.0 {
                    results.push(UnifiedSearchResult {
                        region: SearchRegion::Task,
                        score,
                        node_type: "spec".to_string(),
                        id: format!("spec:{}", spec.id),
                        title: Some(spec.title),
                        summary: Some(truncate_summary(&spec.content, 200)),
                        file_path: Some(spec.file_path),
                        line_range: None,
                        status: None,
                        linked_symbols: None,
                    });
                }
            }
        }

        // Search contexts (have embeddings).
        let contexts = queries.all_contexts().await?;
        for ctx in contexts {
            if let Some(ref emb) = ctx.embedding {
                let score = cosine_similarity(&query_embedding, emb);
                if score > 0.0 {
                    results.push(UnifiedSearchResult {
                        region: SearchRegion::Task,
                        score,
                        node_type: "context".to_string(),
                        id: format!("context:{}", ctx.id),
                        title: None,
                        summary: Some(truncate_summary(&ctx.content, 200)),
                        file_path: None,
                        line_range: None,
                        status: None,
                        linked_symbols: None,
                    });
                }
            }
        }

        // Search tasks via keyword scoring (tasks lack embedding vectors).
        // Score = fraction of query words found in title+description (case-insensitive).
        let tasks = queries.all_tasks().await?;
        let query_words: Vec<&str> = trimmed.split_whitespace().collect();
        for task in tasks {
            if query_words.is_empty() {
                continue;
            }
            let haystack = format!(
                "{} {}",
                task.title.to_lowercase(),
                task.description.to_lowercase()
            );
            let matched = query_words
                .iter()
                .filter(|w| haystack.contains(&w.to_lowercase()[..]))
                .count();
            let score = matched as f32 / query_words.len() as f32;
            if score > 0.0 {
                let linked = cg_queries
                    .list_concerns_for_task(&task.id)
                    .await
                    .unwrap_or_default()
                    .into_iter()
                    .map(|l| l.symbol_name)
                    .collect::<Vec<_>>();
                results.push(UnifiedSearchResult {
                    region: SearchRegion::Task,
                    score,
                    node_type: "task".to_string(),
                    id: format!("task:{}", task.id),
                    title: Some(task.title),
                    summary: Some(truncate_summary(&task.description, 200)),
                    file_path: None,
                    line_range: None,
                    status: Some(task.status.as_str().to_string()),
                    linked_symbols: Some(linked),
                });
            }
        }

        results
    } else {
        Vec::new()
    };

    // ── Merge and rank ──────────────────────────────────────────────
    let merged = merge_unified_results(code_results, task_results, limit);
    let total_matches = merged.len();

    Ok(json!({
        "results": merged,
        "total_matches": total_matches,
    }))
}

/// Truncate text to `max_chars`, breaking at a word boundary when possible.
fn truncate_summary(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    // Find the byte position at max_chars character boundary (safe for multi-byte chars).
    let byte_end = text
        .char_indices()
        .nth(max_chars)
        .map_or(text.len(), |(i, _)| i);
    let truncated = &text[..byte_end];
    if let Some(pos) = truncated.rfind(' ') {
        format!("{}…", &truncated[..pos])
    } else {
        format!("{truncated}…")
    }
}

// ── impact_analysis (T061 — Phase 8) ───────────────────────────────────────

#[derive(Deserialize)]
struct ImpactAnalysisParams {
    symbol_name: String,
    #[serde(default = "default_impact_depth")]
    depth: usize,
    #[serde(default)]
    status_filter: Option<String>,
    #[serde(default = "default_impact_max_nodes")]
    max_nodes: usize,
}

const fn default_impact_depth() -> usize {
    1
}

const fn default_impact_max_nodes() -> usize {
    50
}

/// Impact analysis: traverse code dependencies and cross-region concerns edges
/// to find all tasks affected by changes to a specific code symbol (FR-129).
///
/// 1. Resolve `symbol_name` via exact-name lookup.
/// 2. BFS traverse code graph to `depth` hops (clamped by FR-149).
/// 3. Collect all visited node IDs (root + neighbors).
/// 4. Reverse-lookup `concerns` edges to find linked tasks.
/// 5. Optionally filter tasks by `status_filter`.
/// 6. Return full source bodies for code nodes (FR-148).
///
/// # Errors
/// - `WorkspaceError::NotSet` (1003) if workspace not bound.
/// - `CodeGraphError::SymbolNotFound` (7004) if symbol not in graph.
pub async fn impact_analysis(
    state: SharedState,
    params: Option<Value>,
) -> Result<Value, EngramError> {
    ensure_workspace(&state).await?;

    if state.is_indexing() {
        return Err(EngramError::CodeGraph(CodeGraphError::IndexInProgress));
    }

    let parsed: ImpactAnalysisParams =
        serde_json::from_value(params.unwrap_or_default()).map_err(|e| {
            EngramError::System(SystemError::InvalidParams {
                reason: format!("invalid params: {e}"),
            })
        })?;

    // FR-149: clamp depth to config limits.
    let config = state.workspace_config().await.unwrap_or_default();
    let effective_depth = parsed.depth.clamp(1, config.code_graph.max_traversal_depth);
    let effective_max_nodes = parsed
        .max_nodes
        .clamp(1, 100)
        .min(config.code_graph.max_traversal_nodes);

    let workspace_id = workspace_id(&state).await?;
    let db = connect_db(&workspace_id).await?;
    let cg_queries = CodeGraphQueries::new(db.clone());
    let queries = Queries::new(db);

    // Step 1: Resolve symbol name to code node(s).
    let matches = cg_queries.find_symbols_by_name(&parsed.symbol_name).await?;
    if matches.is_empty() {
        return Err(EngramError::CodeGraph(CodeGraphError::SymbolNotFound {
            name: parsed.symbol_name,
        }));
    }

    let root = &matches[0];

    // Step 2: BFS traverse code graph.
    let bfs = cg_queries
        .bfs_neighborhood(&root.id, effective_depth, effective_max_nodes)
        .await?;

    // Step 3: Collect all visited node IDs (root + neighbors).
    let mut all_node_ids: Vec<String> = vec![root.id.clone()];
    for neighbor in &bfs.neighbors {
        all_node_ids.push(neighbor.id.clone());
    }
    all_node_ids.dedup();

    // Step 4: Reverse-lookup concerns edges to find linked tasks.
    let task_symbol_pairs = cg_queries.find_tasks_for_symbols(&all_node_ids).await?;

    // Deduplicate task IDs and collect dependency paths.
    let mut task_map: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for (task_id, symbol_id) in &task_symbol_pairs {
        task_map
            .entry(task_id.clone())
            .or_default()
            .push(symbol_id.clone());
    }

    // Step 5: Fetch tasks and apply status filter.
    let mut affected_tasks: Vec<Value> = Vec::new();
    for (task_id, dependency_path) in &task_map {
        let raw_id = task_id.strip_prefix("task:").unwrap_or(task_id);
        if let Some(task) = queries.get_task(raw_id).await? {
            // Apply status filter if provided.
            if let Some(ref filter) = parsed.status_filter {
                if task.status.as_str() != filter.as_str() {
                    continue;
                }
            }
            affected_tasks.push(json!({
                "task": {
                    "id": task.id,
                    "title": task.title,
                    "status": task.status.as_str(),
                    "priority": task.priority,
                    "work_item_id": task.work_item_id,
                },
                "dependency_path": dependency_path,
            }));
        }
    }

    // Build code neighborhood JSON (FR-148: full source bodies).
    let code_neighborhood: Vec<Value> = bfs.neighbors.iter().map(symbol_match_to_json).collect();

    let no_task_links = affected_tasks.is_empty();

    Ok(json!({
        "symbol": {
            "name": root.name,
            "type": root.table,
            "file_path": root.file_path,
        },
        "code_neighborhood": code_neighborhood,
        "affected_tasks": affected_tasks,
        "no_task_links": no_task_links,
        "effective_depth": effective_depth,
        "effective_max_nodes": effective_max_nodes,
    }))
}

// ── T034: get_health_report ───────────────────────────────────────────────────

/// Return a structured health report for the running daemon.
///
/// Does **not** require a workspace to be bound (S060) — all metrics are
/// sourced from [`AppState`] and the host process memory via `sysinfo`.
///
/// # Errors
///
/// This function is infallible in practice but returns `Result` to satisfy
/// the tool-dispatch contract.
pub async fn get_health_report(
    state: SharedState,
    _params: Option<Value>,
) -> Result<Value, EngramError> {
    use sysinfo::System;

    let version = env!("CARGO_PKG_VERSION");
    let uptime_secs = state.uptime_seconds();
    let connections = state.active_connections();
    let workspace_id = state.snapshot_workspace().await.map(|s| s.workspace_id);
    let tool_call_count = state.tool_call_count();
    let (p50, p95, p99) = state.latency_percentiles().await;
    let (watcher_events, last_watcher_event) = state.watcher_stats().await;

    let mut sys = System::new_all();
    sys.refresh_memory();
    let pid = sysinfo::get_current_pid().ok();
    let memory_mb = pid
        .and_then(|pid| sys.process(pid))
        .map(|proc| proc.memory() / 1_048_576)
        .unwrap_or(0);

    Ok(json!({
        "version": version,
        "uptime_seconds": uptime_secs,
        "active_connections": connections,
        "workspace_id": workspace_id,
        "tool_call_count": tool_call_count,
        "latency_us": {
            "p50": p50,
            "p95": p95,
            "p99": p99,
        },
        "memory_mb": memory_mb,
        "watcher_events": watcher_events,
        "last_watcher_event": last_watcher_event,
    }))
}
