//! Database layer: embedded SurrealDB connection and schema management.
//!
//! Each workspace gets an isolated SurrealDB database identified by the
//! current git branch name under the workspace data directory.
//! Schema is bootstrapped on the first connection; subsequent calls
//! return the cached handle.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use surrealdb::Surreal;
use surrealdb::engine::local::{Db as LocalDb, SurrealKv};
use tokio::sync::RwLock;

use crate::errors::{EngramError, SystemError};

pub mod queries;
pub mod schema;
pub mod workspace;

pub type Db = Surreal<LocalDb>;

/// Per-workspace connection cache.  Keyed by the resolved database path,
/// each entry holds a cloneable `Surreal<LocalDb>` handle.  `LazyLock`
/// avoids polluting `AppState` and the `static` is safe because `Db` is
/// `Send + Sync`.
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
    db.query(schema::DEFINE_CODE_FILE)
        .await
        .map_err(map_db_err)?;
    db.query(schema::DEFINE_FUNCTION)
        .await
        .map_err(map_db_err)?;
    db.query(schema::DEFINE_CLASS).await.map_err(map_db_err)?;
    db.query(schema::DEFINE_INTERFACE)
        .await
        .map_err(map_db_err)?;
    db.query(schema::DEFINE_CODE_EDGES)
        .await
        .map_err(map_db_err)?;
    db.query(schema::DEFINE_CONTENT_RECORD)
        .await
        .map_err(map_db_err)?;
    db.query(schema::DEFINE_COMMIT_NODE)
        .await
        .map_err(map_db_err)?;
    db.query(schema::DEFINE_FILE_HASH)
        .await
        .map_err(map_db_err)?;
    Ok(())
}

/// Map Surreal errors into EngramError
pub fn map_db_err<E: ToString>(err: E) -> EngramError {
    EngramError::from(SystemError::DatabaseError {
        reason: err.to_string(),
    })
}
