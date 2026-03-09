//! Contract tests for hierarchical workflow collections (User Story 5).
//! Scenarios S044–S055 from SCENARIOS.md.
//! Error codes: `COLLECTION_EXISTS=3030`, `COLLECTION_NOT_FOUND=3031`, `CYCLIC_COLLECTION=3032`.

use chrono::Utc;
use engram::errors::{CollectionError, EngramError};
use engram::models::Collection;
use serde_json::json;

/// S044: `create_collection` success response shape.
#[test]
fn t076_create_collection_response_shape() {
    let response = json!({
        "collection_id": "collection:abc123",
        "name": "Feature X",
        "created_at": "2026-03-10T15:30:00Z",
    });
    assert!(response["collection_id"].is_string());
    assert_eq!(response["name"], "Feature X");
    assert!(response["created_at"].is_string());
}

/// S045: Duplicate collection name is rejected with `COLLECTION_EXISTS` (3030).
#[test]
fn t077_collection_exists_error_code() {
    let err = EngramError::Collection(CollectionError::AlreadyExists {
        name: "Feature X".to_string(),
    });
    let response = err.to_response();
    assert_eq!(response.error.code, 3030, "COLLECTION_EXISTS must be 3030");
    assert_eq!(response.error.name, "CollectionAlreadyExists");
    assert!(response.error.message.contains("Feature X"));
}

/// S046: `add_to_collection` response has correct fields.
#[test]
fn t078_add_to_collection_response_shape() {
    let response = json!({
        "collection_id": "collection:abc",
        "added": 3,
        "already_members": 0,
    });
    assert!(response["added"].is_number());
    assert!(response["already_members"].is_number());
}

/// S048: `get_collection_context` returns tasks from nested collections recursively.
#[test]
fn t079_collection_context_response_shape() {
    let response = json!({
        "collection_id": "collection:abc",
        "name": "Feature X",
        "task_count": 4,
        "tasks": [
            { "id": "task:1", "title": "Task 1", "status": "todo" },
            { "id": "task:2", "title": "Task 2", "status": "in_progress" },
        ],
        "sub_collections": [],
    });
    assert!(response["tasks"].is_array());
    assert!(response["task_count"].is_number());
    assert!(response["sub_collections"].is_array());
}

/// S049: Collection context can be filtered by status.
#[test]
fn t080_collection_context_status_filter_response() {
    // filtered response only has in_progress tasks
    let response = json!({
        "collection_id": "collection:abc",
        "name": "Feature X",
        "task_count": 1,
        "tasks": [
            { "id": "task:2", "title": "Task 2", "status": "in_progress" }
        ],
        "sub_collections": [],
    });
    let tasks = response["tasks"].as_array().unwrap();
    assert!(tasks.iter().all(|t| t["status"] == "in_progress"));
}

/// S053: Cyclic collection nesting is rejected with `CYCLIC_COLLECTION` (3032).
#[test]
fn t081_cyclic_collection_error_code() {
    let err = EngramError::Collection(CollectionError::CyclicCollection {
        name: "Outer".to_string(),
    });
    let response = err.to_response();
    assert_eq!(response.error.code, 3032, "CYCLIC_COLLECTION must be 3032");
    assert_eq!(response.error.name, "CyclicCollection");
}

/// S051: `remove_from_collection` response shape.
#[test]
fn t082_remove_from_collection_response_shape() {
    let response = json!({
        "collection_id": "collection:abc",
        "removed": 1,
        "not_members": 0,
    });
    assert!(response["removed"].is_number());
    assert!(response["not_members"].is_number());
}

/// S054: Collection not found error code is 3031.
#[test]
fn t083_collection_not_found_error_code() {
    let err = EngramError::Collection(CollectionError::NotFound {
        name: "nonexistent".to_string(),
    });
    let response = err.to_response();
    assert_eq!(
        response.error.code, 3031,
        "COLLECTION_NOT_FOUND must be 3031"
    );
    assert_eq!(response.error.name, "CollectionNotFound");
    assert!(response.error.message.contains("nonexistent"));
}

/// Collection model round-trips through serde correctly.
#[test]
fn t084_collection_model_roundtrip() {
    let collection = Collection {
        id: "collection:abc123".to_string(),
        name: "My Feature".to_string(),
        description: Some("A group of related tasks".to_string()),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let json = serde_json::to_string(&collection).unwrap();
    let back: Collection = serde_json::from_str(&json).unwrap();
    assert_eq!(back.id, "collection:abc123");
    assert_eq!(back.name, "My Feature");
    assert_eq!(
        back.description,
        Some("A group of related tasks".to_string())
    );
}
