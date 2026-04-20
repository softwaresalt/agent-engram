//! CozoScript schema definitions for the CozoDB backend.
//!
//! Phase 2 (U2.1) populates these constants with real CozoScript
//! ``:create`` relation statements and wires `run_schema_bootstrap`
//! to apply them against a live `DbInstance`.

use crate::errors::EngramError;

use super::Db;

// ── Bootstrap ─────────────────────────────────────────────────────────────

/// Bootstrap all CozoDB relations for a fresh workspace.
///
/// Creates: `file_node`, `function_meta`, `function_code`,
/// `function_embedding`, `class_meta`, `class_code`, `class_embedding`,
/// `interface_meta`, `interface_code`, `interface_embedding`,
/// `import_node`, `commit_node`.
///
/// # Errors
/// Returns an [`EngramError`] when any schema statement fails to execute.
pub fn run_schema_bootstrap(_db: &Db) -> Result<(), EngramError> {
    unimplemented!("Worker: schema bootstrap not implemented (Phase 2 U2.1)")
}

// ── File node ──────────────────────────────────────────────────────────────

/// CozoScript ```:create``` statement for the ``file_node`` relation.
///
/// Schema:
/// ```text
/// file_node { path: String => language, size_bytes, content_hash, last_indexed_at }
/// ```
pub const CREATE_FILE_NODE: &str = "";

// ── Function relations ─────────────────────────────────────────────────────

/// CozoScript ```:create``` statement for ``function_meta``.
pub const CREATE_FUNCTION_META: &str = "";

/// CozoScript ```:create``` statement for ``function_code``.
pub const CREATE_FUNCTION_CODE: &str = "";

/// CozoScript ```:create``` statement for ``function_embedding``.
pub const CREATE_FUNCTION_EMBEDDING: &str = "";

// ── Class relations ────────────────────────────────────────────────────────

/// CozoScript ```:create``` statement for ``class_meta``.
pub const CREATE_CLASS_META: &str = "";

/// CozoScript ```:create``` statement for ``class_code``.
pub const CREATE_CLASS_CODE: &str = "";

/// CozoScript ```:create``` statement for ``class_embedding``.
pub const CREATE_CLASS_EMBEDDING: &str = "";

// ── Interface relations ────────────────────────────────────────────────────

/// CozoScript ```:create``` statement for ``interface_meta``.
pub const CREATE_INTERFACE_META: &str = "";

/// CozoScript ```:create``` statement for ``interface_code``.
pub const CREATE_INTERFACE_CODE: &str = "";

/// CozoScript ```:create``` statement for ``interface_embedding``.
pub const CREATE_INTERFACE_EMBEDDING: &str = "";

// ── Auxiliary relations ────────────────────────────────────────────────────

/// CozoScript ```:create``` statement for ``import_node``.
pub const CREATE_IMPORT_NODE: &str = "";

/// CozoScript ```:create``` statement for ``commit_node``.
pub const CREATE_COMMIT_NODE: &str = "";
