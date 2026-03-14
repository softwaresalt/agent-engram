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
/// Serialized as snake_case strings to match the SurrealDB schema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    /// A new task was created.
    TaskCreated,
    /// A task field was updated (status, title, description, etc.).
    TaskUpdated,
    /// A task was removed.
    TaskDeleted,
    /// A dependency, implements, or relates_to edge was created.
    EdgeCreated,
    /// A relation edge was removed.
    EdgeDeleted,
    /// A context entry was added.
    ContextCreated,
    /// A new collection was created.
    CollectionCreated,
    /// A collection was modified (name or description changed).
    CollectionUpdated,
    /// A task or sub-collection was added to or removed from a collection.
    CollectionMembershipChanged,
    /// A rollback operation was applied (internal use by the event ledger).
    RollbackApplied,
}

/// An immutable record of a state change within a workspace.
///
/// Events form an append-only ledger keyed by `id` and ordered by `created_at`.
/// The rolling retention policy prunes the oldest events once the ledger exceeds
/// [`Config::event_ledger_max`](crate::config::Config::event_ledger_max).
///
/// Events are stored in SurrealDB only and are NOT dehydrated to `.engram/` files
/// because they are transient operational data, not user-editable workspace state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    /// Unique SurrealDB record ID (e.g. `event:abc123ulid`).
    pub id: String,
    /// The kind of change this event records.
    pub kind: EventKind,
    /// The SurrealDB table of the affected entity (e.g. `"task"`, `"collection"`).
    pub entity_table: String,
    /// The SurrealDB record ID of the affected entity (e.g. `"task:abc123"`).
    pub entity_id: String,
    /// JSON snapshot of the entity state before the change.
    ///
    /// `None` for creation events (entity did not exist before).
    pub previous_value: Option<serde_json::Value>,
    /// JSON snapshot of the entity state after the change.
    ///
    /// `None` for deletion events (entity no longer exists).
    pub new_value: Option<serde_json::Value>,
    /// The MCP client identifier that triggered this change (required).
    pub source_client: String,
    /// ISO-8601 timestamp when the event was recorded (immutable after creation).
    pub created_at: chrono::DateTime<chrono::Utc>,
}
