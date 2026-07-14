use std::path::PathBuf;

use serde::Deserialize;
use serde_json::{Value, json};

use crate::db::connect_db;
use crate::db::queries::{CodeGraphQueries, FindPathResult, QueryGraphResult, SymbolFilter};
use crate::errors::{CodeGraphError, EngramError, QueryError, SystemError, WorkspaceError};
use crate::models::TraversalDirection;
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

    // Read-only: code graph counters may reflect partial data while indexing
    // is in progress, but returning potentially-incomplete statistics is
    // more useful than an IndexInProgress error. Callers can inspect
    // `scan_status.running` in workspace_status to detect mid-index state.

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

    // Read-only: content records may be partially written during a background
    // index. Returning available data is more useful than an error; the
    // caller can check scan_status.running in workspace_status if freshness matters.

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
            source_type: cr.content_type.clone(),
            content: cr.content.clone(),
            embedding: cr.embedding.clone(),
            title: content_record_title(&cr),
            file_path: Some(cr.file_path.clone()),
            line_range: content_record_line_range(&cr),
            record_kind: Some(cr.record_kind.clone()),
            heading_path: cr.heading_path.clone(),
            fallback_reason: cr.fallback_reason.clone(),
            lint_summary: cr.lint_summary.clone(),
            suggestions: cr.suggestions.clone(),
        });
    }

    // Include backlog content records when the filter is unset or explicitly
    // requests "backlog" content.  Backlog records live in a separate relation
    // (`backlog_content_record`) and have no embedding, so they participate
    // only in lexical (BM25) matching.
    let include_backlog = parsed
        .content_type
        .as_deref()
        .is_none_or(|ct| ct == "backlog");
    if include_backlog {
        let backlog_records = queries.select_backlog_content_records(None).await?;
        for bcr in backlog_records {
            candidates.push(SearchCandidate {
                id: format!("backlog_content_record:{}", bcr.file_path),
                source_type: bcr.content_type,
                content: bcr.content,
                embedding: None,
                title: Some(bcr.file_path.clone()),
                file_path: Some(bcr.file_path),
                line_range: None,
                record_kind: Some("file".to_owned()),
                heading_path: Vec::new(),
                fallback_reason: None,
                lint_summary: None,
                suggestions: Vec::new(),
            });
        }
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

    // Read-only: graph state may be partially written during a background
    // index. Returning available symbol graph context is more useful than
    // blocking with an IndexInProgress error.

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

    // Read-only: symbol list may be incomplete during a background index.
    // Returning partial results is more useful than an IndexInProgress error.

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
/// Scoring: cosine similarity on embedding vectors for code symbols and content
/// records; content falls back to a keyword ratio when no embedded records exist
/// yet. Results are merged and ranked by a code-biased key (code ranks above
/// content unless a content result is more relevant by more than
/// [`crate::services::search::CODE_RANK_BOOST`]); the reported `score` on each
/// result is its unboosted per-source score (the boost affects ordering only).
/// `region: "code"` restricts results to code symbols only.
///
/// Returns summary text only, not full bodies (FR-148 exemption).
///
/// # Errors
/// - `QueryEmpty` (4001) for empty or whitespace-only queries (FR-157).
/// - `SearchFailed` (4004) if the embedding model is not loaded/enabled.
/// - `SystemError::DatabaseError` (5001) if embedding generation fails after model load.
/// - `WorkspaceError::NotSet` (1003) if workspace not bound.
// The #[cfg(not(feature = "embeddings"))] early-return guard makes the
// embeddings-specific function body unreachable in no-embeddings builds.
// This is intentional: the guard keeps the non-embeddings build from
// pulling in embedding API call sites.
#[allow(unreachable_code)]
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

    // Read-only: code graph and content vectors may be partially written during
    // a background index. Returning available search results is more useful than
    // an IndexInProgress error; freshness is indicated via scan_status.running.

    // Clamp limit to [1, 50].
    let limit = parsed.limit.clamp(1, 50);

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
                    record_kind: None,
                    heading_path: Vec::new(),
                    fallback_reason: None,
                    lint_summary: None,
                    suggestions: Vec::new(),
                }
            })
            .collect::<Vec<_>>()
    };

    // ── Content records: KNN vector search ──────────────────────────
    // Requires embeddings feature; the cfg guard at the top of this
    // function already returned an error for non-embeddings builds, so
    // we are guaranteed to have a valid query_embedding here.
    //
    // `region: "code"` restricts results to code symbols only — skip the
    // content fetch entirely (search::should_include_content).
    let content_results: Vec<UnifiedSearchResult> =
        if crate::services::search::should_include_content(&parsed.region) {
            let knn = queries
                .vector_search_content_native(
                    &query_embedding,
                    limit,
                    parsed.content_type.as_deref(),
                )
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
                            Some(content_record_unified_result(score, cr))
                        } else {
                            None
                        }
                    })
                    .collect()
            } else {
                knn.into_iter()
                    .map(|(score, cr)| content_record_unified_result(score, cr))
                    .collect()
            }
        } else {
            Vec::new()
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

fn content_record_title(record: &crate::models::ContentRecord) -> Option<String> {
    record.heading_path.last().cloned().or_else(|| {
        std::path::Path::new(&record.file_path)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .filter(|stem| !stem.is_empty())
            .map(ToOwned::to_owned)
    })
}

fn content_record_line_range(record: &crate::models::ContentRecord) -> Option<String> {
    match (record.line_start, record.line_end) {
        (Some(start), Some(end)) => Some(format!("L{start}-L{end}")),
        (Some(start), None) => Some(format!("L{start}")),
        _ => None,
    }
}

fn content_record_unified_result(
    score: f32,
    record: crate::models::ContentRecord,
) -> UnifiedSearchResult {
    UnifiedSearchResult {
        region: SearchRegion::Task,
        score,
        node_type: record.content_type.clone(),
        id: format!("content_record:{}", record.id),
        title: content_record_title(&record),
        file_path: Some(record.file_path.clone()),
        line_range: content_record_line_range(&record),
        summary: Some(truncate_summary(&record.content, 200)),
        status: None,
        linked_symbols: None,
        record_kind: Some(record.record_kind),
        heading_path: record.heading_path,
        fallback_reason: record.fallback_reason,
        lint_summary: record.lint_summary,
        suggestions: record.suggestions,
    }
}

// ── impact_analysis (T061 — Phase 8) ───────────────────────────────────────

#[derive(Deserialize)]
struct ImpactAnalysisParams {
    #[serde(default)]
    symbol_name: String,
    #[serde(default = "default_impact_depth")]
    depth: usize,
    #[serde(default = "default_impact_max_nodes")]
    max_nodes: usize,
    /// Optional semantic concept for combined structural+semantic results.
    #[serde(default)]
    concept: Option<String>,
    /// Optional stable Power BI node-id selector. When supplied it pins the
    /// impact root to exactly this node and bypasses name resolution, so an
    /// ambiguous name (or a code-symbol/Power BI name collision) can be
    /// disambiguated to a single candidate. Additive and back-compatible:
    /// name-based calls that omit it are unchanged.
    #[serde(default)]
    powerbi_node_id: Option<String>,
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

    // Read-only: graph may be partially populated during a background index.
    // Returning available impact data is more useful than an IndexInProgress error.

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

    // Power BI root selection (C3): an explicit `powerbi_node_id` pins the root
    // to exactly one node and bypasses name resolution entirely.
    if let Some(node_id) = parsed
        .powerbi_node_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let Some(root) = cg_queries.find_powerbi_node_by_id(node_id).await? else {
            return Err(EngramError::CodeGraph(CodeGraphError::SymbolNotFound {
                name: node_id.to_owned(),
            }));
        };
        return powerbi_impact_response(&cg_queries, &root, effective_depth, effective_max_nodes)
            .await;
    }

    if parsed.symbol_name.trim().is_empty() {
        return Err(EngramError::System(SystemError::InvalidParams {
            reason: "impact_analysis requires `symbol_name` or `powerbi_node_id`".to_owned(),
        }));
    }

    // Name resolution: code symbols take precedence for back-compat. A Power BI
    // entity that shares a name with a code symbol is reachable only via the
    // explicit `powerbi_node_id` selector above.
    let matches = cg_queries.find_symbols_by_name(&parsed.symbol_name).await?;
    if matches.is_empty() {
        // Fall back to Power BI name resolution before failing.
        let pbi_matches = cg_queries
            .find_powerbi_nodes_by_name(&parsed.symbol_name, None, None)
            .await?;
        let Some(root) = pbi_matches.first() else {
            return Err(EngramError::CodeGraph(CodeGraphError::SymbolNotFound {
                name: parsed.symbol_name,
            }));
        };
        let mut response =
            powerbi_impact_response(&cg_queries, root, effective_depth, effective_max_nodes)
                .await?;
        if pbi_matches.len() > 1 {
            // Ambiguous name — surface every candidate so the caller can re-pin
            // via `powerbi_node_id`.
            response["powerbi_candidates"] = json!(
                pbi_matches
                    .iter()
                    .map(|node| {
                        json!({
                            "id": node.id,
                            "name": node.name,
                            "kind": node.kind.as_str(),
                            "source_path": node.source_path,
                        })
                    })
                    .collect::<Vec<_>>()
            );
        }
        return Ok(response);
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
        "root_kind": "code_symbol",
        "code_neighborhood": code_neighborhood,
        "effective_depth": effective_depth,
        "effective_max_nodes": effective_max_nodes,
    });

    if !hybrid_results.is_empty() {
        response["hybrid_results"] = json!(hybrid_results);
    }

    Ok(response)
}

/// Compute the additive Power BI impact response for a Power BI root node (C3).
///
/// Uses an edge- and node-kind-aware traversal built by composing directed
/// [`CodeGraphQueries::query_graph_neighborhood`] calls (no new traversal
/// engine):
///
/// 1. Incoming `pbi_uses_field` closure from the root yields its dependents —
///    dependent measures (`measure→column`/`measure→measure`) and any visuals
///    that reference the field. Traversing `pbi_contains` here is deliberately
///    avoided because incoming `pbi_contains` on a column/measure would pull in
///    the root's owner table/model.
/// 2. From each *dependent visual*, an incoming `pbi_contains` expansion adds
///    the containing page and report (`visual→page→report`) — applied only from
///    visuals so owner ancestors of the root never enter.
///
/// Returns the JSON response for the Power BI impact root.
async fn powerbi_impact_response(
    cg_queries: &CodeGraphQueries,
    root: &crate::models::PowerBiNode,
    effective_depth: usize,
    effective_max_nodes: usize,
) -> Result<Value, EngramError> {
    use std::collections::HashSet;

    let mut selected: HashSet<String> = HashSet::new();
    selected.insert(root.id.clone());
    let mut neighborhood: Vec<Value> = Vec::new();
    let mut visual_ids: Vec<String> = Vec::new();

    // A single node budget is shared across both traversal phases so the total
    // returned neighbourhood never exceeds the advertised `effective_max_nodes`,
    // regardless of how many dependent visuals Phase 2 expands.
    let mut remaining = effective_max_nodes;

    // Phase 1: dependents via incoming `pbi_uses_field`.
    let dependents = cg_queries
        .query_graph_neighborhood(
            &root.id,
            TraversalDirection::Incoming,
            effective_depth,
            remaining,
            &["pbi_uses_field"],
        )
        .await?;
    for node in &dependents.nodes {
        if remaining == 0 {
            break;
        }
        if !selected.insert(node.id.clone()) {
            continue;
        }
        if node.kind == "powerbi_visual" {
            visual_ids.push(node.id.clone());
        }
        neighborhood.push(powerbi_graph_node_to_json(node));
        remaining -= 1;
    }

    // Phase 2: onward containment (visual→page→report) from dependent visuals,
    // drawing from the same shared budget so the combined result stays bounded.
    for visual_id in &visual_ids {
        if remaining == 0 {
            break;
        }
        let containers = cg_queries
            .query_graph_neighborhood(
                visual_id,
                TraversalDirection::Incoming,
                effective_depth,
                remaining,
                &["pbi_contains"],
            )
            .await?;
        for node in &containers.nodes {
            if remaining == 0 {
                break;
            }
            if !selected.insert(node.id.clone()) {
                continue;
            }
            neighborhood.push(powerbi_graph_node_to_json(node));
            remaining -= 1;
        }
    }

    let response = json!({
        "symbol": {
            "name": root.name,
            "type": root.kind.as_str(),
            "id": root.id,
            "file_path": root.file_path,
        },
        "root_kind": "powerbi_entity",
        "powerbi_neighborhood": neighborhood,
        "effective_depth": effective_depth,
        "effective_max_nodes": effective_max_nodes,
    });
    Ok(response)
}

/// Serialise a graph BFS node into the Power BI neighbourhood JSON shape.
fn powerbi_graph_node_to_json(node: &crate::db::queries::QueryGraphNode) -> Value {
    json!({
        "id": node.id,
        "name": node.name,
        "kind": node.kind,
        "file_path": node.file_path,
    })
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
    let version = env!("CARGO_PKG_VERSION");
    let uptime_secs = state.uptime_seconds();
    let connections = state.active_connections();
    let workspace_snapshot = state.snapshot_workspace().await;
    let workspace_id = workspace_snapshot.as_ref().map(|s| s.workspace_id.clone());
    let tool_call_count = state.tool_call_count();
    let (p50, p95, p99) = state.latency_percentiles().await;
    let (watcher_events, last_watcher_event) = state.watcher_stats().await;

    let memory_mb = crate::services::process_memory::current_process_memory_bytes()
        .map(|bytes| bytes / 1_048_576);

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
                "request_bytes": i64::try_from(summary.total_request_bytes).unwrap_or(i64::MAX)
                    .saturating_sub(i64::try_from(comparison.total_request_bytes).unwrap_or(i64::MAX)),
                "response_bytes": i64::try_from(summary.total_response_bytes).unwrap_or(i64::MAX)
                    .saturating_sub(i64::try_from(comparison.total_response_bytes).unwrap_or(i64::MAX)),
                "input_tokens": i64::try_from(summary.total_input_tokens).unwrap_or(i64::MAX)
                    .saturating_sub(i64::try_from(comparison.total_input_tokens).unwrap_or(i64::MAX)),
                "output_tokens": i64::try_from(summary.total_output_tokens).unwrap_or(i64::MAX)
                    .saturating_sub(i64::try_from(comparison.total_output_tokens).unwrap_or(i64::MAX)),
                "results": i64::try_from(summary.total_result_count).unwrap_or(i64::MAX)
                    .saturating_sub(i64::try_from(comparison.total_result_count).unwrap_or(i64::MAX)),
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
    // Load events once, then derive both the summary and the (report-only)
    // per-correlation breakdown from the same read.
    let (summary, by_correlation_id) = tokio::task::spawn_blocking(move || {
        let events = metrics::load_events(&wp, &br)?;
        let summary = crate::models::metrics::MetricsSummary::from_events(&events);
        let by_correlation_id = crate::models::metrics::correlation_metrics(&events);
        Ok::<_, EngramError>((summary, by_correlation_id))
    })
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
            "On branch {branch}, engram handled {} input tokens / {} output tokens across {} tool calls ({} request bytes, {} response bytes, {} results). Average {:.2} output tokens per call. Most-queried symbols: {top_symbols}.",
            summary.total_input_tokens,
            summary.total_output_tokens,
            summary.total_tool_calls,
            summary.total_request_bytes,
            summary.total_response_bytes,
            summary.total_result_count,
            average_tokens,
        ),
        // 075-S adoption metrics: structured breakdown so autoharness can
        // quantify how much it exercises engram. Additive — the `branch` and
        // `report` fields above are unchanged for existing consumers. This is
        // the only surface that emits the heavy `by_correlation_id` map.
        "metrics": {
            "schema_version": crate::models::metrics::USAGE_SCHEMA_VERSION,
            "total_tool_calls": summary.total_tool_calls,
            "unique_tools_exercised": summary.unique_tools_exercised,
            "distinct_correlation_ids": summary.distinct_correlation_ids,
            "time_range": summary.time_range,
            "by_tool": summary.by_tool,
            "by_correlation_id": by_correlation_id,
        },
    }))
}

// ── query_graph (T074) ────────────────────────────────────────────────────────

// ── query_graph (T048) ────────────────────────────────────────────────────────

/// Hard cap on the number of result nodes returned by any `query_graph` operation.
const HARD_MAX_NODES: usize = 500;

fn default_max_depth() -> usize {
    3
}

fn default_max_nodes() -> usize {
    50
}

/// Structured input for the `query_graph` MCP tool.
///
/// Dispatches to one of three operations via the required `operation` tag.
/// Use `{ "operation": "neighborhood", "root": "fn:..." }` for graph exploration,
/// `{ "operation": "find_path", "from": "...", "to": "..." }` for path queries, or
/// `{ "operation": "transitive_closure", "root": "..." }` for reachability analysis.
#[derive(Debug, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
enum GraphQuery {
    /// BFS neighborhood from a root node (bidirectional by default).
    Neighborhood {
        /// Root node ID (e.g., `fn:abc123`, `class:xyz789`).
        root: String,
        /// Traversal direction — defaults to `both`.
        #[serde(default)]
        direction: TraversalDirection,
        /// Maximum hop depth from root — defaults to 3.
        #[serde(default = "default_max_depth")]
        max_depth: usize,
        /// Maximum nodes to return — defaults to 50, hard-capped at 500.
        #[serde(default = "default_max_nodes")]
        max_nodes: usize,
        /// Edge types to traverse — empty means all types.
        #[serde(default)]
        edge_types: Vec<String>,
    },
    /// BFS shortest-path search between two nodes (forward edges only).
    FindPath {
        /// Start node ID.
        from: String,
        /// End node ID.
        to: String,
        /// Maximum hop depth to search — defaults to 3.
        #[serde(default = "default_max_depth")]
        max_depth: usize,
        /// Edge types to traverse — empty means all types.
        #[serde(default)]
        edge_types: Vec<String>,
    },
    /// All nodes reachable from a root via outgoing edges.
    TransitiveClosure {
        /// Root node ID.
        root: String,
        /// Maximum hop depth — defaults to 3.
        #[serde(default = "default_max_depth")]
        max_depth: usize,
        /// Maximum nodes to return — defaults to 50, hard-capped at 500.
        #[serde(default = "default_max_nodes")]
        max_nodes: usize,
        /// Edge types to traverse — empty means all types.
        #[serde(default)]
        edge_types: Vec<String>,
    },
}

/// Build a JSON response for `neighborhood` and `transitive_closure` results.
fn build_graph_json(operation: &str, root: &str, result: QueryGraphResult) -> Value {
    let node_count = result.nodes.len();
    let nodes: Vec<Value> = result
        .nodes
        .iter()
        .map(|n| {
            json!({
                "id": n.id,
                "kind": n.kind,
                "name": n.name,
                "file_path": n.file_path,
            })
        })
        .collect();
    let edges: Vec<Value> = result
        .edges
        .iter()
        .map(|e| {
            json!({
                "edge_type": e.edge_type,
                "from": e.from,
                "to": e.to,
            })
        })
        .collect();
    json!({
        "operation": operation,
        "root": root,
        "nodes": nodes,
        "edges": edges,
        "node_count": node_count,
        "truncated": result.truncated,
    })
}

/// Build a JSON response for `find_path` results.
fn build_find_path_json(from: &str, to: &str, result: FindPathResult) -> Value {
    let hop_count = if result.path.len() > 1 {
        result.path.len() - 1
    } else {
        0
    };
    json!({
        "operation": "find_path",
        "from": from,
        "to": to,
        "found": result.found,
        "path": result.path,
        "hop_count": hop_count,
    })
}

/// Execute a structured graph query against the workspace code and backlog graph.
///
/// Accepts a tagged `operation` JSON object instead of a raw Datalog string.
/// Three operations are supported:
/// - `neighborhood`: BFS from a root node in one or both directions.
/// - `find_path`: Shortest path between two nodes via outgoing edges.
/// - `transitive_closure`: All nodes reachable from a root via outgoing edges.
///
/// Edge type namespace: code edges (`calls`, `imports`, `defines`, `inherits_from`,
/// `concerns`, `references`) and backlog edges (`parent_of`, `depends_on`,
/// `backlog_references`). Pass an empty `edge_types` array to traverse all types.
#[tracing::instrument(name = "tool.query_graph", skip(state, params))]
pub async fn query_graph(state: SharedState, params: Option<Value>) -> Result<Value, EngramError> {
    let raw = params.unwrap_or_default();

    // Legacy compat: if `query` field is present without `operation`, return a helpful error
    // rather than a confusing parse failure message.
    if let Some(obj) = raw.as_object() {
        if obj.contains_key("query") && !obj.contains_key("operation") {
            return Err(EngramError::System(SystemError::InvalidParams {
                reason: "query_graph now requires a structured operation. \
                         Use {\"operation\":\"neighborhood\",\"root\":\"fn:...\"} \
                         instead of a raw Datalog query string."
                    .into(),
            }));
        }
    }

    let gq: GraphQuery = serde_json::from_value(raw).map_err(|e| {
        EngramError::System(SystemError::InvalidParams {
            reason: e.to_string(),
        })
    })?;

    let (data_dir, branch) = workspace_db(&state).await?;
    let db = connect_db(&data_dir, &branch).await?;
    let cg_queries = CodeGraphQueries::new(db);

    match gq {
        GraphQuery::Neighborhood {
            root,
            direction,
            max_depth,
            max_nodes,
            edge_types,
        } => {
            let capped = max_nodes.min(HARD_MAX_NODES);
            let edge_refs: Vec<&str> = edge_types.iter().map(String::as_str).collect();
            let result = cg_queries
                .query_graph_neighborhood(&root, direction, max_depth, capped, &edge_refs)
                .await?;
            Ok(build_graph_json("neighborhood", &root, result))
        }
        GraphQuery::FindPath {
            from,
            to,
            max_depth,
            edge_types,
        } => {
            let edge_refs: Vec<&str> = edge_types.iter().map(String::as_str).collect();
            let result = cg_queries
                .find_path(&from, &to, max_depth, &edge_refs)
                .await?;
            Ok(build_find_path_json(&from, &to, result))
        }
        GraphQuery::TransitiveClosure {
            root,
            max_depth,
            max_nodes,
            edge_types,
        } => {
            let capped = max_nodes.min(HARD_MAX_NODES);
            let edge_refs: Vec<&str> = edge_types.iter().map(String::as_str).collect();
            let result = cg_queries
                .transitive_closure(&root, max_depth, capped, &edge_refs)
                .await?;
            Ok(build_graph_json("transitive_closure", &root, result))
        }
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

    // Read-only: git-graph tables may be partially written during a background
    // index. Returning available commit data is more useful than blocking the caller.

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

// ── 040.002-T: get_mutable_script_retry_metrics ───────────────────────────────

/// Return mutable-script SQLITE_BUSY retry telemetry.
///
/// Reads the process-global retry counter and last-retry timestamp accumulated
/// by `run_script_busy_retry_mutable`. Does not require a workspace to be bound.
///
/// Response schema: `{ retry_count: u64, last_retry_at: Option<String> }` where
/// `last_retry_at` is an RFC-3339 timestamp or `null`.
///
/// # Errors
///
/// This function is infallible in practice. It returns `Ok` unconditionally
/// because the response is constructed directly with the `json!` macro from
/// plain numeric and optional-string values.
#[allow(clippy::unused_async)] // async required by tool-dispatch contract
pub async fn get_mutable_script_retry_metrics(
    _state: SharedState,
    _params: Option<Value>,
) -> Result<Value, EngramError> {
    let metrics = crate::db::mutable_script_retry_metrics();
    let last_retry_at = metrics.last_retry_at.map(|dt| dt.to_rfc3339());
    Ok(json!({
        "retry_count": metrics.retry_count,
        "last_retry_at": last_retry_at,
    }))
}
