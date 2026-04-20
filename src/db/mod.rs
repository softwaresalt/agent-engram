//! Database layer: connection management and query dispatch.
//!
//! Selects the backing store at compile time via feature flags.  Enable
//! exactly one of `surreal-backend` (default) or `cozo-backend`.

// Exactly one backend must be active at a time.
#[cfg(all(feature = "surreal-backend", feature = "cozo-backend"))]
compile_error!(
    "Features `surreal-backend` and `cozo-backend` are mutually exclusive; \
     enable exactly one database backend at a time."
);

#[cfg(not(any(feature = "surreal-backend", feature = "cozo-backend")))]
compile_error!(
    "No database backend feature enabled; \
     enable exactly one of `surreal-backend` or `cozo-backend`."
);

/// Workspace hash utilities — no backend-specific dependencies.
pub mod workspace;

// ── SurrealDB backend ──────────────────────────────────────────────────────

/// SurrealDB schema constants.
#[cfg(feature = "surreal-backend")]
pub mod schema;

/// SurrealDB query helpers.
#[cfg(feature = "surreal-backend")]
pub mod queries;

#[cfg(feature = "surreal-backend")]
mod surreal_db {
    use std::collections::HashMap;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::LazyLock;

    use surrealdb::Surreal;
    use surrealdb::engine::local::{Db as LocalDb, SurrealKv};
    use tokio::sync::RwLock;

    use crate::errors::{EngramError, SystemError};

    /// The active database handle type for the SurrealDB backend.
    pub type Db = Surreal<LocalDb>;

    /// Per-workspace connection cache.  Keyed by the resolved database path,
    /// each entry holds a cloneable `Surreal<LocalDb>` handle.
    static DB_CACHE: LazyLock<RwLock<HashMap<PathBuf, Db>>> =
        LazyLock::new(|| RwLock::new(HashMap::new()));

    /// Return a cached SurrealDB handle for the given workspace, opening a new
    /// connection only on the first call for each data_dir + branch combination.
    ///
    /// The database is stored at `{data_dir}/db/{branch}/` using the embedded
    /// SurrealKV engine.
    pub async fn connect_db(data_dir: &Path, branch: &str) -> Result<Db, EngramError> {
        let db_path = data_dir.join("db").join(branch);
        let cache_key = db_path.clone();

        // Fast path: existing connection
        {
            let cache = DB_CACHE.read().await;
            if let Some(db) = cache.get(&cache_key) {
                return Ok(db.clone());
            }
        }

        // Slow path: open, schema-bootstrap, then cache
        fs::create_dir_all(&db_path).map_err(|e| {
            EngramError::from(SystemError::DatabaseError {
                reason: format!("failed to create db directory: {e}"),
            })
        })?;

        let db = Surreal::new::<SurrealKv>(db_path)
            .await
            .map_err(map_db_err)?;

        db.use_ns("engram")
            .use_db(branch)
            .await
            .map_err(map_db_err)?;

        ensure_schema(&db).await?;

        let mut cache = DB_CACHE.write().await;
        cache.insert(cache_key, db.clone());

        Ok(db)
    }

    async fn ensure_schema(db: &Db) -> Result<(), EngramError> {
        db.query(super::schema::DEFINE_CODE_FILE)
            .await
            .map_err(map_db_err)?;
        db.query(super::schema::DEFINE_FUNCTION)
            .await
            .map_err(map_db_err)?;
        db.query(super::schema::DEFINE_CLASS)
            .await
            .map_err(map_db_err)?;
        db.query(super::schema::DEFINE_INTERFACE)
            .await
            .map_err(map_db_err)?;
        db.query(super::schema::DEFINE_CODE_EDGES)
            .await
            .map_err(map_db_err)?;
        db.query(super::schema::DEFINE_CONTENT_RECORD)
            .await
            .map_err(map_db_err)?;
        db.query(super::schema::DEFINE_COMMIT_NODE)
            .await
            .map_err(map_db_err)?;
        db.query(super::schema::DEFINE_FILE_HASH)
            .await
            .map_err(map_db_err)?;
        Ok(())
    }

    /// Map any error value into an [`EngramError`] database error.
    pub fn map_db_err<E: ToString>(err: E) -> EngramError {
        EngramError::from(SystemError::DatabaseError {
            reason: err.to_string(),
        })
    }
}

#[cfg(feature = "surreal-backend")]
pub use surreal_db::{Db, connect_db, map_db_err};

// ── CozoDB backend ──────────────────────────────────────────────────────────

/// CozoDB query stubs — Phase 1 compilation shim.
#[cfg(feature = "cozo-backend")]
#[path = "cozo_queries.rs"]
pub mod queries;

#[cfg(feature = "cozo-backend")]
mod cozo_db {
    use std::path::Path;

    use crate::errors::{EngramError, SystemError};

    /// Opaque CozoDB connection handle.
    ///
    /// Phase 2 will replace this with a real `cozo::DbInstance` wrapper
    /// backed by an SQLite store under `.engram/db/{branch}/`.
    #[derive(Clone)]
    pub struct CozoHandle;

    /// The active database handle type for the CozoDB backend.
    pub type Db = CozoHandle;

    /// Open (or return a cached) CozoDB handle for the given workspace.
    ///
    /// Not yet implemented — Phase 2 will wire up `cozo::DbInstance::new`.
    pub async fn connect_db(_data_dir: &Path, _branch: &str) -> Result<Db, EngramError> {
        Err(EngramError::from(SystemError::DatabaseError {
            reason: "CozoDB backend connection not yet implemented (Phase 2)".into(),
        }))
    }

    /// Map any error value into an [`EngramError`] database error.
    pub fn map_db_err<E: ToString>(err: E) -> EngramError {
        EngramError::from(SystemError::DatabaseError {
            reason: err.to_string(),
        })
    }
}

#[cfg(feature = "cozo-backend")]
pub use cozo_db::{Db, connect_db, map_db_err};
