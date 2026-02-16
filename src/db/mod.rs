//! Database layer: embedded SurrealDB connection and schema management.
//!
//! Each workspace gets an isolated SurrealDB database identified by the
//! SHA-256 hash of its canonicalized path. Schema is bootstrapped on the
//! first connection; subsequent calls return the cached handle.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::LazyLock;

use dirs::data_dir;
use surrealdb::Surreal;
use surrealdb::engine::local::{Db as LocalDb, SurrealKv};
use tokio::sync::RwLock;

use crate::errors::{EngramError, SystemError};

pub mod queries;
pub mod schema;
pub mod workspace;

pub type Db = Surreal<LocalDb>;

/// Per-workspace connection cache.  Keyed by workspace hash, each entry
/// holds a cloneable `Surreal<LocalDb>` handle.  `LazyLock` avoids polluting
/// `AppState` and the `static` is safe because `Db` is `Send + Sync`.
static DB_CACHE: LazyLock<RwLock<HashMap<String, Db>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// Return a cached SurrealDB handle for the given workspace, opening a new
/// connection only on the first call for each hash.
pub async fn connect_db(workspace_hash: &str) -> Result<Db, EngramError> {
    // Fast path: existing connection
    {
        let cache = DB_CACHE.read().await;
        if let Some(db) = cache.get(workspace_hash) {
            return Ok(db.clone());
        }
    }

    // Slow path: open, schema-bootstrap, then cache
    let base = data_dir().unwrap_or_else(|| PathBuf::from("./"));
    let db_path = base.join("engram").join("db").join(workspace_hash);

    fs::create_dir_all(&db_path).map_err(|e| {
        EngramError::from(SystemError::DatabaseError {
            reason: format!("failed to create db directory: {e}"),
        })
    })?;

    let db = Surreal::new::<SurrealKv>(db_path)
        .await
        .map_err(map_db_err)?;

    db.use_ns("engram")
        .use_db(workspace_hash)
        .await
        .map_err(map_db_err)?;

    ensure_schema(&db).await?;

    let mut cache = DB_CACHE.write().await;
    cache.insert(workspace_hash.to_string(), db.clone());

    Ok(db)
}

async fn ensure_schema(db: &Db) -> Result<(), EngramError> {
    db.query(schema::DEFINE_SPEC).await.map_err(map_db_err)?;
    db.query(schema::DEFINE_TASK).await.map_err(map_db_err)?;
    db.query(schema::DEFINE_CONTEXT).await.map_err(map_db_err)?;
    db.query(schema::DEFINE_LABEL).await.map_err(map_db_err)?;
    db.query(schema::DEFINE_COMMENT).await.map_err(map_db_err)?;
    db.query(schema::DEFINE_RELATIONSHIPS)
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
