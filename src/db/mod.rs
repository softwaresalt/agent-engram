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
    use std::sync::Arc;
    use std::sync::LazyLock;
    use std::time::Duration;

    use surrealdb::Surreal;
    use surrealdb::engine::local::{Db as LocalDb, SurrealKv};
    use tokio::sync::{Mutex, RwLock};

    use crate::errors::{EngramError, SystemError};

    /// The active database handle type for the SurrealDB backend.
    pub type Db = Surreal<LocalDb>;

    /// Per-workspace connection cache.  Keyed by the resolved database path,
    /// each entry holds a cloneable `Surreal<LocalDb>` handle.
    static DB_CACHE: LazyLock<RwLock<HashMap<PathBuf, Db>>> =
        LazyLock::new(|| RwLock::new(HashMap::new()));

    /// Per-path open locks.  Serialises concurrent `connect_db` callers for
    /// the same database path so that crash-recovery (wipe + sleep + retry)
    /// is performed by exactly one task while others wait for the cache.
    static OPEN_LOCKS: LazyLock<Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));

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

        // Acquire the per-path open lock so that concurrent callers for the
        // same workspace are serialised.  Once the first caller populates the
        // cache, the others return the cached handle without repeating the open.
        let path_lock = {
            let mut locks = OPEN_LOCKS.lock().await;
            Arc::clone(
                locks
                    .entry(cache_key.clone())
                    .or_insert_with(|| Arc::new(Mutex::new(()))),
            )
        };
        let _guard = path_lock.lock().await;

        // Re-check the cache after acquiring the per-path lock.
        {
            let cache = DB_CACHE.read().await;
            if let Some(db) = cache.get(&cache_key) {
                return Ok(db.clone());
            }
        }

        // Slow path: open with crash-recovery.
        // SurrealKV may open a corrupt database without error (WAL replay is
        // deferred to the first data-read transaction).  `try_open_and_bootstrap`
        // therefore runs a verification read after schema bootstrap so that any
        // crash-induced WAL corruption is detected here, before the handle is
        // cached and handed to callers.
        let db = match try_open_and_bootstrap(&db_path, branch).await {
            Ok(db) => db,
            Err(open_err) => {
                let err_str = open_err.to_string();
                let is_corruption = err_str.contains("revision")
                    || err_str.contains("end of file")
                    || err_str.contains("fill whole buffer");
                if is_corruption {
                    tracing::warn!(
                        error = %open_err,
                        db_path = %db_path.display(),
                        "DB files corrupted after crash; wiping and reinitializing"
                    );
                    if db_path.exists() {
                        let _ = fs::remove_dir_all(&db_path);
                    }
                    // Give SurrealKV background threads from the failed open
                    // time to exit before we open the same path again.
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    try_open_and_bootstrap(&db_path, branch).await?
                } else {
                    return Err(open_err);
                }
            }
        };

        let mut cache = DB_CACHE.write().await;
        cache.insert(cache_key, db.clone());
        Ok(db)
    }

    /// Create the DB directory, open the `SurrealKV` connection, select the
    /// namespace/database, apply the schema, and verify that data reads succeed.
    ///
    /// The verification read forces SurrealKV to replay its WAL so that any
    /// crash-induced corruption is detected here rather than on the first user
    /// query after the handle is cached.
    async fn try_open_and_bootstrap(db_path: &PathBuf, branch: &str) -> Result<Db, EngramError> {
        fs::create_dir_all(db_path).map_err(|e| {
            EngramError::from(SystemError::DatabaseError {
                reason: format!("failed to create db directory: {e}"),
            })
        })?;

        let db = Surreal::new::<SurrealKv>(db_path.clone())
            .await
            .map_err(map_db_err)?;

        db.use_ns("engram")
            .use_db(branch)
            .await
            .map_err(map_db_err)?;

        ensure_schema(&db).await?;

        // Verification read: scan one record from each primary table to force
        // WAL replay.  An IO error here means the database is corrupt; the
        // caller's recovery path will wipe and retry.
        db.query("SELECT * FROM code_file LIMIT 1")
            .await
            .map_err(map_db_err)?;

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

/// CozoDB backend — Phase 2 connection and handle management.
#[cfg(feature = "cozo-backend")]
pub mod cozo_backend;

#[cfg(feature = "cozo-backend")]
pub use cozo_backend::{Db, connect_db, map_db_err};
