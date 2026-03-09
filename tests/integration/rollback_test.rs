//! Integration tests for event ledger rollback operations (User Story 3).
//!
//! Tests are written first (TDD) and will fail until Phase 6 implementation.
//! Scenarios: S024, S028 from SCENARIOS.md.

use chrono::Utc;
use engram::db::connect_db;
use engram::db::queries::Queries;
use engram::db::workspace::workspace_hash;
use engram::models::{Event, EventKind};
use engram::services::event_ledger;

/// T053: `insert_event` followed by `count_events` returns correct count,
/// and `delete_oldest_events` prunes down to the max.
#[tokio::test]
async fn t053_event_insert_count_and_prune() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ws_hash = workspace_hash(dir.path());
    let db = connect_db(&ws_hash).await.expect("connect_db");
    let queries = Queries::new(db);

    // Initially empty.
    let count = queries.count_events().await.expect("count_events");
    assert_eq!(count, 0, "ledger should start empty");

    // Insert 3 events.
    for i in 0u32..3 {
        let event = Event {
            id: format!("event:test{i:03}"),
            kind: EventKind::TaskCreated,
            entity_table: "task".to_string(),
            entity_id: format!("task:test{i:03}"),
            previous_value: None,
            new_value: Some(serde_json::json!({ "title": format!("Task {i}") })),
            source_client: "test".to_string(),
            created_at: Utc::now(),
        };
        queries.insert_event(&event).await.expect("insert_event");
    }

    let count = queries
        .count_events()
        .await
        .expect("count_events after inserts");
    assert_eq!(count, 3, "should have 3 events after 3 inserts");

    // Prune 2 oldest.
    queries
        .delete_oldest_events(2)
        .await
        .expect("delete_oldest_events");

    let count = queries
        .count_events()
        .await
        .expect("count_events after prune");
    assert_eq!(count, 1, "should have 1 event after pruning 2");
}

/// T054 (integration): `prepare_rollback` returns `RollbackDenied` when
/// `allow_agent_rollback` is `false`.
#[tokio::test]
async fn t054_rollback_denied_when_disabled() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ws_hash = workspace_hash(dir.path());
    let db = connect_db(&ws_hash).await.expect("connect_db");
    let queries = Queries::new(db);

    let result = event_ledger::prepare_rollback(&queries, "event:any", false).await;
    match result {
        Err(engram::errors::EngramError::Event(engram::errors::EventError::RollbackDenied)) => { /* expected */
        }
        other => panic!("expected RollbackDenied, got {other:?}"),
    }
}
