use std::path::PathBuf;

use serde::Deserialize;
use serde_json::{Value, json};

use crate::db::connect_db;
use crate::db::queries::{CodeGraphQueries, SymbolFilter};
use crate::errors::{
    CodeGraphError, EngramError, GraphQueryError, QueryError, SystemError, WorkspaceError,
};
use crate::server::state::SharedState;
use crate::services::embedding;
use crate::services::metrics;
use crate::services::search::{SearchCandidate, hybrid_search};
use crate::services::search::{SearchRegion, UnifiedSearchResult, merge_unified_results};

async fn ensure_workspace(state: &SharedState) -> Result<(), EngramError> {
    if state.snapshot_workspace().await.is_none() {
        return Err(EngramError::Workspace(WorkspaceError::NotSet));
    }
    Ok(())
}

async fn workspace_db(state: &SharedState) -> Result<(PathBuf, String), EngramError> {
    if let Some(snapshot) = state.snapshot_workspace().await {
        return Ok((snapshot.data_dir.clone(), snapshot.branch.clone()));
    }
    Err(EngramError::Workspace(WorkspaceError::NotSet))
}

async fn workspace_snapshot_path_and_branch(
    state: &SharedState,
) -> Result<(PathBuf, String), EngramError> {
    let snapshot = state
        .snapshot_workspace()
        .await
        .ok_or(EngramError::Workspace(WorkspaceError::NotSet))?;
    Ok((PathBuf::from(snapshot.path), snapshot.branch))
}

async fn load_registry_status(state: &SharedState) -> Result<Option<Value>, EngramError> {
    let Some(snapshot) = state.snapshot_workspace().await else {
        return Ok(None);
    };

    let workspace_path = std::path::PathBuf::from(snapshot.path);
    let registry_path = workspace_path.join(".engram").join("registry.yaml");

    tokio::task::spawn_blocking(move || {
        match crate::services::registry::load_registry(&registry_path) {
            Ok(Some(mut config)) => {
                let _ = crate::services::registry::validate_sources(&mut config, &workspace_path);
                let sources: Vec<Value> = config
                    .sources
                    .iter()
                    .map(|source| {
                        json!({
                            "content_type": source.content_type,
                            "language": source.language,
                            "path": source.path,
                            "status": source.status.as_str(),
                        })
                    })
                    .collect();

                Ok(Some(json!({
                    "sources": sources,
                    "total_sources": config.sources.len(),
                })))
            }
            Ok(None) | Err(_) => Ok(None),
        }
    })
    .await
    .map_err(|e| {
        EngramError::System(SystemError::DatabaseError {
            reason: format!("registry status worker failed: {e}"),
        })
    })?
}

// ── Workspace statistics ─────────────────────────────────────────────────

/// Return aggregate code graph statistics for the current workspace.
pub async fn get_workspace_statistics(
    state: SharedState,
    _params: Option<Value>,
) -> Result<Value, EngramError> {
    ensure_workspace(&state).await?;

    let (data_dir, branch) = workspace_db(&state).await?;
    let db = connect_db(&data_dir, &branch).await?;
    let cg_queries = CodeGraphQueries::new(db);

    let code_files = cg_queries.count_code_files().await.unwrap_or(0);
    let functions = cg_queries.count_functions().await.unwrap_or(0);
    let classes = cg_queries.count_classes().await.unwrap_or(0);
    let interfaces = cg_queries.count_interfaces().await.unwrap_or(0);
    let edges = cg_queries.count_code_edges().await.unwrap_or(0);

    let embedding_status = embedding::status(Some(&cg_queries)).await?;
    let registry_status = load_registry_status(&state).await?;

    let mut result = serde_json::Map::from_iter([
        ("code_files".to_owned(), json!(code_files)),
        ("functions".to_owned(), json!(functions)),
        ("classes".to_owned(), json!(classes)),
        ("interfaces".to_owned(), json!(interfaces)),
        ("edges".to_owned(), json!(edges)),
        (
            "embedding_status".to_owned(),
            serde_json::to_value(&embedding_status).unwrap_or(Value::Null),
        ),
    ]);

    if let Some(reg) = registry_status {
        result.insert("registry".to_owned(), reg);
    }

    Ok(Value::Object(result))
}

#[derive(Deserialize)]
struct QueryMemoryParams {
    query: String,
    #[serde(default = "default_limit")]
    limit: usize,
    /// Optional content type filter (e.g. "spec", "docs", "tests").
    #[serde(default)]
    content_type: Option<String>,
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

    let (data_dir, branch) = workspace_db(&state).await?;
    let db = connect_db(&data_dir, &branch).await?;
    let queries = CodeGraphQueries::new(db);
    let mut candidates: Vec<SearchCandidate> = Vec::new();
    let content_records = queries
        .select_content_records(parsed.content_type.as_deref())
        .await?;
    for cr in content_records {
        candidates.push(SearchCandidate {
            id: format!("content_record:{}", cr.id),
            source_type: cr.content_type,
            content: cr.content,
            embedding: cr.embedding,
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

    let (data_dir, branch) = workspace_db(&state).await?;
    let db = connect_db(&data_dir, &branch).await?;
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
        // Single match: return root + native graph neighborhood
        let root = &matches[0];
        let bfs = cg_queries
            .graph_neighborhood(&root.id, effective_depth, effective_max_nodes)
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

    let (data_dir, branch) = workspace_db(&state).await?;
    let db = connect_db(&data_dir, &branch).await?;
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

// ── unified_search (T057 — Phase 7) ────────────────────────────────────────

#[derive(Deserialize)]
struct UnifiedSearchParams {
    query: String,
    #[serde(default = "default_unified_region")]
    region: String,
    #[serde(default = "default_unified_limit")]
    limit: usize,
    /// Optional content type filter for content records.
    #[serde(default)]
    content_type: Option<String>,
    /// Restrict code symbol results to the graph neighborhood of this symbol.
    #[serde(default)]
    scope_to_symbol: Option<String>,
}

fn default_unified_region() -> String {
    "all".to_string()
}

const fn default_unified_limit() -> usize {
    10
}

/// Unified semantic search across the code graph and content records (FR-128/FR-131).
///
/// Scoring: raw cosine similarity on embedding vectors for code symbols;
/// keyword scoring for content records. Results are merged into a single
/// list sorted by descending score.
///
/// Returns summary text only, not full bodies (FR-148 exemption).
///
/// # Errors
/// - `QueryEmpty` (4001) for empty or whitespace-only queries (FR-157).
/// - `SearchFailed` (4004) if the embedding model is not loaded/enabled.
/// - `SystemError::DatabaseError` (5001) if embedding generation fails after model load.
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

    // Validate region parameter — only "code" and "all" are supported.
    if parsed.region != "code" && parsed.region != "all" {
        return Err(EngramError::System(SystemError::InvalidParams {
            reason: format!("invalid region '{}': expected code or all", parsed.region),
        }));
    }

    // Clamp limit to [1, 50].
    let limit = parsed.limit.clamp(1, 50);

    // Guard: reject semantic search at compile time when the embeddings feature
    // is not compiled in. When it IS enabled, embed_text lazily loads the model
    // on the first call — do not gate on is_available() here.
    #[cfg(not(feature = "embeddings"))]
    return Err(EngramError::Query(QueryError::SearchFailed {
        reason: "Semantic search requires the embeddings feature. \
                 Enable it with `cargo build --features embeddings`. \
                 Text-based search via keyword queries is unaffected."
            .to_owned(),
    }));

    // Embed the query. FR-157: if embedding fails, return 5001.
    let query_embedding = embedding::embed_text(trimmed).map_err(|e| {
        EngramError::System(SystemError::DatabaseError {
            reason: format!("embedding generation failed: {e}"),
        })
    })?;

    let (data_dir, branch) = workspace_db(&state).await?;
    let db = connect_db(&data_dir, &branch).await?;
    let queries = CodeGraphQueries::new(db);
    let code_results = {
        let symbols = if let Some(scope) = parsed
            .scope_to_symbol
            .as_deref()
            .filter(|s| !s.trim().is_empty())
        {
            // Scoped mode: restrict to graph neighborhood of given symbol.
            let scope_matches = queries.find_symbols_by_name(scope).await?;
            if let Some(root) = scope_matches.first() {
                queries
                    .hybrid_graph_vector_search(&root.id, 2, &query_embedding, limit, &[])
                    .await?
            } else {
                queries
                    .vector_search_symbols_native(&query_embedding, limit)
                    .await?
            }
        } else {
            queries
                .vector_search_symbols_native(&query_embedding, limit)
                .await?
        };
        symbols
            .into_iter()
            .map(|(score, s)| {
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
    };

    // ── Content records: KNN vector search ──────────────────────────
    // Requires embeddings feature; the cfg guard at the top of this
    // function already returned an error for non-embeddings builds, so
    // we are guaranteed to have a valid query_embedding here.
    let content_results: Vec<UnifiedSearchResult> = {
        let knn = queries
            .vector_search_content_native(&query_embedding, limit, parsed.content_type.as_deref())
            .await?;

        // If no embedded records exist yet (backfill still in progress),
        // fall back to keyword scoring so the tool stays useful.
        if knn.is_empty() {
            let query_words: Vec<&str> = trimmed.split_whitespace().collect();
            let all_records = queries
                .select_content_records(parsed.content_type.as_deref())
                .await?;
            all_records
                .into_iter()
                .filter_map(|cr| {
                    if query_words.is_empty() {
                        return None;
                    }
                    let haystack = cr.content.to_lowercase();
                    let matched = query_words
                        .iter()
                        .filter(|w| haystack.contains(&w.to_lowercase()[..]))
                        .count();
                    let score = matched as f32 / query_words.len() as f32;
                    if score > 0.0 {
                        Some(UnifiedSearchResult {
                            region: SearchRegion::Task,
                            score,
                            node_type: cr.content_type.clone(),
                            id: format!("content_record:{}", cr.id),
                            title: Some(cr.file_path.clone()),
                            summary: Some(truncate_summary(&cr.content, 200)),
                            file_path: Some(cr.file_path),
                            line_range: None,
                            status: None,
                            linked_symbols: None,
                        })
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            knn.into_iter()
                .map(|(score, cr)| UnifiedSearchResult {
                    region: SearchRegion::Task,
                    score,
                    node_type: cr.content_type.clone(),
                    id: format!("content_record:{}", cr.id),
                    title: Some(cr.file_path.clone()),
                    summary: Some(truncate_summary(&cr.content, 200)),
                    file_path: Some(cr.file_path),
                    line_range: None,
                    status: None,
                    linked_symbols: None,
                })
                .collect()
        }
    };

    // ── Merge and rank ───────────────────────────────────────────────
    let merged = merge_unified_results(code_results, content_results, limit);
    let total_count = merged.len();

    Ok(json!({
        "results": merged,
        "total_count": total_count,
        "total_matches": total_count,
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
    #[serde(default = "default_impact_max_nodes")]
    max_nodes: usize,
    /// Optional semantic concept for combined structural+semantic results.
    #[serde(default)]
    concept: Option<String>,
}

const fn default_impact_depth() -> usize {
    1
}

const fn default_impact_max_nodes() -> usize {
    50
}

/// Impact analysis: traverse the code graph to find symbols affected by
/// changes to a specific code symbol (FR-129).
///
/// 1. Resolve `symbol_name` via exact-name lookup.
/// 2. Native graph traversal to `depth` hops via [`CodeGraphQueries::graph_neighborhood`].
/// 3. Return the root symbol and its code neighborhood with full source bodies (FR-148).
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

    let (data_dir, branch) = workspace_db(&state).await?;
    let db = connect_db(&data_dir, &branch).await?;
    let cg_queries = CodeGraphQueries::new(db);
    let matches = cg_queries.find_symbols_by_name(&parsed.symbol_name).await?;
    if matches.is_empty() {
        return Err(EngramError::CodeGraph(CodeGraphError::SymbolNotFound {
            name: parsed.symbol_name,
        }));
    }

    let root = &matches[0];

    // Step 2: Native graph traversal.
    let bfs = cg_queries
        .graph_neighborhood(&root.id, effective_depth, effective_max_nodes)
        .await?;

    // Build code neighborhood JSON (FR-148: full source bodies).
    let code_neighborhood: Vec<Value> = bfs.neighbors.iter().map(symbol_match_to_json).collect();

    // When a semantic concept is provided and embeddings are available,
    // run a combined structural+semantic query for additional relevance.
    let hybrid_results: Vec<Value> =
        if let Some(concept) = parsed.concept.as_deref().filter(|c| !c.trim().is_empty()) {
            if embedding::is_available() {
                let emb = embedding::embed_text(concept.trim()).map_err(|e| {
                    EngramError::System(SystemError::DatabaseError {
                        reason: format!("embedding generation failed: {e}"),
                    })
                })?;
                let scored = cg_queries
                    .hybrid_graph_vector_search(
                        &root.id,
                        effective_depth,
                        &emb,
                        effective_max_nodes,
                        &[],
                    )
                    .await?;
                scored
                    .into_iter()
                    .map(|(score, s)| {
                        let mut v = symbol_match_to_json(&s);
                        v["relevance_score"] = json!(score);
                        v
                    })
                    .collect()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

    let mut response = json!({
        "symbol": {
            "name": root.name,
            "type": root.table,
            "file_path": root.file_path,
        },
        "code_neighborhood": code_neighborhood,
        "effective_depth": effective_depth,
        "effective_max_nodes": effective_max_nodes,
    });

    if !hybrid_results.is_empty() {
        response["hybrid_results"] = json!(hybrid_results);
    }

    Ok(response)
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
    let workspace_snapshot = state.snapshot_workspace().await;
    let workspace_id = workspace_snapshot.as_ref().map(|s| s.workspace_id.clone());
    let tool_call_count = state.tool_call_count();
    let (p50, p95, p99) = state.latency_percentiles().await;
    let (watcher_events, last_watcher_event) = state.watcher_stats().await;

    let mut sys = System::new();
    let pid = sysinfo::get_current_pid().ok();
    if let Some(pid) = pid {
        sys.refresh_process(pid);
    }
    let memory_mb = pid
        .and_then(|pid| sys.process(pid))
        .map(|proc| proc.memory() / 1_048_576);

    // Collect embedding status — no workspace needed for the basic availability check.
    let embedding_status = embedding::status(None).await?;
    let metrics_summary = if let Some(snapshot) = &workspace_snapshot {
        let wp = PathBuf::from(&snapshot.path);
        let br = snapshot.branch.clone();
        match tokio::task::spawn_blocking(move || metrics::compute_summary(&wp, &br)).await {
            Ok(Ok(summary)) => serde_json::to_value(json!({
                "branch": snapshot.branch,
                "summary": summary,
            }))
            .unwrap_or(Value::Null),
            Ok(Err(EngramError::Metrics(crate::errors::MetricsError::NotFound { .. }))) => {
                Value::Null
            }
            Ok(Err(error)) => return Err(error),
            Err(join_error) => {
                tracing::warn!(error = %join_error, "metrics computation task panicked");
                Value::Null
            }
        }
    } else {
        Value::Null
    };

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
        "embedding_status": embedding_status,
        "metrics_summary": metrics_summary,
        "query_timing": crate::services::query_stats::timing_snapshot(),
    }))
}

#[derive(Deserialize)]
struct BranchMetricsParams {
    #[serde(default)]
    branch_name: Option<String>,
    #[serde(default)]
    compare_to: Option<String>,
}

/// Return metrics summary data for the current branch or compare two branches.
pub async fn get_branch_metrics(
    state: SharedState,
    params: Option<Value>,
) -> Result<Value, EngramError> {
    let parsed: BranchMetricsParams = serde_json::from_value(params.unwrap_or_else(|| json!({})))
        .map_err(|error| {
        EngramError::System(SystemError::InvalidParams {
            reason: error.to_string(),
        })
    })?;
    let (workspace_path, current_branch) = workspace_snapshot_path_and_branch(&state).await?;
    let branch_name = parsed.branch_name.unwrap_or(current_branch);
    let wp = workspace_path.clone();
    let br = branch_name.clone();
    let summary = tokio::task::spawn_blocking(move || metrics::compute_summary(&wp, &br))
        .await
        .map_err(|error| {
            EngramError::Metrics(crate::errors::MetricsError::WriteFailed {
                reason: format!("metrics computation task panicked: {error}"),
            })
        })??;

    if let Some(compare_to) = parsed.compare_to {
        let wp2 = workspace_path.clone();
        let br2 = compare_to.clone();
        let comparison = tokio::task::spawn_blocking(move || metrics::compute_summary(&wp2, &br2))
            .await
            .map_err(|error| {
                EngramError::Metrics(crate::errors::MetricsError::WriteFailed {
                    reason: format!("metrics computation task panicked: {error}"),
                })
            })??;
        return Ok(json!({
            "branch_name": branch_name,
            "summary": summary,
            "comparison": {
                "branch_name": compare_to,
                "summary": comparison,
            },
            "delta": {
                "tool_calls": i64::try_from(summary.total_tool_calls).unwrap_or(i64::MAX)
                    .saturating_sub(i64::try_from(comparison.total_tool_calls).unwrap_or(i64::MAX)),
                "tokens": i64::try_from(summary.total_tokens).unwrap_or(i64::MAX)
                    .saturating_sub(i64::try_from(comparison.total_tokens).unwrap_or(i64::MAX)),
            }
        }));
    }

    Ok(json!({
        "branch_name": branch_name,
        "summary": summary,
    }))
}

/// Return a concise text report describing current-branch token delivery.
pub async fn get_token_savings_report(
    state: SharedState,
    _params: Option<Value>,
) -> Result<Value, EngramError> {
    let (workspace_path, branch) = workspace_snapshot_path_and_branch(&state).await?;
    let wp = workspace_path.clone();
    let br = branch.clone();
    let summary = tokio::task::spawn_blocking(move || metrics::compute_summary(&wp, &br))
        .await
        .map_err(|error| {
            EngramError::Metrics(crate::errors::MetricsError::WriteFailed {
                reason: format!("metrics computation task panicked: {error}"),
            })
        })??;
    #[allow(clippy::cast_precision_loss)]
    let average_tokens = if summary.total_tool_calls == 0 {
        0.0
    } else {
        summary.total_tokens as f64 / summary.total_tool_calls as f64
    };
    let top_symbols = if summary.top_symbols.is_empty() {
        "none".to_owned()
    } else {
        summary
            .top_symbols
            .iter()
            .take(5)
            .map(|entry| format!("{} ({})", entry.name, entry.count))
            .collect::<Vec<_>>()
            .join(", ")
    };

    Ok(json!({
        "branch": branch,
        "report": format!(
            "On branch {branch}, engram delivered {} tokens across {} tool calls. Average {:.2} tokens per call. Most-queried symbols: {top_symbols}.",
            summary.total_tokens,
            summary.total_tool_calls,
            average_tokens,
        ),
    }))
}

// ── query_graph (T074) ────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct QueryGraphParams {
    query: String,
    /// Reserved for future parameterised queries; accepted but not yet used.
    #[serde(default)]
    #[allow(dead_code)]
    params: Option<serde_json::Value>,
}

/// Execute a sandboxed read-only SurrealQL query against the workspace graph.
///
/// The query is sanitised by [`crate::services::gate::sanitize_query`] before
/// execution; any write keyword causes an immediate `QUERY_REJECTED` (4010)
/// error. Execution is bounded by `query_timeout_ms` from [`WorkspaceConfig`]
/// and results are capped at `query_row_limit` rows, with a `"truncated"` flag
/// when the cap is applied.
#[tracing::instrument(name = "tool.query_graph", skip(state, params))]
pub async fn query_graph(state: SharedState, params: Option<Value>) -> Result<Value, EngramError> {
    use std::time::Instant;

    use crate::services::gate::sanitize_query;

    let (data_dir, branch) = workspace_db(&state).await?;

    let parsed: QueryGraphParams =
        serde_json::from_value(params.unwrap_or_default()).map_err(|e| {
            EngramError::System(SystemError::InvalidParams {
                reason: e.to_string(),
            })
        })?;

    if parsed.query.trim().is_empty() {
        return Err(EngramError::Query(QueryError::QueryEmpty));
    }

    // Sanitize: reject write operations before touching the DB.
    sanitize_query(&parsed.query)?;

    // Pull per-workspace limits (fall back to safe defaults if no config is loaded).
    let (timeout_ms, row_limit) = state
        .workspace_config()
        .await
        .map_or((5_000_u64, 1_000_usize), |c| {
            (c.query_timeout_ms, c.query_row_limit)
        });

    let db = connect_db(&data_dir, &branch).await?;
    let start = Instant::now();

    // Inject a LIMIT clause to cap result-set materialization at the DB level.
    // Fetch row_limit + 1 so we can detect truncation without loading everything.
    let fetch_limit = row_limit + 1;
    let bounded_query = inject_limit(&parsed.query, fetch_limit);

    let timed = tokio::time::timeout(
        std::time::Duration::from_millis(timeout_ms),
        db.query(&bounded_query),
    )
    .await;

    match timed {
        Err(_elapsed) => Err(EngramError::GraphQuery(GraphQueryError::Timeout {
            timeout_ms,
        })),
        Ok(Err(e)) => Err(EngramError::GraphQuery(GraphQueryError::Invalid {
            reason: e.to_string(),
        })),
        Ok(Ok(mut response)) => {
            // Fetch row_limit + 1 to detect truncation without materializing
            // an unbounded result set. The user's query is already sanitized
            // (read-only) but may lack a LIMIT clause.
            let rows: Vec<serde_json::Value> = response.take(0).map_err(|e| {
                EngramError::GraphQuery(GraphQueryError::Invalid {
                    reason: e.to_string(),
                })
            })?;

            let truncated = rows.len() > row_limit;
            let returned_rows: Vec<serde_json::Value> = rows.into_iter().take(row_limit).collect();
            let row_count = returned_rows.len() as u64;
            let elapsed_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);

            Ok(json!({
                "rows": returned_rows,
                "row_count": row_count,
                "truncated": truncated,
                "elapsed_ms": elapsed_ms,
            }))
        }
    }
}

/// Appends `LIMIT <n>` to a query when the user hasn't already specified one.
///
/// This ensures the DB never materializes an unbounded result set. If the query
/// already contains a top-level LIMIT clause, it is left unchanged (the
/// configured row_limit still caps the returned rows after the fact).
fn inject_limit(query: &str, limit: usize) -> String {
    let upper = query.to_uppercase();
    // Only inject when the query lacks a top-level LIMIT (outside subqueries).
    // Simple heuristic: check if "LIMIT" appears after the last closing paren.
    let tail = upper.rfind(')').map_or(upper.as_str(), |pos| &upper[pos..]);
    if tail.contains("LIMIT") {
        query.to_string()
    } else {
        format!("{} LIMIT {limit}", query.trim_end_matches(';'))
    }
}

// ── query_changes (T041) ──────────────────────────────────────────────────────

/// Parameters for the `query_changes` MCP tool.
#[cfg(feature = "git-graph")]
#[derive(Deserialize)]
struct QueryChangesParams {
    /// Filter commits that touched this file path.
    #[serde(default)]
    file_path: Option<String>,
    /// Filter commits that affected this named symbol (cross-references code graph).
    #[serde(default)]
    symbol: Option<String>,
    /// Return only commits on or after this ISO-8601 timestamp.
    #[serde(default)]
    since: Option<String>,
    /// Return only commits on or before this ISO-8601 timestamp.
    #[serde(default)]
    until: Option<String>,
    /// Maximum number of commits to return (default: 20).
    #[serde(default)]
    limit: Option<u32>,
}

/// Query indexed git commits filtered by file path, symbol name, or date range.
///
/// Requires the `git-graph` feature and an indexed workspace. Returns error
/// `1001` when no workspace is active.
#[cfg(feature = "git-graph")]
pub async fn query_changes(
    state: SharedState,
    params: Option<Value>,
) -> Result<Value, EngramError> {
    use chrono::DateTime;

    let (data_dir, branch) = if let Some(snap) = state.snapshot_workspace().await {
        (snap.data_dir.clone(), snap.branch.clone())
    } else {
        return Err(EngramError::Workspace(WorkspaceError::NotSet));
    };

    let parsed: QueryChangesParams = serde_json::from_value(params.unwrap_or_else(|| json!({})))
        .map_err(|e| {
            EngramError::System(SystemError::InvalidParams {
                reason: e.to_string(),
            })
        })?;

    let limit = parsed.limit.unwrap_or(20);

    let db = connect_db(&data_dir, &branch).await?;
    let queries = CodeGraphQueries::new(db);
    let since_dt = parsed
        .since
        .as_deref()
        .map(|s| {
            DateTime::parse_from_rfc3339(s)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .map_err(|_| {
                    EngramError::System(SystemError::InvalidParams {
                        reason: format!("invalid `since` timestamp: {s}"),
                    })
                })
        })
        .transpose()?;

    let until_dt = parsed
        .until
        .as_deref()
        .map(|s| {
            DateTime::parse_from_rfc3339(s)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .map_err(|_| {
                    EngramError::System(SystemError::InvalidParams {
                        reason: format!("invalid `until` timestamp: {s}"),
                    })
                })
        })
        .transpose()?;

    // If a symbol is provided, resolve its file path via the code graph so we
    // can filter commits by file. Symbol not found → CodeGraphError::SymbolNotFound.
    let effective_file_path: Option<String> = if let Some(ref sym) = parsed.symbol {
        let cg_db = connect_db(&data_dir, &branch).await?;
        let cg = CodeGraphQueries::new(cg_db);
        let syms = cg.find_symbols_by_name(sym).await?;
        if syms.is_empty() {
            return Err(EngramError::CodeGraph(CodeGraphError::SymbolNotFound {
                name: sym.clone(),
            }));
        }
        // Use the first symbol's file path to filter commits.
        syms.into_iter().next().map(|s| s.file_path)
    } else {
        parsed.file_path.clone()
    };

    let commits = match (
        effective_file_path.as_deref(),
        since_dt.as_ref(),
        until_dt.as_ref(),
    ) {
        (Some(fp), _, _) => queries.select_commits_by_file_path(fp, limit).await?,
        (None, since, until) => {
            queries
                .select_commits_by_date_range(since, until, limit)
                .await?
        }
    };

    let commits_json: Vec<Value> = commits
        .into_iter()
        .map(|c| {
            serde_json::to_value(&c).map_err(|e| {
                EngramError::System(SystemError::DatabaseError {
                    reason: format!("commit serialization failed: {e}"),
                })
            })
        })
        .collect::<Result<_, _>>()?;

    Ok(json!({
        "commits": commits_json,
        "total": commits_json.len(),
        "file_path": effective_file_path,
        "symbol": parsed.symbol,
    }))
}

/// Compute and return an agent efficiency evaluation report.
///
/// Reads recorded [`UsageEvent`]s for the active branch, scores agent
/// tool-usage patterns, and returns an [`EvaluationReport`] as JSON.
///
/// # Errors
///
/// Returns `WorkspaceError::NotSet` (1003) when no workspace is active.
/// Returns `MetricsError::NotFound` (13002) when no usage events have been
/// recorded for the current branch.
pub async fn get_evaluation_report(
    state: SharedState,
    _params: Option<Value>,
) -> Result<Value, EngramError> {
    let (workspace_path, branch) = workspace_snapshot_path_and_branch(&state).await?;
    let config = state.evaluation_config().await.unwrap_or_default();

    let wp = workspace_path.clone();
    let br = branch.clone();
    let events = tokio::task::spawn_blocking(move || metrics::load_events(&wp, &br))
        .await
        .map_err(|e| {
            EngramError::Metrics(crate::errors::MetricsError::WriteFailed {
                reason: format!("metrics load task panicked: {e}"),
            })
        })??;

    let report = crate::services::evaluation::evaluate(&events, &config);

    serde_json::to_value(&report).map_err(|e| {
        EngramError::System(SystemError::DatabaseError {
            reason: format!("failed to serialize evaluation report: {e}"),
        })
    })
}
