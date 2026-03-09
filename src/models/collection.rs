//! Collection entity for hierarchical workflow groupings.
//!
//! A [`Collection`] is a named group of tasks and nested collections that
//! provides hierarchical organisation. Collections dehydrate to
//! `.engram/collections.md` and are re-hydrated on workspace bind.
//!
//! See `specs/005-lifecycle-observability/data-model.md` for the full schema.

use serde::{Deserialize, Serialize};

/// A named grouping of tasks and nested collections.
///
/// Members are linked via `contains` relation edges in SurrealDB, which
/// allows recursive context retrieval with `get_collection_context`.
/// Collection names must be unique within a workspace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Collection {
    /// Unique SurrealDB record ID (e.g. `collection:abc123ulid`).
    pub id: String,
    /// Human-readable name, unique within the workspace.
    pub name: String,
    /// Optional description of the collection`s purpose or scope.
    pub description: Option<String>,
    /// ISO-8601 timestamp when the collection was created (immutable).
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// ISO-8601 timestamp of the last metadata update.
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
