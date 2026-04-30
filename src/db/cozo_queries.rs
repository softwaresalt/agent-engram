//! CozoDB query implementations — Phase 2–4 CRUD, edge, traversal, and vector operations.
//!
//! Provides the same public API as the SurrealDB `queries.rs` module so that
//! call sites compile under `--features cozo-backend` without modification.
//! Methods target the three-table vertical partition layout defined in
//! `cozo_backend::schema` for node types, plus the six edge tables added in
//! Phase 3 and the HNSW vector indexes added in Phase 4.

#![allow(clippy::unused_async)]
#![allow(clippy::missing_errors_doc)]

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use cozo::{DataValue, Num, ScriptMutability};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

use crate::db::cozo_backend::{CozoDb, map_db_err};
use crate::errors::{EngramError, SystemError};
use crate::models::code_edge::CodeEdgeType;

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

/// Result returned by [`CodeGraphQueries::reresolve_references_edges`].
///
/// Mirrors the type from the SurrealDB `queries` module for API compatibility.
/// `lookups` counts the initial batch round-trip only; per-edge fallback queries
/// are not included.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReresolveResult {
    /// Number of self-loop edges promoted to a resolved Class node.
    pub resolved: usize,
    /// Number of initial batch class-lookup database round-trips issued.
    ///
    /// Batch implementations emit ≤ 1 round-trip regardless of edge count.
    /// This value excludes any later per-edge fallback resolution queries.
    pub lookups: usize,
}

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

// ── Private helpers ───────────────────────────────────────────────────────

/// Compute a deterministic edge identifier from two node IDs.
///
/// Uses SHA-256(from + ":" + to) and returns the first 16 hex chars.
/// Visibility is `pub(crate)` so unit tests can call it directly.
#[allow(dead_code)]
pub(crate) fn edge_id(from: &str, to: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(from.as_bytes());
    hasher.update(b":");
    hasher.update(to.as_bytes());
    let result = hasher.finalize();
    hex::encode(&result[..8])
}

/// Build a canonical edge identifier from an edge type and its component parts.
///
/// Format: `"{edge_type}:{parts.join("|")}"`.
/// Used as the stable, human-readable ID for edge CRUD operations.
///
/// # Examples
/// ```
/// use engram::db::queries::derive_edge_id;
/// assert_eq!(derive_edge_id("calls", &["fn:a", "fn:b"]), "calls:fn:a|fn:b");
/// ```
pub fn derive_edge_id(edge_type: &str, parts: &[&str]) -> String {
    format!("{}:{}", edge_type, parts.join("|"))
}

/// Return the current UTC timestamp as an RFC 3339 string.
fn now_utc_str() -> String {
    Utc::now().to_rfc3339()
}

/// Compute cosine similarity between two equal-length f32 slices.
///
/// Returns 0.0 when either vector has zero magnitude.
#[allow(clippy::cast_precision_loss)]
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if mag_a == 0.0 || mag_b == 0.0 {
        return 0.0;
    }
    dot / (mag_a * mag_b)
}

// ── CodeGraphQueries ──────────────────────────────────────────────────────

/// CozoDB-backed `CodeGraphQueries`.
///
/// Fully implemented methods target the three-table vertical-partition layout:
/// `*_meta`, `*_code`, and `*_embedding` per symbol type.  Stub methods
/// return [`backend_err()`] and will be replaced in Phase 3+.
pub struct CodeGraphQueries {
    db: Arc<cozo::DbInstance>,
}

impl CodeGraphQueries {
    /// Create a new `CodeGraphQueries` backed by the given CozoDB handle.
    pub fn new(db: CozoDb) -> Self {
        Self { db: db.inner }
    }

    // ── code_file CRUD ─────────────────────────────────────────────

    /// Insert or replace a code file record.
    pub async fn upsert_code_file(
        &self,
        file: &crate::models::CodeFile,
    ) -> Result<(), EngramError> {
        let script = r#"
?[path, id, language, size_bytes, content_hash, last_indexed_at] <-
    [[$path, $id, $language, $size_bytes, $content_hash, $last_indexed_at]]
:put file_node { path, id, language, size_bytes, content_hash, last_indexed_at }
"#;
        let mut p = BTreeMap::new();
        p.insert("path".to_owned(), DataValue::from(file.path.as_str()));
        p.insert("id".to_owned(), DataValue::from(file.id.as_str()));
        p.insert(
            "language".to_owned(),
            DataValue::from(file.language.as_str()),
        );
        p.insert(
            "size_bytes".to_owned(),
            DataValue::Num(Num::Int(i64::try_from(file.size_bytes).unwrap_or(i64::MAX))),
        );
        p.insert(
            "content_hash".to_owned(),
            DataValue::from(file.content_hash.as_str()),
        );
        p.insert(
            "last_indexed_at".to_owned(),
            DataValue::from(file.last_indexed_at.as_str()),
        );
        self.db
            .run_script(script, p, ScriptMutability::Mutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        Ok(())
    }

    /// Look up a code file by its workspace-relative path.
    pub async fn get_code_file_by_path(
        &self,
        path: &str,
    ) -> Result<Option<crate::models::CodeFile>, EngramError> {
        let script = r#"
?[path, id, language, size_bytes, content_hash, last_indexed_at] :=
    *file_node { path, id, language, size_bytes, content_hash, last_indexed_at },
    path = $path
"#;
        let mut p = BTreeMap::new();
        p.insert("path".to_owned(), DataValue::from(path));
        let result = self
            .db
            .run_script(script, p, ScriptMutability::Immutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        if result.rows.is_empty() {
            return Ok(None);
        }
        let row = &result.rows[0];
        Ok(Some(crate::models::CodeFile {
            path: extract_str(row, 0),
            id: extract_str(row, 1),
            language: extract_str(row, 2),
            size_bytes: u64::try_from(extract_i64(row, 3).max(0)).unwrap_or(0),
            content_hash: extract_str(row, 4),
            last_indexed_at: extract_str(row, 5),
        }))
    }

    /// Delete a code file record by its workspace-relative path.
    pub async fn delete_code_file(&self, path: &str) -> Result<(), EngramError> {
        let script = r#"
?[path] <- [[$path]]
:rm file_node { path }
"#;
        let mut p = BTreeMap::new();
        p.insert("path".to_owned(), DataValue::from(path));
        self.db
            .run_script(script, p, ScriptMutability::Mutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        Ok(())
    }

    /// Stub — list all code files (not yet implemented).
    pub async fn list_code_files(&self) -> Result<Vec<crate::models::CodeFile>, EngramError> {
        let script = r#"
?[path, id, language, size_bytes, content_hash, last_indexed_at] :=
    *file_node { path, id, language, size_bytes, content_hash, last_indexed_at }
"#;
        let result = self
            .db
            .run_script(script, BTreeMap::new(), ScriptMutability::Immutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        Ok(result
            .rows
            .iter()
            .map(|r| crate::models::CodeFile {
                path: extract_str(r, 0),
                id: extract_str(r, 1),
                language: extract_str(r, 2),
                size_bytes: u64::try_from(extract_i64(r, 3).max(0)).unwrap_or(0),
                content_hash: extract_str(r, 4),
                last_indexed_at: extract_str(r, 5),
            })
            .collect())
    }

    // ── Bulk read helpers ─────────────────────────────────────────

    /// List all function records.
    pub async fn all_functions(&self) -> Result<Vec<crate::models::Function>, EngramError> {
        let script = r#"
?[id, name, file_path, line_start, line_end, signature, docstring, body_hash,
  token_count, embed_type, summary, body, embedding] :=
    *function_meta { id, name, file_path, line_start, line_end, signature,
                     docstring, body_hash, token_count, embed_type, summary },
    *function_code { id, body },
    *function_embedding { id, embedding }
"#;
        let result = self
            .db
            .run_script(script, BTreeMap::new(), ScriptMutability::Immutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        Ok(result.rows.iter().map(|r| row_to_function(r)).collect())
    }

    /// List all class records.
    pub async fn all_classes(&self) -> Result<Vec<crate::models::Class>, EngramError> {
        let script = r#"
?[id, name, file_path, line_start, line_end, docstring, body_hash,
  token_count, embed_type, summary, body, embedding] :=
    *class_meta { id, name, file_path, line_start, line_end, docstring,
                  body_hash, token_count, embed_type, summary },
    *class_code { id, body },
    *class_embedding { id, embedding }
"#;
        let result = self
            .db
            .run_script(script, BTreeMap::new(), ScriptMutability::Immutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        Ok(result.rows.iter().map(|r| row_to_class(r)).collect())
    }

    /// List all interface records.
    pub async fn all_interfaces(&self) -> Result<Vec<crate::models::Interface>, EngramError> {
        let script = r#"
?[id, name, file_path, line_start, line_end, docstring, body_hash,
  token_count, embed_type, summary, body, embedding] :=
    *interface_meta { id, name, file_path, line_start, line_end, docstring,
                      body_hash, token_count, embed_type, summary },
    *interface_code { id, body },
    *interface_embedding { id, embedding }
"#;
        let result = self
            .db
            .run_script(script, BTreeMap::new(), ScriptMutability::Immutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        Ok(result.rows.iter().map(|r| row_to_interface(r)).collect())
    }

    /// List all code edges from all edge tables (excludes `references_edge`).
    pub async fn all_code_edges(&self) -> Result<Vec<crate::models::CodeEdge>, EngramError> {
        let mut edges = Vec::new();
        for (kind, table) in &[
            ("calls", "calls_edge"),
            ("imports", "imports_edge"),
            ("defines", "defines_edge"),
            ("inherits_from", "inherits_from_edge"),
            ("concerns", "concerns_edge"),
        ] {
            edges.extend(self.edges_from_table(kind, table).await?);
        }
        Ok(edges)
    }

    // ── function CRUD ─────────────────────────────────────────────

    /// Insert or replace a function record across all three tables.
    pub async fn upsert_function(&self, func: &crate::models::Function) -> Result<(), EngramError> {
        // Table 1: function_meta
        let meta_script = r#"
?[id, name, file_path, line_start, line_end, signature, docstring, body_hash, token_count, embed_type, summary] <-
    [[$id, $name, $file_path, $line_start, $line_end, $signature, $docstring, $body_hash, $token_count, $embed_type, $summary]]
:put function_meta { id, name, file_path, line_start, line_end, signature, docstring, body_hash, token_count, embed_type, summary }
"#;
        let mut p = BTreeMap::new();
        p.insert("id".to_owned(), DataValue::from(func.id.as_str()));
        p.insert("name".to_owned(), DataValue::from(func.name.as_str()));
        p.insert(
            "file_path".to_owned(),
            DataValue::from(func.file_path.as_str()),
        );
        p.insert(
            "line_start".to_owned(),
            DataValue::Num(Num::Int(i64::from(func.line_start))),
        );
        p.insert(
            "line_end".to_owned(),
            DataValue::Num(Num::Int(i64::from(func.line_end))),
        );
        p.insert(
            "signature".to_owned(),
            DataValue::from(func.signature.as_str()),
        );
        p.insert(
            "docstring".to_owned(),
            DataValue::from(func.docstring.as_deref().unwrap_or("")),
        );
        p.insert(
            "body_hash".to_owned(),
            DataValue::from(func.body_hash.as_str()),
        );
        p.insert(
            "token_count".to_owned(),
            DataValue::Num(Num::Int(i64::from(func.token_count))),
        );
        p.insert(
            "embed_type".to_owned(),
            DataValue::from(func.embed_type.as_str()),
        );
        p.insert("summary".to_owned(), DataValue::from(func.summary.as_str()));
        self.db
            .run_script(meta_script, p, ScriptMutability::Mutable)
            .map_err(|e| map_db_err(e.to_string()))?;

        // Table 2: function_code
        let code_script = r#"
?[id, body] <- [[$id, $body]]
:put function_code { id, body }
"#;
        let mut p2 = BTreeMap::new();
        p2.insert("id".to_owned(), DataValue::from(func.id.as_str()));
        p2.insert("body".to_owned(), DataValue::from(func.body.as_str()));
        self.db
            .run_script(code_script, p2, ScriptMutability::Mutable)
            .map_err(|e| map_db_err(e.to_string()))?;

        // Table 3: function_embedding
        let embed_script = r#"
?[id, embedding] <- [[$id, $embedding]]
:put function_embedding { id, embedding }
"#;
        let embedding_dv = DataValue::List(
            func.embedding
                .iter()
                .map(|&f| DataValue::Num(Num::Float(f64::from(f))))
                .collect(),
        );
        let mut p3 = BTreeMap::new();
        p3.insert("id".to_owned(), DataValue::from(func.id.as_str()));
        p3.insert("embedding".to_owned(), embedding_dv);
        self.db
            .run_script(embed_script, p3, ScriptMutability::Mutable)
            .map_err(|e| map_db_err(e.to_string()))?;

        Ok(())
    }

    /// Look up a function by name (first match).
    pub async fn get_function_by_name(
        &self,
        name: &str,
    ) -> Result<Option<crate::models::Function>, EngramError> {
        let script = r#"
?[id, name, file_path, line_start, line_end, signature, docstring, body_hash, token_count, embed_type, summary, body, embedding] :=
    *function_meta { id, name, file_path, line_start, line_end, signature, docstring, body_hash, token_count, embed_type, summary },
    name = $name,
    *function_code { id, body },
    *function_embedding { id, embedding }
"#;
        let mut p = BTreeMap::new();
        p.insert("name".to_owned(), DataValue::from(name));
        let result = self
            .db
            .run_script(script, p, ScriptMutability::Immutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        if result.rows.is_empty() {
            return Ok(None);
        }
        Ok(Some(row_to_function(&result.rows[0])))
    }

    /// Return all functions belonging to the given file.
    pub async fn get_functions_by_file(
        &self,
        file_path: &str,
    ) -> Result<Vec<crate::models::Function>, EngramError> {
        let script = r#"
?[id, name, file_path, line_start, line_end, signature, docstring, body_hash, token_count, embed_type, summary, body, embedding] :=
    *function_meta { id, name, file_path, line_start, line_end, signature, docstring, body_hash, token_count, embed_type, summary },
    file_path = $fp,
    *function_code { id, body },
    *function_embedding { id, embedding }
"#;
        let mut p = BTreeMap::new();
        p.insert("fp".to_owned(), DataValue::from(file_path));
        let result = self
            .db
            .run_script(script, p, ScriptMutability::Immutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        Ok(result.rows.iter().map(|r| row_to_function(r)).collect())
    }

    /// Delete all functions (meta, code, and embedding) belonging to the given file.
    pub async fn delete_functions_by_file(&self, file_path: &str) -> Result<(), EngramError> {
        let mut p = BTreeMap::new();
        p.insert("fp".to_owned(), DataValue::from(file_path));

        // Delete code rows first (while meta still present for the join)
        let del_code = r#"
?[id] := *function_meta { id, file_path: $fp }, *function_code { id }
:rm function_code { id }
"#;
        self.db
            .run_script(del_code, p.clone(), ScriptMutability::Mutable)
            .map_err(|e| map_db_err(e.to_string()))?;

        // Delete embedding rows
        let del_embed = r#"
?[id] := *function_meta { id, file_path: $fp }, *function_embedding { id }
:rm function_embedding { id }
"#;
        self.db
            .run_script(del_embed, p.clone(), ScriptMutability::Mutable)
            .map_err(|e| map_db_err(e.to_string()))?;

        // Finally delete meta rows
        let del_meta = r#"
?[id] := *function_meta { id, file_path: $fp }
:rm function_meta { id }
"#;
        self.db
            .run_script(del_meta, p, ScriptMutability::Mutable)
            .map_err(|e| map_db_err(e.to_string()))?;

        Ok(())
    }

    // ── class CRUD ─────────────────────────────────────────────────

    /// Insert or replace a class record across all three tables.
    pub async fn upsert_class(&self, class: &crate::models::Class) -> Result<(), EngramError> {
        // Table 1: class_meta
        let meta = r#"
?[id, name, file_path, line_start, line_end, docstring, body_hash, token_count, embed_type, summary] <-
    [[$id, $name, $file_path, $line_start, $line_end, $docstring, $body_hash, $token_count, $embed_type, $summary]]
:put class_meta { id, name, file_path, line_start, line_end, docstring, body_hash, token_count, embed_type, summary }
"#;
        let mut p = BTreeMap::new();
        p.insert("id".to_owned(), DataValue::from(class.id.as_str()));
        p.insert("name".to_owned(), DataValue::from(class.name.as_str()));
        p.insert(
            "file_path".to_owned(),
            DataValue::from(class.file_path.as_str()),
        );
        p.insert(
            "line_start".to_owned(),
            DataValue::Num(Num::Int(i64::from(class.line_start))),
        );
        p.insert(
            "line_end".to_owned(),
            DataValue::Num(Num::Int(i64::from(class.line_end))),
        );
        p.insert(
            "docstring".to_owned(),
            DataValue::from(class.docstring.as_deref().unwrap_or("")),
        );
        p.insert(
            "body_hash".to_owned(),
            DataValue::from(class.body_hash.as_str()),
        );
        p.insert(
            "token_count".to_owned(),
            DataValue::Num(Num::Int(i64::from(class.token_count))),
        );
        p.insert(
            "embed_type".to_owned(),
            DataValue::from(class.embed_type.as_str()),
        );
        p.insert(
            "summary".to_owned(),
            DataValue::from(class.summary.as_str()),
        );
        self.db
            .run_script(meta, p, ScriptMutability::Mutable)
            .map_err(|e| map_db_err(e.to_string()))?;

        // Table 2: class_code
        let code_s = r#"
?[id, body] <- [[$id, $body]]
:put class_code { id, body }
"#;
        let mut p2 = BTreeMap::new();
        p2.insert("id".to_owned(), DataValue::from(class.id.as_str()));
        p2.insert("body".to_owned(), DataValue::from(class.body.as_str()));
        self.db
            .run_script(code_s, p2, ScriptMutability::Mutable)
            .map_err(|e| map_db_err(e.to_string()))?;

        // Table 3: class_embedding
        let embed_s = r#"
?[id, embedding] <- [[$id, $embedding]]
:put class_embedding { id, embedding }
"#;
        let emb_dv = DataValue::List(
            class
                .embedding
                .iter()
                .map(|&f| DataValue::Num(Num::Float(f64::from(f))))
                .collect(),
        );
        let mut p3 = BTreeMap::new();
        p3.insert("id".to_owned(), DataValue::from(class.id.as_str()));
        p3.insert("embedding".to_owned(), emb_dv);
        self.db
            .run_script(embed_s, p3, ScriptMutability::Mutable)
            .map_err(|e| map_db_err(e.to_string()))?;

        Ok(())
    }

    /// Look up a class by name (first match).
    pub async fn get_class_by_name(
        &self,
        name: &str,
    ) -> Result<Option<crate::models::Class>, EngramError> {
        let script = r#"
?[id, name, file_path, line_start, line_end, docstring, body_hash, token_count, embed_type, summary, body, embedding] :=
    *class_meta { id, name, file_path, line_start, line_end, docstring, body_hash, token_count, embed_type, summary },
    name = $name,
    *class_code { id, body },
    *class_embedding { id, embedding }
"#;
        let mut p = BTreeMap::new();
        p.insert("name".to_owned(), DataValue::from(name));
        let result = self
            .db
            .run_script(script, p, ScriptMutability::Immutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        if result.rows.is_empty() {
            return Ok(None);
        }
        Ok(Some(row_to_class(&result.rows[0])))
    }

    /// Delete all class records for a given file path.
    pub async fn delete_classes_by_file(&self, file_path: &str) -> Result<(), EngramError> {
        // Get IDs first, then delete from all 3 tables.
        let s = r#"
?[id] := *class_meta { id, file_path }, file_path = $file_path
"#;
        let mut p = BTreeMap::new();
        p.insert("file_path".to_owned(), DataValue::from(file_path));
        let ids = self
            .db
            .run_script(s, p, ScriptMutability::Immutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        for row in &ids.rows {
            let id = extract_str(row, 0);
            for del in &[
                r#"?[id] <- [[$id]] :rm class_meta { id }"#,
                r#"?[id] <- [[$id]] :rm class_code { id }"#,
                r#"?[id] <- [[$id]] :rm class_embedding { id }"#,
            ] {
                let mut dp = BTreeMap::new();
                dp.insert("id".to_owned(), DataValue::from(id.as_str()));
                self.db
                    .run_script(del, dp, ScriptMutability::Mutable)
                    .map_err(|e| map_db_err(e.to_string()))?;
            }
        }
        Ok(())
    }

    // ── interface CRUD ────────────────────────────────────────────

    /// Insert or replace an interface record across all three tables.
    pub async fn upsert_interface(
        &self,
        iface: &crate::models::Interface,
    ) -> Result<(), EngramError> {
        // Table 1: interface_meta
        let meta = r#"
?[id, name, file_path, line_start, line_end, docstring, body_hash, token_count, embed_type, summary] <-
    [[$id, $name, $file_path, $line_start, $line_end, $docstring, $body_hash, $token_count, $embed_type, $summary]]
:put interface_meta { id, name, file_path, line_start, line_end, docstring, body_hash, token_count, embed_type, summary }
"#;
        let mut p = BTreeMap::new();
        p.insert("id".to_owned(), DataValue::from(iface.id.as_str()));
        p.insert("name".to_owned(), DataValue::from(iface.name.as_str()));
        p.insert(
            "file_path".to_owned(),
            DataValue::from(iface.file_path.as_str()),
        );
        p.insert(
            "line_start".to_owned(),
            DataValue::Num(Num::Int(i64::from(iface.line_start))),
        );
        p.insert(
            "line_end".to_owned(),
            DataValue::Num(Num::Int(i64::from(iface.line_end))),
        );
        p.insert(
            "docstring".to_owned(),
            DataValue::from(iface.docstring.as_deref().unwrap_or("")),
        );
        p.insert(
            "body_hash".to_owned(),
            DataValue::from(iface.body_hash.as_str()),
        );
        p.insert(
            "token_count".to_owned(),
            DataValue::Num(Num::Int(i64::from(iface.token_count))),
        );
        p.insert(
            "embed_type".to_owned(),
            DataValue::from(iface.embed_type.as_str()),
        );
        p.insert(
            "summary".to_owned(),
            DataValue::from(iface.summary.as_str()),
        );
        self.db
            .run_script(meta, p, ScriptMutability::Mutable)
            .map_err(|e| map_db_err(e.to_string()))?;

        // Table 2: interface_code
        let code_s = r#"
?[id, body] <- [[$id, $body]]
:put interface_code { id, body }
"#;
        let mut p2 = BTreeMap::new();
        p2.insert("id".to_owned(), DataValue::from(iface.id.as_str()));
        p2.insert("body".to_owned(), DataValue::from(iface.body.as_str()));
        self.db
            .run_script(code_s, p2, ScriptMutability::Mutable)
            .map_err(|e| map_db_err(e.to_string()))?;

        // Table 3: interface_embedding
        let embed_s = r#"
?[id, embedding] <- [[$id, $embedding]]
:put interface_embedding { id, embedding }
"#;
        let emb_dv = DataValue::List(
            iface
                .embedding
                .iter()
                .map(|&f| DataValue::Num(Num::Float(f64::from(f))))
                .collect(),
        );
        let mut p3 = BTreeMap::new();
        p3.insert("id".to_owned(), DataValue::from(iface.id.as_str()));
        p3.insert("embedding".to_owned(), emb_dv);
        self.db
            .run_script(embed_s, p3, ScriptMutability::Mutable)
            .map_err(|e| map_db_err(e.to_string()))?;

        Ok(())
    }

    /// Look up an interface by name (first match).
    pub async fn get_interface_by_name(
        &self,
        name: &str,
    ) -> Result<Option<crate::models::Interface>, EngramError> {
        let script = r#"
?[id, name, file_path, line_start, line_end, docstring, body_hash, token_count, embed_type, summary, body, embedding] :=
    *interface_meta { id, name, file_path, line_start, line_end, docstring, body_hash, token_count, embed_type, summary },
    name = $name,
    *interface_code { id, body },
    *interface_embedding { id, embedding }
"#;
        let mut p = BTreeMap::new();
        p.insert("name".to_owned(), DataValue::from(name));
        let result = self
            .db
            .run_script(script, p, ScriptMutability::Immutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        if result.rows.is_empty() {
            return Ok(None);
        }
        Ok(Some(row_to_interface(&result.rows[0])))
    }

    /// Delete all interface records for a given file path.
    pub async fn delete_interfaces_by_file(&self, file_path: &str) -> Result<(), EngramError> {
        let s = r#"
?[id] := *interface_meta { id, file_path }, file_path = $file_path
"#;
        let mut p = BTreeMap::new();
        p.insert("file_path".to_owned(), DataValue::from(file_path));
        let ids = self
            .db
            .run_script(s, p, ScriptMutability::Immutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        for row in &ids.rows {
            let id = extract_str(row, 0);
            for del in &[
                r#"?[id] <- [[$id]] :rm interface_meta { id }"#,
                r#"?[id] <- [[$id]] :rm interface_code { id }"#,
                r#"?[id] <- [[$id]] :rm interface_embedding { id }"#,
            ] {
                let mut dp = BTreeMap::new();
                dp.insert("id".to_owned(), DataValue::from(id.as_str()));
                self.db
                    .run_script(del, dp, ScriptMutability::Mutable)
                    .map_err(|e| map_db_err(e.to_string()))?;
            }
        }
        Ok(())
    }

    // ── Edge CRUD ──────────────────────────────────────────────────

    /// Upsert a function-to-function call edge.
    #[allow(clippy::similar_names)]
    pub async fn create_calls_edge(
        &self,
        caller_id: &str,
        callee_id: &str,
    ) -> Result<(), EngramError> {
        let ts = now_utc_str();
        let script = r#"
?[from, to, created_at] <- [[$from, $to, $created_at]]
:put calls_edge { from, to => created_at }
"#;
        let mut p = BTreeMap::new();
        p.insert("from".to_owned(), DataValue::from(caller_id));
        p.insert("to".to_owned(), DataValue::from(callee_id));
        p.insert("created_at".to_owned(), DataValue::from(ts.as_str()));
        self.db
            .run_script(script, p, ScriptMutability::Mutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        Ok(())
    }

    /// Upsert a file-to-file import edge.
    #[allow(clippy::similar_names)]
    pub async fn create_imports_edge(
        &self,
        importer_id: &str,
        imported_id: &str,
        import_path: &str,
    ) -> Result<(), EngramError> {
        let ts = now_utc_str();
        let script = r#"
?[from, to, import_path, created_at] <- [[$from, $to, $import_path, $created_at]]
:put imports_edge { from, to, import_path => created_at }
"#;
        let mut p = BTreeMap::new();
        p.insert("from".to_owned(), DataValue::from(importer_id));
        p.insert("to".to_owned(), DataValue::from(imported_id));
        p.insert("import_path".to_owned(), DataValue::from(import_path));
        p.insert("created_at".to_owned(), DataValue::from(ts.as_str()));
        self.db
            .run_script(script, p, ScriptMutability::Mutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        Ok(())
    }

    /// Upsert a file-to-symbol defines edge.
    pub async fn create_defines_edge(
        &self,
        file_id: &str,
        symbol_table: &str,
        symbol_id: &str,
    ) -> Result<(), EngramError> {
        let ts = now_utc_str();
        let script = r#"
?[from, to, symbol_table, created_at] <- [[$from, $to, $symbol_table, $created_at]]
:put defines_edge { from, to => symbol_table, created_at }
"#;
        let mut p = BTreeMap::new();
        p.insert("from".to_owned(), DataValue::from(file_id));
        p.insert("to".to_owned(), DataValue::from(symbol_id));
        p.insert("symbol_table".to_owned(), DataValue::from(symbol_table));
        p.insert("created_at".to_owned(), DataValue::from(ts.as_str()));
        self.db
            .run_script(script, p, ScriptMutability::Mutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        Ok(())
    }

    /// Upsert a class/interface inheritance edge.
    pub async fn create_inherits_edge(
        &self,
        child_table: &str,
        child_id: &str,
        parent_table: &str,
        parent_id: &str,
    ) -> Result<(), EngramError> {
        let ts = now_utc_str();
        let script = r#"
?[from, to, from_table, to_table, created_at] <-
    [[$from, $to, $from_table, $to_table, $created_at]]
:put inherits_from_edge { from, to => from_table, to_table, created_at }
"#;
        let mut p = BTreeMap::new();
        p.insert("from".to_owned(), DataValue::from(child_id));
        p.insert("to".to_owned(), DataValue::from(parent_id));
        p.insert("from_table".to_owned(), DataValue::from(child_table));
        p.insert("to_table".to_owned(), DataValue::from(parent_table));
        p.insert("created_at".to_owned(), DataValue::from(ts.as_str()));
        self.db
            .run_script(script, p, ScriptMutability::Mutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        Ok(())
    }

    /// Upsert a task-to-symbol concerns edge.
    pub async fn create_concerns_edge(
        &self,
        task_id: &str,
        symbol_table: &str,
        symbol_id: &str,
        linked_by: &str,
    ) -> Result<(), EngramError> {
        let ts = now_utc_str();
        let script = r#"
?[task_id, symbol_id, symbol_table, linked_by, created_at] <-
    [[$task_id, $symbol_id, $symbol_table, $linked_by, $created_at]]
:put concerns_edge { task_id, symbol_id => symbol_table, linked_by, created_at }
"#;
        let mut p = BTreeMap::new();
        p.insert("task_id".to_owned(), DataValue::from(task_id));
        p.insert("symbol_id".to_owned(), DataValue::from(symbol_id));
        p.insert("symbol_table".to_owned(), DataValue::from(symbol_table));
        p.insert("linked_by".to_owned(), DataValue::from(linked_by));
        p.insert("created_at".to_owned(), DataValue::from(ts.as_str()));
        self.db
            .run_script(script, p, ScriptMutability::Mutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        Ok(())
    }

    /// Upsert a qualified-name reference edge.
    pub async fn create_references_edge(
        &self,
        source_id: &str,
        target_id: &str,
        qualified_name: Option<&str>,
    ) -> Result<(), EngramError> {
        let ts = now_utc_str();
        let qn = qualified_name.unwrap_or("");
        let script = r#"
?[from, to, qualified_name, created_at] <-
    [[$from, $to, $qualified_name, $created_at]]
:put references_edge { from, to, qualified_name => created_at }
"#;
        let mut p = BTreeMap::new();
        p.insert("from".to_owned(), DataValue::from(source_id));
        p.insert("to".to_owned(), DataValue::from(target_id));
        p.insert("qualified_name".to_owned(), DataValue::from(qn));
        p.insert("created_at".to_owned(), DataValue::from(ts.as_str()));
        self.db
            .run_script(script, p, ScriptMutability::Mutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        Ok(())
    }

    /// Return an empty `ReresolveResult`.
    ///
    /// The SurrealDB backend uses this to fix self-loop `references` edges produced
    /// when a symbol references itself before it has been assigned its final ID.
    /// In the CozoDB backend, `references_edge` includes `qualified_name` as part of
    /// its composite key, which prevents such self-loops from colliding — so there is
    /// nothing to re-resolve. Returning zero counts is correct behavior here.
    pub async fn reresolve_references_edges(&self) -> Result<ReresolveResult, EngramError> {
        Ok(ReresolveResult {
            resolved: 0,
            lookups: 0,
        })
    }

    /// Look up a class ID by case-insensitive name match.
    pub async fn get_class_by_name_ci(&self, name: &str) -> Result<Option<String>, EngramError> {
        let name_lower = name.to_lowercase();
        let script = r#"
?[id, name] := *class_meta { id, name }
"#;
        let result = self
            .db
            .run_script(script, BTreeMap::new(), ScriptMutability::Immutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        Ok(result.rows.iter().find_map(|r| {
            let n = extract_str(r, 1);
            if n.to_lowercase() == name_lower {
                Some(extract_str(r, 0))
            } else {
                None
            }
        }))
    }

    /// Attempt to resolve a qualified name to a symbol ID.
    ///
    /// Searches function, class, and interface meta tables for a name match.
    pub async fn resolve_reference_target(
        &self,
        qualified_name: &str,
    ) -> Result<Option<String>, EngramError> {
        // Check functions.
        let script = r#"?[id] := *function_meta { id, name }, name = $name"#;
        let mut p = BTreeMap::new();
        p.insert("name".to_owned(), DataValue::from(qualified_name));
        let r = self
            .db
            .run_script(script, p.clone(), ScriptMutability::Immutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        if !r.rows.is_empty() {
            return Ok(Some(extract_str(&r.rows[0], 0)));
        }
        // Check classes.
        let script2 = r#"?[id] := *class_meta { id, name }, name = $name"#;
        let r2 = self
            .db
            .run_script(script2, p.clone(), ScriptMutability::Immutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        if !r2.rows.is_empty() {
            return Ok(Some(extract_str(&r2.rows[0], 0)));
        }
        // Check interfaces.
        let script3 = r#"?[id] := *interface_meta { id, name }, name = $name"#;
        let r3 = self
            .db
            .run_script(script3, p, ScriptMutability::Immutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        if !r3.rows.is_empty() {
            return Ok(Some(extract_str(&r3.rows[0], 0)));
        }
        Ok(None)
    }

    /// Delete all outgoing edges from `file_id` in the given edge table.
    ///
    /// `edge_table` must be one of: `calls`, `imports`, `defines`, `inherits_from`, `references`.
    pub async fn delete_edges_from_file(
        &self,
        edge_table: &str,
        file_id: &str,
    ) -> Result<(), EngramError> {
        let table_name = match edge_table {
            "calls" => "calls_edge",
            "imports" => "imports_edge",
            "defines" => "defines_edge",
            "inherits_from" => "inherits_from_edge",
            "references" => "references_edge",
            other => {
                return Err(EngramError::from(SystemError::DatabaseError {
                    reason: format!("unknown edge table: {other}"),
                }));
            }
        };
        // Query the key columns for rows where from = file_id, then delete.
        self.delete_outgoing_edges(table_name, file_id).await
    }

    /// Delete all edges touching any symbol that belongs to `file_path`.
    pub async fn clear_file_graph(&self, file_path: &str) -> Result<(), EngramError> {
        // Collect all symbol IDs for this file across all symbol tables.
        let mut symbol_ids: Vec<String> = Vec::new();
        for table in &["function_meta", "class_meta", "interface_meta"] {
            let script = format!("?[id] := *{table} {{ id, file_path }}, file_path = $file_path");
            let mut p = BTreeMap::new();
            p.insert("file_path".to_owned(), DataValue::from(file_path));
            let r = self
                .db
                .run_script(&script, p, ScriptMutability::Immutable)
                .map_err(|e| map_db_err(e.to_string()))?;
            for row in &r.rows {
                symbol_ids.push(extract_str(row, 0));
            }
        }

        // Look up the file node ID (keyed by path) so we can clear file-level edges.
        let file_node_id = {
            let script = r#"?[id] := *file_node { path, id }, path = $path"#;
            let mut p = BTreeMap::new();
            p.insert("path".to_owned(), DataValue::from(file_path));
            let r = self
                .db
                .run_script(script, p, ScriptMutability::Immutable)
                .map_err(|e| map_db_err(e.to_string()))?;
            r.rows.first().map(|row| extract_str(row, 0))
        };

        // Build the full set of node IDs whose edges need clearing.
        let mut all_ids = symbol_ids.clone();
        if let Some(ref fid) = file_node_id {
            all_ids.push(fid.clone());
        }

        // Delete all edges touching these nodes (propagate errors).
        for sid in &all_ids {
            for et in &[
                "calls_edge",
                "imports_edge",
                "defines_edge",
                "inherits_from_edge",
                "references_edge",
                "concerns_edge",
            ] {
                self.delete_outgoing_edges(et, sid).await?;
                self.delete_incoming_edges(et, sid).await?;
            }
        }

        // Delete symbol rows for all symbols belonging to this file.
        for sid in &symbol_ids {
            let prefix = sid.split(':').next().unwrap_or("");
            let tables: Option<(&str, &str, &str)> = match prefix {
                "fn" | "function" => Some(("function_meta", "function_code", "function_embedding")),
                "class" => Some(("class_meta", "class_code", "class_embedding")),
                "iface" | "interface" => {
                    Some(("interface_meta", "interface_code", "interface_embedding"))
                }
                _ => None,
            };
            if let Some((meta_tbl, code_tbl, embed_tbl)) = tables {
                for tbl in &[meta_tbl, code_tbl, embed_tbl] {
                    let del = format!("?[id] <- [[$id]] :rm {tbl} {{ id }}");
                    let mut p = BTreeMap::new();
                    p.insert("id".to_owned(), DataValue::from(sid.as_str()));
                    self.db
                        .run_script(&del, p, ScriptMutability::Mutable)
                        .map_err(|e| map_db_err(e.to_string()))?;
                }
            }
        }

        Ok(())
    }

    // ── Concerns edge management ──────────────────────────────────

    /// Return all concerns edges where the target symbol lives in the given file.
    pub async fn get_concerns_edges_for_file(
        &self,
        file_path: &str,
    ) -> Result<Vec<ConcernsEdgeInfo>, EngramError> {
        let mut out = Vec::new();
        for (tbl, meta_tbl) in &[
            ("function", "function_meta"),
            ("class", "class_meta"),
            ("interface", "interface_meta"),
        ] {
            let script = format!(
                r#"?[task_id, symbol_id, symbol_table, linked_by, symbol_name, body_hash] :=
    *concerns_edge {{ task_id, symbol_id, symbol_table, linked_by }},
    symbol_table = "{tbl}",
    *{meta_tbl} {{ id: symbol_id, name: symbol_name, file_path: fp, body_hash }},
    fp = $file_path"#
            );
            let mut p = BTreeMap::new();
            p.insert("file_path".to_owned(), DataValue::from(file_path));
            let r = self
                .db
                .run_script(&script, p, ScriptMutability::Immutable)
                .map_err(|e| map_db_err(e.to_string()))?;
            for row in &r.rows {
                out.push(ConcernsEdgeInfo {
                    task_id: extract_str(row, 0),
                    symbol_table: extract_str(row, 2),
                    symbol_id: extract_str(row, 1),
                    symbol_name: extract_str(row, 4),
                    symbol_body_hash: extract_str(row, 5),
                    linked_by: extract_str(row, 3),
                });
            }
        }
        Ok(out)
    }

    /// Delete all concerns edges pointing to a given symbol.
    pub async fn delete_concerns_edges_for_symbol(
        &self,
        symbol_table: &str,
        symbol_id: &str,
    ) -> Result<usize, EngramError> {
        let select_script = r#"
?[task_id, symbol_id] :=
    *concerns_edge { task_id, symbol_id, symbol_table },
    symbol_table = $symbol_table,
    symbol_id = $symbol_id
"#;
        let mut p = BTreeMap::new();
        p.insert("symbol_table".to_owned(), DataValue::from(symbol_table));
        p.insert("symbol_id".to_owned(), DataValue::from(symbol_id));
        let rows = self
            .db
            .run_script(select_script, p.clone(), ScriptMutability::Immutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        let count = rows.rows.len();
        if count == 0 {
            return Ok(0);
        }
        let del_script = r#"
?[task_id, symbol_id] :=
    *concerns_edge { task_id, symbol_id, symbol_table },
    symbol_table = $symbol_table,
    symbol_id = $symbol_id
:rm concerns_edge { task_id, symbol_id }
"#;
        self.db
            .run_script(del_script, p, ScriptMutability::Mutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        Ok(count)
    }

    /// Find symbols matching a name and body hash across all symbol tables.
    pub async fn find_symbols_by_name_and_hash(
        &self,
        name: &str,
        body_hash: &str,
    ) -> Result<Vec<SymbolIdentity>, EngramError> {
        let mut out = Vec::new();
        for (tbl, meta_tbl, embed_tbl) in &[
            ("function", "function_meta", "function_embedding"),
            ("class", "class_meta", "class_embedding"),
            ("interface", "interface_meta", "interface_embedding"),
        ] {
            let script = format!(
                r#"?[id, name, file_path, body_hash, embedding] :=
    *{meta_tbl} {{ id, name, file_path, body_hash }},
    name = $name, body_hash = $body_hash,
    *{embed_tbl} {{ id, embedding }}"#
            );
            let mut p = BTreeMap::new();
            p.insert("name".to_owned(), DataValue::from(name));
            p.insert("body_hash".to_owned(), DataValue::from(body_hash));
            let r = self
                .db
                .run_script(&script, p, ScriptMutability::Immutable)
                .map_err(|e| map_db_err(e.to_string()))?;
            for row in &r.rows {
                out.push(SymbolIdentity {
                    table: (*tbl).to_owned(),
                    id: extract_str(row, 0),
                    name: extract_str(row, 1),
                    file_path: extract_str(row, 2),
                    body_hash: extract_str(row, 3),
                    embedding: extract_embedding(row, 4),
                });
            }
        }
        Ok(out)
    }

    /// Return all symbol identities (across all types) for a given file.
    pub async fn get_symbol_identities_for_file(
        &self,
        file_path: &str,
    ) -> Result<Vec<SymbolIdentity>, EngramError> {
        let mut out = Vec::new();
        for (tbl, meta_tbl, embed_tbl) in &[
            ("function", "function_meta", "function_embedding"),
            ("class", "class_meta", "class_embedding"),
            ("interface", "interface_meta", "interface_embedding"),
        ] {
            let script = format!(
                r#"?[id, name, file_path, body_hash, embedding] :=
    *{meta_tbl} {{ id, name, file_path, body_hash }},
    file_path = $file_path,
    *{embed_tbl} {{ id, embedding }}"#
            );
            let mut p = BTreeMap::new();
            p.insert("file_path".to_owned(), DataValue::from(file_path));
            let r = self
                .db
                .run_script(&script, p, ScriptMutability::Immutable)
                .map_err(|e| map_db_err(e.to_string()))?;
            for row in &r.rows {
                out.push(SymbolIdentity {
                    table: (*tbl).to_owned(),
                    id: extract_str(row, 0),
                    name: extract_str(row, 1),
                    file_path: extract_str(row, 2),
                    body_hash: extract_str(row, 3),
                    embedding: extract_embedding(row, 4),
                });
            }
        }
        Ok(out)
    }

    /// Check whether a specific concerns edge already exists.
    pub async fn concerns_edge_exists(
        &self,
        task_id: &str,
        symbol_table: &str,
        symbol_id: &str,
    ) -> Result<bool, EngramError> {
        let script = r#"
?[count(task_id)] :=
    *concerns_edge { task_id, symbol_id, symbol_table },
    task_id = $task_id,
    symbol_id = $symbol_id,
    symbol_table = $symbol_table
"#;
        let mut p = BTreeMap::new();
        p.insert("task_id".to_owned(), DataValue::from(task_id));
        p.insert("symbol_id".to_owned(), DataValue::from(symbol_id));
        p.insert("symbol_table".to_owned(), DataValue::from(symbol_table));
        let r = self
            .db
            .run_script(script, p, ScriptMutability::Immutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        Ok(extract_count(&r) > 0)
    }

    /// Delete concerns edges for a task whose target symbol matches the given name.
    pub async fn delete_concerns_by_task_and_symbol_name(
        &self,
        task_id: &str,
        symbol_name: &str,
    ) -> Result<usize, EngramError> {
        let mut deleted = 0usize;
        for (tbl, meta_tbl) in &[
            ("function", "function_meta"),
            ("class", "class_meta"),
            ("interface", "interface_meta"),
        ] {
            // COUNT first, then delete — CozoDB :rm does not reliably return
            // one row per deleted record; use a separate SELECT for the count.
            let count_script = format!(
                r#"?[task_id, symbol_id] :=
    *concerns_edge {{ task_id, symbol_id, symbol_table }},
    task_id = $task_id,
    symbol_table = "{tbl}",
    *{meta_tbl} {{ id: symbol_id, name }},
    name = $name"#
            );
            let mut p = BTreeMap::new();
            p.insert("task_id".to_owned(), DataValue::from(task_id));
            p.insert("name".to_owned(), DataValue::from(symbol_name));
            let count_r = self
                .db
                .run_script(&count_script, p.clone(), ScriptMutability::Immutable)
                .map_err(|e| map_db_err(e.to_string()))?;
            let count = count_r.rows.len();
            if count == 0 {
                continue;
            }
            let del_script = format!(
                r#"?[task_id, symbol_id] :=
    *concerns_edge {{ task_id, symbol_id, symbol_table }},
    task_id = $task_id,
    symbol_table = "{tbl}",
    *{meta_tbl} {{ id: symbol_id, name }},
    name = $name
:rm concerns_edge {{ task_id, symbol_id }}"#
            );
            self.db
                .run_script(&del_script, p, ScriptMutability::Mutable)
                .map_err(|e| map_db_err(e.to_string()))?;
            deleted += count;
        }
        Ok(deleted)
    }

    /// List all concerns edges for a task, with resolved symbol metadata.
    pub async fn list_concerns_for_task(
        &self,
        task_id: &str,
    ) -> Result<Vec<ConcernsLink>, EngramError> {
        let mut out = Vec::new();
        for (tbl, meta_tbl) in &[
            ("function", "function_meta"),
            ("class", "class_meta"),
            ("interface", "interface_meta"),
        ] {
            let script = format!(
                r#"?[symbol_id, symbol_table, linked_by, symbol_name, file_path] :=
    *concerns_edge {{ task_id, symbol_id, symbol_table, linked_by }},
    task_id = $task_id,
    symbol_table = "{tbl}",
    *{meta_tbl} {{ id: symbol_id, name: symbol_name, file_path }}"#
            );
            let mut p = BTreeMap::new();
            p.insert("task_id".to_owned(), DataValue::from(task_id));
            let r = self
                .db
                .run_script(&script, p, ScriptMutability::Immutable)
                .map_err(|e| map_db_err(e.to_string()))?;
            for row in &r.rows {
                out.push(ConcernsLink {
                    symbol_table: extract_str(row, 1),
                    symbol_id: extract_str(row, 0),
                    symbol_name: extract_str(row, 3),
                    file_path: extract_str(row, 4),
                    linked_by: extract_str(row, 2),
                });
            }
        }
        Ok(out)
    }

    /// Find task IDs that concern any of the given symbol IDs.
    pub async fn find_tasks_for_symbols(
        &self,
        symbol_ids: &[String],
    ) -> Result<Vec<(String, String)>, EngramError> {
        if symbol_ids.is_empty() {
            return Ok(vec![]);
        }
        let script = r#"
?[task_id, symbol_id] :=
    *concerns_edge { task_id, symbol_id }
"#;
        let r = self
            .db
            .run_script(script, BTreeMap::new(), ScriptMutability::Immutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        let id_set: std::collections::HashSet<&String> = symbol_ids.iter().collect();
        Ok(r.rows
            .iter()
            .filter_map(|row| {
                let sid = extract_str(row, 1);
                if id_set.contains(&sid) {
                    Some((extract_str(row, 0), sid))
                } else {
                    None
                }
            })
            .collect())
    }

    // ── BFS traversal ─────────────────────────────────────────────

    /// Search all symbol tables for symbols whose name matches `name`.
    pub async fn find_symbols_by_name(&self, name: &str) -> Result<Vec<SymbolMatch>, EngramError> {
        let mut out = Vec::new();
        for (tbl, meta_tbl, code_tbl, embed_tbl) in &[
            (
                "function",
                "function_meta",
                "function_code",
                "function_embedding",
            ),
            ("class", "class_meta", "class_code", "class_embedding"),
            (
                "interface",
                "interface_meta",
                "interface_code",
                "interface_embedding",
            ),
        ] {
            // Query meta + optionally join code/embed; outer-join style via separate queries.
            let meta_script = format!(
                r#"?[id, name, file_path, line_start, line_end, embed_type, summary] :=
    *{meta_tbl} {{ id, name, file_path, line_start, line_end, embed_type, summary }},
    name = $name"#
            );
            let mut p = BTreeMap::new();
            p.insert("name".to_owned(), DataValue::from(name));
            let meta_r = self
                .db
                .run_script(&meta_script, p, ScriptMutability::Immutable)
                .map_err(|e| map_db_err(e.to_string()))?;
            for meta_row in &meta_r.rows {
                let id = extract_str(meta_row, 0);
                // Attempt to fetch body and embedding, tolerate miss
                let body = {
                    let s = format!("?[body] := *{code_tbl} {{ id, body }}, id = $id");
                    let mut cp = BTreeMap::new();
                    cp.insert("id".to_owned(), DataValue::from(id.as_str()));
                    self.db
                        .run_script(&s, cp, ScriptMutability::Immutable)
                        .ok()
                        .and_then(|r| r.rows.into_iter().next())
                        .map_or_else(String::new, |r| extract_str(&r, 0))
                };
                let embedding = {
                    let s = format!("?[embedding] := *{embed_tbl} {{ id, embedding }}, id = $id");
                    let mut ep = BTreeMap::new();
                    ep.insert("id".to_owned(), DataValue::from(id.as_str()));
                    self.db
                        .run_script(&s, ep, ScriptMutability::Immutable)
                        .ok()
                        .and_then(|r| r.rows.into_iter().next())
                        .map_or_else(Vec::new, |r| extract_embedding(&r, 0))
                };
                out.push(SymbolMatch {
                    table: (*tbl).to_owned(),
                    id,
                    name: extract_str(meta_row, 1),
                    file_path: extract_str(meta_row, 2),
                    line_start: Some(extract_u32(meta_row, 3)),
                    line_end: Some(extract_u32(meta_row, 4)),
                    signature: None,
                    body,
                    embed_type: extract_opt_str(meta_row, 5),
                    summary: extract_opt_str(meta_row, 6),
                    embedding,
                });
            }
        }
        Ok(out)
    }

    /// BFS neighborhood traversal up to `max_depth` hops from `root_id`.
    ///
    /// Implemented as iterative multi-hop Rust BFS — one batch of 1-hop
    /// queries per depth level (avoids recursive Datalog complexity).
    pub async fn bfs_neighborhood(
        &self,
        root_id: &str,
        max_depth: usize,
        max_nodes: usize,
    ) -> Result<BfsResult, EngramError> {
        self.bfs_impl(root_id, max_depth, max_nodes, &[]).await
    }

    /// Resolve a symbol node ID to its full metadata (across all symbol types).
    pub async fn resolve_symbol(&self, node_id: &str) -> Result<Option<SymbolMatch>, EngramError> {
        let prefix = if node_id.starts_with("fn:") {
            "function"
        } else if node_id.starts_with("class:") {
            "class"
        } else if node_id.starts_with("iface:") {
            "interface"
        } else if node_id.starts_with("code_file:") || node_id.starts_with("file:") {
            "code_file"
        } else {
            // Try all symbol tables.
            for (tbl, meta_tbl, code_tbl, embed_tbl) in &[
                (
                    "function",
                    "function_meta",
                    "function_code",
                    "function_embedding",
                ),
                ("class", "class_meta", "class_code", "class_embedding"),
                (
                    "interface",
                    "interface_meta",
                    "interface_code",
                    "interface_embedding",
                ),
            ] {
                if let Some(m) = self
                    .resolve_from_table(node_id, tbl, meta_tbl, code_tbl, embed_tbl)
                    .await?
                {
                    return Ok(Some(m));
                }
            }
            return Ok(None);
        };

        match prefix {
            "function" => {
                self.resolve_from_table(
                    node_id,
                    "function",
                    "function_meta",
                    "function_code",
                    "function_embedding",
                )
                .await
            }
            "class" => {
                self.resolve_from_table(
                    node_id,
                    "class",
                    "class_meta",
                    "class_code",
                    "class_embedding",
                )
                .await
            }
            "interface" => {
                self.resolve_from_table(
                    node_id,
                    "interface",
                    "interface_meta",
                    "interface_code",
                    "interface_embedding",
                )
                .await
            }
            "code_file" => {
                // file_node lookup by `id` column (used when edge endpoints store node IDs).
                let script = r#"
?[path, id, language] := *file_node { path, id, language }, id = $id
"#;
                let mut p = BTreeMap::new();
                p.insert("id".to_owned(), DataValue::from(node_id));
                let r = self
                    .db
                    .run_script(script, p, ScriptMutability::Immutable)
                    .map_err(|e| map_db_err(e.to_string()))?;
                if r.rows.is_empty() {
                    return Ok(None);
                }
                Ok(Some(SymbolMatch {
                    table: "file".to_owned(),
                    id: extract_str(&r.rows[0], 1),
                    name: extract_str(&r.rows[0], 0),
                    file_path: extract_str(&r.rows[0], 0),
                    line_start: None,
                    line_end: None,
                    signature: None,
                    body: String::new(),
                    embed_type: None,
                    summary: None,
                    embedding: vec![],
                }))
            }
            _ => {
                // file_node path lookup (when node_id is the raw path).
                let script = r#"
?[path, id, language] := *file_node { path, id, language }, path = $path
"#;
                let mut p = BTreeMap::new();
                p.insert("path".to_owned(), DataValue::from(node_id));
                let r = self
                    .db
                    .run_script(script, p, ScriptMutability::Immutable)
                    .map_err(|e| map_db_err(e.to_string()))?;
                if r.rows.is_empty() {
                    return Ok(None);
                }
                Ok(Some(SymbolMatch {
                    table: "file".to_owned(),
                    id: extract_str(&r.rows[0], 1),
                    name: extract_str(&r.rows[0], 0),
                    file_path: extract_str(&r.rows[0], 0),
                    line_start: None,
                    line_end: None,
                    signature: None,
                    body: String::new(),
                    embed_type: None,
                    summary: None,
                    embedding: vec![],
                }))
            }
        }
    }

    // ── Symbol listing ────────────────────────────────────────────

    /// List symbols matching `filter`, with pagination.
    pub async fn list_symbols(
        &self,
        filter: &SymbolFilter,
    ) -> Result<SymbolListResult, EngramError> {
        let mut symbols = Vec::new();

        let types: Vec<(&str, &str)> = match filter.node_type.as_deref() {
            Some("function") => vec![("function", "function_meta")],
            Some("class") => vec![("class", "class_meta")],
            Some("interface") => vec![("interface", "interface_meta")],
            _ => vec![
                ("function", "function_meta"),
                ("class", "class_meta"),
                ("interface", "interface_meta"),
            ],
        };

        for (kind, tbl) in &types {
            let fp_clause = filter
                .file_path
                .as_deref()
                .map(|_| ", file_path = $file_path")
                .unwrap_or("");
            let script = format!(
                "?[id, name, file_path, line_start, line_end] := *{tbl} {{ id, name, file_path, line_start, line_end }}{fp_clause}"
            );
            let mut p = BTreeMap::new();
            if let Some(fp) = &filter.file_path {
                p.insert("file_path".to_owned(), DataValue::from(fp.as_str()));
            }
            let r = self
                .db
                .run_script(&script, p, ScriptMutability::Immutable)
                .map_err(|e| map_db_err(e.to_string()))?;
            for row in &r.rows {
                let name = extract_str(row, 1);
                if let Some(prefix) = &filter.name_prefix {
                    if !name.starts_with(prefix.as_str()) {
                        continue;
                    }
                }
                symbols.push(SymbolListEntry {
                    name,
                    node_type: (*kind).to_owned(),
                    file_path: extract_str(row, 2),
                    line_start: {
                        let v = extract_u32(row, 3);
                        if v == 0 { None } else { Some(v) }
                    },
                    line_end: {
                        let v = extract_u32(row, 4);
                        if v == 0 { None } else { Some(v) }
                    },
                });
            }
        }

        // Sort by name then node_type for deterministic pagination, matching
        // the SurrealDB backend's `ORDER BY name ASC` behaviour.
        symbols.sort_by(|a, b| a.name.cmp(&b.name).then(a.node_type.cmp(&b.node_type)));

        let total_count = symbols.len();
        let start = filter.offset.min(total_count);
        let end = (start + filter.limit).min(total_count);
        let has_more = end < total_count;
        Ok(SymbolListResult {
            symbols: symbols[start..end].to_vec(),
            total_count,
            has_more,
        })
    }

    // ── Vector search ─────────────────────────────────────────────

    /// Vector search — returns ranked symbol matches (score stripped).
    pub async fn vector_search_symbols(
        &self,
        query_embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<SymbolMatch>, EngramError> {
        let mut scored = self
            .vector_search_symbols_native(query_embedding, limit)
            .await?;
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        Ok(scored.into_iter().map(|(_, m)| m).collect())
    }

    /// Vector search — returns (score, match) pairs using a full linear scan.
    ///
    /// Performs a full linear scan across all symbol embedding tables and computes
    /// cosine similarity against `query_embedding`. HNSW indexes exist in the
    /// schema but are not explicitly invoked here; CozoDB may use them internally
    /// for query planning, but callers should not assume HNSW acceleration.
    pub async fn vector_search_symbols_native(
        &self,
        query_embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<(f32, SymbolMatch)>, EngramError> {
        // Try linear scan across all three embedding tables.
        let mut scored = Vec::new();
        for (tbl, meta_tbl, code_tbl, embed_tbl) in &[
            (
                "function",
                "function_meta",
                "function_code",
                "function_embedding",
            ),
            ("class", "class_meta", "class_code", "class_embedding"),
            (
                "interface",
                "interface_meta",
                "interface_code",
                "interface_embedding",
            ),
        ] {
            let script = format!(
                r#"?[id, name, file_path, line_start, line_end, embed_type, summary, body, embedding] :=
    *{meta_tbl} {{ id, name, file_path, line_start, line_end, embed_type, summary }},
    *{code_tbl} {{ id, body }},
    *{embed_tbl} {{ id, embedding }}"#
            );
            let r = self
                .db
                .run_script(&script, BTreeMap::new(), ScriptMutability::Immutable)
                .map_err(|e| map_db_err(e.to_string()))?;
            for row in &r.rows {
                let emb = extract_embedding(row, 8);
                if emb.is_empty() {
                    continue;
                }
                let score = cosine_similarity(query_embedding, &emb);
                scored.push((
                    score,
                    SymbolMatch {
                        table: (*tbl).to_owned(),
                        id: extract_str(row, 0),
                        name: extract_str(row, 1),
                        file_path: extract_str(row, 2),
                        line_start: Some(extract_u32(row, 3)),
                        line_end: Some(extract_u32(row, 4)),
                        signature: None,
                        body: extract_str(row, 7),
                        embed_type: extract_opt_str(row, 5),
                        summary: extract_opt_str(row, 6),
                        embedding: emb,
                    },
                ));
            }
        }
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);
        Ok(scored)
    }

    /// Hybrid graph + vector search from a root node.
    ///
    /// Expands the BFS neighborhood up to `max_depth`, then re-ranks
    /// the discovered neighbors by cosine similarity to `query_embedding`.
    pub async fn hybrid_graph_vector_search(
        &self,
        root_id: &str,
        max_depth: usize,
        query_embedding: &[f32],
        limit: usize,
        edge_types: &[&str],
    ) -> Result<Vec<(f32, SymbolMatch)>, EngramError> {
        // Code edge types supported for hybrid traversal (matches SurrealDB defaults).
        const ALLOWED: &[&str] = &["calls", "imports", "defines", "inherits_from", "concerns"];
        for et in edge_types {
            if !ALLOWED.contains(et) {
                return Err(EngramError::from(SystemError::DatabaseError {
                    reason: format!("unknown edge type: {et}"),
                }));
            }
        }

        let bfs = self
            .bfs_impl(root_id, max_depth, limit * 4, edge_types)
            .await?;

        // BFS already filtered to allowed edge types during traversal.
        let neighbors = bfs.neighbors;

        let mut scored: Vec<(f32, SymbolMatch)> = neighbors
            .into_iter()
            .map(|m| {
                let score = if m.embedding.is_empty() {
                    0.0_f32
                } else {
                    cosine_similarity(query_embedding, &m.embedding)
                };
                (score, m)
            })
            .collect();
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);
        Ok(scored)
    }

    /// Graph neighborhood — alias for `bfs_neighborhood`.
    pub async fn graph_neighborhood(
        &self,
        root_id: &str,
        max_depth: usize,
        max_nodes: usize,
    ) -> Result<BfsResult, EngramError> {
        self.bfs_impl(root_id, max_depth, max_nodes, &[]).await
    }

    // ── Embedding updates ─────────────────────────────────────────

    /// Update the embedding vector for a symbol (function, class, or interface).
    ///
    /// Detects the target table from the ID prefix (`fn:`, `class:`, `iface:`).
    pub async fn update_symbol_embedding(
        &self,
        sym_id: &str,
        embedding: Vec<f32>,
    ) -> Result<(), EngramError> {
        let table = if sym_id.starts_with("fn:") || sym_id.starts_with("function:") {
            "function_embedding"
        } else if sym_id.starts_with("class:") {
            "class_embedding"
        } else if sym_id.starts_with("iface:") || sym_id.starts_with("interface:") {
            "interface_embedding"
        } else {
            return Err(EngramError::from(SystemError::DatabaseError {
                reason: format!("cannot determine embedding table for id: {sym_id}"),
            }));
        };
        let emb_dv = DataValue::List(
            embedding
                .iter()
                .map(|&f| DataValue::Num(Num::Float(f64::from(f))))
                .collect(),
        );
        let script =
            format!("?[id, embedding] <- [[$id, $embedding]] :put {table} {{ id, embedding }}");
        let mut p = BTreeMap::new();
        p.insert("id".to_owned(), DataValue::from(sym_id));
        p.insert("embedding".to_owned(), emb_dv);
        self.db
            .run_script(&script, p, ScriptMutability::Mutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        Ok(())
    }

    /// Delete embedding rows whose vector is empty (zero-length list).
    ///
    /// Returns the count of removed rows.
    pub async fn gc_corrupted_embeddings(&self) -> Result<usize, EngramError> {
        let mut count = 0usize;
        for tbl in &[
            "function_embedding",
            "class_embedding",
            "interface_embedding",
        ] {
            let query = format!(
                "?[id] := *{tbl} {{ id, embedding }}, emb_len = length(embedding), emb_len = 0"
            );
            let r = self
                .db
                .run_script(&query, BTreeMap::new(), ScriptMutability::Immutable)
                .map_err(|e| map_db_err(e.to_string()))?;
            for row in &r.rows {
                let id = extract_str(row, 0);
                let del = format!("?[id] <- [[$id]] :rm {tbl} {{ id }}");
                let mut p = BTreeMap::new();
                p.insert("id".to_owned(), DataValue::from(id.as_str()));
                self.db
                    .run_script(&del, p, ScriptMutability::Mutable)
                    .map_err(|e| map_db_err(e.to_string()))?;
                count += 1;
            }
        }
        Ok(count)
    }

    // ── Count queries ─────────────────────────────────────────────

    /// Return the total number of code files indexed.
    pub async fn count_code_files(&self) -> Result<u64, EngramError> {
        let script = "?[count(path)] := *file_node { path }";
        let result = self
            .db
            .run_script(script, BTreeMap::new(), ScriptMutability::Immutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        Ok(extract_count(&result))
    }

    /// Return the total number of function records indexed.
    pub async fn count_functions(&self) -> Result<u64, EngramError> {
        let script = "?[count(id)] := *function_meta { id }";
        let result = self
            .db
            .run_script(script, BTreeMap::new(), ScriptMutability::Immutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        Ok(extract_count(&result))
    }

    /// Return the total number of class records indexed.
    pub async fn count_classes(&self) -> Result<u64, EngramError> {
        let result = self
            .db
            .run_script(
                "?[count(id)] := *class_meta { id }",
                BTreeMap::new(),
                ScriptMutability::Immutable,
            )
            .map_err(|e| map_db_err(e.to_string()))?;
        Ok(extract_count(&result))
    }

    /// Return the total number of interface records indexed.
    pub async fn count_interfaces(&self) -> Result<u64, EngramError> {
        let result = self
            .db
            .run_script(
                "?[count(id)] := *interface_meta { id }",
                BTreeMap::new(),
                ScriptMutability::Immutable,
            )
            .map_err(|e| map_db_err(e.to_string()))?;
        Ok(extract_count(&result))
    }

    /// Return the total number of code edges (calls + imports + defines + inherits_from).
    ///
    /// The `references_edge` table is intentionally excluded for SurrealDB parity.
    pub async fn count_code_edges(&self) -> Result<u64, EngramError> {
        let mut total = 0u64;
        for tbl in &[
            "calls_edge",
            "imports_edge",
            "defines_edge",
            "inherits_from_edge",
        ] {
            let script = format!("?[count(from)] := *{tbl} {{ from }}");
            let r = self
                .db
                .run_script(&script, BTreeMap::new(), ScriptMutability::Immutable)
                .map_err(|e| map_db_err(e.to_string()))?;
            total += extract_count(&r);
        }
        Ok(total)
    }

    // ── Bulk concerns ─────────────────────────────────────────────

    /// List all concerns edges for multiple tasks, grouped by task ID.
    pub async fn list_concerns_for_tasks(
        &self,
        task_ids: &[String],
    ) -> Result<HashMap<String, Vec<ConcernsLink>>, EngramError> {
        if task_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let mut map: HashMap<String, Vec<ConcernsLink>> = HashMap::new();
        for tid in task_ids {
            let links = self.list_concerns_for_task(tid).await?;
            if !links.is_empty() {
                map.insert(tid.clone(), links);
            }
        }
        Ok(map)
    }

    // ── Content record queries ────────────────────────────────────

    /// Upsert a `ContentRecord` into the `content_record` table.
    pub async fn upsert_content_record(
        &self,
        record: &crate::models::ContentRecord,
    ) -> Result<(), EngramError> {
        let script = r#"
?[file_path, id, content_type, content_hash, content, source_path,
  file_size_bytes, ingested_at, embedding] <-
    [[$file_path, $id, $content_type, $content_hash, $content, $source_path,
      $file_size_bytes, $ingested_at, $embedding]]
:put content_record {
    file_path => id, content_type, content_hash, content, source_path,
    file_size_bytes, ingested_at, embedding
}
"#;
        let emb_dv = DataValue::List(
            record
                .embedding
                .as_deref()
                .unwrap_or(&[])
                .iter()
                .map(|&f| DataValue::Num(Num::Float(f64::from(f))))
                .collect(),
        );
        let mut p = BTreeMap::new();
        p.insert(
            "file_path".to_owned(),
            DataValue::from(record.file_path.as_str()),
        );
        p.insert("id".to_owned(), DataValue::from(record.id.as_str()));
        p.insert(
            "content_type".to_owned(),
            DataValue::from(record.content_type.as_str()),
        );
        p.insert(
            "content_hash".to_owned(),
            DataValue::from(record.content_hash.as_str()),
        );
        p.insert(
            "content".to_owned(),
            DataValue::from(record.content.as_str()),
        );
        p.insert(
            "source_path".to_owned(),
            DataValue::from(record.source_path.as_str()),
        );
        p.insert(
            "file_size_bytes".to_owned(),
            DataValue::Num(Num::Int(
                i64::try_from(record.file_size_bytes).unwrap_or(i64::MAX),
            )),
        );
        p.insert(
            "ingested_at".to_owned(),
            DataValue::from(record.ingested_at.to_rfc3339().as_str()),
        );
        p.insert("embedding".to_owned(), emb_dv);
        self.db
            .run_script(script, p, ScriptMutability::Mutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        Ok(())
    }

    /// Return all content records, optionally filtered by `content_type`.
    pub async fn select_content_records(
        &self,
        content_type: Option<&str>,
    ) -> Result<Vec<crate::models::ContentRecord>, EngramError> {
        let ct_clause = content_type
            .map(|_| ", content_type = $content_type")
            .unwrap_or("");
        let script = format!(
            r#"?[file_path, id, content_type, content_hash, content, source_path,
  file_size_bytes, ingested_at, embedding] :=
    *content_record {{ file_path, id, content_type, content_hash, content,
                      source_path, file_size_bytes, ingested_at, embedding }}{ct_clause}"#
        );
        let mut p = BTreeMap::new();
        if let Some(ct) = content_type {
            p.insert("content_type".to_owned(), DataValue::from(ct));
        }
        let result = self
            .db
            .run_script(&script, p, ScriptMutability::Immutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        result
            .rows
            .iter()
            .map(|row| {
                let ingested_str = extract_str(row, 7);
                let ingested_at = chrono::DateTime::parse_from_rfc3339(&ingested_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());
                let emb = extract_embedding(row, 8);
                Ok(crate::models::ContentRecord {
                    file_path: extract_str(row, 0),
                    id: extract_str(row, 1),
                    content_type: extract_str(row, 2),
                    content_hash: extract_str(row, 3),
                    content: extract_str(row, 4),
                    source_path: extract_str(row, 5),
                    file_size_bytes: u64::try_from(extract_i64(row, 6).max(0)).unwrap_or(0),
                    ingested_at,
                    embedding: if emb.is_empty() { None } else { Some(emb) },
                })
            })
            .collect()
    }

    /// Update the embedding on an existing content record (keyed by `record_id` = `file_path`).
    pub async fn update_content_record_embedding(
        &self,
        record_id: &str,
        embedding: Vec<f32>,
    ) -> Result<(), EngramError> {
        // Read existing record first to satisfy the full PUT contract.
        let get = r#"
?[file_path, id, content_type, content_hash, content, source_path,
  file_size_bytes, ingested_at] :=
    *content_record { file_path, id, content_type, content_hash, content,
                      source_path, file_size_bytes, ingested_at },
    file_path = $file_path
"#;
        let mut gp = BTreeMap::new();
        gp.insert("file_path".to_owned(), DataValue::from(record_id));
        let existing = self
            .db
            .run_script(get, gp, ScriptMutability::Immutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        if existing.rows.is_empty() {
            return Ok(()); // nothing to update
        }
        let row = &existing.rows[0];
        let emb_dv = DataValue::List(
            embedding
                .iter()
                .map(|&f| DataValue::Num(Num::Float(f64::from(f))))
                .collect(),
        );
        let put = r#"
?[file_path, id, content_type, content_hash, content, source_path,
  file_size_bytes, ingested_at, embedding] <-
    [[$file_path, $id, $ct, $ch, $content, $sp, $fsb, $ia, $embedding]]
:put content_record {
    file_path => id, content_type, content_hash, content, source_path,
    file_size_bytes, ingested_at, embedding
}
"#;
        let mut p = BTreeMap::new();
        p.insert("file_path".to_owned(), DataValue::from(record_id));
        p.insert(
            "id".to_owned(),
            DataValue::from(extract_str(row, 1).as_str()),
        );
        p.insert(
            "ct".to_owned(),
            DataValue::from(extract_str(row, 2).as_str()),
        );
        p.insert(
            "ch".to_owned(),
            DataValue::from(extract_str(row, 3).as_str()),
        );
        p.insert(
            "content".to_owned(),
            DataValue::from(extract_str(row, 4).as_str()),
        );
        p.insert(
            "sp".to_owned(),
            DataValue::from(extract_str(row, 5).as_str()),
        );
        p.insert(
            "fsb".to_owned(),
            DataValue::Num(Num::Int(extract_i64(row, 6))),
        );
        p.insert(
            "ia".to_owned(),
            DataValue::from(extract_str(row, 7).as_str()),
        );
        p.insert("embedding".to_owned(), emb_dv);
        self.db
            .run_script(put, p, ScriptMutability::Mutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        Ok(())
    }

    /// Delete a content record by its file path.
    pub async fn delete_content_record_by_path(&self, file_path: &str) -> Result<(), EngramError> {
        let script = r#"
?[file_path] <- [[$file_path]]
:rm content_record { file_path }
"#;
        let mut p = BTreeMap::new();
        p.insert("file_path".to_owned(), DataValue::from(file_path));
        self.db
            .run_script(script, p, ScriptMutability::Mutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        Ok(())
    }

    /// Vector search over content records — linear scan with cosine ranking.
    ///
    /// Returns `Ok(vec![])` (not an error) when no content records are present,
    /// so that `unified_search` degrades gracefully.
    pub async fn vector_search_content_native(
        &self,
        query_embedding: &[f32],
        limit: usize,
        content_type: Option<&str>,
    ) -> Result<Vec<(f32, crate::models::ContentRecord)>, EngramError> {
        let records = self.select_content_records(content_type).await?;
        let mut scored: Vec<(f32, crate::models::ContentRecord)> = records
            .into_iter()
            .filter_map(|rec| {
                let emb = rec.embedding.as_deref()?;
                if emb.is_empty() {
                    return None;
                }
                let score = cosine_similarity(query_embedding, emb);
                Some((score, rec))
            })
            .collect();
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);
        Ok(scored)
    }

    // ── Commit node queries ───────────────────────────────────────

    /// Upsert a `CommitNode`, serializing `changes` as JSON strings.
    pub async fn upsert_commit_node(
        &self,
        node: &crate::models::CommitNode,
    ) -> Result<(), EngramError> {
        let changes = node
            .changes
            .iter()
            .map(|c| {
                serde_json::to_string(c)
                    .map(|s| DataValue::from(s.as_str()))
                    .map_err(|e| {
                        EngramError::from(SystemError::DatabaseError {
                            reason: format!("commit change serialization failed: {e}"),
                        })
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let parent_hashes: Vec<DataValue> = node
            .parent_hashes
            .iter()
            .map(|h| DataValue::from(h.as_str()))
            .collect();
        let script = r#"
?[id, hash, short_hash, author_name, author_email, timestamp, message,
  parent_hashes, changes] <-
    [[$id, $hash, $short_hash, $author_name, $author_email, $timestamp,
      $message, $parent_hashes, $changes]]
:put commit_node {
    id => hash, short_hash, author_name, author_email, timestamp, message,
    parent_hashes, changes
}
"#;
        let mut p = BTreeMap::new();
        p.insert("id".to_owned(), DataValue::from(node.id.as_str()));
        p.insert("hash".to_owned(), DataValue::from(node.hash.as_str()));
        p.insert(
            "short_hash".to_owned(),
            DataValue::from(node.short_hash.as_str()),
        );
        p.insert(
            "author_name".to_owned(),
            DataValue::from(node.author_name.as_str()),
        );
        p.insert(
            "author_email".to_owned(),
            DataValue::from(node.author_email.as_str()),
        );
        p.insert(
            "timestamp".to_owned(),
            DataValue::Num(Num::Int(node.timestamp.timestamp())),
        );
        p.insert("message".to_owned(), DataValue::from(node.message.as_str()));
        p.insert("parent_hashes".to_owned(), DataValue::List(parent_hashes));
        p.insert("changes".to_owned(), DataValue::List(changes));
        self.db
            .run_script(script, p, ScriptMutability::Mutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        Ok(())
    }

    /// Return commits whose timestamp falls in `[since, until]`.
    pub async fn select_commits_by_date_range(
        &self,
        since: Option<&DateTime<Utc>>,
        until: Option<&DateTime<Utc>>,
        limit: u32,
    ) -> Result<Vec<crate::models::CommitNode>, EngramError> {
        let script = r#"
?[id, hash, short_hash, author_name, author_email, timestamp, message,
  parent_hashes, changes] :=
    *commit_node { id, hash, short_hash, author_name, author_email, timestamp,
                   message, parent_hashes, changes }
:order -timestamp
"#;
        let r = self
            .db
            .run_script(script, BTreeMap::new(), ScriptMutability::Immutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        let since_ts = since.map(DateTime::timestamp);
        let until_ts = until.map(DateTime::timestamp);
        let lim = usize::try_from(limit).unwrap_or(usize::MAX);
        Ok(r.rows
            .iter()
            .filter(|row| {
                let ts = extract_i64(row, 5);
                since_ts.is_none_or(|s| ts >= s) && until_ts.is_none_or(|u| ts <= u)
            })
            .take(lim)
            .map(|row| row_to_commit_node(row))
            .collect())
    }

    /// Return commits that changed `file_path` (scans `changes` JSON strings).
    pub async fn select_commits_by_file_path(
        &self,
        file_path: &str,
        limit: u32,
    ) -> Result<Vec<crate::models::CommitNode>, EngramError> {
        let script = r#"
?[id, hash, short_hash, author_name, author_email, timestamp, message,
  parent_hashes, changes] :=
    *commit_node { id, hash, short_hash, author_name, author_email, timestamp,
                   message, parent_hashes, changes }
:order -timestamp
"#;
        let r = self
            .db
            .run_script(script, BTreeMap::new(), ScriptMutability::Immutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        let lim = usize::try_from(limit).unwrap_or(usize::MAX);
        Ok(r.rows
            .iter()
            .filter(|row| {
                // changes column is a list of JSON strings — scan for file_path.
                match row.get(8) {
                    Some(DataValue::List(v)) => v.iter().any(|dv| {
                        if let DataValue::Str(s) = dv {
                            s.contains(file_path)
                        } else {
                            false
                        }
                    }),
                    _ => false,
                }
            })
            .take(lim)
            .map(|row| row_to_commit_node(row))
            .collect())
    }

    /// Return the hash of the most recently indexed commit, or `None`.
    pub async fn latest_indexed_commit_hash(&self) -> Result<Option<String>, EngramError> {
        let script = r#"
?[hash, timestamp] := *commit_node { hash, timestamp }
:order -timestamp
:limit 1
"#;
        let r = self
            .db
            .run_script(script, BTreeMap::new(), ScriptMutability::Immutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        Ok(r.rows.first().map(|row| extract_str(row, 0)))
    }

    // ── File hash queries ─────────────────────────────────────────

    /// Upsert a file hash record.
    pub async fn upsert_file_hash(
        &self,
        file_path: &str,
        content_hash: &str,
        size_bytes: u64,
    ) -> Result<(), EngramError> {
        let ts = now_utc_str();
        let script = r#"
?[file_path, content_hash, size_bytes, recorded_at] <-
    [[$file_path, $content_hash, $size_bytes, $recorded_at]]
:put file_hash { file_path => content_hash, size_bytes, recorded_at }
"#;
        let mut p = BTreeMap::new();
        p.insert("file_path".to_owned(), DataValue::from(file_path));
        p.insert("content_hash".to_owned(), DataValue::from(content_hash));
        p.insert(
            "size_bytes".to_owned(),
            DataValue::Num(Num::Int(i64::try_from(size_bytes).unwrap_or(i64::MAX))),
        );
        p.insert("recorded_at".to_owned(), DataValue::from(ts.as_str()));
        self.db
            .run_script(script, p, ScriptMutability::Mutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        Ok(())
    }

    /// Return all file hash records.
    pub async fn get_all_file_hashes(&self) -> Result<Vec<FileHashRecord>, EngramError> {
        let script = r#"
?[file_path, content_hash, size_bytes, recorded_at] :=
    *file_hash { file_path, content_hash, size_bytes, recorded_at }
"#;
        let r = self
            .db
            .run_script(script, BTreeMap::new(), ScriptMutability::Immutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        r.rows
            .iter()
            .map(|row| {
                let ts_str = extract_str(row, 3);
                let recorded_at = chrono::DateTime::parse_from_rfc3339(&ts_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());
                Ok(FileHashRecord {
                    file_path: extract_str(row, 0),
                    content_hash: extract_str(row, 1),
                    size_bytes: u64::try_from(extract_i64(row, 2).max(0)).unwrap_or(0),
                    recorded_at,
                })
            })
            .collect()
    }

    /// Delete a file hash record by file path.
    pub async fn delete_file_hash_by_path(&self, file_path: &str) -> Result<(), EngramError> {
        let script = r#"
?[file_path] <- [[$file_path]]
:rm file_hash { file_path }
"#;
        let mut p = BTreeMap::new();
        p.insert("file_path".to_owned(), DataValue::from(file_path));
        self.db
            .run_script(script, p, ScriptMutability::Mutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        Ok(())
    }

    // ── Private helpers ───────────────────────────────────────────

    /// Collect all edges from an edge table.
    async fn edges_from_table(
        &self,
        kind: &str,
        table: &str,
    ) -> Result<Vec<crate::models::CodeEdge>, EngramError> {
        let script = if table == "concerns_edge" {
            // concerns_edge key: task_id, symbol_id. Includes linked_by in projection.
            "?[from, to, linked_by, created_at] := *concerns_edge { task_id, symbol_id, linked_by, created_at }, from = task_id, to = symbol_id".to_owned()
        } else if table == "imports_edge" {
            format!(
                "?[from, to, import_path, created_at] := *{table} {{ from, to, import_path, created_at }}"
            )
        } else {
            format!("?[from, to, created_at] := *{table} {{ from, to, created_at }}")
        };
        let r = self
            .db
            .run_script(&script, BTreeMap::new(), ScriptMutability::Immutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        let edge_type = match kind {
            "imports" => CodeEdgeType::Imports,
            "defines" => CodeEdgeType::Defines,
            "inherits_from" => CodeEdgeType::InheritsFrom,
            "concerns" => CodeEdgeType::Concerns,
            _ => CodeEdgeType::Calls,
        };
        if table == "imports_edge" {
            Ok(r.rows
                .iter()
                .map(|row| crate::models::CodeEdge {
                    edge_type: edge_type.clone(),
                    from: extract_str(row, 0),
                    to: extract_str(row, 1),
                    import_path: {
                        let s = extract_str(row, 2);
                        if s.is_empty() { None } else { Some(s) }
                    },
                    linked_by: None,
                    created_at: extract_str(row, 3),
                })
                .collect())
        } else if table == "concerns_edge" {
            // [from, to, linked_by, created_at]
            Ok(r.rows
                .iter()
                .map(|row| crate::models::CodeEdge {
                    edge_type: edge_type.clone(),
                    from: extract_str(row, 0),
                    to: extract_str(row, 1),
                    import_path: None,
                    linked_by: {
                        let s = extract_str(row, 2);
                        if s.is_empty() { None } else { Some(s) }
                    },
                    created_at: extract_str(row, 3),
                })
                .collect())
        } else {
            Ok(r.rows
                .iter()
                .map(|row| crate::models::CodeEdge {
                    edge_type: edge_type.clone(),
                    from: extract_str(row, 0),
                    to: extract_str(row, 1),
                    import_path: None,
                    linked_by: None,
                    created_at: extract_str(row, 2),
                })
                .collect())
        }
    }

    /// Delete all rows in `table` where the source node matches `node_id`.
    async fn delete_outgoing_edges(&self, table: &str, node_id: &str) -> Result<(), EngramError> {
        if table == "concerns_edge" {
            // concerns_edge uses task_id/symbol_id rather than from/to.
            let get = "?[task_id, symbol_id] := *concerns_edge { task_id, symbol_id }, task_id = $task_id";
            let mut p = BTreeMap::new();
            p.insert("task_id".to_owned(), DataValue::from(node_id));
            let r = self
                .db
                .run_script(get, p, ScriptMutability::Immutable)
                .map_err(|e| map_db_err(e.to_string()))?;
            for row in &r.rows {
                let tid = extract_str(row, 0);
                let sid = extract_str(row, 1);
                let del = "?[task_id, symbol_id] <- [[$task_id, $symbol_id]] :rm concerns_edge { task_id, symbol_id }";
                let mut dp = BTreeMap::new();
                dp.insert("task_id".to_owned(), DataValue::from(tid.as_str()));
                dp.insert("symbol_id".to_owned(), DataValue::from(sid.as_str()));
                self.db
                    .run_script(del, dp, ScriptMutability::Mutable)
                    .map_err(|e| map_db_err(e.to_string()))?;
            }
            return Ok(());
        }
        if table == "imports_edge" {
            // imports_edge composite key: (from, to, import_path).
            let get =
                "?[from, to, import_path] := *imports_edge { from, to, import_path }, from = $from";
            let mut p = BTreeMap::new();
            p.insert("from".to_owned(), DataValue::from(node_id));
            let r = self
                .db
                .run_script(get, p, ScriptMutability::Immutable)
                .map_err(|e| map_db_err(e.to_string()))?;
            for row in &r.rows {
                let from = extract_str(row, 0);
                let to = extract_str(row, 1);
                let ip = extract_str(row, 2);
                let del = "?[from, to, import_path] <- [[$from, $to, $ip]] :rm imports_edge { from, to, import_path }";
                let mut dp = BTreeMap::new();
                dp.insert("from".to_owned(), DataValue::from(from.as_str()));
                dp.insert("to".to_owned(), DataValue::from(to.as_str()));
                dp.insert("ip".to_owned(), DataValue::from(ip.as_str()));
                self.db
                    .run_script(del, dp, ScriptMutability::Mutable)
                    .map_err(|e| map_db_err(e.to_string()))?;
            }
            return Ok(());
        }
        if table == "references_edge" {
            // references_edge composite key: (from, to, qualified_name).
            let get = "?[from, to, qualified_name] := *references_edge { from, to, qualified_name }, from = $from";
            let mut p = BTreeMap::new();
            p.insert("from".to_owned(), DataValue::from(node_id));
            let r = self
                .db
                .run_script(get, p, ScriptMutability::Immutable)
                .map_err(|e| map_db_err(e.to_string()))?;
            for row in &r.rows {
                let from = extract_str(row, 0);
                let to = extract_str(row, 1);
                let qn = extract_str(row, 2);
                let del = "?[from, to, qualified_name] <- [[$from, $to, $qn]] :rm references_edge { from, to, qualified_name }";
                let mut dp = BTreeMap::new();
                dp.insert("from".to_owned(), DataValue::from(from.as_str()));
                dp.insert("to".to_owned(), DataValue::from(to.as_str()));
                dp.insert("qn".to_owned(), DataValue::from(qn.as_str()));
                self.db
                    .run_script(del, dp, ScriptMutability::Mutable)
                    .map_err(|e| map_db_err(e.to_string()))?;
            }
            return Ok(());
        }
        let get = format!("?[from, to] := *{table} {{ from, to }}, from = $from");
        let mut p = BTreeMap::new();
        p.insert("from".to_owned(), DataValue::from(node_id));
        let r = self
            .db
            .run_script(&get, p, ScriptMutability::Immutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        for row in &r.rows {
            let from = extract_str(row, 0);
            let to = extract_str(row, 1);
            let del = format!("?[from, to] <- [[$from, $to]] :rm {table} {{ from, to }}");
            let mut dp = BTreeMap::new();
            dp.insert("from".to_owned(), DataValue::from(from.as_str()));
            dp.insert("to".to_owned(), DataValue::from(to.as_str()));
            self.db
                .run_script(&del, dp, ScriptMutability::Mutable)
                .map_err(|e| map_db_err(e.to_string()))?;
        }
        Ok(())
    }

    /// Delete all rows in `table` where the target node matches `node_id`.
    async fn delete_incoming_edges(&self, table: &str, node_id: &str) -> Result<(), EngramError> {
        if table == "concerns_edge" {
            // concerns_edge uses task_id/symbol_id rather than from/to.
            let get = "?[task_id, symbol_id] := *concerns_edge { task_id, symbol_id }, symbol_id = $symbol_id";
            let mut p = BTreeMap::new();
            p.insert("symbol_id".to_owned(), DataValue::from(node_id));
            let r = self
                .db
                .run_script(get, p, ScriptMutability::Immutable)
                .map_err(|e| map_db_err(e.to_string()))?;
            for row in &r.rows {
                let tid = extract_str(row, 0);
                let sid = extract_str(row, 1);
                let del = "?[task_id, symbol_id] <- [[$task_id, $symbol_id]] :rm concerns_edge { task_id, symbol_id }";
                let mut dp = BTreeMap::new();
                dp.insert("task_id".to_owned(), DataValue::from(tid.as_str()));
                dp.insert("symbol_id".to_owned(), DataValue::from(sid.as_str()));
                self.db
                    .run_script(del, dp, ScriptMutability::Mutable)
                    .map_err(|e| map_db_err(e.to_string()))?;
            }
            return Ok(());
        }
        if table == "imports_edge" {
            // imports_edge composite key: (from, to, import_path).
            let get =
                "?[from, to, import_path] := *imports_edge { from, to, import_path }, to = $to";
            let mut p = BTreeMap::new();
            p.insert("to".to_owned(), DataValue::from(node_id));
            let r = self
                .db
                .run_script(get, p, ScriptMutability::Immutable)
                .map_err(|e| map_db_err(e.to_string()))?;
            for row in &r.rows {
                let from = extract_str(row, 0);
                let to = extract_str(row, 1);
                let ip = extract_str(row, 2);
                let del = "?[from, to, import_path] <- [[$from, $to, $ip]] :rm imports_edge { from, to, import_path }";
                let mut dp = BTreeMap::new();
                dp.insert("from".to_owned(), DataValue::from(from.as_str()));
                dp.insert("to".to_owned(), DataValue::from(to.as_str()));
                dp.insert("ip".to_owned(), DataValue::from(ip.as_str()));
                self.db
                    .run_script(del, dp, ScriptMutability::Mutable)
                    .map_err(|e| map_db_err(e.to_string()))?;
            }
            return Ok(());
        }
        if table == "references_edge" {
            // references_edge composite key: (from, to, qualified_name).
            let get = "?[from, to, qualified_name] := *references_edge { from, to, qualified_name }, to = $to";
            let mut p = BTreeMap::new();
            p.insert("to".to_owned(), DataValue::from(node_id));
            let r = self
                .db
                .run_script(get, p, ScriptMutability::Immutable)
                .map_err(|e| map_db_err(e.to_string()))?;
            for row in &r.rows {
                let from = extract_str(row, 0);
                let to = extract_str(row, 1);
                let qn = extract_str(row, 2);
                let del = "?[from, to, qualified_name] <- [[$from, $to, $qn]] :rm references_edge { from, to, qualified_name }";
                let mut dp = BTreeMap::new();
                dp.insert("from".to_owned(), DataValue::from(from.as_str()));
                dp.insert("to".to_owned(), DataValue::from(to.as_str()));
                dp.insert("qn".to_owned(), DataValue::from(qn.as_str()));
                self.db
                    .run_script(del, dp, ScriptMutability::Mutable)
                    .map_err(|e| map_db_err(e.to_string()))?;
            }
            return Ok(());
        }
        let get = format!("?[from, to] := *{table} {{ from, to }}, to = $to");
        let mut p = BTreeMap::new();
        p.insert("to".to_owned(), DataValue::from(node_id));
        let r = self
            .db
            .run_script(&get, p, ScriptMutability::Immutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        for row in &r.rows {
            let from = extract_str(row, 0);
            let to = extract_str(row, 1);
            let del = format!("?[from, to] <- [[$from, $to]] :rm {table} {{ from, to }}");
            let mut dp = BTreeMap::new();
            dp.insert("from".to_owned(), DataValue::from(from.as_str()));
            dp.insert("to".to_owned(), DataValue::from(to.as_str()));
            self.db
                .run_script(&del, dp, ScriptMutability::Mutable)
                .map_err(|e| map_db_err(e.to_string()))?;
        }
        Ok(())
    }

    /// BFS implementation — iterative multi-hop traversal.
    ///
    /// Matches SurrealDB backend behavior: bidirectional traversal over code edges
    /// (calls/imports/defines/inherits_from/concerns), excludes references edges,
    /// and only enqueues a node when it resolves to a known symbol.
    ///
    /// `allowed_edge_types`: if non-empty, only edges whose type label is in this
    /// slice are traversed. Pass an empty slice to traverse all edge types.
    async fn bfs_impl(
        &self,
        root_id: &str,
        max_depth: usize,
        max_nodes: usize,
        allowed_edge_types: &[&str],
    ) -> Result<BfsResult, EngramError> {
        const CODE_EDGE_TABLES: &[(&str, &str)] = &[
            ("calls", "calls_edge"),
            ("imports", "imports_edge"),
            ("defines", "defines_edge"),
            ("inherits_from", "inherits_from_edge"),
            ("concerns", "concerns_edge"),
        ];

        let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut neighbors: Vec<SymbolMatch> = Vec::new();
        let mut edges: Vec<BfsEdge> = Vec::new();
        let mut frontier: Vec<String> = vec![root_id.to_owned()];
        visited.insert(root_id.to_owned());
        let mut truncated = false;

        'outer: for _ in 0..max_depth {
            if frontier.is_empty() {
                break;
            }
            let mut next_frontier = Vec::new();
            for node in &frontier {
                for (et, tbl) in CODE_EDGE_TABLES {
                    // Skip tables not in the allowed set when a filter is supplied.
                    if !allowed_edge_types.is_empty() && !allowed_edge_types.contains(et) {
                        continue;
                    }
                    // Build outgoing/incoming query scripts — concerns_edge uses
                    // task_id/symbol_id keys rather than the standard from/to columns.
                    let (out_script, in_script) = if *tbl == "concerns_edge" {
                        (
                            "?[from, to] := *concerns_edge { task_id, symbol_id }, task_id = $node, from = task_id, to = symbol_id".to_owned(),
                            "?[from, to] := *concerns_edge { task_id, symbol_id }, symbol_id = $node, from = task_id, to = symbol_id".to_owned(),
                        )
                    } else {
                        (
                            format!("?[from, to] := *{tbl} {{ from, to }}, from = $node"),
                            format!("?[from, to] := *{tbl} {{ from, to }}, to = $node"),
                        )
                    };

                    // Outgoing edges: node → target
                    let mut p = BTreeMap::new();
                    p.insert("node".to_owned(), DataValue::from(node.as_str()));
                    let r = self
                        .db
                        .run_script(&out_script, p, ScriptMutability::Immutable)
                        .map_err(|e| map_db_err(e.to_string()))?;
                    for row in &r.rows {
                        let target = extract_str(row, 1);
                        if visited.contains(&target) {
                            continue;
                        }
                        if let Ok(Some(sym)) = self.resolve_symbol(&target).await {
                            if neighbors.len() >= max_nodes {
                                truncated = true;
                                break 'outer;
                            }
                            edges.push(BfsEdge {
                                edge_type: (*et).to_owned(),
                                from: node.clone(),
                                to: target.clone(),
                            });
                            visited.insert(target.clone());
                            neighbors.push(sym);
                            next_frontier.push(target);
                        }
                    }

                    // Incoming edges: source → node (reverse direction)
                    let mut p2 = BTreeMap::new();
                    p2.insert("node".to_owned(), DataValue::from(node.as_str()));
                    let r2 = self
                        .db
                        .run_script(&in_script, p2, ScriptMutability::Immutable)
                        .map_err(|e| map_db_err(e.to_string()))?;
                    for row in &r2.rows {
                        let source = extract_str(row, 0);
                        if visited.contains(&source) {
                            continue;
                        }
                        if let Ok(Some(sym)) = self.resolve_symbol(&source).await {
                            if neighbors.len() >= max_nodes {
                                truncated = true;
                                break 'outer;
                            }
                            edges.push(BfsEdge {
                                edge_type: (*et).to_owned(),
                                from: source.clone(),
                                to: node.clone(),
                            });
                            visited.insert(source.clone());
                            neighbors.push(sym);
                            next_frontier.push(source);
                        }
                    }
                }
            }
            frontier = next_frontier;
        }

        Ok(BfsResult {
            neighbors,
            edges,
            truncated,
        })
    }

    /// Resolve a symbol from a specific table by ID.
    async fn resolve_from_table(
        &self,
        node_id: &str,
        tbl: &str,
        meta_tbl: &str,
        code_tbl: &str,
        embed_tbl: &str,
    ) -> Result<Option<SymbolMatch>, EngramError> {
        let script = format!(
            r#"?[id, name, file_path, line_start, line_end, embed_type, summary, body, embedding] :=
    *{meta_tbl} {{ id, name, file_path, line_start, line_end, embed_type, summary }},
    id = $id,
    *{code_tbl} {{ id, body }},
    *{embed_tbl} {{ id, embedding }}"#
        );
        let mut p = BTreeMap::new();
        p.insert("id".to_owned(), DataValue::from(node_id));
        let r = self
            .db
            .run_script(&script, p, ScriptMutability::Immutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        if r.rows.is_empty() {
            return Ok(None);
        }
        let row = &r.rows[0];
        Ok(Some(SymbolMatch {
            table: tbl.to_owned(),
            id: extract_str(row, 0),
            name: extract_str(row, 1),
            file_path: extract_str(row, 2),
            line_start: Some(extract_u32(row, 3)),
            line_end: Some(extract_u32(row, 4)),
            signature: None,
            body: extract_str(row, 7),
            embed_type: extract_opt_str(row, 5),
            summary: extract_opt_str(row, 6),
            embedding: extract_embedding(row, 8),
        }))
    }
}

fn extract_str(row: &[DataValue], col: usize) -> String {
    match row.get(col) {
        Some(DataValue::Str(s)) => s.to_string(),
        _ => String::new(),
    }
}

fn extract_i64(row: &[DataValue], col: usize) -> i64 {
    match row.get(col) {
        Some(DataValue::Num(Num::Int(i))) => *i,
        _ => 0,
    }
}

fn extract_u32(row: &[DataValue], col: usize) -> u32 {
    u32::try_from(extract_i64(row, col).max(0)).unwrap_or(0)
}

#[allow(clippy::cast_precision_loss)]
#[allow(clippy::cast_possible_truncation)]
fn extract_embedding(row: &[DataValue], col: usize) -> Vec<f32> {
    match row.get(col) {
        Some(DataValue::List(v)) => v
            .iter()
            .filter_map(|dv| match dv {
                DataValue::Num(Num::Float(f)) => Some(*f as f32),
                DataValue::Num(Num::Int(i)) => Some(*i as f32),
                _ => None,
            })
            .collect(),
        _ => vec![],
    }
}

fn extract_opt_str(row: &[DataValue], col: usize) -> Option<String> {
    match row.get(col) {
        Some(DataValue::Str(s)) if !s.is_empty() => Some(s.to_string()),
        _ => None,
    }
}

fn extract_count(rows: &cozo::NamedRows) -> u64 {
    rows.rows
        .first()
        .and_then(|r| r.first())
        .and_then(|v| match v {
            DataValue::Num(Num::Int(i)) => u64::try_from(*i).ok(),
            _ => None,
        })
        .unwrap_or(0)
}

fn row_to_function(row: &[DataValue]) -> crate::models::Function {
    // columns: id(0), name(1), file_path(2), line_start(3), line_end(4),
    //          signature(5), docstring(6), body_hash(7), token_count(8),
    //          embed_type(9), summary(10), body(11), embedding(12)
    crate::models::Function {
        id: extract_str(row, 0),
        name: extract_str(row, 1),
        file_path: extract_str(row, 2),
        line_start: extract_u32(row, 3),
        line_end: extract_u32(row, 4),
        signature: extract_str(row, 5),
        docstring: extract_opt_str(row, 6),
        body_hash: extract_str(row, 7),
        token_count: extract_u32(row, 8),
        embed_type: extract_str(row, 9),
        summary: extract_str(row, 10),
        body: extract_str(row, 11),
        embedding: extract_embedding(row, 12),
    }
}

fn row_to_class(row: &[DataValue]) -> crate::models::Class {
    // columns: id(0), name(1), file_path(2), line_start(3), line_end(4),
    //          docstring(5), body_hash(6), token_count(7), embed_type(8),
    //          summary(9), body(10), embedding(11)
    crate::models::Class {
        id: extract_str(row, 0),
        name: extract_str(row, 1),
        file_path: extract_str(row, 2),
        line_start: extract_u32(row, 3),
        line_end: extract_u32(row, 4),
        docstring: extract_opt_str(row, 5),
        body_hash: extract_str(row, 6),
        token_count: extract_u32(row, 7),
        embed_type: extract_str(row, 8),
        summary: extract_str(row, 9),
        body: extract_str(row, 10),
        embedding: extract_embedding(row, 11),
    }
}

fn row_to_interface(row: &[DataValue]) -> crate::models::Interface {
    // columns: id(0), name(1), file_path(2), line_start(3), line_end(4),
    //          docstring(5), body_hash(6), token_count(7), embed_type(8),
    //          summary(9), body(10), embedding(11)
    crate::models::Interface {
        id: extract_str(row, 0),
        name: extract_str(row, 1),
        file_path: extract_str(row, 2),
        line_start: extract_u32(row, 3),
        line_end: extract_u32(row, 4),
        docstring: extract_opt_str(row, 5),
        body_hash: extract_str(row, 6),
        token_count: extract_u32(row, 7),
        embed_type: extract_str(row, 8),
        summary: extract_str(row, 9),
        body: extract_str(row, 10),
        embedding: extract_embedding(row, 11),
    }
}

fn row_to_commit_node(row: &[DataValue]) -> crate::models::CommitNode {
    // columns: id(0), hash(1), short_hash(2), author_name(3), author_email(4),
    //          timestamp(5), message(6), parent_hashes(7), changes(8)
    let ts = extract_i64(row, 5);
    let timestamp = chrono::DateTime::from_timestamp(ts, 0)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(Utc::now);
    let parent_hashes = match row.get(7) {
        Some(DataValue::List(v)) => v
            .iter()
            .filter_map(|dv| match dv {
                DataValue::Str(s) => Some(s.to_string()),
                _ => None,
            })
            .collect(),
        _ => vec![],
    };
    let changes = match row.get(8) {
        Some(DataValue::List(v)) => v
            .iter()
            .filter_map(|dv| match dv {
                DataValue::Str(s) => serde_json::from_str::<crate::models::ChangeRecord>(s).ok(),
                _ => None,
            })
            .collect(),
        _ => vec![],
    };
    crate::models::CommitNode {
        id: extract_str(row, 0),
        hash: extract_str(row, 1),
        short_hash: extract_str(row, 2),
        author_name: extract_str(row, 3),
        author_email: extract_str(row, 4),
        timestamp,
        message: extract_str(row, 6),
        parent_hashes,
        changes,
    }
}
