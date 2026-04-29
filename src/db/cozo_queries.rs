//! CozoDB query implementations — Phase 2 CRUD and count operations.
//!
//! Provides the same public API as the SurrealDB `queries.rs` module so that
//! call sites compile under `--features cozo-backend` without modification.
//! Methods that are fully implemented target the three-table vertical partition
//! layout defined in `cozo_backend::schema`.  Remaining methods return a
//! "not yet implemented" error and will be filled in Phase 3+.

#![allow(clippy::unused_async)]
#![allow(clippy::missing_errors_doc)]

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use cozo::{DataValue, Num, ScriptMutability};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::db::cozo_backend::{CozoDb, map_db_err};
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
                 use surreal-backend feature until Phase 3+"
            .into(),
    })
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

    /// Stub — not yet implemented.
    pub async fn delete_classes_by_file(&self, _file_path: &str) -> Result<(), EngramError> {
        Err(backend_err())
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
    pub async fn create_references_edge(
        &self,
        _source_id: &str,
        _target_id: &str,
        _qualified_name: Option<&str>,
    ) -> Result<(), EngramError> {
        Err(backend_err())
    }

    /// Stub — not yet implemented.
    pub async fn reresolve_references_edges(&self) -> Result<usize, EngramError> {
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

    /// Returns an empty list — symbol search against CozoDB is not yet implemented (Phase 3+).
    ///
    /// Returning `Ok(vec![])` lets callers correctly surface `SymbolNotFound` rather
    /// than a backend error when no match exists.
    pub async fn find_symbols_by_name(&self, _name: &str) -> Result<Vec<SymbolMatch>, EngramError> {
        Ok(vec![])
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
    pub async fn resolve_symbol(&self, _node_id: &str) -> Result<Option<SymbolMatch>, EngramError> {
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
    pub async fn delete_content_record_by_path(&self, _file_path: &str) -> Result<(), EngramError> {
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

// ── Row extraction helpers ────────────────────────────────────────────────

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
