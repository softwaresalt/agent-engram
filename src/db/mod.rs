//! Database layer: `CozoDB` connection management and query dispatch.

#[cfg(not(feature = "cozo-backend"))]
compile_error!(
    "The `cozo-backend` feature is required; it is the only supported backend. \
     Build with: cargo build --features cozo-backend"
);

/// Workspace hash utilities.
pub mod workspace;

/// CozoDB query helpers.
#[path = "cozo_queries.rs"]
pub mod queries;

/// CozoDB backend — connection and handle management.
pub mod cozo_backend;

pub use cozo_backend::{Db, connect_db, map_db_err};
pub use queries::{RetryMetrics, mutable_script_retry_metrics};
