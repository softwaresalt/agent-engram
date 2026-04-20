//! CozoScript schema definitions for the CozoDB backend.
//!
//! Phase 2 (U2.1) — relation-creation constants and bootstrap execution.
//!
//! Three-table layout per symbol type (function / class / interface):
//!  * `*_meta`      — identity and source-position fields (key lookups)
//!  * `*_code`      — raw body text (separate to avoid over-fetching)
//!  * `*_embedding` — float vector (isolated for KNN efficiency)

use std::collections::BTreeMap;

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
    ];

    for script in &scripts {
        match cozo_db.run_script(script, BTreeMap::new(), cozo::ScriptMutability::Mutable) {
            Ok(_) => {}
            Err(e) => {
                let msg = e.to_string().to_lowercase();
                if msg.contains("already")
                    || msg.contains("defined")
                    || msg.contains("conflicts")
                    || msg.contains("existing")
                {
                    continue;
                }
                return Err(map_db_err(format!("schema bootstrap: {e}")));
            }
        }
    }

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
