//! CozoScript schema definitions for the CozoDB backend.
//!
//! Phase 2 (U2.1) — relation-creation constants and bootstrap execution.
//! Phase 3 (U3.x) — edge tables, content record, file hash.
//! Phase 4 (U4.x) — HNSW vector indexes.
//!
//! Three-table layout per symbol type (function / class / interface):
//!  * `*_meta`      — identity and source-position fields (key lookups)
//!  * `*_code`      — raw body text (separate to avoid over-fetching)
//!  * `*_embedding` — float vector (isolated for KNN efficiency)
//!
//! Edge tables (composite-key deduplication):
//!  * `calls_edge`          — function→function call
//!  * `imports_edge`        — file→file import (includes import_path in key)
//!  * `defines_edge`        — file→symbol containment
//!  * `inherits_from_edge`  — class/interface inheritance
//!  * `concerns_edge`       — task→symbol cross-region link
//!  * `references_edge`     — source→target qualified-name reference

use std::{collections::BTreeMap, time::Duration};

use crate::errors::EngramError;

use super::{SchemaTarget, map_db_err};

// ── Bootstrap ─────────────────────────────────────────────────────────────

/// Bootstrap all CozoDB relations for a workspace.
///
/// Runs every `CREATE_*` script against the DB instance acquired from `db`.
/// For `CozoHandle`, a fresh in-memory instance is opened (validates syntax).
/// For `CozoDb`, scripts are run against the persistent SQLite store.
///
/// This function is idempotent: `:create` errors for already-existing
/// relations are silently ignored.
///
/// # Errors
/// Returns [`EngramError`] when any script fails for a reason other than
/// the relation already existing.
pub fn run_schema_bootstrap(db: &impl SchemaTarget) -> Result<(), EngramError> {
    let cozo_db = db.cozo_instance()?;
    run_scripts(&cozo_db)
}

/// Execute all `:create` scripts against `cozo_db`, ignoring "already exists" errors.
///
/// Each script is retried up to 20 times with exponential back-off when
/// SQLite returns `SQLITE_BUSY` ("database is locked").  This handles the
/// case where an already-open CozoDB handle is writing (e.g. indexing) when
/// a second `connect_db` caller reaches schema bootstrap.
fn run_scripts(cozo_db: &cozo::DbInstance) -> Result<(), EngramError> {
    let scripts = [
        CREATE_FILE_NODE,
        CREATE_FUNCTION_META,
        CREATE_FUNCTION_CODE,
        CREATE_FUNCTION_EMBEDDING,
        CREATE_CLASS_META,
        CREATE_CLASS_CODE,
        CREATE_CLASS_EMBEDDING,
        CREATE_INTERFACE_META,
        CREATE_INTERFACE_CODE,
        CREATE_INTERFACE_EMBEDDING,
        CREATE_IMPORT_NODE,
        CREATE_COMMIT_NODE,
        // Phase 3: edge tables
        CREATE_CALLS_EDGE,
        CREATE_IMPORTS_EDGE,
        CREATE_DEFINES_EDGE,
        CREATE_INHERITS_FROM_EDGE,
        CREATE_CONCERNS_EDGE,
        CREATE_REFERENCES_EDGE,
        // Phase 3: auxiliary tables
        CREATE_CONTENT_RECORD,
        CREATE_FILE_HASH,
        // 002-F: backlog hydration relations
        CREATE_BACKLOG_NODE,
        CREATE_BACKLOG_EDGE,
        CREATE_BACKLOG_CONTENT_RECORD,
    ];

    for script in &scripts {
        run_script_retrying(cozo_db, script)?;
    }

    // Phase 4: HNSW vector indexes. Creation may fail on empty tables or when the
    // storage backend does not support vector indexes. Suppress only known-benign
    // failures; warn on unexpected ones so regressions remain visible.
    let hnsw_scripts = [
        HNSW_FUNCTION_EMBEDDING,
        HNSW_CLASS_EMBEDDING,
        HNSW_INTERFACE_EMBEDDING,
    ];
    for script in &hnsw_scripts {
        if let Err(e) = cozo_db.run_script(script, BTreeMap::new(), cozo::ScriptMutability::Mutable)
        {
            let msg = e.to_string().to_lowercase();
            let is_known_benign = msg.contains("empty")
                || msg.contains("no rows")
                || msg.contains("unsupported")
                || msg.contains("not support")
                || msg.contains("invalid option")
                || msg.contains("vector index")
                || msg.contains("hnsw")
                || msg.contains("already exists");
            if !is_known_benign {
                tracing::warn!(
                    "unexpected HNSW index creation failure during schema bootstrap: {e}"
                );
            }
        }
    }

    Ok(())
}

/// Run a single CozoDB script, retrying on `SQLITE_BUSY` ("database is locked").
///
/// Schema bootstrap scripts are idempotent (`:create` is a no-op when the
/// relation already exists), so retrying is always safe.  Twenty attempts
/// with 25 ms → 500 ms exponential back-off gives ≈ 7.8 s of total retry
/// headroom (25+50+100+200+400+500×15), which is more than enough for a
/// concurrent writer to release its write transaction.
fn run_script_retrying(cozo_db: &cozo::DbInstance, script: &str) -> Result<(), EngramError> {
    const MAX_ATTEMPTS: u32 = 20;

    for attempt in 0..MAX_ATTEMPTS {
        match cozo_db.run_script(script, BTreeMap::new(), cozo::ScriptMutability::Mutable) {
            Ok(_) => return Ok(()),
            Err(e) => {
                let msg = e.to_string().to_lowercase();
                if msg.contains("already")
                    || msg.contains("defined")
                    || msg.contains("conflicts")
                    || msg.contains("existing")
                {
                    return Ok(());
                }
                if (msg.contains("locked") || msg.contains("busy")) && attempt + 1 < MAX_ATTEMPTS {
                    let delay_ms = std::cmp::min(25u64 << attempt.min(5), 500);
                    std::thread::sleep(Duration::from_millis(delay_ms));
                    continue;
                }
                return Err(map_db_err(format!("schema bootstrap: {e}")));
            }
        }
    }
    unreachable!("loop exits via return")
}

// ── File node ──────────────────────────────────────────────────────────────

/// CozoScript `:create` for `file_node` — source file metadata.
///
/// Key: `path`. Values: `id`, language, size in bytes, content hash, timestamp.
pub const CREATE_FILE_NODE: &str = r#"
:create file_node {
    path: String
    =>
    id: String,
    language: String,
    size_bytes: Int,
    content_hash: String,
    last_indexed_at: String,
}
"#;

// ── Function relations ─────────────────────────────────────────────────────

/// CozoScript `:create` for `function_meta` — identity and source-position.
///
/// Key: `id` (prefixed UUID, e.g. `fn:abc123`).
pub const CREATE_FUNCTION_META: &str = r#"
:create function_meta {
    id: String
    =>
    name: String,
    file_path: String,
    line_start: Int,
    line_end: Int,
    signature: String,
    docstring: String,
    body_hash: String,
    token_count: Int,
    embed_type: String,
    summary: String,
}
"#;

/// CozoScript `:create` for `function_code` — raw source body.
///
/// Stored separately from meta to avoid over-fetching on symbol queries.
pub const CREATE_FUNCTION_CODE: &str = r#"
:create function_code {
    id: String
    =>
    body: String,
}
"#;

/// CozoScript `:create` for `function_embedding` — float vector (384-dim).
///
/// Stored separately for KNN index efficiency.
/// Dimension matches [`crate::services::embedding::EMBEDDING_DIM`] = 384.
pub const CREATE_FUNCTION_EMBEDDING: &str = r#"
:create function_embedding {
    id: String
    =>
    embedding: [Float],
}
"#;

// ── Class relations ────────────────────────────────────────────────────────

/// CozoScript `:create` for `class_meta` — identity and source-position.
pub const CREATE_CLASS_META: &str = r#"
:create class_meta {
    id: String
    =>
    name: String,
    file_path: String,
    line_start: Int,
    line_end: Int,
    docstring: String,
    body_hash: String,
    token_count: Int,
    embed_type: String,
    summary: String,
}
"#;

/// CozoScript `:create` for `class_code` — raw source body.
pub const CREATE_CLASS_CODE: &str = r#"
:create class_code {
    id: String
    =>
    body: String,
}
"#;

/// CozoScript `:create` for `class_embedding` — float vector (384-dim).
pub const CREATE_CLASS_EMBEDDING: &str = r#"
:create class_embedding {
    id: String
    =>
    embedding: [Float],
}
"#;

// ── Interface relations ────────────────────────────────────────────────────

/// CozoScript `:create` for `interface_meta` — identity and source-position.
pub const CREATE_INTERFACE_META: &str = r#"
:create interface_meta {
    id: String
    =>
    name: String,
    file_path: String,
    line_start: Int,
    line_end: Int,
    docstring: String,
    body_hash: String,
    token_count: Int,
    embed_type: String,
    summary: String,
}
"#;

/// CozoScript `:create` for `interface_code` — raw source body.
pub const CREATE_INTERFACE_CODE: &str = r#"
:create interface_code {
    id: String
    =>
    body: String,
}
"#;

/// CozoScript `:create` for `interface_embedding` — float vector (384-dim).
pub const CREATE_INTERFACE_EMBEDDING: &str = r#"
:create interface_embedding {
    id: String
    =>
    embedding: [Float],
}
"#;

// ── Auxiliary relations ────────────────────────────────────────────────────

/// CozoScript `:create` for `import_node` — file-level import edges.
pub const CREATE_IMPORT_NODE: &str = r#"
:create import_node {
    id: String
    =>
    file_path: String,
    import_path: String,
}
"#;

/// CozoScript `:create` for `commit_node` — git commit metadata.
pub const CREATE_COMMIT_NODE: &str = r#"
:create commit_node {
    id: String
    =>
    hash: String,
    short_hash: String,
    author_name: String,
    author_email: String,
    timestamp: Int,
    message: String,
    parent_hashes: [String],
    changes: [String],
}
"#;

// ── Phase 3: Edge tables ───────────────────────────────────────────────────

/// CozoScript `:create` for `calls_edge` — function-to-function call.
///
/// Key: `(from, to)` composite — one entry per unique caller/callee pair.
pub const CREATE_CALLS_EDGE: &str = r#"
:create calls_edge {
    from: String,
    to: String
    =>
    created_at: String,
}
"#;

/// CozoScript `:create` for `imports_edge` — file-level import dependency.
///
/// Key: `(from, to, import_path)` — import_path in the key allows multiple
/// imports of the same target via different aliases without colliding.
pub const CREATE_IMPORTS_EDGE: &str = r#"
:create imports_edge {
    from: String,
    to: String,
    import_path: String
    =>
    created_at: String,
}
"#;

/// CozoScript `:create` for `defines_edge` — file-to-symbol containment.
///
/// Key: `(from, to)` — one edge per (file, symbol) pair.
pub const CREATE_DEFINES_EDGE: &str = r#"
:create defines_edge {
    from: String,
    to: String
    =>
    symbol_table: String,
    created_at: String,
}
"#;

/// CozoScript `:create` for `inherits_from_edge` — class/interface inheritance.
///
/// Key: `(from, to)` — one edge per (child, parent) pair.
pub const CREATE_INHERITS_FROM_EDGE: &str = r#"
:create inherits_from_edge {
    from: String,
    to: String
    =>
    from_table: String,
    to_table: String,
    created_at: String,
}
"#;

/// CozoScript `:create` for `concerns_edge` — cross-region task-to-symbol link.
///
/// Key: `(task_id, symbol_id)` — one edge per (task, symbol) pair.
pub const CREATE_CONCERNS_EDGE: &str = r#"
:create concerns_edge {
    task_id: String,
    symbol_id: String
    =>
    symbol_table: String,
    linked_by: String,
    created_at: String,
}
"#;

/// CozoScript `:create` for `references_edge` — qualified-name reference.
///
/// Key: `(from, to, qualified_name)` — qualified_name in the key allows
/// multiple reference types to the same target from the same source.
pub const CREATE_REFERENCES_EDGE: &str = r#"
:create references_edge {
    from: String,
    to: String,
    qualified_name: String
    =>
    created_at: String,
}
"#;

// ── Phase 3: Auxiliary tables ──────────────────────────────────────────────

/// CozoScript `:create` for `content_record` — ingested workspace content.
///
/// Key: `id` — one record per retrieval unit.
pub const CREATE_CONTENT_RECORD: &str = r#"
:create content_record {
    id: String
    =>
    file_path: String,
    content_type: String,
    content_hash: String,
    content: String,
    source_path: String,
    file_size_bytes: Int,
    ingested_at: String,
    record_kind: String,
    chunk_id: String,
    chunk_index: Int,
    heading_path: [String],
    line_start: Int,
    line_end: Int,
    fallback_reason: String,
    lint_summary: String,
    suggestions: [String],
    embedding: [Float],
}
"#;

/// CozoScript `:create` for `file_hash` — content-hash cache for change detection.
///
/// Key: `file_path` — one entry per workspace-relative file path.
pub const CREATE_FILE_HASH: &str = r#"
:create file_hash {
    file_path: String
    =>
    content_hash: String,
    size_bytes: Int,
    recorded_at: String,
}
"#;

// ── Phase 4: HNSW vector indexes ──────────────────────────────────────────

/// CozoScript `::hnsw create` for `function_embedding` cosine index.
///
/// Created at bootstrap; errors on empty tables are silently suppressed.
pub const HNSW_FUNCTION_EMBEDDING: &str = r#"
::hnsw create function_embedding:embedding_hnsw {
    fields: [embedding],
    dim: 384,
    distance: Cosine,
    m: 50,
    ef_construction: 20,
    index_filter: length(embedding) == 384,
}
"#;

/// CozoScript `::hnsw create` for `class_embedding` cosine index.
pub const HNSW_CLASS_EMBEDDING: &str = r#"
::hnsw create class_embedding:embedding_hnsw {
    fields: [embedding],
    dim: 384,
    distance: Cosine,
    m: 50,
    ef_construction: 20,
    index_filter: length(embedding) == 384,
}
"#;

/// CozoScript `::hnsw create` for `interface_embedding` cosine index.
pub const HNSW_INTERFACE_EMBEDDING: &str = r#"
::hnsw create interface_embedding:embedding_hnsw {
    fields: [embedding],
    dim: 384,
    distance: Cosine,
    m: 50,
    ef_construction: 20,
    index_filter: length(embedding) == 384,
}
"#;

// ── 002-F: Backlog hydration relations ────────────────────────────────────

/// CozoScript `:create` for `backlog_node` — a single backlog artifact.
///
/// Key: `id` — artifact identifier from frontmatter (e.g. `001-F`, `001.001-T`).
/// Stored separately from the code graph symbol tables to keep backlog
/// traversal queries isolated and avoid cross-domain key conflicts.
pub const CREATE_BACKLOG_NODE: &str = r#"
:create backlog_node {
    id: String
    =>
    title: String,
    kind: String,
    status: String,
    labels: String,
    file_path: String,
    content_hash: String,
    source_path: String,
    ingested_at: String,
}
"#;

/// CozoScript `:create` for `backlog_edge` — a directed relationship between
/// two backlog nodes.
///
/// Key: `(from_id, to_id, edge_type)` — composite key supports multiple
/// relationship types between the same pair of nodes.
pub const CREATE_BACKLOG_EDGE: &str = r#"
:create backlog_edge {
    from_id: String,
    to_id: String,
    edge_type: String
    =>
    source_path: String,
}
"#;

/// CozoScript `:create` for `backlog_content_record` — full text of a
/// backlog file, stored separately from `content_record` to prevent key
/// collisions when backlog paths overlap other indexed content sources
/// (e.g. `docs/` or `.backlogit/` when those are also indexed).
///
/// Key: `file_path` — workspace-relative path (unique per backlog source).
pub const CREATE_BACKLOG_CONTENT_RECORD: &str = r#"
:create backlog_content_record {
    file_path: String
    =>
    content_type: String,
    content_hash: String,
    content: String,
    source_path: String,
    ingested_at: String,
}
"#;
