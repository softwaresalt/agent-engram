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
        // 082-F: cross-file call resolution staging
        CREATE_STAGED_CALL,
        // 082.003-T (remediation): durable schema metadata / rollback markers.
        // Created BEFORE the migrate call below so the marker relation exists
        // when `migrate_calls_edge_resolution` consults it.
        CREATE_SCHEMA_META,
        // Phase 3: auxiliary tables
        CREATE_CONTENT_RECORD,
        CREATE_FILE_HASH,
        // 002-F: backlog hydration relations
        CREATE_BACKLOG_NODE,
        CREATE_BACKLOG_EDGE,
        CREATE_BACKLOG_CONTENT_RECORD,
        // 061-F: Power BI graph relations
        CREATE_POWERBI_NODE,
        CREATE_POWERBI_EDGE,
    ];

    for script in &scripts {
        run_script_retrying(cozo_db, script)?;
    }

    // 082.003-T: upgrade pre-existing `calls_edge` relations that predate the
    // `resolution` provenance attribute. Idempotent: no-op once the column
    // exists, so it is safe to run on every bootstrap.
    migrate_calls_edge_resolution(cozo_db)?;

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

/// Durable `schema_meta` key recording that the 082.003-T `calls_edge.resolution`
/// column was intentionally rolled back by an operator.
///
/// While this marker is set, `migrate_calls_edge_resolution` refuses to re-add
/// the column on subsequent bootstraps, so `migrate-down` survives daemon
/// reopen. Forward re-enable (clearing the marker) is out of scope for the
/// remediation.
pub(crate) const CALLS_RESOLUTION_ROLLED_BACK_KEY: &str = "calls_resolution_rolled_back";

/// Return `true` when the durable `schema_meta` flag `key` is present and set to
/// `"true"`.
///
/// A missing `schema_meta` relation is reported as `false` (a legacy database
/// predating the marker relation has, by definition, never been rolled back).
/// This mirrors the graceful missing-relation handling in
/// [`calls_edge_has_resolution`].
///
/// # Errors
/// Returns [`EngramError`] when the query fails for a reason other than the
/// relation not existing.
fn schema_meta_flag_set(cozo_db: &cozo::DbInstance, key: &str) -> Result<bool, EngramError> {
    let mut params = BTreeMap::new();
    params.insert("key".to_string(), cozo::DataValue::from(key));
    match cozo_db.run_script(
        "?[value] := *schema_meta{key, value}, key = $key",
        params,
        cozo::ScriptMutability::Immutable,
    ) {
        Ok(rows) => Ok(rows.rows.iter().any(
            |row| matches!(row.first(), Some(cozo::DataValue::Str(v)) if v.as_str() == "true"),
        )),
        Err(e) => {
            let msg = e.to_string().to_lowercase();
            if msg.contains("not found")
                || msg.contains("does not exist")
                || msg.contains("cannot find")
            {
                Ok(false)
            } else {
                Err(map_db_err(format!("schema_meta flag introspection: {e}")))
            }
        }
    }
}

/// Set the durable `schema_meta` flag `key` to `"true"`.
///
/// Idempotent: the `schema_meta` relation is created if absent (safe when the
/// caller bypassed full bootstrap), then the key/value pair is upserted.
///
/// # Errors
/// Returns [`EngramError`] when the relation create or upsert fails.
fn set_schema_meta_flag(cozo_db: &cozo::DbInstance, key: &str) -> Result<(), EngramError> {
    run_script_retrying(cozo_db, CREATE_SCHEMA_META)?;
    let mut params = BTreeMap::new();
    params.insert("key".to_string(), cozo::DataValue::from(key));
    let put = r#"
?[key, value] <- [[$key, "true"]]

:put schema_meta { key => value }
"#;
    cozo_db
        .run_script(put, params, cozo::ScriptMutability::Mutable)
        .map_err(|e| map_db_err(format!("schema_meta flag upsert: {e}")))?;
    Ok(())
}

/// Return `true` when the `calls_edge` relation carries the `resolution`
/// column (082.003-T provenance attribute).
///
/// Uses `::columns` introspection so it works regardless of whether the
/// relation currently holds any rows. A missing `calls_edge` relation is
/// reported as `false` (the caller bootstraps it before migrating).
///
/// # Errors
/// Returns [`EngramError`] when the introspection query fails for a reason
/// other than the relation not existing.
pub(crate) fn calls_edge_has_resolution(cozo_db: &cozo::DbInstance) -> Result<bool, EngramError> {
    match cozo_db.run_script(
        "::columns calls_edge",
        BTreeMap::new(),
        cozo::ScriptMutability::Immutable,
    ) {
        Ok(rows) => Ok(rows.rows.iter().any(|row| {
            matches!(row.first(), Some(cozo::DataValue::Str(name)) if name.as_str() == "resolution")
        })),
        Err(e) => {
            let msg = e.to_string().to_lowercase();
            if msg.contains("not found")
                || msg.contains("does not exist")
                || msg.contains("cannot find")
            {
                Ok(false)
            } else {
                Err(map_db_err(format!("calls_edge column introspection: {e}")))
            }
        }
    }
}

/// Upgrade a legacy `calls_edge` relation to carry the `resolution` provenance
/// attribute (082.003-T).
///
/// Databases created before 082.003-T store `calls_edge` as
/// `{from, to => created_at}`. This migration rewrites the relation to
/// `{from, to => created_at, resolution}` and defaults every pre-existing row
/// to `direct` (all historical edges were in-file resolved). It is idempotent:
/// once the `resolution` column is present the function returns immediately, so
/// it is safe to invoke on every schema bootstrap.
///
/// # Errors
/// Returns [`EngramError`] when column introspection or the `:replace` rewrite
/// fails.
pub(crate) fn migrate_calls_edge_resolution(cozo_db: &cozo::DbInstance) -> Result<(), EngramError> {
    if calls_edge_has_resolution(cozo_db)? {
        return Ok(());
    }
    // 082.003-T (remediation): a durable rollback must survive daemon reopen.
    // `run_scripts` invokes this shape-detected migration on every `connect_db`,
    // so once an operator has intentionally rolled the column back (via
    // `migrate-down` / `rollback_calls_edge_resolution`) we must NOT silently
    // re-add it here. The durable `schema_meta` marker records that intent.
    if schema_meta_flag_set(cozo_db, CALLS_RESOLUTION_ROLLED_BACK_KEY)? {
        return Ok(());
    }
    let migrate = r#"
?[from, to, created_at, resolution] :=
    *calls_edge{from, to, created_at},
    resolution = "direct"

:replace calls_edge { from, to => created_at, resolution }
"#;
    cozo_db
        .run_script(migrate, BTreeMap::new(), cozo::ScriptMutability::Mutable)
        .map_err(|e| map_db_err(format!("calls_edge resolution migration: {e}")))?;
    Ok(())
}

/// Down-migration: drop the `resolution` attribute from `calls_edge`,
/// reverting it to the legacy `{from, to => created_at}` schema (082.010-T).
///
/// This is the structural half of the rollback; callers must retract
/// `calls_resolved_singleton` edges *before* invoking it (while the column
/// still exists). Idempotent: no-op when the `resolution` column is already
/// absent, so a rollback can be safely rerun.
///
/// # Errors
/// Returns [`EngramError`] when column introspection or the `:replace` rewrite
/// fails.
pub(crate) fn rollback_calls_edge_resolution(
    cozo_db: &cozo::DbInstance,
) -> Result<(), EngramError> {
    // 082.003-T (remediation): record the durable rollback intent up-front, so a
    // crash between the drop and marker write cannot leave the schema in a state
    // where the next bootstrap re-adds the column. Setting the marker first is
    // safe: `migrate_calls_edge_resolution` only consults it when the column is
    // already absent, and forward re-enable (clearing the marker) is out of
    // scope. The `schema_meta` relation is provisioned by `run_scripts`, but we
    // create it defensively here too so a direct rollback on a legacy in-memory
    // database (bypassing bootstrap) still records the marker.
    set_schema_meta_flag(cozo_db, CALLS_RESOLUTION_ROLLED_BACK_KEY)?;
    if !calls_edge_has_resolution(cozo_db)? {
        return Ok(());
    }
    let rollback = r#"
?[from, to, created_at] := *calls_edge{from, to, created_at}

:replace calls_edge { from, to => created_at }
"#;
    cozo_db
        .run_script(rollback, BTreeMap::new(), cozo::ScriptMutability::Mutable)
        .map_err(|e| map_db_err(format!("calls_edge resolution rollback: {e}")))?;
    Ok(())
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
/// The `resolution` value records provenance (082.003-T): `direct` for an
/// in-file resolved call, `calls_resolved_singleton` for a cross-file call
/// resolved by the unambiguous-name post-pass (082.008-T). Databases created
/// before this attribute existed are upgraded by
/// [`migrate_calls_edge_resolution`].
pub const CREATE_CALLS_EDGE: &str = r#"
:create calls_edge {
    from: String,
    to: String
    =>
    created_at: String,
    resolution: String,
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

/// CozoScript `:create` for `staged_call` — a call site whose callee could not
/// be resolved within the caller's own file (082.002-T).
///
/// Key: `(caller_id, callee_name, source_file)` — every staged row carries its
/// source file so the staging lifecycle (082.009-T) can clear a file's rows
/// before re-indexing. The deferred post-pass (082.008-T) reads these rows and
/// resolves each callee against the workspace-global symbol index.
pub const CREATE_STAGED_CALL: &str = r#"
:create staged_call {
    caller_id: String,
    callee_name: String,
    source_file: String
    =>
    created_at: String,
}
"#;

/// CozoScript `:create` for `schema_meta` — durable key/value schema metadata
/// (082.003-T remediation).
///
/// Key: `key`. Value: `value`. Records durable migration state such as the
/// `calls_resolution_rolled_back` marker, so an intentional `migrate-down`
/// survives daemon reopen (the shape-detected up-migration consults this
/// relation before re-adding a rolled-back column).
pub const CREATE_SCHEMA_META: &str = r#"
:create schema_meta {
    key: String
    =>
    value: String,
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

// ── 061-F: Power BI graph relations ──────────────────────────────────────

/// CozoScript `:create` for `powerbi_node` — a single Power BI entity node.
///
/// Key: `id` — stable synthetic ID derived from the workspace-relative path
/// and entity name.  Stored separately from code-symbol and backlog tables
/// to prevent cross-domain key conflicts and keep Power BI traversal queries
/// isolated.
pub const CREATE_POWERBI_NODE: &str = r#"
:create powerbi_node {
    id: String
    =>
    name: String,
    kind: String,
    file_path: String,
    source_path: String,
    content_hash: String,
    ingested_at: String,
}
"#;

/// CozoScript `:create` for `powerbi_edge` — a directed relationship between
/// two Power BI nodes.
///
/// Key: `(from_id, to_id, edge_type)` — composite key supports multiple
/// relationship types between the same pair of nodes.  Edge type strings use
/// the `pbi_` namespace prefix (e.g. `pbi_contains`, `pbi_relates_to_table`)
/// to avoid collisions with code and backlog edge types in shared traversal.
pub const CREATE_POWERBI_EDGE: &str = r#"
:create powerbi_edge {
    from_id: String,
    to_id: String,
    edge_type: String
    =>
    source_path: String,
}
"#;

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{
        calls_edge_has_resolution, migrate_calls_edge_resolution, rollback_calls_edge_resolution,
        run_scripts,
    };

    /// Build an in-memory CozoDB with a *legacy* `calls_edge` relation
    /// (`{from, to => created_at}`, no `resolution` column) holding one row.
    fn legacy_db_with_one_edge() -> cozo::DbInstance {
        let db = cozo::DbInstance::new("mem", "", Default::default())
            .expect("in-memory cozo instance must open");
        db.run_script(
            ":create calls_edge { from: String, to: String => created_at: String }",
            BTreeMap::new(),
            cozo::ScriptMutability::Mutable,
        )
        .expect("legacy calls_edge relation must create");
        db.run_script(
            r#"?[from, to, created_at] <- [["fn:a", "fn:b", "2026-01-01T00:00:00Z"]]
:put calls_edge { from, to => created_at }"#,
            BTreeMap::new(),
            cozo::ScriptMutability::Mutable,
        )
        .expect("legacy row must insert");
        db
    }

    /// Read the single `resolution` value stored for the `(fn:a, fn:b)` edge.
    fn resolution_of_ab(db: &cozo::DbInstance) -> String {
        let rows = db
            .run_script(
                r#"?[resolution] := *calls_edge{from, to, resolution}, from = "fn:a", to = "fn:b""#,
                BTreeMap::new(),
                cozo::ScriptMutability::Immutable,
            )
            .expect("resolution query must run")
            .rows;
        assert_eq!(rows.len(), 1, "exactly one (fn:a, fn:b) edge expected");
        match rows[0].first() {
            Some(cozo::DataValue::Str(s)) => s.to_string(),
            other => panic!("resolution must be a string, got {other:?}"),
        }
    }

    // Scenario 1: migrating a legacy relation adds the `resolution` column and
    // defaults every pre-existing row to `direct` (082.003-T).
    #[test]
    fn migration_adds_resolution_column_defaulting_to_direct() {
        let db = legacy_db_with_one_edge();
        assert!(
            !calls_edge_has_resolution(&db).expect("column probe must run"),
            "legacy relation must not yet carry the resolution column"
        );

        migrate_calls_edge_resolution(&db).expect("migration must succeed");

        assert!(
            calls_edge_has_resolution(&db).expect("column probe must run"),
            "migration must add the resolution column"
        );
        assert_eq!(
            resolution_of_ab(&db),
            "direct",
            "pre-existing rows must default to `direct`"
        );
    }

    // Scenario 1 (idempotency): re-running the migration on an already-upgraded
    // relation is a no-op and preserves existing provenance values.
    #[test]
    fn migration_is_idempotent() {
        let db = legacy_db_with_one_edge();
        migrate_calls_edge_resolution(&db).expect("first migration must succeed");
        migrate_calls_edge_resolution(&db).expect("second migration must be a no-op");
        assert_eq!(
            resolution_of_ab(&db),
            "direct",
            "re-running the migration must not alter existing provenance"
        );
    }

    // Scenario 2 (082.010-T): after singletons are retracted, dropping the
    // `resolution` column reverts `calls_edge` to `{from, to => created_at}` and
    // the legacy 2-attribute writer round-trips against the reverted schema.
    #[test]
    fn rollback_drops_resolution_and_legacy_writer_round_trips() {
        let db = legacy_db_with_one_edge();
        migrate_calls_edge_resolution(&db).expect("migration must succeed");
        assert!(
            calls_edge_has_resolution(&db).expect("column probe must run"),
            "relation must carry the resolution column before rollback"
        );

        rollback_calls_edge_resolution(&db).expect("rollback must succeed");
        assert!(
            !calls_edge_has_resolution(&db).expect("column probe must run"),
            "rollback must drop the resolution column"
        );

        // The reverted schema accepts the legacy `{from, to => created_at}` writer.
        db.run_script(
            r#"?[from, to, created_at] <- [["fn:c", "fn:d", "2026-02-02T00:00:00Z"]]
:put calls_edge { from, to => created_at }"#,
            BTreeMap::new(),
            cozo::ScriptMutability::Mutable,
        )
        .expect("legacy writer must round-trip on the reverted schema");
        let rows = db
            .run_script(
                r#"?[from, to, created_at] := *calls_edge{from, to, created_at}, from = "fn:c""#,
                BTreeMap::new(),
                cozo::ScriptMutability::Immutable,
            )
            .expect("legacy read must run")
            .rows;
        assert_eq!(rows.len(), 1, "legacy row must be readable after rollback");
    }

    // Scenario 2 (idempotency): the structural drop is a no-op when the
    // `resolution` column is already absent.
    #[test]
    fn rollback_calls_edge_resolution_is_idempotent() {
        let db = legacy_db_with_one_edge();
        rollback_calls_edge_resolution(&db).expect("rollback on legacy schema must be a no-op");
        assert!(
            !calls_edge_has_resolution(&db).expect("column probe must run"),
            "legacy relation must remain columnless after a no-op rollback"
        );
    }

    // 082.003-T durability (remediation): a rollback must SURVIVE a subsequent
    // schema bootstrap (daemon reopen). `run_scripts` calls the shape-detected
    // `migrate_calls_edge_resolution` on EVERY `connect_db`; without a durable
    // rollback marker that up-migration re-adds the `resolution` column on the
    // very next reopen, silently undoing `migrate-down`. This pins the fix: once
    // rolled back, the column must stay absent across a full bootstrap.
    #[test]
    fn rollback_survives_reopen_bootstrap() {
        let db = cozo::DbInstance::new("mem", "", Default::default())
            .expect("in-memory cozo instance must open");

        // Initial bootstrap provisions calls_edge WITH the resolution column
        // (CREATE_CALLS_EDGE) and runs the up-migration (a no-op here).
        run_scripts(&db).expect("initial bootstrap must succeed");
        assert!(
            calls_edge_has_resolution(&db).expect("column probe must run"),
            "initial bootstrap must provision the resolution column"
        );

        // Operator rollback: drop the column and record the durable marker.
        rollback_calls_edge_resolution(&db).expect("rollback must succeed");
        assert!(
            !calls_edge_has_resolution(&db).expect("column probe must run"),
            "rollback must drop the resolution column"
        );

        // Reopen: a second full bootstrap must NOT re-add the column now that the
        // durable rollback marker is set.
        run_scripts(&db).expect("reopen bootstrap must succeed");
        assert!(
            !calls_edge_has_resolution(&db).expect("column probe must run"),
            "rollback must survive a reopen bootstrap: the resolution column must stay absent"
        );
    }
}
