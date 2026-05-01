//! Database layer: CozoDB connection management and query dispatch.

/// Workspace hash utilities.
pub mod workspace;

/// CozoDB query helpers.
#[path = "cozo_queries.rs"]
pub mod queries;

/// CozoDB backend — connection and handle management.
pub mod cozo_backend;

pub use cozo_backend::{Db, connect_db, map_db_err};
