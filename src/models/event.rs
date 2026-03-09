//! Event entity and `EventKind` enum for the append-only state-change ledger.
//!
//! Every mutation to workspace entities (tasks, specs, dependencies, collections)
//! produces an [`Event`] record. Events are stored in SurrealDB only — they are
//! transient operational data and are not dehydrated to `.engram/` files.
//!
//! See `specs/005-lifecycle-observability/data-model.md` for the full schema.

use serde::{Deserialize, Serialize};

/// The kind of state change recorded in an [`Event`].
///
/// Variants correspond directly to the write operations available via MCP tools.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    /// A new task was created.
    TaskCreated,
    /// A task field was updated.
    TaskUpdated,
    /// A task was deleted.
    TaskDeleted,
    /// A dependency edge was added.
    EdgeAdded,
    /// A dependency edge was removed.
    EdgeRemoved,
    /// A collection was created.
    CollectionCreated,
    /// A collection was deleted.
    CollectionDeleted,
    /// A task was added to a collection.
    MemberAdded,
    /// A task was removed from a collection.
    MemberRemoved,
    /// A rollback was applied.
    RollbackApplied,
}

/// An immutable record of a state change within a workspace.
///
/// Events form an append-only ledger keyed by `id` and ordered by `created_at`.
/// The rolling retention policy prunes the oldest events once the ledger exceeds
/// [`Config::event_ledger_max`](crate::config::Config::event_ledger_max).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    /// Unique SurrealDB record ID (`event:<uuid>`).
    pub id: String,
    /// The kind of change this event records.
    pub kind: EventKind,
    /// The SurrealDB table of the affected entity (e.g. `"task"`, `"collection"`).
    pub entity_table: String,
    /// The SurrealDB record ID of the affected entity.
    pub entity_id: String,
    /// JSON snapshot of the entity state before the change, if applicable.
    pub previous_value: Option<serde_json::Value>,
    /// JSON snapshot of the entity state after the change.
    pub new_value: Option<serde_json::Value>,
    /// The MCP client identifier that triggered this change.
    pub source_client: Option<String>,
    /// ISO-8601 timestamp when the event was recorded.
    pub created_at: chrono::DateTime<chrono::Utc>,
}
