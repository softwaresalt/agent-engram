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
        run_script_retrying(cozo_db, script, "schema bootstrap", true)?;
    }

    // 082.003-T: upgrade pre-existing `calls_edge` relations that predate the
    // `resolution` provenance attribute. Idempotent: no-op once the column
    // exists, so it is safe to run on every bootstrap.
    migrate_calls_edge_resolution(cozo_db)?;

    // 091.008-T (Option C A6): additive `function_meta.canonical_path` column,
    // precision-neutral. Idempotent shape-detected upgrade; existing rows default
    // to "" (never a canonical match target — D4). New/re-parsed rows are
    // populated opportunistically at index time; pre-existing rows stay "" until
    // a full re-index or the deferred ID-preserving backfill. That backfill is a
    // follow-up NOT included in Unit B (091.012-T): the A8 forced re-index was
    // removed as unsafe (symbol IDs are random UUIDs, so re-parsing unchanged
    // files would disturb the existing edge set). Until it ships, an
    // already-indexed workspace gains canonical edges only for files re-parsed by
    // a normal index/sync (fail-closed: pre-existing rows stay "" = no edge).
    migrate_function_meta_canonical_path(cozo_db)?;
    migrate_staged_call_provenance(cozo_db)?;

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

/// Run a single CozoDB `script`, retrying on transient `SQLITE_BUSY`
/// ("database is locked").
///
/// Delegates to [`retry_cozo_script`]. `context` labels any surfaced error
/// (e.g. `"schema bootstrap"`, `"calls_edge resolution migration"`).
/// `allow_already_exists` MUST be `true` only for idempotent `:create` scripts;
/// `:replace` migrate/rollback rewrites pass `false` so a genuine failure is
/// never masked as success (086.001-T F1).
fn run_script_retrying(
    cozo_db: &cozo::DbInstance,
    script: &str,
    context: &str,
    allow_already_exists: bool,
) -> Result<(), EngramError> {
    retry_cozo_script(
        context,
        allow_already_exists,
        || {
            cozo_db
                .run_script(script, BTreeMap::new(), cozo::ScriptMutability::Mutable)
                .map(|_| ())
        },
        |attempt| std::thread::sleep(busy_backoff(attempt)),
    )
}

// ── 086.001-T: shared bounded-retry driver for CozoDB scripts ──────────────
//
// Extracted so the destructive `calls_edge` resolution migrate/rollback `:replace`
// rewrites — which previously bypassed the retry helper and called `run_script`
// directly — route through the SAME SQLITE_BUSY-tolerant path as the rest of
// bootstrap. Under a concurrent open on a shared data dir the one-time upgrade
// can otherwise fail with SQLITE_BUSY and abort startup instead of retrying.

/// Maximum attempts for a single retried CozoDB script.
const MAX_SCRIPT_ATTEMPTS: u32 = 20;

/// Retry classification for a CozoDB script error message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScriptOutcome {
    /// The relation already exists — a benign, idempotent `:create` outcome.
    AlreadyExists,
    /// A transient `SQLITE_BUSY` ("database is locked") — safe to retry.
    Busy,
    /// A genuine error that must be surfaced to the caller.
    Fatal,
}

/// Classify a CozoDB error message for the bounded-retry driver.
///
/// Mirrors the historical bootstrap behaviour: a `:create` against an existing
/// relation is a benign no-op, a `SQLITE_BUSY` ("database is locked") is a
/// transient worth retrying, and anything else is a genuine failure.
fn classify_script_error(message: &str) -> ScriptOutcome {
    let msg = message.to_lowercase();
    if msg.contains("already")
        || msg.contains("defined")
        || msg.contains("conflicts")
        || msg.contains("existing")
    {
        ScriptOutcome::AlreadyExists
    } else if msg.contains("locked") || msg.contains("busy") {
        ScriptOutcome::Busy
    } else {
        ScriptOutcome::Fatal
    }
}

/// Capped exponential back-off for busy retry `attempt` (25 ms → 500 ms cap).
///
/// 25 ms doubling through attempt 4 (25+50+100+200+400), then a 500 ms cap from
/// attempt 5, gives ≈ 7.8 s of total retry headroom over [`MAX_SCRIPT_ATTEMPTS`],
/// more than enough for a concurrent writer to release its write transaction.
fn busy_backoff(attempt: u32) -> Duration {
    Duration::from_millis(std::cmp::min(25u64 << attempt.min(5), 500))
}

/// Bounded-retry driver: run `run`, retrying a transient busy (invoking `sleep`
/// with the attempt index between tries). `allow_already_exists` controls whether
/// an "already exists" outcome is treated as benign success — TRUE for idempotent
/// `:create` bootstrap, FALSE for `:replace` migrate/rollback rewrites where such
/// a message would indicate a genuine failure that must NOT be masked (086.001-T
/// F1). Any non-retryable error is surfaced immediately as an [`EngramError`]
/// labelled with `context`; an exhausted budget surfaces an error, never a panic.
fn retry_cozo_script<E, F, S>(
    context: &str,
    allow_already_exists: bool,
    mut run: F,
    mut sleep: S,
) -> Result<(), EngramError>
where
    E: std::fmt::Display,
    F: FnMut() -> Result<(), E>,
    S: FnMut(u32),
{
    for attempt in 0..MAX_SCRIPT_ATTEMPTS {
        match run() {
            Ok(()) => return Ok(()),
            Err(e) => match classify_script_error(&e.to_string()) {
                ScriptOutcome::AlreadyExists if allow_already_exists => return Ok(()),
                ScriptOutcome::Busy if attempt + 1 < MAX_SCRIPT_ATTEMPTS => sleep(attempt),
                _ => return Err(map_db_err(format!("{context}: {e}"))),
            },
        }
    }
    // The final attempt always returns via the match above, so this is provably
    // unreachable; surface an error rather than panic to keep the fn total (F3).
    Err(map_db_err(format!(
        "{context}: retry budget exhausted after {MAX_SCRIPT_ATTEMPTS} attempts"
    )))
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
    run_script_retrying(cozo_db, CREATE_SCHEMA_META, "schema_meta bootstrap", true)?;
    let mut params = BTreeMap::new();
    params.insert("key".to_string(), cozo::DataValue::from(key));
    let put = r#"
?[key, value] <- [[$key, "true"]]

:put schema_meta { key => value }
"#;
    // Route the idempotent marker upsert through the busy-tolerant retry path too
    // (Rust-review nit): a transient SQLITE_BUSY must not fail the durable
    // rollback marker write. `:put` never legitimately reports "already exists",
    // so already-exists is treated as fatal (allow_already_exists = false).
    retry_cozo_script(
        "schema_meta flag upsert",
        false,
        || {
            cozo_db
                .run_script(put, params.clone(), cozo::ScriptMutability::Mutable)
                .map(|_| ())
        },
        |attempt| std::thread::sleep(busy_backoff(attempt)),
    )?;
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
    run_script_retrying(cozo_db, migrate, "calls_edge resolution migration", false)?;
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
    run_script_retrying(cozo_db, rollback, "calls_edge resolution rollback", false)?;
    Ok(())
}

/// Return `true` when `function_meta` carries the additive `canonical_path`
/// column (091.008-T / Option C A6).
///
/// Uses `::columns` introspection so it works regardless of row count. A missing
/// `function_meta` relation is reported as `false` (the caller bootstraps it
/// before migrating).
///
/// # Errors
/// Returns [`EngramError`] when the introspection query fails for a reason other
/// than the relation not existing.
pub(crate) fn function_meta_has_canonical_path(
    cozo_db: &cozo::DbInstance,
) -> Result<bool, EngramError> {
    match cozo_db.run_script(
        "::columns function_meta",
        BTreeMap::new(),
        cozo::ScriptMutability::Immutable,
    ) {
        Ok(rows) => Ok(rows.rows.iter().any(|row| {
            matches!(row.first(), Some(cozo::DataValue::Str(name)) if name.as_str() == "canonical_path")
        })),
        Err(e) => {
            let msg = e.to_string().to_lowercase();
            if msg.contains("not found")
                || msg.contains("does not exist")
                || msg.contains("cannot find")
            {
                Ok(false)
            } else {
                Err(map_db_err(format!(
                    "function_meta column introspection: {e}"
                )))
            }
        }
    }
}

/// Return `true` when `staged_call` carries Unit-B raw provenance columns.
///
/// Uses `::columns` introspection so a legacy empty staging relation is still
/// detected accurately.
pub(crate) fn staged_call_has_provenance(cozo_db: &cozo::DbInstance) -> Result<bool, EngramError> {
    match cozo_db.run_script(
        "::columns staged_call",
        BTreeMap::new(),
        cozo::ScriptMutability::Immutable,
    ) {
        Ok(rows) => Ok(rows.rows.iter().any(|row| {
            matches!(row.first(), Some(cozo::DataValue::Str(name)) if name.as_str() == "raw_qualifier")
        })),
        Err(e) => {
            let msg = e.to_string().to_lowercase();
            if msg.contains("not found")
                || msg.contains("does not exist")
                || msg.contains("cannot find")
            {
                Ok(false)
            } else {
                Err(map_db_err(format!("staged_call column introspection: {e}")))
            }
        }
    }
}

/// Upgrade a legacy `function_meta` relation to carry the additive
/// `canonical_path` column (091.008-T / Option C A6, precision-neutral).
///
/// Databases created before A6 store `function_meta` without `canonical_path`.
/// This rewrites the relation to add the column, defaulting every pre-existing
/// row to `""` (empty is **never** a canonical match target — D4). The existing
/// `name` column is left untouched, so search, references, and bare-name
/// resolution are unaffected. Idempotent: once the column is present the
/// function returns immediately, so it is safe to run on every bootstrap.
/// `canonical_path` is populated opportunistically at index time for
/// new/re-parsed rows; pre-existing rows remain `""` until a full re-index or
/// the deferred ID-preserving backfill (a follow-up NOT included in Unit B; the
/// A8 forced re-index was removed as unsafe). Empty is never a match target (D4),
/// so the gap is fail-closed.
///
/// # Errors
/// Returns [`EngramError`] when column introspection or the `:replace` rewrite
/// fails.
pub(crate) fn migrate_function_meta_canonical_path(
    cozo_db: &cozo::DbInstance,
) -> Result<(), EngramError> {
    if function_meta_has_canonical_path(cozo_db)? {
        return Ok(());
    }
    let migrate = r#"
?[id, name, file_path, line_start, line_end, signature, docstring, body_hash, token_count, embed_type, summary, canonical_path] :=
    *function_meta{id, name, file_path, line_start, line_end, signature, docstring, body_hash, token_count, embed_type, summary},
    canonical_path = ""

:replace function_meta { id => name, file_path, line_start, line_end, signature, docstring, body_hash, token_count, embed_type, summary, canonical_path }
"#;
    run_script_retrying(
        cozo_db,
        migrate,
        "function_meta canonical_path migration",
        false,
    )?;
    Ok(())
}

/// Upgrade legacy `staged_call` rows to carry Unit-B raw provenance markers.
///
/// Existing 084-S/089-F rows default each marker to `""`, which keeps them on
/// the legacy bare-name path and preserves dehydrate/rehydrate compatibility.
/// `raw_qualifier` is promoted into the relation key (091.012-T) so distinct
/// qualified calls to the same callee name in one caller no longer collide;
/// legacy rows migrate with `raw_qualifier = ""`.
pub(crate) fn migrate_staged_call_provenance(
    cozo_db: &cozo::DbInstance,
) -> Result<(), EngramError> {
    if staged_call_has_provenance(cozo_db)? {
        return Ok(());
    }
    let migrate = r#"
?[caller_id, callee_name, source_file, created_at, raw_qualifier, qualifier_kind, enclosing_canonical_type] :=
    *staged_call{caller_id, callee_name, source_file, created_at},
    raw_qualifier = "",
    qualifier_kind = "",
    enclosing_canonical_type = ""

:replace staged_call { caller_id, callee_name, source_file, raw_qualifier, qualifier_kind => created_at, enclosing_canonical_type }
"#;
    run_script_retrying(cozo_db, migrate, "staged_call provenance migration", false)?;
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
    canonical_path: String,
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
/// Key: `(caller_id, callee_name, source_file, raw_qualifier, qualifier_kind)` —
/// both `raw_qualifier` and `qualifier_kind` are part of the key so two distinct
/// qualified calls to the same callee name in one caller stay separate rows and
/// each resolves to its own canonical target instead of overwriting one another.
/// `raw_qualifier` (091.012-T) separates different qualifiers of the same callee
/// name (e.g. `crate::a::build()` vs `crate::b::build()`). `qualifier_kind`
/// additionally separates call sites that share a `raw_qualifier` but differ in
/// kind and resolved target — notably `self::foo()` (kind `module`) vs
/// `self.foo()` (kind `method`), which both carry `raw_qualifier == "self"` and
/// would otherwise collide, silently dropping one canonical edge. Every staged
/// row carries its source file so the staging lifecycle (082.009-T) can clear a
/// file's rows before re-indexing. The deferred post-pass (082.008-T /
/// 091.012-T) reads these rows and resolves each callee against the
/// workspace-global symbol index.
pub const CREATE_STAGED_CALL: &str = r#"
:create staged_call {
    caller_id: String,
    callee_name: String,
    source_file: String,
    raw_qualifier: String,
    qualifier_kind: String
    =>
    created_at: String,
    enclosing_canonical_type: String,
}
"#;

/// CozoScript `:create` for the index-time unsafe module-prefix snapshot.
///
/// Key: `snapshot_id` (currently `"current"`). The single row's `prefixes`
/// value records the exact unsafe-prefix set used by the latest full index or
/// incremental sync for this branch-scoped database. This relation is created
/// by the writer, not bootstrap, so older databases with no persisted snapshot
/// remain distinguishable from freshly indexed databases whose prefix set is
/// legitimately empty.
pub const CREATE_INDEX_UNSAFE_MODULE_PREFIX_SNAPSHOT: &str = r#"
:create index_unsafe_module_prefix_snapshot {
    snapshot_id: String
    =>
    prefixes: [String],
    recorded_at: String,
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
        MAX_SCRIPT_ATTEMPTS, ScriptOutcome, busy_backoff, classify_script_error, retry_cozo_script,
    };
    use super::{
        calls_edge_has_resolution, function_meta_has_canonical_path, migrate_calls_edge_resolution,
        migrate_function_meta_canonical_path, rollback_calls_edge_resolution, run_scripts,
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

    /// Build an in-memory CozoDB with a *legacy* `function_meta` relation (no
    /// `canonical_path` column) holding one row (091.008-T / Option C A6).
    fn legacy_function_meta_db() -> cozo::DbInstance {
        let db = cozo::DbInstance::new("mem", "", Default::default())
            .expect("in-memory cozo instance must open");
        db.run_script(
            ":create function_meta { id: String => name: String, file_path: String, line_start: Int, line_end: Int, signature: String, docstring: String, body_hash: String, token_count: Int, embed_type: String, summary: String }",
            BTreeMap::new(),
            cozo::ScriptMutability::Mutable,
        )
        .expect("legacy function_meta relation must create");
        db.run_script(
            r#"?[id, name, file_path, line_start, line_end, signature, docstring, body_hash, token_count, embed_type, summary] <-
    [["fn:x", "greet", "src/lib.rs", 1, 2, "fn greet()", "", "hash", 3, "explicit_code", ""]]
:put function_meta { id, name, file_path, line_start, line_end, signature, docstring, body_hash, token_count, embed_type, summary }"#,
            BTreeMap::new(),
            cozo::ScriptMutability::Mutable,
        )
        .expect("legacy function_meta row must insert");
        db
    }

    // 091.008-T: migrating a legacy `function_meta` adds the additive
    // `canonical_path` column and defaults every pre-existing row to "" (D4).
    #[test]
    fn migration_adds_canonical_path_column_defaulting_empty() {
        let db = legacy_function_meta_db();
        assert!(
            !function_meta_has_canonical_path(&db).expect("column probe must run"),
            "legacy relation must not yet carry canonical_path"
        );

        migrate_function_meta_canonical_path(&db).expect("migration must succeed");

        assert!(
            function_meta_has_canonical_path(&db).expect("column probe must run"),
            "migration must add the canonical_path column"
        );
        let rows = db
            .run_script(
                r#"?[canonical_path] := *function_meta{id, canonical_path}, id = "fn:x""#,
                BTreeMap::new(),
                cozo::ScriptMutability::Immutable,
            )
            .expect("canonical_path query must run")
            .rows;
        assert_eq!(rows.len(), 1, "exactly one fn:x row expected");
        match rows[0].first() {
            Some(cozo::DataValue::Str(s)) => {
                assert_eq!(s.as_str(), "", "pre-existing rows must default to empty");
            }
            other => panic!("canonical_path must be a string, got {other:?}"),
        }
    }

    // 091.008-T: re-running the canonical_path migration is a no-op.
    #[test]
    fn canonical_path_migration_is_idempotent() {
        let db = legacy_function_meta_db();
        migrate_function_meta_canonical_path(&db).expect("first migration must succeed");
        migrate_function_meta_canonical_path(&db).expect("second migration must be a no-op");
        assert!(function_meta_has_canonical_path(&db).expect("column probe must run"));
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

    // ── 086.001-T: bounded-retry driver used by migrate/rollback routing ─────

    // classify_script_error maps a busy/locked message to the retryable class.
    #[test]
    fn classify_script_error_recognizes_transient_busy() {
        assert_eq!(
            classify_script_error("Cannot execute: database is locked"),
            ScriptOutcome::Busy
        );
        assert_eq!(
            classify_script_error("SQLITE_BUSY error"),
            ScriptOutcome::Busy
        );
    }

    // classify_script_error maps an "already exists" message to the benign class.
    #[test]
    fn classify_script_error_recognizes_already_exists() {
        assert_eq!(
            classify_script_error("relation calls_edge already exists"),
            ScriptOutcome::AlreadyExists
        );
        assert_eq!(
            classify_script_error("conflicts with an existing relation"),
            ScriptOutcome::AlreadyExists
        );
    }

    // classify_script_error surfaces any other message as fatal.
    #[test]
    fn classify_script_error_treats_other_errors_as_fatal() {
        assert_eq!(
            classify_script_error("parse error: unexpected token"),
            ScriptOutcome::Fatal
        );
    }

    // busy_backoff grows exponentially from 25 ms and caps at 500 ms.
    #[test]
    fn busy_backoff_is_capped_exponential() {
        use std::time::Duration;
        assert_eq!(busy_backoff(0), Duration::from_millis(25));
        assert_eq!(busy_backoff(4), Duration::from_millis(400));
        assert_eq!(busy_backoff(5), Duration::from_millis(500));
        assert_eq!(busy_backoff(19), Duration::from_millis(500));
    }

    // A transient busy is retried until the operation succeeds, within budget.
    #[test]
    fn retry_cozo_script_succeeds_after_transient_busy() {
        let mut attempts = 0u32;
        let mut sleeps = 0u32;
        let result = retry_cozo_script(
            "test",
            true,
            || {
                attempts += 1;
                if attempts < 3 {
                    Err("database is locked")
                } else {
                    Ok(())
                }
            },
            |_attempt| sleeps += 1,
        );
        assert!(
            result.is_ok(),
            "must succeed once the busy clears: {result:?}"
        );
        assert_eq!(
            attempts, 3,
            "must retry twice then succeed on the third try"
        );
        assert_eq!(sleeps, 2, "must back off before each retry");
    }

    // A persistent busy is bounded and surfaces an EngramError (never a panic).
    #[test]
    fn retry_cozo_script_gives_up_after_max_attempts() {
        let mut attempts = 0u32;
        let result = retry_cozo_script::<&str, _, _>(
            "bootstrap",
            true,
            || {
                attempts += 1;
                Err("database is locked")
            },
            |_attempt| {},
        );
        assert!(
            result.is_err(),
            "persistent busy must give up with an error"
        );
        assert_eq!(
            attempts, MAX_SCRIPT_ATTEMPTS,
            "retry budget must be bounded by MAX_SCRIPT_ATTEMPTS"
        );
    }

    // An "already exists" outcome is treated as success without retrying.
    // With allow_already_exists = true (`:create` bootstrap), an "already exists"
    // outcome is treated as success without retrying.
    #[test]
    fn retry_cozo_script_swallows_already_exists_when_allowed() {
        let mut attempts = 0u32;
        let result = retry_cozo_script(
            "test",
            true,
            || {
                attempts += 1;
                Err("relation already exists")
            },
            |_attempt| {},
        );
        assert!(result.is_ok(), "already-exists must be benign success");
        assert_eq!(attempts, 1, "benign outcome must not retry");
    }

    // With allow_already_exists = false (`:replace` migrate/rollback), an
    // "already exists" outcome must NOT be masked as success — it surfaces as an
    // error so a genuine `:replace` failure cannot report a false success (F1).
    #[test]
    fn retry_cozo_script_surfaces_already_exists_as_fatal_when_disallowed() {
        let mut attempts = 0u32;
        let result = retry_cozo_script::<&str, _, _>(
            "calls_edge resolution rollback",
            false,
            || {
                attempts += 1;
                Err("relation already exists")
            },
            |_attempt| {},
        );
        assert!(
            result.is_err(),
            "a :replace rewrite must not swallow already-exists"
        );
        assert_eq!(attempts, 1, "a fatal outcome must not retry");
    }

    // A fatal error is surfaced immediately without retrying.
    #[test]
    fn retry_cozo_script_surfaces_fatal_immediately() {
        let mut attempts = 0u32;
        let result = retry_cozo_script::<&str, _, _>(
            "test",
            true,
            || {
                attempts += 1;
                Err("syntax error near ':'")
            },
            |_attempt| {},
        );
        assert!(result.is_err(), "fatal error must surface");
        assert_eq!(attempts, 1, "fatal error must not retry");
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
