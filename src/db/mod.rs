#![allow(dead_code)]

use std::path::PathBuf;

use dirs::data_dir;
use surrealdb::Surreal;
use surrealdb::engine::local::{Db as LocalDb, SurrealKv};

use crate::errors::{SystemError, TMemError};

pub mod queries;
pub mod schema;
pub mod workspace;

pub type Db = Surreal<LocalDb>;

/// Connect to SurrealDB embedded store scoped to the workspace hash and ensure schema.
pub async fn connect_db(workspace_hash: &str) -> Result<Db, TMemError> {
    let base = data_dir().unwrap_or_else(|| PathBuf::from("./"));
    let db_path = base.join("t-mem").join("db");

    let db = Surreal::new::<SurrealKv>(db_path)
        .await
        .map_err(map_db_err)?;

    db.use_ns("tmem")
        .use_db(workspace_hash)
        .await
        .map_err(map_db_err)?;

    ensure_schema(&db).await?;

    Ok(db)
}

async fn ensure_schema(db: &Db) -> Result<(), TMemError> {
    db.query(schema::DEFINE_SPEC).await.map_err(map_db_err)?;
    db.query(schema::DEFINE_TASK).await.map_err(map_db_err)?;
    db.query(schema::DEFINE_CONTEXT).await.map_err(map_db_err)?;
    db.query(schema::DEFINE_RELATIONSHIPS)
        .await
        .map_err(map_db_err)?;
    Ok(())
}

/// Map Surreal errors into TMemError
pub fn map_db_err<E: ToString>(err: E) -> TMemError {
    TMemError::from(SystemError::DatabaseError {
        reason: err.to_string(),
    })
}
