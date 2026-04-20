//! CozoDB query stub — Phase 1 compilation shim.
//!
//! Provides the same public API as the SurrealDB `queries.rs` module so that
//! call sites compile under `--features cozo-backend` without modification.
//! All methods return a "not yet implemented" error.  Phase 2 will replace
//! these stubs with real Cozo Datalog implementations.

#![allow(clippy::unused_async)]

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::db::Db;
use crate::errors::{EngramError, SystemError};

pub use crate::models::FileHashRecord;

/// Threshold in milliseconds above which a query is logged at WARN level.
///
/// Mirrors the constant from the SurrealDB `queries` module; included here
/// for API compatibility with any external code that references
/// `crate::db::queries::SLOW_QUERY_THRESHOLD_MS`.
pub const SLOW_QUERY_THRESHOLD_MS: u128 = 100;

/// Emit tracing span fields and a log event for a completed query.
///
/// Mirrors the function from the SurrealDB `queries` module for API
/// compatibility.  The implementation is backend-agnostic — it records
/// span fields and emits a tracing event with no database interaction.
pub fn record_query_metrics(
    query_type: &str,
    table: &str,
    result_count: usize,
    elapsed: std::time::Duration,
) {
    let elapsed_ms = elapsed.as_millis();
    let span = tracing::Span::current();
    span.record("query_type", query_type);
    span.record("table", table);
    span.record("result_count", result_count);
    let elapsed_ms_u64 = u64::try_from(elapsed_ms).unwrap_or(u64::MAX);
    tracing::info!(
        query_type,
        table,
        result_count,
        elapsed_ms = elapsed_ms_u64,
        "query completed"
    );
}

// ── Shared data types ─────────────────────────────────────────────────────
//
// These types are duplicated from `queries.rs` because they have no
// SurrealDB dependencies.  They will be deduplicated into a shared types
// module during Phase 2+ refactoring.

/// Information about a `concerns` edge targeting a symbol.
#[derive(Debug, Clone)]
pub struct ConcernsEdgeInfo {
    /// The task ID that has the concerns edge (e.g., `task:abc123`).
    pub task_id: String,
    /// Table name of the target symbol (function, class, interface).
    pub symbol_table: String,
    /// Full qualified ID of the target symbol.
    pub symbol_id: String,
    /// Name of the target symbol (populated by caller).
    pub symbol_name: String,
    /// Body hash of the target symbol (populated by caller).
    pub symbol_body_hash: String,
    /// The client/agent that created the link.
    pub linked_by: String,
}

/// Symbol identity tuple for hash-resilient concerns relinking.
#[derive(Debug, Clone)]
pub struct SymbolIdentity {
    /// Table name (function, class, interface).
    pub table: String,
    /// Full qualified ID (e.g., `function:abc123`).
    pub id: String,
    /// Symbol name.
    pub name: String,
    /// Workspace-relative file path.
    pub file_path: String,
    /// Body hash for identity matching.
    pub body_hash: String,
    /// Embedding vector.
    pub embedding: Vec<f32>,
}

/// A resolved `concerns` edge link with symbol metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConcernsLink {
    /// Table name of the target symbol (function, class, interface).
    pub symbol_table: String,
    /// Full qualified ID of the target symbol.
    pub symbol_id: String,
    /// Symbol name.
    pub symbol_name: String,
    /// File path of the symbol.
    pub file_path: String,
    /// The client/agent that created the link.
    pub linked_by: String,
}

/// A matched code symbol with full metadata.
#[derive(Debug, Clone)]
pub struct SymbolMatch {
    /// Table name (function, class, interface, code_file).
    pub table: String,
    /// Full qualified ID (e.g., `function:abc123`).
    pub id: String,
    /// Symbol name.
    pub name: String,
    /// Workspace-relative file path.
    pub file_path: String,
    /// 1-based start line, if applicable.
    pub line_start: Option<u32>,
    /// 1-based end line, if applicable.
    pub line_end: Option<u32>,
    /// Function signature, if applicable.
    pub signature: Option<String>,
    /// Full source body.
    pub body: String,
    /// Embedding type (`explicit_code` or `summary_pointer`).
    pub embed_type: Option<String>,
    /// Summary text.
    pub summary: Option<String>,
    /// Embedding vector.
    pub embedding: Vec<f32>,
}

/// An edge discovered during BFS traversal.
#[derive(Debug, Clone)]
pub struct BfsEdge {
    /// Edge type (calls, imports, defines, inherits_from).
    pub edge_type: String,
    /// Source node ID.
    pub from: String,
    /// Target node ID.
    pub to: String,
}

/// Result of a BFS neighborhood query.
#[derive(Debug)]
pub struct BfsResult {
    /// Neighbor nodes discovered.
    pub neighbors: Vec<SymbolMatch>,
    /// Edges connecting root to neighbors.
    pub edges: Vec<BfsEdge>,
    /// Whether the traversal was truncated at `max_nodes`.
    pub truncated: bool,
}

/// Filter criteria for `list_symbols`.
#[derive(Debug, Default)]
pub struct SymbolFilter {
    /// Filter by workspace-relative file path.
    pub file_path: Option<String>,
    /// Filter by node type (function, class, interface).
    pub node_type: Option<String>,
    /// Filter by name prefix.
    pub name_prefix: Option<String>,
    /// Maximum results per page.
    pub limit: usize,
    /// Offset for pagination.
    pub offset: usize,
}

/// A single entry in a `list_symbols` result.
#[derive(Debug, Clone, Serialize)]
pub struct SymbolListEntry {
    /// Symbol name.
    pub name: String,
    /// Node type (function, class, interface).
    #[serde(rename = "type")]
    pub node_type: String,
    /// Workspace-relative file path.
    pub file_path: String,
    /// 1-based start line.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_start: Option<u32>,
    /// 1-based end line.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_end: Option<u32>,
}

/// Result of a `list_symbols` query.
#[derive(Debug)]
pub struct SymbolListResult {
    /// The matched symbols for this page.
    pub symbols: Vec<SymbolListEntry>,
    /// Total count of matching symbols.
    pub total_count: usize,
    /// Whether more results exist beyond limit+offset.
    pub has_more: bool,
}

// ── Stub error helper ─────────────────────────────────────────────────────

fn backend_err() -> EngramError {
    EngramError::from(SystemError::DatabaseError {
        reason: "CozoDB backend not yet implemented; \
                 use surreal-backend feature until Phase 2"
            .into(),
    })
}

// ── CodeGraphQueries stub ─────────────────────────────────────────────────

/// CozoDB stub for `CodeGraphQueries`.
///
/// All methods return [`backend_err()`] until Phase 2 implements real
/// Datalog queries against a `cozo::DbInstance`.
#[allow(dead_code)]
pub struct CodeGraphQueries {
    _db: Db,
}

impl CodeGraphQueries {
    /// Create a new stub wrapping the given CozoDB handle.
    pub fn new(_db: Db) -> Self {
        Self { _db }
    }

    // ── code_file CRUD ─────────────────────────────────────────────

    /// Stub — not yet implemented.
    pub async fn upsert_code_file(
        &self,
        _file: &crate::models::CodeFile,
    ) -> Result<(), EngramError> {
        Err(backend_err())
    }

    /// Stub — not yet implemented.
    pub async fn get_code_file_by_path(
        &self,
        _path: &str,
    ) -> Result<Option<crate::models::CodeFile>, EngramError> {
        Err(backend_err())
    }

    /// Stub — not yet implemented.
    pub async fn delete_code_file(&self, _path: &str) -> Result<(), EngramError> {
        Err(backend_err())
    }

    /// Stub — not yet implemented.
    pub async fn list_code_files(&self) -> Result<Vec<crate::models::CodeFile>, EngramError> {
        Err(backend_err())
    }

    // ── Bulk read helpers ─────────────────────────────────────────

    /// Stub — not yet implemented.
    pub async fn all_functions(&self) -> Result<Vec<crate::models::Function>, EngramError> {
        Err(backend_err())
    }

    /// Stub — not yet implemented.
    pub async fn all_classes(&self) -> Result<Vec<crate::models::Class>, EngramError> {
        Err(backend_err())
    }

    /// Stub — not yet implemented.
    pub async fn all_interfaces(&self) -> Result<Vec<crate::models::Interface>, EngramError> {
        Err(backend_err())
    }

    /// Stub — not yet implemented.
    pub async fn all_code_edges(&self) -> Result<Vec<crate::models::CodeEdge>, EngramError> {
        Err(backend_err())
    }

    // ── function CRUD ─────────────────────────────────────────────

    /// Stub — not yet implemented.
    pub async fn upsert_function(
        &self,
        _func: &crate::models::Function,
    ) -> Result<(), EngramError> {
        Err(backend_err())
    }

    /// Stub — not yet implemented.
    pub async fn get_function_by_name(
        &self,
        _name: &str,
    ) -> Result<Option<crate::models::Function>, EngramError> {
        Err(backend_err())
    }

    /// Stub — not yet implemented.
    pub async fn get_functions_by_file(
        &self,
        _file_path: &str,
    ) -> Result<Vec<crate::models::Function>, EngramError> {
        Err(backend_err())
    }

    /// Stub — not yet implemented.
    pub async fn delete_functions_by_file(&self, _file_path: &str) -> Result<(), EngramError> {
        Err(backend_err())
    }

    // ── class CRUD ─────────────────────────────────────────────────

    /// Stub — not yet implemented.
    pub async fn upsert_class(&self, _class: &crate::models::Class) -> Result<(), EngramError> {
        Err(backend_err())
    }

    /// Stub — not yet implemented.
    pub async fn get_class_by_name(
        &self,
        _name: &str,
    ) -> Result<Option<crate::models::Class>, EngramError> {
        Err(backend_err())
    }

    /// Stub — not yet implemented.
    pub async fn delete_classes_by_file(&self, _file_path: &str) -> Result<(), EngramError> {
        Err(backend_err())
    }

    // ── interface CRUD ────────────────────────────────────────────

    /// Stub — not yet implemented.
    pub async fn upsert_interface(
        &self,
        _iface: &crate::models::Interface,
    ) -> Result<(), EngramError> {
        Err(backend_err())
    }

    /// Stub — not yet implemented.
    pub async fn get_interface_by_name(
        &self,
        _name: &str,
    ) -> Result<Option<crate::models::Interface>, EngramError> {
        Err(backend_err())
    }

    /// Stub — not yet implemented.
    pub async fn delete_interfaces_by_file(&self, _file_path: &str) -> Result<(), EngramError> {
        Err(backend_err())
    }

    // ── Edge CRUD ──────────────────────────────────────────────────

    /// Stub — not yet implemented.
    pub async fn create_calls_edge(
        &self,
        _caller_id: &str,
        _callee_id: &str,
    ) -> Result<(), EngramError> {
        Err(backend_err())
    }

    /// Stub — not yet implemented.
    pub async fn create_imports_edge(
        &self,
        _importer_id: &str,
        _imported_id: &str,
        _import_path: &str,
    ) -> Result<(), EngramError> {
        Err(backend_err())
    }

    /// Stub — not yet implemented.
    pub async fn create_defines_edge(
        &self,
        _file_id: &str,
        _symbol_table: &str,
        _symbol_id: &str,
    ) -> Result<(), EngramError> {
        Err(backend_err())
    }

    /// Stub — not yet implemented.
    pub async fn create_inherits_edge(
        &self,
        _child_table: &str,
        _child_id: &str,
        _parent_table: &str,
        _parent_id: &str,
    ) -> Result<(), EngramError> {
        Err(backend_err())
    }

    /// Stub — not yet implemented.
    pub async fn create_concerns_edge(
        &self,
        _task_id: &str,
        _symbol_table: &str,
        _symbol_id: &str,
        _linked_by: &str,
    ) -> Result<(), EngramError> {
        Err(backend_err())
    }

    /// Stub — not yet implemented.
    pub async fn delete_edges_from_file(
        &self,
        _edge_table: &str,
        _file_id: &str,
    ) -> Result<(), EngramError> {
        Err(backend_err())
    }

    /// Stub — not yet implemented.
    pub async fn clear_file_graph(&self, _file_path: &str) -> Result<(), EngramError> {
        Err(backend_err())
    }

    // ── Concerns edge management ──────────────────────────────────

    /// Stub — not yet implemented.
    pub async fn get_concerns_edges_for_file(
        &self,
        _file_path: &str,
    ) -> Result<Vec<ConcernsEdgeInfo>, EngramError> {
        Err(backend_err())
    }

    /// Stub — not yet implemented.
    pub async fn delete_concerns_edges_for_symbol(
        &self,
        _symbol_table: &str,
        _symbol_id: &str,
    ) -> Result<usize, EngramError> {
        Err(backend_err())
    }

    /// Stub — not yet implemented.
    pub async fn find_symbols_by_name_and_hash(
        &self,
        _name: &str,
        _body_hash: &str,
    ) -> Result<Vec<SymbolIdentity>, EngramError> {
        Err(backend_err())
    }

    /// Stub — not yet implemented.
    pub async fn get_symbol_identities_for_file(
        &self,
        _file_path: &str,
    ) -> Result<Vec<SymbolIdentity>, EngramError> {
        Err(backend_err())
    }

    /// Stub — not yet implemented.
    pub async fn concerns_edge_exists(
        &self,
        _task_id: &str,
        _symbol_table: &str,
        _symbol_id: &str,
    ) -> Result<bool, EngramError> {
        Err(backend_err())
    }

    /// Stub — not yet implemented.
    pub async fn delete_concerns_by_task_and_symbol_name(
        &self,
        _task_id: &str,
        _symbol_name: &str,
    ) -> Result<usize, EngramError> {
        Err(backend_err())
    }

    /// Stub — not yet implemented.
    pub async fn list_concerns_for_task(
        &self,
        _task_id: &str,
    ) -> Result<Vec<ConcernsLink>, EngramError> {
        Err(backend_err())
    }

    /// Stub — not yet implemented.
    pub async fn find_tasks_for_symbols(
        &self,
        _symbol_ids: &[String],
    ) -> Result<Vec<(String, String)>, EngramError> {
        Err(backend_err())
    }

    // ── BFS traversal ─────────────────────────────────────────────

    /// Stub — not yet implemented.
    pub async fn find_symbols_by_name(
        &self,
        _name: &str,
    ) -> Result<Vec<SymbolMatch>, EngramError> {
        Err(backend_err())
    }

    /// Stub — not yet implemented.
    pub async fn bfs_neighborhood(
        &self,
        _root_id: &str,
        _max_depth: usize,
        _max_nodes: usize,
    ) -> Result<BfsResult, EngramError> {
        Err(backend_err())
    }

    /// Stub — not yet implemented.
    pub async fn resolve_symbol(
        &self,
        _node_id: &str,
    ) -> Result<Option<SymbolMatch>, EngramError> {
        Err(backend_err())
    }

    // ── Symbol listing ────────────────────────────────────────────

    /// Stub — not yet implemented.
    pub async fn list_symbols(
        &self,
        _filter: &SymbolFilter,
    ) -> Result<SymbolListResult, EngramError> {
        Err(backend_err())
    }

    // ── Vector search ─────────────────────────────────────────────

    /// Stub — not yet implemented.
    pub async fn vector_search_symbols(
        &self,
        _query_embedding: &[f32],
        _limit: usize,
    ) -> Result<Vec<SymbolMatch>, EngramError> {
        Err(backend_err())
    }

    /// Stub — not yet implemented.
    pub async fn vector_search_symbols_native(
        &self,
        _query_embedding: &[f32],
        _limit: usize,
    ) -> Result<Vec<(f32, SymbolMatch)>, EngramError> {
        Err(backend_err())
    }

    /// Stub — not yet implemented.
    pub async fn hybrid_graph_vector_search(
        &self,
        _root_id: &str,
        _max_depth: usize,
        _query_embedding: &[f32],
        _limit: usize,
        _edge_types: &[&str],
    ) -> Result<Vec<(f32, SymbolMatch)>, EngramError> {
        Err(backend_err())
    }

    /// Stub — not yet implemented.
    pub async fn graph_neighborhood(
        &self,
        _root_id: &str,
        _max_depth: usize,
        _max_nodes: usize,
    ) -> Result<BfsResult, EngramError> {
        Err(backend_err())
    }

    // ── Embedding updates ─────────────────────────────────────────

    /// Stub — not yet implemented.
    pub async fn update_symbol_embedding(
        &self,
        _sym_id: &str,
        _embedding: Vec<f32>,
    ) -> Result<(), EngramError> {
        Err(backend_err())
    }

    /// Stub — not yet implemented.
    pub async fn gc_corrupted_embeddings(&self) -> Result<usize, EngramError> {
        Err(backend_err())
    }

    // ── Count queries ─────────────────────────────────────────────

    /// Stub — not yet implemented.
    pub async fn count_code_files(&self) -> Result<u64, EngramError> {
        Err(backend_err())
    }

    /// Stub — not yet implemented.
    pub async fn count_functions(&self) -> Result<u64, EngramError> {
        Err(backend_err())
    }

    /// Stub — not yet implemented.
    pub async fn count_classes(&self) -> Result<u64, EngramError> {
        Err(backend_err())
    }

    /// Stub — not yet implemented.
    pub async fn count_interfaces(&self) -> Result<u64, EngramError> {
        Err(backend_err())
    }

    /// Stub — not yet implemented.
    pub async fn count_code_edges(&self) -> Result<u64, EngramError> {
        Err(backend_err())
    }

    // ── Bulk concerns ─────────────────────────────────────────────

    /// Stub — not yet implemented.
    pub async fn list_concerns_for_tasks(
        &self,
        _task_ids: &[String],
    ) -> Result<HashMap<String, Vec<ConcernsLink>>, EngramError> {
        Err(backend_err())
    }

    // ── Content record queries ────────────────────────────────────

    /// Stub — not yet implemented.
    pub async fn upsert_content_record(
        &self,
        _record: &crate::models::ContentRecord,
    ) -> Result<(), EngramError> {
        Err(backend_err())
    }

    /// Stub — not yet implemented.
    pub async fn select_content_records(
        &self,
        _content_type: Option<&str>,
    ) -> Result<Vec<crate::models::ContentRecord>, EngramError> {
        Err(backend_err())
    }

    /// Stub — not yet implemented.
    pub async fn update_content_record_embedding(
        &self,
        _record_id: &str,
        _embedding: Vec<f32>,
    ) -> Result<(), EngramError> {
        Err(backend_err())
    }

    /// Stub — not yet implemented.
    pub async fn delete_content_record_by_path(
        &self,
        _file_path: &str,
    ) -> Result<(), EngramError> {
        Err(backend_err())
    }

    /// Stub — not yet implemented.
    pub async fn vector_search_content_native(
        &self,
        _query_embedding: &[f32],
        _limit: usize,
        _content_type: Option<&str>,
    ) -> Result<Vec<(f32, crate::models::ContentRecord)>, EngramError> {
        Err(backend_err())
    }

    // ── Commit node queries ───────────────────────────────────────

    /// Stub — not yet implemented.
    pub async fn upsert_commit_node(
        &self,
        _node: &crate::models::CommitNode,
    ) -> Result<(), EngramError> {
        Err(backend_err())
    }

    /// Stub — not yet implemented.
    pub async fn select_commits_by_date_range(
        &self,
        _since: Option<&DateTime<Utc>>,
        _until: Option<&DateTime<Utc>>,
        _limit: u32,
    ) -> Result<Vec<crate::models::CommitNode>, EngramError> {
        Err(backend_err())
    }

    /// Stub — not yet implemented.
    pub async fn select_commits_by_file_path(
        &self,
        _file_path: &str,
        _limit: u32,
    ) -> Result<Vec<crate::models::CommitNode>, EngramError> {
        Err(backend_err())
    }

    /// Stub — not yet implemented.
    pub async fn latest_indexed_commit_hash(&self) -> Result<Option<String>, EngramError> {
        Err(backend_err())
    }

    // ── File hash queries ─────────────────────────────────────────

    /// Stub — not yet implemented.
    pub async fn upsert_file_hash(
        &self,
        _file_path: &str,
        _content_hash: &str,
        _size_bytes: u64,
    ) -> Result<(), EngramError> {
        Err(backend_err())
    }

    /// Stub — not yet implemented.
    pub async fn get_all_file_hashes(&self) -> Result<Vec<FileHashRecord>, EngramError> {
        Err(backend_err())
    }

    /// Stub — not yet implemented.
    pub async fn delete_file_hash_by_path(&self, _file_path: &str) -> Result<(), EngramError> {
        Err(backend_err())
    }
}
