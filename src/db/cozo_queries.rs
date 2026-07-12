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
use std::collections::HashSet;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use cozo::{DataValue, Num, ScriptMutability};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::db::cozo_backend::{CozoDb, map_db_err};
use crate::errors::{EngramError, SystemError};
use crate::models::TraversalDirection;
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
    if elapsed_ms > SLOW_QUERY_THRESHOLD_MS {
        tracing::warn!(
            query_type,
            table,
            result_count,
            elapsed_ms = elapsed_ms_u64,
            "slow query detected"
        );
    } else {
        tracing::info!(
            query_type,
            table,
            result_count,
            elapsed_ms = elapsed_ms_u64,
            "query completed"
        );
    }
}

// ── Mutable-script retry telemetry (040-F) ───────────────────────────────────

/// Monotonic count of SQLITE_BUSY retries in `run_script_busy_retry_mutable`.
static MUTABLE_RETRY_COUNT: AtomicU64 = AtomicU64::new(0);
/// Epoch-milliseconds timestamp of the most recent retry; `0` is the sentinel
/// meaning "no retry has ever occurred".
static MUTABLE_LAST_RETRY_EPOCH_MS: AtomicU64 = AtomicU64::new(0);

const SQLITE_BUSY_MAX_ATTEMPTS: u32 = 5;
const SQLITE_BUSY_INITIAL_DELAY_MS: u64 = 50;
const SQLITE_BUSY_MAX_DELAY_MS: u64 = 500;

/// Snapshot of mutable-script SQLITE_BUSY retry telemetry.
///
/// Exposed by the `get_mutable_script_retry_metrics` MCP tool.
#[derive(Debug, Clone, Serialize)]
pub struct RetryMetrics {
    /// Monotonic total of retries since the daemon started.
    pub retry_count: u64,
    /// Timestamp of the most recent retry, or `None` if no retry has occurred.
    pub last_retry_at: Option<DateTime<Utc>>,
}

/// Read the current mutable-script retry metrics from the process-global atomics.
///
/// The returned snapshot is point-in-time; concurrent retries may advance the
/// counter between consecutive calls. Use delta arithmetic, not absolute values,
/// when detecting new activity.
pub fn mutable_script_retry_metrics() -> RetryMetrics {
    let retry_count = MUTABLE_RETRY_COUNT.load(Ordering::Relaxed);
    let epoch_ms = MUTABLE_LAST_RETRY_EPOCH_MS.load(Ordering::Relaxed);
    let last_retry_at = (epoch_ms != 0)
        .then(|| {
            i64::try_from(epoch_ms)
                .ok()
                .and_then(DateTime::from_timestamp_millis)
        })
        .flatten();
    RetryMetrics {
        retry_count,
        last_retry_at,
    }
}

/// Zero both retry atomics.
///
/// **For test isolation only.** Do not call in production code.
#[cfg(test)]
pub(crate) fn reset_retry_metrics() {
    MUTABLE_RETRY_COUNT.store(0, Ordering::Relaxed);
    MUTABLE_LAST_RETRY_EPOCH_MS.store(0, Ordering::Relaxed);
}

fn is_busy_error(message: &str) -> bool {
    let normalized = message.to_lowercase();
    normalized.contains("locked") || normalized.contains("busy")
}

fn record_mutable_retry_telemetry() {
    MUTABLE_RETRY_COUNT.fetch_add(1, Ordering::Relaxed);
    // `0` is the sentinel for "no retry yet", so clamp to at least 1ms.
    // This guards against both clock-before-epoch (negative timestamp_millis,
    // where u64::try_from fails) and the exact Unix epoch (0ms), both of
    // which would otherwise write the sentinel and mask a real retry.
    let now_ms = u64::try_from(Utc::now().timestamp_millis())
        .unwrap_or(0)
        .max(1);
    MUTABLE_LAST_RETRY_EPOCH_MS.store(now_ms, Ordering::Relaxed);
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

/// A call site whose callee could not be resolved within the caller's own
/// file, staged for the deferred cross-file post-pass (082.002-T).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedCall {
    /// Fully-qualified ID of the calling function (e.g. `function:...`).
    pub caller_id: String,
    /// Bare name of the called function as it appears at the call site.
    pub callee_name: String,
    /// Workspace-relative path of the file the call site lives in.
    pub source_file: String,
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

/// A node in a structured `query_graph` result — code symbol or backlog artifact.
#[derive(Debug, Clone)]
pub struct QueryGraphNode {
    /// Full qualified ID (e.g., `fn:abc123`, `048-F`).
    pub id: String,
    /// Node kind: symbol table name (`function`, `class`, etc.) or `backlog_artifact`.
    pub kind: String,
    /// Display name.
    pub name: String,
    /// Workspace-relative file path (present for code symbols, absent for backlog artifacts).
    pub file_path: Option<String>,
}

/// Result of a structured `query_graph` neighborhood or transitive-closure operation.
#[derive(Debug)]
pub struct QueryGraphResult {
    /// Nodes discovered during traversal (excludes the root).
    pub nodes: Vec<QueryGraphNode>,
    /// Edges connecting the traversed nodes.
    pub edges: Vec<BfsEdge>,
    /// Whether traversal was truncated at `max_nodes`.
    pub truncated: bool,
}

/// Result of a `find_path` graph query.
#[derive(Debug)]
pub struct FindPathResult {
    /// Whether a path was found within `max_depth` hops.
    pub found: bool,
    /// Sequence of node IDs from `from` to `to` (inclusive). Empty when `found` is false.
    pub path: Vec<String>,
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

    /// Run a mutable script, retrying up to 5 times on `SQLITE_BUSY`.
    ///
    /// Each write to the three-table symbol layout (`*_meta`, `*_code`,
    /// `*_embedding`) can race with a concurrent write transaction.  Five
    /// attempts with 50 ms → 500 ms exponential back-off give ≈ 1.5 s of
    /// headroom — enough for `background_db_hydration` to release its
    /// write lock — without holding up the async executor for long.
    async fn run_script_busy_retry_mutable(
        &self,
        script: &str,
        params: BTreeMap<String, DataValue>,
    ) -> Result<cozo::NamedRows, EngramError> {
        let mut delay = std::time::Duration::from_millis(SQLITE_BUSY_INITIAL_DELAY_MS);
        for attempt in 0..SQLITE_BUSY_MAX_ATTEMPTS {
            match self
                .db
                .run_script(script, params.clone(), ScriptMutability::Mutable)
            {
                Ok(r) => return Ok(r),
                Err(e) => {
                    let msg = e.to_string();
                    if is_busy_error(&msg) && attempt + 1 < SQLITE_BUSY_MAX_ATTEMPTS {
                        tracing::warn!(
                            attempt = attempt + 1,
                            max_attempts = SQLITE_BUSY_MAX_ATTEMPTS,
                            delay_ms = delay.as_millis(),
                            error = %msg,
                            "SQLITE_BUSY retry: retrying mutable run_script"
                        );
                        record_mutable_retry_telemetry();
                        tokio::time::sleep(delay).await;
                        delay = (delay * 2)
                            .min(std::time::Duration::from_millis(SQLITE_BUSY_MAX_DELAY_MS));
                        continue;
                    }
                    return Err(map_db_err(msg));
                }
            }
        }
        Err(EngramError::System(SystemError::DatabaseError {
            reason: "SQLITE_BUSY retry loop terminated without executing an attempt".to_string(),
        }))
    }

    async fn run_script_busy_retry_immutable(
        &self,
        script: &str,
        params: BTreeMap<String, DataValue>,
    ) -> Result<cozo::NamedRows, EngramError> {
        let mut delay = std::time::Duration::from_millis(SQLITE_BUSY_INITIAL_DELAY_MS);
        for attempt in 0..SQLITE_BUSY_MAX_ATTEMPTS {
            match self
                .db
                .run_script(script, params.clone(), ScriptMutability::Immutable)
            {
                Ok(r) => return Ok(r),
                Err(e) => {
                    let msg = e.to_string();
                    if is_busy_error(&msg) && attempt + 1 < SQLITE_BUSY_MAX_ATTEMPTS {
                        tracing::warn!(
                            attempt = attempt + 1,
                            max_attempts = SQLITE_BUSY_MAX_ATTEMPTS,
                            delay_ms = delay.as_millis(),
                            error = %msg,
                            "SQLITE_BUSY retry: retrying immutable run_script"
                        );
                        tokio::time::sleep(delay).await;
                        delay = (delay * 2)
                            .min(std::time::Duration::from_millis(SQLITE_BUSY_MAX_DELAY_MS));
                        continue;
                    }
                    return Err(map_db_err(msg));
                }
            }
        }
        Err(EngramError::System(SystemError::DatabaseError {
            reason: "SQLITE_BUSY retry loop terminated without executing an attempt".to_string(),
        }))
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

    /// List every indexed function for the retrieval-eval corpus, left-joining
    /// the body and embedding (084.009-T / 78AA205D).
    ///
    /// Unlike [`Self::all_functions`] — an INNER join that silently drops a
    /// function whose `function_code` or `function_embedding` row is absent
    /// (a partial write, e.g. a `SQLITE_BUSY` mid-upsert; see
    /// [`Self::all_function_metas`]) — this returns every `function_meta` row so
    /// the semantic-eval denominator reflects *every* indexed function. A
    /// function lacking a code/embedding row is included with an empty
    /// `body` / `embedding`; the retrieval eval derives a bare-name query for it
    /// (`derive_query`) and scores it keyword-only. Functions that *do* carry a
    /// body/embedding are returned unchanged, so metrics over a fully-indexed
    /// corpus are identical to the INNER-join path.
    pub async fn all_functions_for_eval(
        &self,
    ) -> Result<Vec<crate::models::Function>, EngramError> {
        let script = r#"
fn_has_code[id] := *function_code { id }
fn_body[id, body] := *function_code { id, body }
fn_body[id, body] := *function_meta { id }, not fn_has_code[id], body = ''

fn_has_emb[id] := *function_embedding { id }
fn_emb[id, embedding] := *function_embedding { id, embedding }
fn_emb[id, embedding] := *function_meta { id }, not fn_has_emb[id], embedding = []

?[id, name, file_path, line_start, line_end, signature, docstring, body_hash,
  token_count, embed_type, summary, body, embedding] :=
    *function_meta { id, name, file_path, line_start, line_end, signature,
                     docstring, body_hash, token_count, embed_type, summary },
    fn_body[id, body],
    fn_emb[id, embedding]
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

    /// List all function records from `function_meta` only, using empty defaults
    /// for `body` and `embedding`.
    ///
    /// Used by dehydration as a fallback when `function_code` or
    /// `function_embedding` rows are absent due to a partial write (e.g. a
    /// `SQLITE_BUSY` failure mid-upsert). Callers that need the full body or a
    /// meaningful embedding should use [`Self::all_functions`] instead.
    pub async fn all_function_metas(&self) -> Result<Vec<crate::models::Function>, EngramError> {
        let script = r#"
?[id, name, file_path, line_start, line_end, signature, docstring, body_hash,
  token_count, embed_type, summary] :=
    *function_meta { id, name, file_path, line_start, line_end, signature,
                     docstring, body_hash, token_count, embed_type, summary }
"#;
        let result = self
            .db
            .run_script(script, BTreeMap::new(), ScriptMutability::Immutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        Ok(result
            .rows
            .iter()
            .map(|r| crate::models::Function {
                id: extract_str(r, 0),
                name: extract_str(r, 1),
                file_path: extract_str(r, 2),
                line_start: extract_u32(r, 3),
                line_end: extract_u32(r, 4),
                signature: extract_str(r, 5),
                docstring: extract_opt_str(r, 6),
                body: String::new(),
                body_hash: extract_str(r, 7),
                token_count: extract_u32(r, 8),
                embed_type: extract_str(r, 9),
                embedding: vec![0.0_f32; crate::services::embedding::EMBEDDING_DIM],
                summary: extract_str(r, 10),
            })
            .collect())
    }

    /// List all class records from `class_meta` only, using empty defaults for
    /// `body` and `embedding`. Dehydration fallback — see [`Self::all_function_metas`].
    pub async fn all_class_metas(&self) -> Result<Vec<crate::models::Class>, EngramError> {
        let script = r#"
?[id, name, file_path, line_start, line_end, docstring, body_hash,
  token_count, embed_type, summary] :=
    *class_meta { id, name, file_path, line_start, line_end, docstring,
                  body_hash, token_count, embed_type, summary }
"#;
        let result = self
            .db
            .run_script(script, BTreeMap::new(), ScriptMutability::Immutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        Ok(result
            .rows
            .iter()
            .map(|r| crate::models::Class {
                id: extract_str(r, 0),
                name: extract_str(r, 1),
                file_path: extract_str(r, 2),
                line_start: extract_u32(r, 3),
                line_end: extract_u32(r, 4),
                docstring: extract_opt_str(r, 5),
                body: String::new(),
                body_hash: extract_str(r, 6),
                token_count: extract_u32(r, 7),
                embed_type: extract_str(r, 8),
                embedding: vec![0.0_f32; crate::services::embedding::EMBEDDING_DIM],
                summary: extract_str(r, 9),
            })
            .collect())
    }

    /// List all interface records from `interface_meta` only, using empty
    /// defaults for `body` and `embedding`. Dehydration fallback — see
    /// [`Self::all_function_metas`].
    pub async fn all_interface_metas(&self) -> Result<Vec<crate::models::Interface>, EngramError> {
        let script = r#"
?[id, name, file_path, line_start, line_end, docstring, body_hash,
  token_count, embed_type, summary] :=
    *interface_meta { id, name, file_path, line_start, line_end, docstring,
                      body_hash, token_count, embed_type, summary }
"#;
        let result = self
            .db
            .run_script(script, BTreeMap::new(), ScriptMutability::Immutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        Ok(result
            .rows
            .iter()
            .map(|r| crate::models::Interface {
                id: extract_str(r, 0),
                name: extract_str(r, 1),
                file_path: extract_str(r, 2),
                line_start: extract_u32(r, 3),
                line_end: extract_u32(r, 4),
                docstring: extract_opt_str(r, 5),
                body: String::new(),
                body_hash: extract_str(r, 6),
                token_count: extract_u32(r, 7),
                embed_type: extract_str(r, 8),
                embedding: vec![0.0_f32; crate::services::embedding::EMBEDDING_DIM],
                summary: extract_str(r, 9),
            })
            .collect())
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
        self.run_script_busy_retry_mutable(meta_script, p)
            .await
            .map(|_| ())?;

        // Table 2: function_code
        let code_script = r#"
?[id, body] <- [[$id, $body]]
:put function_code { id, body }
"#;
        let mut p2 = BTreeMap::new();
        p2.insert("id".to_owned(), DataValue::from(func.id.as_str()));
        p2.insert("body".to_owned(), DataValue::from(func.body.as_str()));
        self.run_script_busy_retry_mutable(code_script, p2)
            .await
            .map(|_| ())?;
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
        self.run_script_busy_retry_mutable(embed_script, p3)
            .await
            .map(|_| ())?;

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
        self.run_script_busy_retry_mutable(meta, p)
            .await
            .map(|_| ())?;

        // Table 2: class_code
        let code_s = r#"
?[id, body] <- [[$id, $body]]
:put class_code { id, body }
"#;
        let mut p2 = BTreeMap::new();
        p2.insert("id".to_owned(), DataValue::from(class.id.as_str()));
        p2.insert("body".to_owned(), DataValue::from(class.body.as_str()));
        self.run_script_busy_retry_mutable(code_s, p2)
            .await
            .map(|_| ())?;

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
        self.run_script_busy_retry_mutable(embed_s, p3)
            .await
            .map(|_| ())?;

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
        self.run_script_busy_retry_mutable(meta, p)
            .await
            .map(|_| ())?;

        // Table 2: interface_code
        let code_s = r#"
?[id, body] <- [[$id, $body]]
:put interface_code { id, body }
"#;
        let mut p2 = BTreeMap::new();
        p2.insert("id".to_owned(), DataValue::from(iface.id.as_str()));
        p2.insert("body".to_owned(), DataValue::from(iface.body.as_str()));
        self.run_script_busy_retry_mutable(code_s, p2)
            .await
            .map(|_| ())?;

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
        self.run_script_busy_retry_mutable(embed_s, p3)
            .await
            .map(|_| ())?;

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

    /// Upsert a function-to-function call edge with `direct` provenance.
    ///
    /// This is the stable two-argument writer used by the ~30 in-file call
    /// resolution sites. It records the edge as `direct` (082.003-T); the
    /// cross-file post-pass uses [`Self::create_calls_edge_with_resolution`]
    /// with `calls_resolved_singleton` instead.
    #[allow(clippy::similar_names)]
    pub async fn create_calls_edge(
        &self,
        caller_id: &str,
        callee_id: &str,
    ) -> Result<(), EngramError> {
        self.create_calls_edge_with_resolution(caller_id, callee_id, "direct")
            .await
    }

    /// Upsert a function-to-function call edge with an explicit provenance
    /// value (082.003-T).
    ///
    /// `resolution` records how the edge was resolved: `direct` for an in-file
    /// resolved call, `calls_resolved_singleton` for a cross-file call resolved
    /// by the unambiguous-name post-pass (082.008-T). Keyed by `(from, to)`, so
    /// re-writing the same pair updates its provenance in place.
    #[allow(clippy::similar_names)]
    pub async fn create_calls_edge_with_resolution(
        &self,
        caller_id: &str,
        callee_id: &str,
        resolution: &str,
    ) -> Result<(), EngramError> {
        let ts = now_utc_str();
        let script = r#"
?[from, to, created_at, resolution] <- [[$from, $to, $created_at, $resolution]]
:put calls_edge { from, to => created_at, resolution }
"#;
        let mut p = BTreeMap::new();
        p.insert("from".to_owned(), DataValue::from(caller_id));
        p.insert("to".to_owned(), DataValue::from(callee_id));
        p.insert("created_at".to_owned(), DataValue::from(ts.as_str()));
        p.insert("resolution".to_owned(), DataValue::from(resolution));
        self.db
            .run_script(script, p, ScriptMutability::Mutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        Ok(())
    }

    /// Count `calls_edge` rows grouped by their `resolution` provenance value
    /// (082.003-T).
    ///
    /// Returns a map of provenance value (`direct`,
    /// `calls_resolved_singleton`, …) to the number of edges carrying it.
    pub async fn count_calls_edges_by_resolution(
        &self,
    ) -> Result<HashMap<String, u64>, EngramError> {
        let script = r#"
?[resolution, count(from)] := *calls_edge{from, resolution}
"#;
        let r = self
            .db
            .run_script(script, BTreeMap::new(), ScriptMutability::Immutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        let mut counts = HashMap::new();
        for row in &r.rows {
            let resolution = extract_str(row, 0);
            let count = u64::try_from(extract_i64(row, 1).max(0)).unwrap_or(0);
            counts.insert(resolution, count);
        }
        Ok(counts)
    }

    /// Enumerate every `(from, to)` pair whose provenance equals `resolution`
    /// (082.003-T).
    ///
    /// Used by the lifecycle (082.009-T) and rollback (082.010-T) paths to
    /// select edges resolved by a specific strategy — e.g.
    /// `calls_resolved_singleton` edges that must be retracted before a
    /// reindex.
    pub async fn list_calls_edges_by_resolution(
        &self,
        resolution: &str,
    ) -> Result<Vec<(String, String)>, EngramError> {
        let script = r#"
?[from, to] := *calls_edge{from, to, resolution}, resolution = $resolution
"#;
        let mut p = BTreeMap::new();
        p.insert("resolution".to_owned(), DataValue::from(resolution));
        let r = self
            .db
            .run_script(script, p, ScriptMutability::Immutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        Ok(r.rows
            .iter()
            .map(|row| (extract_str(row, 0), extract_str(row, 1)))
            .collect())
    }

    /// Retract every `calls_resolved_singleton` edge, preserving `direct`
    /// edges (082.010-T rollback step 1).
    ///
    /// Must run while the `resolution` column still exists. When the column is
    /// already absent (rollback already applied) this is a no-op returning `0`,
    /// keeping the rollback idempotent. Returns the number of edges retracted.
    pub async fn retract_all_calls_resolved_singleton_edges(&self) -> Result<usize, EngramError> {
        if !crate::db::cozo_backend::schema::calls_edge_has_resolution(&self.db)? {
            return Ok(0);
        }
        let singletons = self
            .list_calls_edges_by_resolution("calls_resolved_singleton")
            .await?;
        if singletons.is_empty() {
            return Ok(0);
        }
        let script = r#"
?[from, to] := *calls_edge{from, to, resolution}, resolution = "calls_resolved_singleton"
:rm calls_edge { from, to }
"#;
        self.db
            .run_script(script, BTreeMap::new(), ScriptMutability::Mutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        Ok(singletons.len())
    }

    /// Operator-invocable down-migration entry point (082.010-T).
    ///
    /// Orchestrates the rollback in a STRICT ORDER so every provenance query
    /// runs while its column still exists:
    ///   1. retract ALL `calls_resolved_singleton` edges (direct preserved)
    ///      while the `resolution` column is present;
    ///   2. THEN drop the `resolution` column, reverting `calls_edge` to
    ///      `{from, to => created_at}`.
    ///
    /// Idempotent: a second invocation retracts nothing and finds no column to
    /// drop, returning `0`. Returns the number of singleton edges retracted.
    /// The operator-invocable CLI trigger is 082.013-T.
    pub async fn rollback_calls_resolution(&self) -> Result<usize, EngramError> {
        let retracted = self.retract_all_calls_resolved_singleton_edges().await?;
        crate::db::cozo_backend::schema::rollback_calls_edge_resolution(&self.db)?;
        Ok(retracted)
    }

    /// Record a call site whose callee could not be resolved within the
    /// caller's own file, for the deferred cross-file post-pass (082.002-T).
    ///
    /// Keyed by `(caller_id, callee_name, source_file)`, so re-staging the same
    /// unresolved call is idempotent.
    pub async fn put_staged_call(
        &self,
        caller_id: &str,
        callee_name: &str,
        source_file: &str,
    ) -> Result<(), EngramError> {
        let ts = now_utc_str();
        let script = r#"
?[caller_id, callee_name, source_file, created_at] <-
    [[$caller_id, $callee_name, $source_file, $created_at]]
:put staged_call { caller_id, callee_name, source_file => created_at }
"#;
        let mut p = BTreeMap::new();
        p.insert("caller_id".to_owned(), DataValue::from(caller_id));
        p.insert("callee_name".to_owned(), DataValue::from(callee_name));
        p.insert("source_file".to_owned(), DataValue::from(source_file));
        p.insert("created_at".to_owned(), DataValue::from(ts.as_str()));
        self.db
            .run_script(script, p, ScriptMutability::Mutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        Ok(())
    }

    /// List every staged (unresolved) call site currently recorded.
    pub async fn list_staged_calls(&self) -> Result<Vec<StagedCall>, EngramError> {
        let script = r#"
?[caller_id, callee_name, source_file] :=
    *staged_call { caller_id, callee_name, source_file }
"#;
        let r = self
            .db
            .run_script(script, BTreeMap::new(), ScriptMutability::Immutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        Ok(r.rows
            .iter()
            .map(|row| StagedCall {
                caller_id: extract_str(row, 0),
                callee_name: extract_str(row, 1),
                source_file: extract_str(row, 2),
            })
            .collect())
    }

    /// Remove every staged (unresolved) call recorded for `source_file`
    /// (082.009-T clear-before-reindex / deletion cleanup).
    ///
    /// Clearing a file's prior staged rows before it is re-staged (or after it
    /// is deleted) prevents a stale unresolved call from being resolved into a
    /// stale edge by a later forced post-pass.
    pub async fn clear_staged_calls_for_file(&self, source_file: &str) -> Result<(), EngramError> {
        let script = r#"
?[caller_id, callee_name, source_file] :=
    *staged_call { caller_id, callee_name, source_file },
    source_file = $source_file
:rm staged_call { caller_id, callee_name, source_file }
"#;
        let mut p = BTreeMap::new();
        p.insert("source_file".to_owned(), DataValue::from(source_file));
        self.db
            .run_script(script, p, ScriptMutability::Mutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        Ok(())
    }

    /// Retract `calls_resolved_singleton` edges whose caller OR callee is a
    /// function defined in `file_path` (082.009-T).
    ///
    /// Must be invoked BEFORE the file's function metadata is deleted, because
    /// it maps file → function IDs via `function_meta.file_path`. Retracting
    /// these stale singleton edges before a reindex or deletion prevents
    /// dangling cross-file edges when a caller or callee changes or is removed.
    /// `direct` edges are left untouched — they are re-created by in-file
    /// resolution on reindex.
    pub async fn retract_resolved_calls_edges_for_file(
        &self,
        file_path: &str,
    ) -> Result<(), EngramError> {
        let script = r#"
stale[from, to] :=
    *calls_edge { from, to, resolution },
    resolution = "calls_resolved_singleton",
    *function_meta { id: from, file_path },
    file_path = $file_path
stale[from, to] :=
    *calls_edge { from, to, resolution },
    resolution = "calls_resolved_singleton",
    *function_meta { id: to, file_path },
    file_path = $file_path
?[from, to] := stale[from, to]
:rm calls_edge { from, to }
"#;
        let mut p = BTreeMap::new();
        p.insert("file_path".to_owned(), DataValue::from(file_path));
        self.db
            .run_script(script, p, ScriptMutability::Mutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        Ok(())
    }

    /// Retract any `calls_resolved_singleton` edge from `caller_id` whose callee
    /// is a function named `callee_name` (082.008-T targeted revalidation).
    ///
    /// Used by the cross-file post-pass to drop a singleton whose callee name is
    /// no longer unambiguously resolvable (zero or two-or-more definitions),
    /// without disturbing singletons for other callers or names. `direct` edges
    /// are untouched. A no-op when the `resolution` column is absent.
    pub async fn retract_singleton_edge_from_caller_by_name(
        &self,
        caller_id: &str,
        callee_name: &str,
    ) -> Result<(), EngramError> {
        if !crate::db::cozo_backend::schema::calls_edge_has_resolution(&self.db)? {
            return Ok(());
        }
        let script = r#"
?[from, to] :=
    *calls_edge { from, to, resolution },
    resolution = "calls_resolved_singleton",
    from = $from,
    *function_meta { id: to, name },
    name = $name
:rm calls_edge { from, to }
"#;
        let mut p = BTreeMap::new();
        p.insert("from".to_owned(), DataValue::from(caller_id));
        p.insert("name".to_owned(), DataValue::from(callee_name));
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

    /// Post-pass: resolve staged cross-file calls (082.008-T).
    ///
    /// Builds a workspace-global `name -> [function_id]` index from
    /// `function_meta`, then for each staged call (082.002-T) whose callee name
    /// matches **exactly one** function, creates a `calls_edge` tagged with the
    /// canonical provenance `calls_resolved_singleton`. Names with zero matches
    /// (no such function) or two-or-more matches (ambiguous) are skipped, which
    /// bounds false edges to the unambiguous-name case.
    ///
    /// Intended to run in the full / `--force` index path only; the incremental
    /// sync path does not invoke it (performance gate).
    ///
    /// Returns the number of *newly created* singleton edges (`resolved`) and
    /// the number of staged calls examined (`lookups`). `resolved` counts only
    /// edges that did not already carry `calls_resolved_singleton` provenance,
    /// so a no-op re-index (staged rows persist and their singletons are already
    /// present) reports `resolved == 0` rather than recounting every prior
    /// singleton as newly created. Callers surface `resolved` via
    /// `IndexResult.edges_created`.
    ///
    /// Revalidating, but non-destructive: for each staged call whose callee name
    /// resolves to exactly one function the singleton edge is upserted; for a
    /// staged call whose name resolves to zero or to two-or-more functions, any
    /// singleton previously resolved from that caller for that name is retracted
    /// (targeted revalidation), so a call that became ambiguous — or lost its
    /// unique target — does not leave a stale edge. Retraction is scoped to
    /// currently-staged callers only, so singleton edges whose staging was not
    /// repopulated (e.g. after JSONL rehydration or a fresh upgrade, where edges
    /// are restored but `staged_call` rows are not) are preserved rather than
    /// destroyed. `direct` (in-file) edges are never touched.
    pub async fn reresolve_calls_edges(&self) -> Result<ReresolveResult, EngramError> {
        // Workspace-global name -> [id] index (one round-trip).
        let script = r#"?[name, id] := *function_meta { id, name }"#;
        let r = self
            .db
            .run_script(script, BTreeMap::new(), ScriptMutability::Immutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        let mut name_index: HashMap<String, Vec<String>> = HashMap::new();
        for row in &r.rows {
            let name = extract_str(row, 0);
            let id = extract_str(row, 1);
            name_index.entry(name).or_default().push(id);
        }

        let staged = self.list_staged_calls().await?;
        let lookups = staged.len();
        // Snapshot the singleton edges that already exist so `resolved` counts
        // only genuinely new provenance. Because staged rows persist across a
        // non-forced re-index, the upsert below re-writes every prior singleton;
        // without this guard each such re-write would be recounted as a newly
        // created edge and over-report `IndexResult.edges_created`. The set also
        // dedupes repeated (caller, target) pairs within a single run.
        let mut existing: HashSet<(String, String)> = self
            .list_calls_edges_by_resolution("calls_resolved_singleton")
            .await?
            .into_iter()
            .collect();
        let mut resolved = 0usize;
        for call in &staged {
            // Resolve solely when a single function carries the callee name. A
            // zero or ambiguous (2+) match retracts any stale singleton this
            // caller previously had for the name, so revalidation stays targeted
            // and never touches singletons whose staging was not repopulated.
            match name_index.get(&call.callee_name) {
                Some(ids) if ids.len() == 1 => {
                    self.create_calls_edge_with_resolution(
                        &call.caller_id,
                        &ids[0],
                        "calls_resolved_singleton",
                    )
                    .await?;
                    // Count only the first time this (caller, target) pair is
                    // seen as a singleton — pre-existing edges and within-run
                    // duplicates are excluded.
                    if existing.insert((call.caller_id.clone(), ids[0].clone())) {
                        resolved += 1;
                    }
                }
                _ => {
                    self.retract_singleton_edge_from_caller_by_name(
                        &call.caller_id,
                        &call.callee_name,
                    )
                    .await?;
                }
            }
        }
        Ok(ReresolveResult { resolved, lookups })
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
        let start = std::time::Instant::now();
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
        crate::services::query_stats::record_timing(
            "symbol_lookup",
            u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX),
        );
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

        let start = std::time::Instant::now();

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
        crate::services::query_stats::record_timing(
            "hybrid_search",
            u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX),
        );
        Ok(scored)
    }

    /// Graph neighborhood — alias for `bfs_neighborhood`.
    pub async fn graph_neighborhood(
        &self,
        root_id: &str,
        max_depth: usize,
        max_nodes: usize,
    ) -> Result<BfsResult, EngramError> {
        let start = std::time::Instant::now();
        let result = self.bfs_impl(root_id, max_depth, max_nodes, &[]).await;
        crate::services::query_stats::record_timing(
            "graph_traversal",
            u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX),
        );
        result
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
            .run_script_busy_retry_immutable(script, BTreeMap::new())
            .await?;
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

    /// Return the total number of resolved `calls` edges (081-F numerator).
    pub async fn count_calls_edges(&self) -> Result<u64, EngramError> {
        let r = self
            .db
            .run_script(
                "?[count(from)] := *calls_edge { from }",
                BTreeMap::new(),
                ScriptMutability::Immutable,
            )
            .map_err(|e| map_db_err(e.to_string()))?;
        Ok(extract_count(&r))
    }

    /// Count resolved `calls` edges whose caller resides in a file of one of the
    /// given `languages` (084.002-T / D6F70DCC) — the language-gated 081-F
    /// numerator.
    ///
    /// Each edge is joined to its caller's (`from`) file language and retained
    /// only when that language matches one of `languages` (case-insensitive,
    /// mirroring the call-site denominator gate in
    /// [`crate::services::retrieval_eval::count_call_sites`]'s caller so
    /// `resolution_recall` numerator and denominator cover the same set of files).
    /// An empty `languages` slice disables the gate and counts every edge — parity
    /// with the denominator's opt-in behavior — so recall stays a ratio of
    /// commensurable, identically-scoped units. Counts distinct `(from, to)`
    /// relations (the `calls_edge` key); language matching is done in Rust rather
    /// than in-query to avoid depending on Cozo string builtins.
    pub async fn count_calls_edges_in_languages(
        &self,
        languages: &[String],
    ) -> Result<u64, EngramError> {
        if languages.is_empty() {
            return self.count_calls_edges().await;
        }
        // One row per edge, carrying the caller's file language. Both joins are
        // keyed lookups (`function_meta.id`, `file_node.path`), so the `(from,to)`
        // edge key stays unique across rows and counting matched rows counts
        // distinct call relations.
        let script = r#"
?[from, to, language] :=
    *calls_edge { from, to },
    *function_meta { id: from, file_path },
    *file_node { path: file_path, language }
"#;
        let r = self
            .db
            .run_script(script, BTreeMap::new(), ScriptMutability::Immutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        let matched = r
            .rows
            .iter()
            .filter(|row| {
                let language = extract_str(row, 2);
                languages
                    .iter()
                    .any(|lang| lang.eq_ignore_ascii_case(&language))
            })
            .count();
        Ok(u64::try_from(matched).unwrap_or(0))
    }

    /// Return the number of `calls` edges whose callee (`to`) matches no indexed
    /// function definition — a dangling / stale edge (081-F provenance read).
    ///
    /// `calls` edges are created only when the callee resolves to a function ID,
    /// so a `to` with no `function_meta` row indicates a stale reference (e.g.
    /// left over after a partial re-index). This count is a **conservative lower
    /// bound** on the false-edge rate: it catches dangling targets but not edges
    /// resolved to an existing-but-incorrect (ambiguous) definition. Detecting
    /// the latter needs retained callee provenance and is tracked follow-up work.
    pub async fn count_dangling_calls_edges(&self) -> Result<u64, EngramError> {
        let script = r#"
has_def[id] := *function_meta { id }
?[count(from)] := *calls_edge { from, to }, not has_def[to]
"#;
        let r = self
            .db
            .run_script(script, BTreeMap::new(), ScriptMutability::Immutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        Ok(extract_count(&r))
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
?[id, file_path, content_type, content_hash, content, source_path,
  file_size_bytes, ingested_at, record_kind, chunk_id, chunk_index,
  heading_path, line_start, line_end, fallback_reason, lint_summary,
  suggestions, embedding] <-
    [[$id, $file_path, $content_type, $content_hash, $content, $source_path,
      $file_size_bytes, $ingested_at, $record_kind, $chunk_id, $chunk_index,
      $heading_path, $line_start, $line_end, $fallback_reason, $lint_summary,
      $suggestions, $embedding]]
:put content_record {
    id => file_path, content_type, content_hash, content, source_path,
    file_size_bytes, ingested_at, record_kind, chunk_id, chunk_index,
    heading_path, line_start, line_end, fallback_reason, lint_summary,
    suggestions, embedding
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
        let heading_path = string_list_to_datavalue(&record.heading_path);
        let suggestions = string_list_to_datavalue(&record.suggestions);
        let mut p = BTreeMap::new();
        p.insert("id".to_owned(), DataValue::from(record.id.as_str()));
        p.insert(
            "file_path".to_owned(),
            DataValue::from(record.file_path.as_str()),
        );
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
        p.insert(
            "record_kind".to_owned(),
            DataValue::from(record.record_kind.as_str()),
        );
        p.insert(
            "chunk_id".to_owned(),
            DataValue::from(record.chunk_id.as_deref().unwrap_or("")),
        );
        p.insert(
            "chunk_index".to_owned(),
            optional_u32_to_datavalue(record.chunk_index),
        );
        p.insert("heading_path".to_owned(), heading_path);
        p.insert(
            "line_start".to_owned(),
            optional_u32_to_datavalue(record.line_start),
        );
        p.insert(
            "line_end".to_owned(),
            optional_u32_to_datavalue(record.line_end),
        );
        p.insert(
            "fallback_reason".to_owned(),
            DataValue::from(record.fallback_reason.as_deref().unwrap_or("")),
        );
        p.insert(
            "lint_summary".to_owned(),
            DataValue::from(record.lint_summary.as_deref().unwrap_or("")),
        );
        p.insert("suggestions".to_owned(), suggestions);
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
            r#"?[id, file_path, content_type, content_hash, content, source_path,
  file_size_bytes, ingested_at, record_kind, chunk_id, chunk_index,
  heading_path, line_start, line_end, fallback_reason, lint_summary,
  suggestions, embedding] :=
    *content_record {{ id, file_path, content_type, content_hash, content,
                      source_path, file_size_bytes, ingested_at, record_kind,
                      chunk_id, chunk_index, heading_path, line_start, line_end,
                      fallback_reason, lint_summary, suggestions, embedding }}{ct_clause}"#
        );
        let mut p = BTreeMap::new();
        if let Some(ct) = content_type {
            p.insert("content_type".to_owned(), DataValue::from(ct));
        }
        let result = self
            .db
            .run_script(&script, p, ScriptMutability::Immutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        let mut records = result
            .rows
            .iter()
            .map(|row| {
                let ingested_str = extract_str(row, 7);
                let ingested_at = chrono::DateTime::parse_from_rfc3339(&ingested_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());
                let emb = extract_embedding(row, 17);
                crate::models::ContentRecord {
                    id: extract_str(row, 0),
                    file_path: extract_str(row, 1),
                    content_type: extract_str(row, 2),
                    content_hash: extract_str(row, 3),
                    content: extract_str(row, 4),
                    source_path: extract_str(row, 5),
                    file_size_bytes: u64::try_from(extract_i64(row, 6).max(0)).unwrap_or(0),
                    ingested_at,
                    record_kind: extract_str(row, 8),
                    chunk_id: extract_opt_str(row, 9),
                    chunk_index: extract_opt_u32(row, 10),
                    heading_path: extract_string_list(row, 11),
                    line_start: extract_opt_u32(row, 12),
                    line_end: extract_opt_u32(row, 13),
                    fallback_reason: extract_opt_str(row, 14),
                    lint_summary: extract_opt_str(row, 15),
                    suggestions: extract_string_list(row, 16),
                    embedding: if emb.is_empty() { None } else { Some(emb) },
                }
            })
            .collect::<Vec<_>>();
        records.sort_by(|left, right| {
            left.file_path
                .cmp(&right.file_path)
                .then_with(|| left.chunk_index.cmp(&right.chunk_index))
                .then_with(|| left.id.cmp(&right.id))
        });
        Ok(records)
    }

    /// Update the embedding on an existing content record (keyed by `record_id` = `file_path`).
    /// Update the embedding for a content record identified by its `id` column.
    ///
    /// Looks up the record by `id` (not `file_path`) to match the shared query API
    /// and Surreal-backend semantics where callers pass the record's UUID/hash id.
    pub async fn update_content_record_embedding(
        &self,
        record_id: &str,
        embedding: Vec<f32>,
    ) -> Result<(), EngramError> {
        // Look up by `id` column (not the `file_path` key) to match caller semantics.
        let get = r#"
?[id, file_path, content_type, content_hash, content, source_path,
  file_size_bytes, ingested_at, record_kind, chunk_id, chunk_index,
  heading_path, line_start, line_end, fallback_reason, lint_summary,
  suggestions] :=
    *content_record { id, file_path, content_type, content_hash, content,
                      source_path, file_size_bytes, ingested_at, record_kind,
                      chunk_id, chunk_index, heading_path, line_start, line_end,
                      fallback_reason, lint_summary, suggestions },
    id = $id
"#;
        let mut gp = BTreeMap::new();
        gp.insert("id".to_owned(), DataValue::from(record_id));
        let existing = self
            .db
            .run_script(get, gp, ScriptMutability::Immutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        if existing.rows.is_empty() {
            return Ok(()); // nothing to update
        }
        let row = &existing.rows[0];
        let file_path = extract_str(row, 1);
        let record_kind = extract_str(row, 8);
        let chunk_id = extract_str(row, 9);
        let fallback_reason = extract_str(row, 14);
        let lint_summary = extract_str(row, 15);
        let heading_path = string_list_to_datavalue(&extract_string_list(row, 11));
        let suggestions = string_list_to_datavalue(&extract_string_list(row, 16));
        let emb_dv = DataValue::List(
            embedding
                .iter()
                .map(|&f| DataValue::Num(Num::Float(f64::from(f))))
                .collect(),
        );
        let put = r#"
?[id, file_path, content_type, content_hash, content, source_path,
  file_size_bytes, ingested_at, record_kind, chunk_id, chunk_index,
  heading_path, line_start, line_end, fallback_reason, lint_summary,
  suggestions, embedding] <-
    [[$id, $file_path, $ct, $ch, $content, $sp, $fsb, $ia, $record_kind,
      $chunk_id, $chunk_index, $heading_path, $line_start, $line_end,
      $fallback_reason, $lint_summary, $suggestions, $embedding]]
:put content_record {
    id => file_path, content_type, content_hash, content, source_path,
    file_size_bytes, ingested_at, record_kind, chunk_id, chunk_index,
    heading_path, line_start, line_end, fallback_reason, lint_summary,
    suggestions, embedding
}
"#;
        let mut p = BTreeMap::new();
        p.insert("file_path".to_owned(), DataValue::from(file_path.as_str()));
        p.insert("id".to_owned(), DataValue::from(record_id));
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
        p.insert(
            "record_kind".to_owned(),
            DataValue::from(record_kind.as_str()),
        );
        p.insert("chunk_id".to_owned(), DataValue::from(chunk_id.as_str()));
        p.insert(
            "chunk_index".to_owned(),
            optional_u32_to_datavalue(extract_opt_u32(row, 10)),
        );
        p.insert("heading_path".to_owned(), heading_path);
        p.insert(
            "line_start".to_owned(),
            optional_u32_to_datavalue(extract_opt_u32(row, 12)),
        );
        p.insert(
            "line_end".to_owned(),
            optional_u32_to_datavalue(extract_opt_u32(row, 13)),
        );
        p.insert(
            "fallback_reason".to_owned(),
            DataValue::from(fallback_reason.as_str()),
        );
        p.insert(
            "lint_summary".to_owned(),
            DataValue::from(lint_summary.as_str()),
        );
        p.insert("suggestions".to_owned(), suggestions);
        p.insert("embedding".to_owned(), emb_dv);
        self.db
            .run_script(put, p, ScriptMutability::Mutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        Ok(())
    }

    /// Delete content records for a single `(file_path, content_type, source_path)` scope.
    pub async fn delete_content_records_by_scope(
        &self,
        file_path: &str,
        content_type: &str,
        source_path: &str,
    ) -> Result<(), EngramError> {
        let script = r#"
?[id] :=
    *content_record { id, file_path, content_type, source_path },
    file_path = $file_path,
    content_type = $content_type,
    source_path = $source_path
:rm content_record { id }
"#;
        let mut p = BTreeMap::new();
        p.insert("file_path".to_owned(), DataValue::from(file_path));
        p.insert("content_type".to_owned(), DataValue::from(content_type));
        p.insert("source_path".to_owned(), DataValue::from(source_path));
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
        self.run_script_busy_retry_mutable(script, p).await?;
        Ok(())
    }

    /// Return all file hash records.
    pub async fn get_all_file_hashes(&self) -> Result<Vec<FileHashRecord>, EngramError> {
        let script = r#"
?[file_path, content_hash, size_bytes, recorded_at] :=
    *file_hash { file_path, content_hash, size_bytes, recorded_at }
"#;
        let r = self
            .run_script_busy_retry_immutable(script, BTreeMap::new())
            .await?;
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
        self.run_script_busy_retry_mutable(script, p).await?;
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
        } else if table == "calls_edge" {
            // calls_edge carries `resolution` provenance (082.011-T): project it
            // so exported edges retain `direct` / `calls_resolved_singleton`
            // end-to-end through dehydration.
            "?[from, to, created_at, resolution] := *calls_edge { from, to, created_at, resolution }".to_owned()
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
                    resolution: None,
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
                    resolution: None,
                    created_at: extract_str(row, 3),
                })
                .collect())
        } else if table == "calls_edge" {
            // [from, to, created_at, resolution] — carry provenance on export.
            Ok(r.rows
                .iter()
                .map(|row| crate::models::CodeEdge {
                    edge_type: edge_type.clone(),
                    from: extract_str(row, 0),
                    to: extract_str(row, 1),
                    import_path: None,
                    linked_by: None,
                    resolution: {
                        let s = extract_str(row, 3);
                        if s.is_empty() { None } else { Some(s) }
                    },
                    created_at: extract_str(row, 2),
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
                    resolution: None,
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

    // ── Structured query_graph traversal ─────────────────────────────────────

    /// Resolve a backlog artifact ID to a [`QueryGraphNode`] using `backlog_node` metadata.
    ///
    /// Returns `None` when the ID is not found in `backlog_node`, so callers can
    /// fall back to a bare-ID node if needed.
    async fn resolve_backlog_node(&self, id: &str) -> Result<Option<QueryGraphNode>, EngramError> {
        let script = "?[id, title, kind, file_path] := *backlog_node { id, title, kind, file_path }, id = $id";
        let mut p = BTreeMap::new();
        p.insert("id".to_owned(), DataValue::from(id));
        let result = self
            .db
            .run_script(script, p, ScriptMutability::Immutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        if result.rows.is_empty() {
            return Ok(None);
        }
        let row = &result.rows[0];
        let node_id = extract_str(row, 0);
        let title = extract_str(row, 1);
        let kind = extract_str(row, 2);
        let file_path_val = extract_str(row, 3);
        Ok(Some(QueryGraphNode {
            id: node_id,
            kind: if kind.is_empty() {
                "backlog_artifact".to_owned()
            } else {
                kind
            },
            name: if title.is_empty() {
                id.to_owned()
            } else {
                title
            },
            file_path: if file_path_val.is_empty() {
                None
            } else {
                Some(file_path_val)
            },
        }))
    }

    /// BFS traversal with direction control and backlog edge support — used by
    /// `query_graph_neighborhood`, `transitive_closure`, and `find_path`.
    ///
    /// Extends [`bfs_impl`] with:
    /// - `direction`: `Outgoing`, `Incoming`, or `Both` (default in `bfs_impl`)
    /// - Backlog edge types (`parent_of`, `depends_on`, `backlog_references`)
    /// - Code `references` edge support (excluded from `bfs_impl` for SurrealDB parity)
    ///
    /// Results use [`QueryGraphNode`] rather than [`SymbolMatch`] to unify
    /// code symbols and backlog artifacts in a single traversal result.
    async fn bfs_directed_impl(
        &self,
        root_id: &str,
        max_depth: usize,
        max_nodes: usize,
        allowed_edge_types: &[&str],
        direction: TraversalDirection,
    ) -> Result<QueryGraphResult, EngramError> {
        // (edge_type_label, table_name) — concerns_edge is detected by table name at runtime
        const CODE_EDGE_TABLES: &[(&str, &str)] = &[
            ("calls", "calls_edge"),
            ("imports", "imports_edge"),
            ("defines", "defines_edge"),
            ("inherits_from", "inherits_from_edge"),
            ("concerns", "concerns_edge"),
            ("references", "references_edge"),
        ];
        // (api_label, db_edge_type_value) — all routed through `backlog_edge` table
        const BACKLOG_EDGE_TYPES: &[(&str, &str)] = &[
            ("parent_of", "parent_of"),
            ("depends_on", "depends_on"),
            ("backlog_references", "references"),
        ];
        // (api_label, db_edge_type_value) — all routed through `powerbi_edge` table
        const POWERBI_EDGE_TYPES: &[(&str, &str)] = &[
            ("pbi_contains", "pbi_contains"),
            ("pbi_uses_field", "pbi_uses_field"),
            ("pbi_depends_on_model", "pbi_depends_on_model"),
            ("pbi_belongs_to_report", "pbi_belongs_to_report"),
            ("pbi_relates_to_table", "pbi_relates_to_table"),
        ];

        let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut nodes: Vec<QueryGraphNode> = Vec::new();
        let mut edges: Vec<BfsEdge> = Vec::new();
        let mut frontier: Vec<String> = vec![root_id.to_owned()];
        visited.insert(root_id.to_owned());
        let mut truncated = false;

        let traverse_out = matches!(
            direction,
            TraversalDirection::Outgoing | TraversalDirection::Both
        );
        let traverse_in = matches!(
            direction,
            TraversalDirection::Incoming | TraversalDirection::Both
        );

        'outer: for _ in 0..max_depth {
            if frontier.is_empty() {
                break;
            }
            let mut next_frontier: Vec<String> = Vec::new();

            for node in &frontier {
                // ── Code edges ──────────────────────────────────────────────
                for (et, tbl) in CODE_EDGE_TABLES {
                    if !allowed_edge_types.is_empty() && !allowed_edge_types.contains(et) {
                        continue;
                    }
                    // concerns_edge uses task_id/symbol_id instead of from/to.
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

                    if traverse_out {
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
                                if nodes.len() >= max_nodes {
                                    truncated = true;
                                    break 'outer;
                                }
                                edges.push(BfsEdge {
                                    edge_type: (*et).to_owned(),
                                    from: node.clone(),
                                    to: target.clone(),
                                });
                                visited.insert(target.clone());
                                nodes.push(QueryGraphNode {
                                    id: sym.id,
                                    kind: sym.table,
                                    name: sym.name,
                                    file_path: Some(sym.file_path),
                                });
                                next_frontier.push(target);
                            }
                        }
                    }

                    if traverse_in {
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
                            // For concerns_edge incoming, column 0 is `task_id` (a
                            // backlog artifact ID). Try backlog first, fall back to
                            // code-symbol resolution for other edge types.
                            let graph_node = if *tbl == "concerns_edge" {
                                self.resolve_backlog_node(&source)
                                    .await
                                    .ok()
                                    .flatten()
                                    .or_else(|| {
                                        // Bare-ID fallback so the node is not silently dropped.
                                        Some(QueryGraphNode {
                                            id: source.clone(),
                                            kind: "backlog_artifact".to_owned(),
                                            name: source.clone(),
                                            file_path: None,
                                        })
                                    })
                            } else {
                                self.resolve_symbol(&source)
                                    .await
                                    .ok()
                                    .flatten()
                                    .map(|sym| QueryGraphNode {
                                        id: sym.id,
                                        kind: sym.table,
                                        name: sym.name,
                                        file_path: Some(sym.file_path),
                                    })
                            };
                            if let Some(gn) = graph_node {
                                if nodes.len() >= max_nodes {
                                    truncated = true;
                                    break 'outer;
                                }
                                edges.push(BfsEdge {
                                    edge_type: (*et).to_owned(),
                                    from: source.clone(),
                                    to: node.clone(),
                                });
                                visited.insert(source.clone());
                                nodes.push(gn);
                                next_frontier.push(source);
                            }
                        }
                    }
                }

                // ── Backlog edges ────────────────────────────────────────────
                for (api_label, db_et) in BACKLOG_EDGE_TYPES {
                    if !allowed_edge_types.is_empty() && !allowed_edge_types.contains(api_label) {
                        continue;
                    }
                    let out_script =
                        "?[from, to] := *backlog_edge { from_id, to_id, edge_type }, from_id = $node, from = from_id, to = to_id, edge_type = $et"
                            .to_owned();
                    let in_script =
                        "?[from, to] := *backlog_edge { from_id, to_id, edge_type }, to_id = $node, from = from_id, to = to_id, edge_type = $et"
                            .to_owned();

                    if traverse_out {
                        let mut p = BTreeMap::new();
                        p.insert("node".to_owned(), DataValue::from(node.as_str()));
                        p.insert("et".to_owned(), DataValue::from(*db_et));
                        let r = self
                            .db
                            .run_script(&out_script, p, ScriptMutability::Immutable)
                            .map_err(|e| map_db_err(e.to_string()))?;
                        for row in &r.rows {
                            let target = extract_str(row, 1);
                            if visited.contains(&target) {
                                continue;
                            }
                            if nodes.len() >= max_nodes {
                                truncated = true;
                                break 'outer;
                            }
                            edges.push(BfsEdge {
                                edge_type: (*api_label).to_owned(),
                                from: node.clone(),
                                to: target.clone(),
                            });
                            visited.insert(target.clone());
                            // Enrich with backlog_node metadata; fall back to bare ID.
                            let gn = self
                                .resolve_backlog_node(&target)
                                .await
                                .ok()
                                .flatten()
                                .unwrap_or_else(|| QueryGraphNode {
                                    id: target.clone(),
                                    kind: "backlog_artifact".to_owned(),
                                    name: target.clone(),
                                    file_path: None,
                                });
                            nodes.push(gn);
                            next_frontier.push(target);
                        }
                    }

                    if traverse_in {
                        let mut p2 = BTreeMap::new();
                        p2.insert("node".to_owned(), DataValue::from(node.as_str()));
                        p2.insert("et".to_owned(), DataValue::from(*db_et));
                        let r2 = self
                            .db
                            .run_script(&in_script, p2, ScriptMutability::Immutable)
                            .map_err(|e| map_db_err(e.to_string()))?;
                        for row in &r2.rows {
                            let source = extract_str(row, 0);
                            if visited.contains(&source) {
                                continue;
                            }
                            if nodes.len() >= max_nodes {
                                truncated = true;
                                break 'outer;
                            }
                            edges.push(BfsEdge {
                                edge_type: (*api_label).to_owned(),
                                from: source.clone(),
                                to: node.clone(),
                            });
                            visited.insert(source.clone());
                            // Enrich with backlog_node metadata; fall back to bare ID.
                            let gn = self
                                .resolve_backlog_node(&source)
                                .await
                                .ok()
                                .flatten()
                                .unwrap_or_else(|| QueryGraphNode {
                                    id: source.clone(),
                                    kind: "backlog_artifact".to_owned(),
                                    name: source.clone(),
                                    file_path: None,
                                });
                            nodes.push(gn);
                            next_frontier.push(source);
                        }
                    }
                }

                // ── Power BI edges ──────────────────────────────────────────
                for (api_label, db_et) in POWERBI_EDGE_TYPES {
                    if !allowed_edge_types.is_empty() && !allowed_edge_types.contains(api_label) {
                        continue;
                    }
                    let out_script =
                        "?[from, to] := *powerbi_edge { from_id, to_id, edge_type }, from_id = $node, from = from_id, to = to_id, edge_type = $et"
                            .to_owned();
                    let in_script =
                        "?[from, to] := *powerbi_edge { from_id, to_id, edge_type }, to_id = $node, from = from_id, to = to_id, edge_type = $et"
                            .to_owned();

                    if traverse_out {
                        let mut p = BTreeMap::new();
                        p.insert("node".to_owned(), DataValue::from(node.as_str()));
                        p.insert("et".to_owned(), DataValue::from(*db_et));
                        let r = self
                            .db
                            .run_script(&out_script, p, ScriptMutability::Immutable)
                            .map_err(|e| map_db_err(e.to_string()))?;
                        for row in &r.rows {
                            let target = extract_str(row, 1);
                            if visited.contains(&target) {
                                continue;
                            }
                            if nodes.len() >= max_nodes {
                                truncated = true;
                                break 'outer;
                            }
                            edges.push(BfsEdge {
                                edge_type: (*api_label).to_owned(),
                                from: node.clone(),
                                to: target.clone(),
                            });
                            visited.insert(target.clone());
                            let gn = self
                                .resolve_powerbi_node(&target)
                                .await
                                .ok()
                                .flatten()
                                .unwrap_or_else(|| QueryGraphNode {
                                    id: target.clone(),
                                    kind: "powerbi_entity".to_owned(),
                                    name: target.clone(),
                                    file_path: None,
                                });
                            nodes.push(gn);
                            next_frontier.push(target);
                        }
                    }

                    if traverse_in {
                        let mut p2 = BTreeMap::new();
                        p2.insert("node".to_owned(), DataValue::from(node.as_str()));
                        p2.insert("et".to_owned(), DataValue::from(*db_et));
                        let r2 = self
                            .db
                            .run_script(&in_script, p2, ScriptMutability::Immutable)
                            .map_err(|e| map_db_err(e.to_string()))?;
                        for row in &r2.rows {
                            let source = extract_str(row, 0);
                            if visited.contains(&source) {
                                continue;
                            }
                            if nodes.len() >= max_nodes {
                                truncated = true;
                                break 'outer;
                            }
                            edges.push(BfsEdge {
                                edge_type: (*api_label).to_owned(),
                                from: source.clone(),
                                to: node.clone(),
                            });
                            visited.insert(source.clone());
                            let gn = self
                                .resolve_powerbi_node(&source)
                                .await
                                .ok()
                                .flatten()
                                .unwrap_or_else(|| QueryGraphNode {
                                    id: source.clone(),
                                    kind: "powerbi_entity".to_owned(),
                                    name: source.clone(),
                                    file_path: None,
                                });
                            nodes.push(gn);
                            next_frontier.push(source);
                        }
                    }
                }
            }
            frontier = next_frontier;
        }

        Ok(QueryGraphResult {
            nodes,
            edges,
            truncated,
        })
    }

    /// Structured graph neighborhood — BFS from `root_id` up to `max_depth` hops.
    ///
    /// Supports bidirectional or directed traversal via `direction`.
    /// Pass an empty `edge_types` slice to traverse all available edge types
    /// (code + backlog). Non-empty slices restrict traversal to the named types.
    ///
    /// Code edge types: `calls`, `imports`, `defines`, `inherits_from`, `concerns`, `references`.
    /// Backlog edge types: `parent_of`, `depends_on`, `backlog_references`.
    pub async fn query_graph_neighborhood(
        &self,
        root_id: &str,
        direction: TraversalDirection,
        max_depth: usize,
        max_nodes: usize,
        edge_types: &[&str],
    ) -> Result<QueryGraphResult, EngramError> {
        let start = std::time::Instant::now();
        let result = self
            .bfs_directed_impl(root_id, max_depth, max_nodes, edge_types, direction)
            .await;
        crate::services::query_stats::record_timing(
            "query_graph_neighborhood",
            u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX),
        );
        result
    }

    /// Transitive closure — all nodes reachable from `root_id` following outgoing edges.
    ///
    /// Unlike `query_graph_neighborhood`, only forward (outgoing) edges are traversed.
    /// Results include both code symbols and backlog artifacts reachable in `max_depth`
    /// hops respecting the `edge_types` filter.
    pub async fn transitive_closure(
        &self,
        root_id: &str,
        max_depth: usize,
        max_nodes: usize,
        edge_types: &[&str],
    ) -> Result<QueryGraphResult, EngramError> {
        let start = std::time::Instant::now();
        let result = self
            .bfs_directed_impl(
                root_id,
                max_depth,
                max_nodes,
                edge_types,
                TraversalDirection::Outgoing,
            )
            .await;
        crate::services::query_stats::record_timing(
            "transitive_closure",
            u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX),
        );
        result
    }

    /// BFS shortest-path search from `from_id` to `to_id` via outgoing edges.
    ///
    /// Returns the sequence of node IDs on the shortest path (inclusive of both
    /// endpoints). Returns `found: false` when no path exists within `max_depth` hops.
    ///
    /// Only forward (outgoing) edges are traversed. Pass an empty `edge_types`
    /// slice to allow all edge types.
    pub async fn find_path(
        &self,
        from_id: &str,
        to_id: &str,
        max_depth: usize,
        edge_types: &[&str],
    ) -> Result<FindPathResult, EngramError> {
        const CODE_EDGE_TABLES: &[(&str, &str)] = &[
            ("calls", "calls_edge"),
            ("imports", "imports_edge"),
            ("defines", "defines_edge"),
            ("inherits_from", "inherits_from_edge"),
            ("concerns", "concerns_edge"),
            ("references", "references_edge"),
        ];
        const BACKLOG_EDGE_TYPES: &[(&str, &str)] = &[
            ("parent_of", "parent_of"),
            ("depends_on", "depends_on"),
            ("backlog_references", "references"),
        ];
        const POWERBI_EDGE_TYPES_FP: &[(&str, &str)] = &[
            ("pbi_contains", "pbi_contains"),
            ("pbi_uses_field", "pbi_uses_field"),
            ("pbi_depends_on_model", "pbi_depends_on_model"),
            ("pbi_belongs_to_report", "pbi_belongs_to_report"),
            ("pbi_relates_to_table", "pbi_relates_to_table"),
        ];

        if from_id == to_id {
            return Ok(FindPathResult {
                found: true,
                path: vec![from_id.to_owned()],
            });
        }

        let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut parent: HashMap<String, String> = HashMap::new();
        let mut frontier: Vec<String> = vec![from_id.to_owned()];
        visited.insert(from_id.to_owned());
        let mut found = false;

        'outer: for _ in 0..max_depth {
            if frontier.is_empty() {
                break;
            }
            let mut next_frontier: Vec<String> = Vec::new();

            for node in &frontier {
                // Code edges (outgoing only for path finding)
                for (et, tbl) in CODE_EDGE_TABLES {
                    if !edge_types.is_empty() && !edge_types.contains(et) {
                        continue;
                    }
                    let script = if *tbl == "concerns_edge" {
                        "?[from, to] := *concerns_edge { task_id, symbol_id }, task_id = $node, from = task_id, to = symbol_id".to_owned()
                    } else {
                        format!("?[from, to] := *{tbl} {{ from, to }}, from = $node")
                    };
                    let mut p = BTreeMap::new();
                    p.insert("node".to_owned(), DataValue::from(node.as_str()));
                    let r = self
                        .db
                        .run_script(&script, p, ScriptMutability::Immutable)
                        .map_err(|e| map_db_err(e.to_string()))?;
                    for row in &r.rows {
                        let target = extract_str(row, 1);
                        if visited.contains(&target) {
                            continue;
                        }
                        parent.insert(target.clone(), node.clone());
                        visited.insert(target.clone());
                        if target == to_id {
                            found = true;
                            break 'outer;
                        }
                        next_frontier.push(target);
                    }
                }

                // Backlog edges (outgoing only)
                for (api_label, db_et) in BACKLOG_EDGE_TYPES {
                    if !edge_types.is_empty() && !edge_types.contains(api_label) {
                        continue;
                    }
                    let script =
                        "?[from, to] := *backlog_edge { from_id, to_id, edge_type }, from_id = $node, from = from_id, to = to_id, edge_type = $et"
                            .to_owned();
                    let mut p = BTreeMap::new();
                    p.insert("node".to_owned(), DataValue::from(node.as_str()));
                    p.insert("et".to_owned(), DataValue::from(*db_et));
                    let r = self
                        .db
                        .run_script(&script, p, ScriptMutability::Immutable)
                        .map_err(|e| map_db_err(e.to_string()))?;
                    for row in &r.rows {
                        let target = extract_str(row, 1);
                        if visited.contains(&target) {
                            continue;
                        }
                        parent.insert(target.clone(), node.clone());
                        visited.insert(target.clone());
                        if target == to_id {
                            found = true;
                            break 'outer;
                        }
                        next_frontier.push(target);
                    }
                }

                // Power BI edges (outgoing only)
                for (api_label, db_et) in POWERBI_EDGE_TYPES_FP {
                    if !edge_types.is_empty() && !edge_types.contains(api_label) {
                        continue;
                    }
                    let script =
                        "?[from, to] := *powerbi_edge { from_id, to_id, edge_type }, from_id = $node, from = from_id, to = to_id, edge_type = $et"
                            .to_owned();
                    let mut p = BTreeMap::new();
                    p.insert("node".to_owned(), DataValue::from(node.as_str()));
                    p.insert("et".to_owned(), DataValue::from(*db_et));
                    let r = self
                        .db
                        .run_script(&script, p, ScriptMutability::Immutable)
                        .map_err(|e| map_db_err(e.to_string()))?;
                    for row in &r.rows {
                        let target = extract_str(row, 1);
                        if visited.contains(&target) {
                            continue;
                        }
                        parent.insert(target.clone(), node.clone());
                        visited.insert(target.clone());
                        if target == to_id {
                            found = true;
                            break 'outer;
                        }
                        next_frontier.push(target);
                    }
                }
            }
            frontier = next_frontier;
        }

        if found {
            // Reconstruct path from parent map (reverse walk from `to_id` to `from_id`).
            let mut path = vec![to_id.to_owned()];
            let mut cur = to_id.to_owned();
            while cur != from_id {
                if let Some(p) = parent.get(&cur) {
                    cur = p.clone();
                    path.push(cur.clone());
                } else {
                    break;
                }
            }
            path.reverse();
            Ok(FindPathResult { found: true, path })
        } else {
            Ok(FindPathResult {
                found: false,
                path: Vec::new(),
            })
        }
    }
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

    // ── Backlog node queries (002-F) ─────────────────────────────────

    /// Batch upsert backlog nodes into `backlog_node`.
    ///
    /// Each node is inserted as a separate mutable script with SQLITE_BUSY
    /// retry (per compound learning on per-statement retry granularity).
    pub async fn upsert_backlog_nodes(
        &self,
        nodes: &[crate::models::BacklogNode],
    ) -> Result<(), EngramError> {
        let script = r#"
?[id, title, kind, status, labels, file_path, content_hash, source_path, ingested_at] <-
    [[$id, $title, $kind, $status, $labels, $file_path, $content_hash, $source_path, $ingested_at]]
:put backlog_node {
    id => title, kind, status, labels, file_path, content_hash, source_path, ingested_at
}
"#;
        for node in nodes {
            let labels_str = node.labels.join(",");
            let mut p = BTreeMap::new();
            p.insert("id".to_owned(), DataValue::from(node.id.as_str()));
            p.insert("title".to_owned(), DataValue::from(node.title.as_str()));
            p.insert("kind".to_owned(), DataValue::from(node.kind.as_str()));
            p.insert("status".to_owned(), DataValue::from(node.status.as_str()));
            p.insert("labels".to_owned(), DataValue::from(labels_str.as_str()));
            p.insert(
                "file_path".to_owned(),
                DataValue::from(node.file_path.as_str()),
            );
            p.insert(
                "content_hash".to_owned(),
                DataValue::from(node.content_hash.as_str()),
            );
            p.insert(
                "source_path".to_owned(),
                DataValue::from(node.source_path.as_str()),
            );
            p.insert(
                "ingested_at".to_owned(),
                DataValue::from(node.ingested_at.to_rfc3339().as_str()),
            );
            self.run_script_busy_retry_mutable(script, p).await?;
        }
        Ok(())
    }

    /// Batch upsert backlog edges into `backlog_edge`.
    pub async fn upsert_backlog_edges(
        &self,
        edges: &[crate::models::BacklogEdge],
    ) -> Result<(), EngramError> {
        let script = r#"
?[from_id, to_id, edge_type, source_path] <-
    [[$from_id, $to_id, $edge_type, $source_path]]
:put backlog_edge {
    from_id, to_id, edge_type => source_path
}
"#;
        for edge in edges {
            let mut p = BTreeMap::new();
            p.insert("from_id".to_owned(), DataValue::from(edge.from_id.as_str()));
            p.insert("to_id".to_owned(), DataValue::from(edge.to_id.as_str()));
            p.insert(
                "edge_type".to_owned(),
                DataValue::from(edge.edge_type.as_str()),
            );
            p.insert(
                "source_path".to_owned(),
                DataValue::from(edge.source_path.as_str()),
            );
            self.run_script_busy_retry_mutable(script, p).await?;
        }
        Ok(())
    }

    /// Batch upsert backlog content records into `backlog_content_record`.
    pub async fn upsert_backlog_content_records(
        &self,
        records: &[crate::models::BacklogContentRecord],
    ) -> Result<(), EngramError> {
        let script = r#"
?[file_path, content_type, content_hash, content, source_path, ingested_at] <-
    [[$file_path, $content_type, $content_hash, $content, $source_path, $ingested_at]]
:put backlog_content_record {
    file_path => content_type, content_hash, content, source_path, ingested_at
}
"#;
        for record in records {
            let mut p = BTreeMap::new();
            p.insert(
                "file_path".to_owned(),
                DataValue::from(record.file_path.as_str()),
            );
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
                "ingested_at".to_owned(),
                DataValue::from(record.ingested_at.to_rfc3339().as_str()),
            );
            self.run_script_busy_retry_mutable(script, p).await?;
        }
        Ok(())
    }

    /// Return all backlog nodes, optionally filtered by `source_path`.
    pub async fn select_backlog_nodes(
        &self,
        source_path: Option<&str>,
    ) -> Result<Vec<crate::models::BacklogNode>, EngramError> {
        let sp_clause = source_path
            .map(|_| ", source_path = $source_path")
            .unwrap_or("");
        let script = format!(
            r#"?[id, title, kind, status, labels, file_path, content_hash, source_path, ingested_at] :=
    *backlog_node {{ id, title, kind, status, labels, file_path, content_hash, source_path, ingested_at }}{sp_clause}"#
        );
        let mut p = BTreeMap::new();
        if let Some(sp) = source_path {
            p.insert("source_path".to_owned(), DataValue::from(sp));
        }
        let result = self
            .db
            .run_script(&script, p, ScriptMutability::Immutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        result
            .rows
            .iter()
            .map(|row| {
                let ingested_str = extract_str(row, 8);
                let ingested_at = chrono::DateTime::parse_from_rfc3339(&ingested_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());
                let labels_str = extract_str(row, 4);
                let labels = if labels_str.is_empty() {
                    vec![]
                } else {
                    labels_str.split(',').map(str::to_string).collect()
                };
                Ok(crate::models::BacklogNode {
                    id: extract_str(row, 0),
                    title: extract_str(row, 1),
                    kind: extract_str(row, 2),
                    status: extract_str(row, 3),
                    labels,
                    file_path: extract_str(row, 5),
                    content_hash: extract_str(row, 6),
                    source_path: extract_str(row, 7),
                    ingested_at,
                })
            })
            .collect()
    }

    /// Return all backlog content records, optionally filtered by `source_path`.
    pub async fn select_backlog_content_records(
        &self,
        source_path: Option<&str>,
    ) -> Result<Vec<crate::models::BacklogContentRecord>, EngramError> {
        let sp_clause = source_path
            .map(|_| ", source_path = $source_path")
            .unwrap_or("");
        let script = format!(
            r#"?[file_path, content_type, content_hash, content, source_path, ingested_at] :=
    *backlog_content_record {{ file_path, content_type, content_hash, content, source_path, ingested_at }}{sp_clause}"#
        );
        let mut p = BTreeMap::new();
        if let Some(sp) = source_path {
            p.insert("source_path".to_owned(), DataValue::from(sp));
        }
        let result = self
            .db
            .run_script(&script, p, ScriptMutability::Immutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        result
            .rows
            .iter()
            .map(|row| {
                let ingested_str = extract_str(row, 5);
                let ingested_at = chrono::DateTime::parse_from_rfc3339(&ingested_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());
                Ok(crate::models::BacklogContentRecord {
                    file_path: extract_str(row, 0),
                    content_type: extract_str(row, 1),
                    content_hash: extract_str(row, 2),
                    content: extract_str(row, 3),
                    source_path: extract_str(row, 4),
                    ingested_at,
                })
            })
            .collect()
    }

    /// Delete a backlog node by its file path, along with all edges
    /// where the node's `id` is `from_id` or `to_id`.
    ///
    /// Uses per-statement retry (not a transaction) per compound learning
    /// on SQLITE_BUSY granularity.
    pub async fn delete_backlog_node_by_file_path(
        &self,
        file_path: &str,
    ) -> Result<(), EngramError> {
        // Find the node ID for this file path.
        let find = r#"?[id] := *backlog_node { id, file_path }, file_path = $file_path"#;
        let mut p = BTreeMap::new();
        p.insert("file_path".to_owned(), DataValue::from(file_path));
        let r = self
            .db
            .run_script(find, p, ScriptMutability::Immutable)
            .map_err(|e| map_db_err(e.to_string()))?;

        let ids: Vec<String> = r.rows.iter().map(|row| extract_str(row, 0)).collect();

        for id in &ids {
            // Delete outgoing backlog edges.
            let find_out = r#"?[from_id, to_id, edge_type] := *backlog_edge { from_id, to_id, edge_type }, from_id = $id"#;
            let mut po = BTreeMap::new();
            po.insert("id".to_owned(), DataValue::from(id.as_str()));
            let out_rows = self
                .db
                .run_script(find_out, po, ScriptMutability::Immutable)
                .map_err(|e| map_db_err(e.to_string()))?;
            for row in &out_rows.rows {
                let from = extract_str(row, 0);
                let to = extract_str(row, 1);
                let et = extract_str(row, 2);
                let del = r#"?[from_id, to_id, edge_type] <- [[$from_id, $to_id, $edge_type]] :rm backlog_edge { from_id, to_id, edge_type }"#;
                let mut dp = BTreeMap::new();
                dp.insert("from_id".to_owned(), DataValue::from(from.as_str()));
                dp.insert("to_id".to_owned(), DataValue::from(to.as_str()));
                dp.insert("edge_type".to_owned(), DataValue::from(et.as_str()));
                self.run_script_busy_retry_mutable(del, dp).await?;
            }

            // Delete incoming backlog edges.
            let find_in = r#"?[from_id, to_id, edge_type] := *backlog_edge { from_id, to_id, edge_type }, to_id = $id"#;
            let mut pi = BTreeMap::new();
            pi.insert("id".to_owned(), DataValue::from(id.as_str()));
            let in_rows = self
                .db
                .run_script(find_in, pi, ScriptMutability::Immutable)
                .map_err(|e| map_db_err(e.to_string()))?;
            for row in &in_rows.rows {
                let from = extract_str(row, 0);
                let to = extract_str(row, 1);
                let et = extract_str(row, 2);
                let del = r#"?[from_id, to_id, edge_type] <- [[$from_id, $to_id, $edge_type]] :rm backlog_edge { from_id, to_id, edge_type }"#;
                let mut dp = BTreeMap::new();
                dp.insert("from_id".to_owned(), DataValue::from(from.as_str()));
                dp.insert("to_id".to_owned(), DataValue::from(to.as_str()));
                dp.insert("edge_type".to_owned(), DataValue::from(et.as_str()));
                self.run_script_busy_retry_mutable(del, dp).await?;
            }

            // Delete the node itself.
            let del_node = r#"?[id] <- [[$id]] :rm backlog_node { id }"#;
            let mut pn = BTreeMap::new();
            pn.insert("id".to_owned(), DataValue::from(id.as_str()));
            self.run_script_busy_retry_mutable(del_node, pn).await?;
        }
        Ok(())
    }

    /// Delete all backlog nodes and edges belonging to a registry source.
    ///
    /// Used when an entire content source is removed from the registry.
    /// For per-file removal use [`delete_backlog_node_by_file_path`] instead.
    pub async fn delete_backlog_nodes_by_source(
        &self,
        source_path: &str,
    ) -> Result<(), EngramError> {
        // Delete edges for this source.
        let find_edges = r#"?[from_id, to_id, edge_type] := *backlog_edge { from_id, to_id, edge_type, source_path }, source_path = $source_path"#;
        let mut p = BTreeMap::new();
        p.insert("source_path".to_owned(), DataValue::from(source_path));
        let edge_rows = self
            .db
            .run_script(find_edges, p, ScriptMutability::Immutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        for row in &edge_rows.rows {
            let from = extract_str(row, 0);
            let to = extract_str(row, 1);
            let et = extract_str(row, 2);
            let del = r#"?[from_id, to_id, edge_type] <- [[$from_id, $to_id, $edge_type]] :rm backlog_edge { from_id, to_id, edge_type }"#;
            let mut dp = BTreeMap::new();
            dp.insert("from_id".to_owned(), DataValue::from(from.as_str()));
            dp.insert("to_id".to_owned(), DataValue::from(to.as_str()));
            dp.insert("edge_type".to_owned(), DataValue::from(et.as_str()));
            self.run_script_busy_retry_mutable(del, dp).await?;
        }

        // Delete nodes for this source.
        let find_nodes =
            r#"?[id] := *backlog_node { id, source_path }, source_path = $source_path"#;
        let mut p2 = BTreeMap::new();
        p2.insert("source_path".to_owned(), DataValue::from(source_path));
        let node_rows = self
            .db
            .run_script(find_nodes, p2, ScriptMutability::Immutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        for row in &node_rows.rows {
            let id = extract_str(row, 0);
            let del = r#"?[id] <- [[$id]] :rm backlog_node { id }"#;
            let mut dp = BTreeMap::new();
            dp.insert("id".to_owned(), DataValue::from(id.as_str()));
            self.run_script_busy_retry_mutable(del, dp).await?;
        }

        // Delete content records for this source.
        let find_records = r#"?[file_path] := *backlog_content_record { file_path, source_path }, source_path = $source_path"#;
        let mut p3 = BTreeMap::new();
        p3.insert("source_path".to_owned(), DataValue::from(source_path));
        let record_rows = self
            .db
            .run_script(find_records, p3, ScriptMutability::Immutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        for row in &record_rows.rows {
            let file_path = extract_str(row, 0);
            let del = r#"?[file_path] <- [[$file_path]] :rm backlog_content_record { file_path }"#;
            let mut dp = BTreeMap::new();
            dp.insert("file_path".to_owned(), DataValue::from(file_path.as_str()));
            self.run_script_busy_retry_mutable(del, dp).await?;
        }
        Ok(())
    }

    /// Delete a backlog content record by file path.
    pub async fn delete_backlog_content_record_by_path(
        &self,
        file_path: &str,
    ) -> Result<(), EngramError> {
        let script = r#"?[file_path] <- [[$file_path]] :rm backlog_content_record { file_path }"#;
        let mut p = BTreeMap::new();
        p.insert("file_path".to_owned(), DataValue::from(file_path));
        self.run_script_busy_retry_mutable(script, p).await?;
        Ok(())
    }

    // ── Power BI node queries (061-F) ─────────────────────────────────────

    /// Batch upsert Power BI graph nodes into `powerbi_node`.
    ///
    /// Each node is inserted as a separate mutable script with SQLITE_BUSY
    /// retry (per compound learning on per-statement retry granularity).
    pub async fn upsert_powerbi_nodes(
        &self,
        nodes: &[crate::models::PowerBiNode],
    ) -> Result<(), EngramError> {
        let script = r#"
?[id, name, kind, file_path, source_path, content_hash, ingested_at] <-
    [[$id, $name, $kind, $file_path, $source_path, $content_hash, $ingested_at]]
:put powerbi_node {
    id => name, kind, file_path, source_path, content_hash, ingested_at
}
"#;
        for node in nodes {
            let mut p = BTreeMap::new();
            p.insert("id".to_owned(), DataValue::from(node.id.as_str()));
            p.insert("name".to_owned(), DataValue::from(node.name.as_str()));
            p.insert("kind".to_owned(), DataValue::from(node.kind.as_str()));
            p.insert(
                "file_path".to_owned(),
                DataValue::from(node.file_path.as_str()),
            );
            p.insert(
                "source_path".to_owned(),
                DataValue::from(node.source_path.as_str()),
            );
            p.insert(
                "content_hash".to_owned(),
                DataValue::from(node.content_hash.as_str()),
            );
            p.insert(
                "ingested_at".to_owned(),
                DataValue::from(node.ingested_at.to_rfc3339().as_str()),
            );
            self.run_script_busy_retry_mutable(script, p).await?;
        }
        Ok(())
    }

    /// Batch upsert Power BI graph edges into `powerbi_edge`.
    pub async fn upsert_powerbi_edges(
        &self,
        edges: &[crate::models::PowerBiEdge],
    ) -> Result<(), EngramError> {
        let script = r#"
?[from_id, to_id, edge_type, source_path] <-
    [[$from_id, $to_id, $edge_type, $source_path]]
:put powerbi_edge {
    from_id, to_id, edge_type => source_path
}
"#;
        for edge in edges {
            let mut p = BTreeMap::new();
            p.insert("from_id".to_owned(), DataValue::from(edge.from_id.as_str()));
            p.insert("to_id".to_owned(), DataValue::from(edge.to_id.as_str()));
            p.insert(
                "edge_type".to_owned(),
                DataValue::from(edge.edge_type.as_str()),
            );
            p.insert(
                "source_path".to_owned(),
                DataValue::from(edge.source_path.as_str()),
            );
            self.run_script_busy_retry_mutable(script, p).await?;
        }
        Ok(())
    }

    /// Return all Power BI nodes, optionally filtered by `source_path`.
    pub async fn select_powerbi_nodes(
        &self,
        source_path: Option<&str>,
    ) -> Result<Vec<crate::models::PowerBiNode>, EngramError> {
        let sp_clause = source_path
            .map(|_| ", source_path = $source_path")
            .unwrap_or("");
        let script = format!(
            r#"?[id, name, kind, file_path, source_path, content_hash, ingested_at] :=
    *powerbi_node {{ id, name, kind, file_path, source_path, content_hash, ingested_at }}{sp_clause}"#
        );
        let mut p = BTreeMap::new();
        if let Some(sp) = source_path {
            p.insert("source_path".to_owned(), DataValue::from(sp));
        }
        let result = self
            .db
            .run_script(&script, p, ScriptMutability::Immutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        result
            .rows
            .iter()
            .map(|row| {
                let ingested_str = extract_str(row, 6);
                let ingested_at = chrono::DateTime::parse_from_rfc3339(&ingested_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());
                let kind_str = extract_str(row, 2);
                let kind = parse_powerbi_node_kind(&kind_str)?;
                Ok(crate::models::PowerBiNode {
                    id: extract_str(row, 0),
                    name: extract_str(row, 1),
                    kind,
                    file_path: extract_str(row, 3),
                    source_path: extract_str(row, 4),
                    content_hash: extract_str(row, 5),
                    ingested_at,
                })
            })
            .collect()
    }

    /// Delete all Power BI nodes and edges belonging to a registry source.
    ///
    /// Used when an entire Power BI content source is removed from the registry.
    /// For per-file removal use [`delete_powerbi_nodes_by_file_path`] instead.
    pub async fn delete_powerbi_nodes_by_source(
        &self,
        source_path: &str,
    ) -> Result<(), EngramError> {
        // Delete edges for this source.
        let find_edges = r#"?[from_id, to_id, edge_type] := *powerbi_edge { from_id, to_id, edge_type, source_path }, source_path = $source_path"#;
        let mut p = BTreeMap::new();
        p.insert("source_path".to_owned(), DataValue::from(source_path));
        let edge_rows = self
            .db
            .run_script(find_edges, p, ScriptMutability::Immutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        for row in &edge_rows.rows {
            let from = extract_str(row, 0);
            let to = extract_str(row, 1);
            let et = extract_str(row, 2);
            let del = r#"?[from_id, to_id, edge_type] <- [[$from_id, $to_id, $edge_type]] :rm powerbi_edge { from_id, to_id, edge_type }"#;
            let mut dp = BTreeMap::new();
            dp.insert("from_id".to_owned(), DataValue::from(from.as_str()));
            dp.insert("to_id".to_owned(), DataValue::from(to.as_str()));
            dp.insert("edge_type".to_owned(), DataValue::from(et.as_str()));
            self.run_script_busy_retry_mutable(del, dp).await?;
        }

        // Delete nodes for this source.
        let find_nodes =
            r#"?[id] := *powerbi_node { id, source_path }, source_path = $source_path"#;
        let mut p2 = BTreeMap::new();
        p2.insert("source_path".to_owned(), DataValue::from(source_path));
        let node_rows = self
            .db
            .run_script(find_nodes, p2, ScriptMutability::Immutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        for row in &node_rows.rows {
            let id = extract_str(row, 0);
            let del = r#"?[id] <- [[$id]] :rm powerbi_node { id }"#;
            let mut dp = BTreeMap::new();
            dp.insert("id".to_owned(), DataValue::from(id.as_str()));
            self.run_script_busy_retry_mutable(del, dp).await?;
        }

        Ok(())
    }

    /// Delete all Power BI nodes whose `file_path` matches, along with all
    /// edges where any of those node IDs appear as `from_id` or `to_id`.
    ///
    /// Used by the deletion sweep when a source file is removed from disk.
    pub async fn delete_powerbi_nodes_by_file_path(
        &self,
        file_path: &str,
    ) -> Result<(), EngramError> {
        let find = r#"?[id] := *powerbi_node { id, file_path }, file_path = $file_path"#;
        let mut p = BTreeMap::new();
        p.insert("file_path".to_owned(), DataValue::from(file_path));
        let r = self
            .db
            .run_script(find, p, ScriptMutability::Immutable)
            .map_err(|e| map_db_err(e.to_string()))?;

        let ids: Vec<String> = r.rows.iter().map(|row| extract_str(row, 0)).collect();

        for id in &ids {
            // Delete outgoing edges.
            let find_out = r#"?[from_id, to_id, edge_type] := *powerbi_edge { from_id, to_id, edge_type }, from_id = $id"#;
            let mut po = BTreeMap::new();
            po.insert("id".to_owned(), DataValue::from(id.as_str()));
            let out_rows = self
                .db
                .run_script(find_out, po, ScriptMutability::Immutable)
                .map_err(|e| map_db_err(e.to_string()))?;
            for row in &out_rows.rows {
                let from = extract_str(row, 0);
                let to = extract_str(row, 1);
                let et = extract_str(row, 2);
                let del = r#"?[from_id, to_id, edge_type] <- [[$from_id, $to_id, $edge_type]] :rm powerbi_edge { from_id, to_id, edge_type }"#;
                let mut dp = BTreeMap::new();
                dp.insert("from_id".to_owned(), DataValue::from(from.as_str()));
                dp.insert("to_id".to_owned(), DataValue::from(to.as_str()));
                dp.insert("edge_type".to_owned(), DataValue::from(et.as_str()));
                self.run_script_busy_retry_mutable(del, dp).await?;
            }

            // Delete incoming edges.
            let find_in = r#"?[from_id, to_id, edge_type] := *powerbi_edge { from_id, to_id, edge_type }, to_id = $id"#;
            let mut pi = BTreeMap::new();
            pi.insert("id".to_owned(), DataValue::from(id.as_str()));
            let in_rows = self
                .db
                .run_script(find_in, pi, ScriptMutability::Immutable)
                .map_err(|e| map_db_err(e.to_string()))?;
            for row in &in_rows.rows {
                let from = extract_str(row, 0);
                let to = extract_str(row, 1);
                let et = extract_str(row, 2);
                let del = r#"?[from_id, to_id, edge_type] <- [[$from_id, $to_id, $edge_type]] :rm powerbi_edge { from_id, to_id, edge_type }"#;
                let mut dp = BTreeMap::new();
                dp.insert("from_id".to_owned(), DataValue::from(from.as_str()));
                dp.insert("to_id".to_owned(), DataValue::from(to.as_str()));
                dp.insert("edge_type".to_owned(), DataValue::from(et.as_str()));
                self.run_script_busy_retry_mutable(del, dp).await?;
            }

            // Delete the node itself.
            let del_node = r#"?[id] <- [[$id]] :rm powerbi_node { id }"#;
            let mut pn = BTreeMap::new();
            pn.insert("id".to_owned(), DataValue::from(id.as_str()));
            self.run_script_busy_retry_mutable(del_node, pn).await?;
        }
        Ok(())
    }

    /// Resolve a Power BI entity ID to a [`QueryGraphNode`] using `powerbi_node` metadata.
    ///
    /// Returns `None` when the ID is not found in `powerbi_node`.
    async fn resolve_powerbi_node(&self, id: &str) -> Result<Option<QueryGraphNode>, EngramError> {
        let script =
            "?[id, name, kind, file_path] := *powerbi_node { id, name, kind, file_path }, id = $id";
        let mut p = BTreeMap::new();
        p.insert("id".to_owned(), DataValue::from(id));
        let result = self
            .db
            .run_script(script, p, ScriptMutability::Immutable)
            .map_err(|e| map_db_err(e.to_string()))?;
        if result.rows.is_empty() {
            return Ok(None);
        }
        let row = &result.rows[0];
        let node_id = extract_str(row, 0);
        let name = extract_str(row, 1);
        let kind = extract_str(row, 2);
        let file_path_val = extract_str(row, 3);
        Ok(Some(QueryGraphNode {
            id: node_id,
            kind: if kind.is_empty() {
                "powerbi_entity".to_owned()
            } else {
                format!("powerbi_{kind}")
            },
            name: if name.is_empty() { id.to_owned() } else { name },
            file_path: if file_path_val.is_empty() {
                None
            } else {
                Some(file_path_val)
            },
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

fn extract_opt_u32(row: &[DataValue], col: usize) -> Option<u32> {
    match row.get(col) {
        Some(DataValue::Num(Num::Int(value))) if *value >= 0 => u32::try_from(*value).ok(),
        _ => None,
    }
}

fn extract_string_list(row: &[DataValue], col: usize) -> Vec<String> {
    match row.get(col) {
        Some(DataValue::List(values)) => values
            .iter()
            .filter_map(|value| match value {
                DataValue::Str(text) => Some(text.to_string()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn optional_u32_to_datavalue(value: Option<u32>) -> DataValue {
    value
        .map(i64::from)
        .map(Num::Int)
        .map(DataValue::Num)
        .unwrap_or_else(|| DataValue::Num(Num::Int(-1)))
}

fn string_list_to_datavalue(values: &[String]) -> DataValue {
    DataValue::List(
        values
            .iter()
            .map(|value| DataValue::from(value.as_str()))
            .collect(),
    )
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

// ── Unit tests (040.001-T) ────────────────────────────────────────────────────

/// Map a `kind` string from the `powerbi_node` relation to a [`PowerBiNodeKind`]
/// variant.
///
/// Returns `Err` for any string that is not a recognized variant, preventing
/// silently-wrong data from masquerading as `DataSource`.
fn parse_powerbi_node_kind(
    kind_str: &str,
) -> Result<crate::models::powerbi_graph::PowerBiNodeKind, EngramError> {
    use crate::models::powerbi_graph::PowerBiNodeKind;
    match kind_str {
        "report" => Ok(PowerBiNodeKind::Report),
        "page" => Ok(PowerBiNodeKind::Page),
        "visual" => Ok(PowerBiNodeKind::Visual),
        "semantic_model" => Ok(PowerBiNodeKind::SemanticModel),
        "table" => Ok(PowerBiNodeKind::Table),
        "column" => Ok(PowerBiNodeKind::Column),
        "measure" => Ok(PowerBiNodeKind::Measure),
        "expression" => Ok(PowerBiNodeKind::Expression),
        "relationship" => Ok(PowerBiNodeKind::Relationship),
        "data_source" => Ok(PowerBiNodeKind::DataSource),
        "partition" => Ok(PowerBiNodeKind::Partition),
        _ => Err(map_db_err(format!(
            "unrecognized powerbi_node kind: {kind_str:?}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering;
    use std::sync::{Mutex, OnceLock};

    use super::*;

    /// Serializes the two delta tests so they don't interleave on shared process-global atomics.
    ///
    /// Rust unit tests run in parallel by default; without this guard, one test's
    /// `reset_retry_metrics()` can fire between another test's store and assertion.
    static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    fn acquire_test_lock() -> std::sync::MutexGuard<'static, ()> {
        TEST_LOCK
            .get_or_init(Mutex::default)
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// AC: `retry_count` delta ≥ 1 after a simulated SQLITE_BUSY retry.
    #[test]
    fn t040_001_retry_count_increments() {
        let _guard = acquire_test_lock();
        reset_retry_metrics();
        let before = mutable_script_retry_metrics().retry_count;
        // Simulate the atomic increment that run_script_busy_retry_mutable will perform.
        MUTABLE_RETRY_COUNT.fetch_add(1, Ordering::Relaxed);
        let after = mutable_script_retry_metrics().retry_count;
        assert!(
            after.wrapping_sub(before) >= 1,
            "retry_count must be at least 1 more than baseline; before={before} after={after}"
        );
    }

    /// AC: `last_retry_at` is `None` on reset and `Some` after a simulated retry.
    #[test]
    fn t040_001_last_retry_at_transitions() {
        let _guard = acquire_test_lock();
        reset_retry_metrics();
        assert!(
            mutable_script_retry_metrics().last_retry_at.is_none(),
            "last_retry_at must be None after reset"
        );
        // Use a fixed non-zero epoch-ms value to avoid the sentinel (0 = "no retry")
        // and eliminate any dependency on the system clock.
        let fixed_ms: u64 = 1_000_000_000_000; // 2001-09-09 — safely after the Unix epoch sentinel
        MUTABLE_LAST_RETRY_EPOCH_MS.store(fixed_ms, Ordering::Relaxed);
        assert!(
            mutable_script_retry_metrics().last_retry_at.is_some(),
            "last_retry_at must be Some after a simulated retry"
        );
    }

    /// S-DBC-PBI-01: `parse_powerbi_node_kind` maps all canonical kind strings and
    /// errors on unrecognized values.
    #[test]
    fn parse_powerbi_node_kind_known_and_unknown() {
        use crate::models::powerbi_graph::PowerBiNodeKind;

        let cases = [
            ("report", PowerBiNodeKind::Report),
            ("page", PowerBiNodeKind::Page),
            ("visual", PowerBiNodeKind::Visual),
            ("semantic_model", PowerBiNodeKind::SemanticModel),
            ("table", PowerBiNodeKind::Table),
            ("column", PowerBiNodeKind::Column),
            ("measure", PowerBiNodeKind::Measure),
            ("expression", PowerBiNodeKind::Expression),
            ("relationship", PowerBiNodeKind::Relationship),
            ("data_source", PowerBiNodeKind::DataSource),
            ("partition", PowerBiNodeKind::Partition),
        ];
        for (s, expected) in cases {
            let got = parse_powerbi_node_kind(s)
                .unwrap_or_else(|e| panic!("expected Ok for {s:?} but got Err: {e}"));
            assert_eq!(got, expected, "wrong variant for {s:?}");
        }

        // Unknown kind must be an error, not silently mapped to DataSource.
        assert!(
            parse_powerbi_node_kind("unknown_kind").is_err(),
            "unknown kind should return Err"
        );
        assert!(
            parse_powerbi_node_kind("").is_err(),
            "empty string should return Err"
        );
    }

    // ── 084.009-T: semantic corpus completeness (LEFT JOIN) ──────────────────

    /// `all_functions_for_eval` must keep a `function_meta` row whose
    /// `function_code` / `function_embedding` rows are absent (a partial write),
    /// where the INNER-join `all_functions` drops it — so the semantic-eval
    /// denominator reflects every indexed function (78AA205D). A fully
    /// materialized function is returned unchanged by both.
    #[tokio::test]
    async fn all_functions_for_eval_includes_meta_only_rows() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let db = crate::db::connect_db(tmp.path(), "eval-completeness")
            .await
            .expect("connect_db");
        let q = CodeGraphQueries::new(db);

        // A fully-materialized function (meta + code + embedding).
        let full = crate::models::Function {
            id: "function:full".to_owned(),
            name: "full".to_owned(),
            file_path: "src/full.rs".to_owned(),
            line_start: 1,
            line_end: 3,
            signature: "fn full()".to_owned(),
            docstring: Some("documented".to_owned()),
            body: "fn full() {}".to_owned(),
            body_hash: "hash-full".to_owned(),
            token_count: 3,
            embed_type: "explicit_code".to_owned(),
            embedding: vec![0.1_f32; crate::services::embedding::EMBEDDING_DIM],
            summary: String::new(),
        };
        q.upsert_function(&full).await.expect("upsert full");

        // A partial write: `function_meta` only, with no `function_code` /
        // `function_embedding` row (the SQLITE_BUSY-mid-upsert scenario).
        let meta_only = r#"
?[id, name, file_path, line_start, line_end, signature, docstring, body_hash, token_count, embed_type, summary] <-
    [[$id, $name, $file_path, $line_start, $line_end, $signature, $docstring, $body_hash, $token_count, $embed_type, $summary]]
:put function_meta { id, name, file_path, line_start, line_end, signature, docstring, body_hash, token_count, embed_type, summary }
"#;
        let mut p = BTreeMap::new();
        p.insert("id".to_owned(), DataValue::from("function:partial"));
        p.insert("name".to_owned(), DataValue::from("partial"));
        p.insert("file_path".to_owned(), DataValue::from("src/partial.rs"));
        p.insert("line_start".to_owned(), DataValue::Num(Num::Int(1)));
        p.insert("line_end".to_owned(), DataValue::Num(Num::Int(2)));
        p.insert("signature".to_owned(), DataValue::from("fn partial()"));
        // Deliberately docstring-less to exercise the bare-name query fallback.
        p.insert("docstring".to_owned(), DataValue::from(""));
        p.insert("body_hash".to_owned(), DataValue::from("hash-partial"));
        p.insert("token_count".to_owned(), DataValue::Num(Num::Int(0)));
        p.insert("embed_type".to_owned(), DataValue::from("explicit_code"));
        p.insert("summary".to_owned(), DataValue::from(""));
        q.run_script_busy_retry_mutable(meta_only, p)
            .await
            .expect("insert meta-only row");

        let inner = q.all_functions().await.expect("all_functions");
        let eval = q
            .all_functions_for_eval()
            .await
            .expect("all_functions_for_eval");

        assert_eq!(
            inner.len(),
            1,
            "INNER join drops the meta-only function (denominator gap)"
        );
        assert_eq!(
            eval.len(),
            2,
            "LEFT join keeps every function_meta row in the eval denominator"
        );

        let partial = eval
            .iter()
            .find(|f| f.id == "function:partial")
            .expect("meta-only function must be present in the eval corpus");
        assert!(
            partial.embedding.is_empty(),
            "a meta-only function carries no embedding (scored keyword-only)"
        );
        assert!(
            partial.body.is_empty(),
            "a meta-only function carries an empty body"
        );

        let full_out = eval
            .iter()
            .find(|f| f.id == "function:full")
            .expect("materialized function must still be present");
        assert_eq!(
            full_out.embedding.len(),
            crate::services::embedding::EMBEDDING_DIM,
            "a materialized function keeps its embedding (counts unchanged)"
        );
        assert_eq!(full_out.body, "fn full() {}", "materialized body preserved");
    }
}
