//! CozoDB backend — Phase 2 connection and handle management.
//!
//! `CozoHandle` is a unit-struct marker for schema unit tests.
//! `CozoDb` is the real production handle backed by `Arc<cozo::DbInstance>`.

pub mod schema;

use std::{path::Path, sync::Arc};

use crate::errors::{EngramError, SystemError};

// ── Handle types ──────────────────────────────────────────────────────────────

/// Unit-struct CozoDB marker, used only by schema unit tests.
///
/// The test harness creates `let handle = CozoHandle;` (unit struct syntax).
/// Does NOT hold database state; use [`CozoDb`] for all production paths.
#[derive(Clone, Debug)]
pub struct CozoHandle;

/// Production CozoDB connection handle backed by a `cozo::DbInstance`.
///
/// `CozoDb: Send + Sync` because `cozo::DbInstance` uses internal `Arc<RwLock<…>>`
/// for all mutable state (CozoDB 0.7 guarantee). Do not remove the `Arc` wrapper
/// without re-verifying thread safety.
#[derive(Clone)]
pub struct CozoDb {
    pub(crate) inner: Arc<cozo::DbInstance>,
}

impl std::fmt::Debug for CozoDb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CozoDb")
            .field("inner", &"<cozo::DbInstance>")
            .finish()
    }
}

/// The active database handle type for the CozoDB backend.
pub type Db = CozoDb;

// ── Bootstrap trait ───────────────────────────────────────────────────────────

/// Provides a `cozo::DbInstance` to [`schema::run_schema_bootstrap`].
///
/// `CozoHandle` opens a temporary in-memory DB (validates CozoScript syntax).
/// `CozoDb` returns the persistent DB instance.
pub(crate) trait SchemaTarget {
    /// Acquire the `cozo::DbInstance` for schema bootstrap.
    ///
    /// # Errors
    /// Returns [`EngramError`] if the instance cannot be created or accessed.
    fn cozo_instance(&self) -> Result<Arc<cozo::DbInstance>, EngramError>;
}

impl SchemaTarget for CozoHandle {
    fn cozo_instance(&self) -> Result<Arc<cozo::DbInstance>, EngramError> {
        cozo::DbInstance::new("mem", "", Default::default())
            .map(Arc::new)
            .map_err(|e| map_db_err(e.to_string()))
    }
}

impl SchemaTarget for CozoDb {
    fn cozo_instance(&self) -> Result<Arc<cozo::DbInstance>, EngramError> {
        Ok(Arc::clone(&self.inner))
    }
}

// ── Connection ────────────────────────────────────────────────────────────────

/// Open a CozoDB SQLite handle for the given workspace branch.
///
/// Creates `{data_dir}/cozo/{branch}/engram.db` if it does not exist,
/// opens a SQLite-backed `cozo::DbInstance`, and bootstraps the schema
/// idempotently (`:create` errors for existing relations are silently ignored).
///
/// # Errors
/// Returns [`EngramError`] when the directory cannot be created, the
/// database cannot be opened, or schema bootstrap fails with an unexpected error.
pub async fn connect_db(data_dir: &Path, branch: &str) -> Result<Db, EngramError> {
    let branch_safe = branch.replace(['/', '\\', ':'], "_");
    let db_dir = data_dir.join("cozo").join(&branch_safe);
    std::fs::create_dir_all(&db_dir)
        .map_err(|e| map_db_err(format!("cannot create CozoDB dir: {e}")))?;

    let db_path = db_dir.join("engram.db");
    let db_path_str = db_path
        .to_str()
        .ok_or_else(|| map_db_err("CozoDB path is not valid UTF-8"))?;

    let db = cozo::DbInstance::new("sqlite", db_path_str, Default::default())
        .map_err(|e| map_db_err(format!("cannot open CozoDB SQLite store: {e}")))?;

    let cozo_db = CozoDb {
        inner: Arc::new(db),
    };

    schema::run_schema_bootstrap(&cozo_db)?;

    Ok(cozo_db)
}

// ── Error mapping ─────────────────────────────────────────────────────────────

/// Map any error value into an [`EngramError`] database error.
pub fn map_db_err<E: ToString>(err: E) -> EngramError {
    EngramError::from(SystemError::DatabaseError {
        reason: err.to_string(),
    })
}
