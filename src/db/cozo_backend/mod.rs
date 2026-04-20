//! CozoDB backend — Phase 2 connection and handle management.
//!
//! Replaces the inline `cozo_db` stub in `db/mod.rs` with a proper module
//! that will wire up `cozo::DbInstance` in Phase 2 (U2.1).
//!
//! Until `connect_db` is implemented, all callers receive a
//! `SystemError::DatabaseError` sentinel that describes the missing
//! implementation.  This preserves the Phase 1 smoke-test invariant.

pub mod schema;

use std::path::Path;

use crate::errors::{EngramError, SystemError};

// ── Handle type ────────────────────────────────────────────────────────────

/// Opaque CozoDB connection handle.
///
/// Phase 2 (U2.1) will replace this with a real `cozo::DbInstance` wrapper
/// backed by an SQLite store under `.engram/db/{branch}/`.  The struct is
/// kept as a unit type so that the rest of the codebase compiles and test
/// harnesses can construct a handle without a live database.
#[derive(Clone, Debug)]
pub struct CozoHandle;

/// The active database handle type for the CozoDB backend.
pub type Db = CozoHandle;

// ── Connection ─────────────────────────────────────────────────────────────

/// Open (or return a cached) CozoDB handle for the given workspace.
///
/// Not yet implemented — Phase 2 (U2.1) will call
/// `cozo::DbInstance::new("sqlite", path, Default::default())`.
///
/// # Errors
/// Always returns `SystemError::DatabaseError` until Phase 2.
pub async fn connect_db(_data_dir: &Path, _branch: &str) -> Result<Db, EngramError> {
    Err(EngramError::from(SystemError::DatabaseError {
        reason: "CozoDB backend connection not yet implemented (Phase 2)".into(),
    }))
}

// ── Error mapping ──────────────────────────────────────────────────────────

/// Map any error value into an [`EngramError`] database error.
pub fn map_db_err<E: ToString>(err: E) -> EngramError {
    EngramError::from(SystemError::DatabaseError {
        reason: err.to_string(),
    })
}
