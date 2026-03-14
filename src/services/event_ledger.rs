//! Append-only event ledger with rolling retention and rollback support.
//!
//! Records all state changes as immutable [`Event`] entries in SurrealDB.
//! Enforces a configurable maximum ledger size (`event_ledger_max`) by
//! pruning the oldest events when the limit is exceeded.
//!
//! See `specs/005-lifecycle-observability/spec.md` User Story 3 for requirements.

use chrono::Utc;
use serde_json::Value;
use uuid::Uuid;

use crate::db::queries::Queries;
use crate::errors::{EngramError, EventError};
use crate::models::event::{Event, EventKind};

/// Records a new event in the event ledger, then prunes old events if necessary.
///
/// Called by every write tool after a successful mutation. The `source_client`
/// string identifies the MCP client that triggered the change.
///
/// Returns the generated `event:<uuid>` ID string on success.
#[allow(clippy::too_many_arguments)]
pub async fn record_event(
    queries: &Queries,
    kind: EventKind,
    entity_table: &str,
    entity_id: &str,
    previous_value: Option<Value>,
    new_value: Option<Value>,
    source_client: &str,
    max_events: usize,
) -> Result<String, EngramError> {
    let raw_id = Uuid::new_v4().to_string().replace('-', "");
    let event_id = format!("event:{raw_id}");
    let event = Event {
        id: event_id.clone(),
        kind,
        entity_table: entity_table.to_string(),
        entity_id: entity_id.to_string(),
        previous_value,
        new_value,
        source_client: source_client.to_string(),
        created_at: Utc::now(),
    };
    queries.insert_event(&event).await?;

    // Rolling retention: prune oldest when ledger exceeds the cap.
    let count = queries.count_events().await?;
    if count > max_events as u64 {
        let excess = count - max_events as u64;
        queries.delete_oldest_events(excess).await?;
    }

    Ok(event_id)
}

/// Validates that rollback is permitted and that the target event exists.
///
/// Returns the events that occurred **after** `event_id` (in chronological
/// order). These events will be reversed by [`apply_rollback`] to restore
/// the workspace to the state it held immediately after `event_id`.
///
/// # Errors
///
/// Returns [`EventError::RollbackDenied`] when `allow_agent_rollback` is
/// `false`, or [`EventError::EventNotFound`] when the target event is absent.
pub async fn prepare_rollback(
    queries: &Queries,
    event_id: &str,
    allow_agent_rollback: bool,
) -> Result<Vec<Event>, EngramError> {
    if !allow_agent_rollback {
        return Err(EngramError::Event(EventError::RollbackDenied));
    }
    let events = queries.get_events_after(event_id).await?;
    Ok(events)
}

/// Applies a rollback by reversing each event in reverse-chronological order.
///
/// Iterates `events_to_reverse` from newest to oldest, undoing each mutation.
/// After reversal, a `RollbackApplied` event is recorded in the ledger.
///
/// Returns `(reversed_count, rollback_event_id)` on success.
pub async fn apply_rollback(
    queries: &Queries,
    events_to_reverse: Vec<Event>,
    target_event_id: &str,
    source_client: &str,
    max_events: usize,
) -> Result<(usize, String), EngramError> {
    let count = events_to_reverse.len();

    // Reverse chronological order — undo newest change first.
    for event in events_to_reverse.into_iter().rev() {
        match event.kind {
            EventKind::TaskCreated => {
                // Undo creation by deleting the task.
                let task_id = event
                    .entity_id
                    .strip_prefix("task:")
                    .unwrap_or(&event.entity_id)
                    .to_string();
                queries.delete_task(&task_id).await?;
            }
            EventKind::TaskUpdated | EventKind::TaskDeleted => {
                // Restore previous state.
                if let Some(prev) = event.previous_value {
                    queries
                        .restore_task_snapshot(&event.entity_id, &prev)
                        .await?;
                } else {
                    return Err(EngramError::Event(EventError::RollbackConflict {
                        entity_id: event.entity_id.clone(),
                        event_id: event.id.clone(),
                    }));
                }
            }
            EventKind::EdgeCreated => {
                // Undo edge creation by deleting the relation.
                queries.delete_relation_by_id(&event.entity_id).await?;
            }
            EventKind::EdgeDeleted => {
                // Restore edge from the previous-value snapshot.
                if let Some(prev) = event.previous_value {
                    queries
                        .restore_relation_snapshot(&event.entity_id, &prev)
                        .await?;
                } else {
                    return Err(EngramError::Event(EventError::RollbackConflict {
                        entity_id: event.entity_id.clone(),
                        event_id: event.id.clone(),
                    }));
                }
            }
            EventKind::ContextCreated => {
                queries.delete_context_by_id(&event.entity_id).await?;
            }
            EventKind::CollectionCreated => {
                // Undo creation by deleting the collection (previous_value is
                // None for creation events, so restore_snapshot would no-op).
                queries.delete_collection_by_id(&event.entity_id).await?;
            }
            EventKind::CollectionUpdated | EventKind::CollectionMembershipChanged => {
                if let Some(prev) = event.previous_value {
                    queries
                        .restore_collection_snapshot(&event.entity_id, &prev)
                        .await?;
                } else {
                    return Err(EngramError::Event(EventError::RollbackConflict {
                        entity_id: event.entity_id.clone(),
                        event_id: event.id.clone(),
                    }));
                }
            }
            EventKind::RollbackApplied => {
                // Never reverse a rollback event — skip silently.
            }
        }
    }

    // Record the rollback itself in the ledger.
    let rollback_id = record_event(
        queries,
        EventKind::RollbackApplied,
        "event",
        target_event_id,
        None,
        Some(serde_json::json!({ "reversed_count": count })),
        source_client,
        max_events,
    )
    .await?;

    Ok((count, rollback_id))
}
