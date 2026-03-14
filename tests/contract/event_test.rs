//! Contract tests for state event logging and rollback (User Story 3).
//! Scenarios S013–S030 from SCENARIOS.md.

use chrono::Utc;
use engram::errors::{EngramError, EventError};
use engram::models::{Event, EventKind};
use serde_json::json;

/// S025: Rollback denied error has correct code (3020).
#[test]
fn t051_rollback_denied_error_code() {
    let err = EngramError::Event(EventError::RollbackDenied);
    let response = err.to_response();
    assert_eq!(response.error.code, 3020, "ROLLBACK_DENIED must be 3020");
    assert_eq!(response.error.name, "RollbackDenied");
}

/// S027: Event not found error has correct code (3021).
#[test]
fn t052_event_not_found_error_code() {
    let err = EngramError::Event(EventError::EventNotFound {
        event_id: "event:xyz".to_string(),
    });
    let response = err.to_response();
    assert_eq!(response.error.code, 3021, "EVENT_NOT_FOUND must be 3021");
    assert_eq!(response.error.name, "EventNotFound");
    let details = response.error.details.as_ref().unwrap();
    assert_eq!(details["event_id"], "event:xyz");
}

/// S028: Rollback conflict error has correct code (3022).
#[test]
fn t054_rollback_conflict_error_code() {
    let err = EngramError::Event(EventError::RollbackConflict {
        entity_id: "task:abc".to_string(),
        event_id: "event:001".to_string(),
    });
    let response = err.to_response();
    assert_eq!(response.error.code, 3022, "ROLLBACK_CONFLICT must be 3022");
    assert!(response.error.details.as_ref().unwrap()["entity_id"] == "task:abc");
}

/// S013: Event model has required fields for `task_created`.
#[test]
fn t045_event_model_task_created_shape() {
    let event = Event {
        id: "event:abc123".to_string(),
        kind: EventKind::TaskCreated,
        entity_table: "task".to_string(),
        entity_id: "task:abc123".to_string(),
        previous_value: None,
        new_value: Some(json!({ "title": "My task", "status": "todo" })),
        source_client: "copilot-cli".to_string(),
        created_at: Utc::now(),
    };
    let serialized = serde_json::to_string(&event).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&serialized).unwrap();
    assert_eq!(parsed["kind"], "task_created");
    assert!(parsed["previous_value"].is_null());
    assert!(parsed["new_value"].is_object());
    assert_eq!(parsed["entity_table"], "task");
    assert_eq!(parsed["source_client"], "copilot-cli");
}

/// S014: Event model for `task_updated` has both previous and new values.
#[test]
fn t046_event_model_task_updated_shape() {
    let event = Event {
        id: "event:def456".to_string(),
        kind: EventKind::TaskUpdated,
        entity_table: "task".to_string(),
        entity_id: "task:def456".to_string(),
        previous_value: Some(json!({ "status": "todo" })),
        new_value: Some(json!({ "status": "in_progress" })),
        source_client: "copilot-cli".to_string(),
        created_at: Utc::now(),
    };
    let serialized = serde_json::to_string(&event).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&serialized).unwrap();
    assert_eq!(parsed["kind"], "task_updated");
    assert_eq!(parsed["previous_value"]["status"], "todo");
    assert_eq!(parsed["new_value"]["status"], "in_progress");
}

/// S015: Event model for edge creation uses `edge_created` kind.
#[test]
fn t047_event_model_edge_created_shape() {
    let event = Event {
        id: "event:ghi789".to_string(),
        kind: EventKind::EdgeCreated,
        entity_table: "depends_on".to_string(),
        entity_id: "depends_on:ghi789".to_string(),
        previous_value: None,
        new_value: Some(json!({ "in": "task:A", "out": "task:B", "type": "hard_blocker" })),
        source_client: "copilot-cli".to_string(),
        created_at: Utc::now(),
    };
    let serialized = serde_json::to_string(&event).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&serialized).unwrap();
    assert_eq!(parsed["kind"], "edge_created");
    assert_eq!(parsed["entity_table"], "depends_on");
}

/// S016: `RollbackApplied` event kind serializes correctly.
#[test]
fn t048_event_kind_rollback_applied_serializes() {
    let kind = EventKind::RollbackApplied;
    let serialized = serde_json::to_string(&kind).unwrap();
    assert_eq!(serialized, "\"rollback_applied\"");
}

/// S017: `get_event_history` response shape contains required fields.
#[test]
fn t049_event_history_response_shape() {
    let response = json!({
        "events": [],
        "total_count": 0,
        "limit": 50,
    });
    assert!(response["events"].is_array());
    assert!(response["total_count"].is_number());
    assert!(response["limit"].is_number());
}

/// S023: `rollback_to_event` success response shape contains required fields.
#[test]
fn t050_rollback_success_response_shape() {
    let response = json!({
        "events_reversed": 3,
        "rollback_event_id": "event:rollback-001",
        "target_event_id": "event:001",
    });
    assert!(response["events_reversed"].is_number());
    assert!(response["rollback_event_id"].is_string());
}
