#![allow(dead_code)]

use crate::errors::{SystemError, TMemError};

pub mod workspace;

/// Placeholder database wrapper. This will be wired to SurrealDB in subsequent phases.
pub struct Database {
    workspace_hash: String,
}

impl Database {
    pub async fn connect(workspace_hash: impl Into<String>) -> Result<Self, TMemError> {
        // TODO: integrate SurrealDB embedded backend (kv-surrealkv) with namespaces per workspace
        Ok(Self {
            workspace_hash: workspace_hash.into(),
        })
    }

    pub fn namespace(&self) -> &str {
        &self.workspace_hash
    }
}

/// Simple guard to signal database errors
pub fn map_db_err(reason: impl Into<String>) -> TMemError {
    TMemError::from(SystemError::DatabaseError {
        reason: reason.into(),
    })
}
