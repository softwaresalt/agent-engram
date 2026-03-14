//! Contract tests for dependency gate enforcement (User Story 1).
//!
//! Tests validate the error response shape and warning shape for gate enforcement
//! without requiring a live database — these are schema/format contract tests.
//!
//! Scenarios: S001–S012 from SCENARIOS.md.
//! Error codes: `TASK_BLOCKED`=3015, `CYCLIC_DEPENDENCY`=3003.

use engram::errors::{EngramError, TaskError};
use serde_json::json;

/// Constructs a blocked `TaskError` with structured blocker data.
fn make_blocked_error(task_id: &str, blockers: Vec<serde_json::Value>) -> EngramError {
    EngramError::Task(TaskError::Blocked {
        id: task_id.to_string(),
        blocker_count: blockers.len(),
        blockers,
    })
}

/// S001: Blocked error response has required fields and the correct code (3015).
#[test]
fn t014_blocked_error_response_shape() {
    let blocker = json!({
        "id": "task:blocker-a",
        "status": "todo",
        "dependency_type": "hard_blocker",
        "transitively_blocks": false,
    });
    let err = make_blocked_error("target-b", vec![blocker]);
    let response = err.to_response();

    assert_eq!(response.error.code, 3015, "TASK_BLOCKED must be 3015");
    assert_eq!(response.error.name, "TaskBlocked");
    assert!(
        response.error.message.contains("target-b"),
        "message must mention the blocked task id"
    );

    let details = response
        .error
        .details
        .as_ref()
        .expect("details must be present");
    let blockers = details["blockers"]
        .as_array()
        .expect("blockers must be an array");
    assert_eq!(blockers.len(), 1, "one blocker expected");
    assert_eq!(blockers[0]["id"], "task:blocker-a");
    assert_eq!(blockers[0]["status"], "todo");
    assert_eq!(blockers[0]["dependency_type"], "hard_blocker");
}

/// S002: Success response shape includes required fields and no error key.
#[test]
fn t015_success_response_shape() {
    let success = json!({
        "task_id": "task:B",
        "previous_status": "todo",
        "new_status": "in_progress",
        "context_id": "context:abc",
        "updated_at": "2026-03-10T15:30:00Z",
    });

    assert_eq!(success["task_id"], "task:B");
    assert_eq!(success["new_status"], "in_progress");
    assert!(success["context_id"].is_string());
    assert!(success["updated_at"].is_string());
    // No "error" key on success
    assert!(success.get("error").is_none());
}

/// S003: Transitive blockers are all included in the response and marked accordingly.
#[test]
fn t016_transitive_blockers_all_listed() {
    let direct = json!({
        "id": "task:B",
        "status": "todo",
        "dependency_type": "hard_blocker",
        "transitively_blocks": false,
    });
    let transitive = json!({
        "id": "task:A",
        "status": "todo",
        "dependency_type": "hard_blocker",
        "transitively_blocks": true,
    });
    let err = make_blocked_error("task-C", vec![direct, transitive]);
    let response = err.to_response();

    let details = response.error.details.as_ref().expect("details present");
    let blockers = details["blockers"]
        .as_array()
        .expect("blockers must be array");
    assert_eq!(
        blockers.len(),
        2,
        "both blockers must be listed for 3-task chain"
    );

    let has_transitive = blockers
        .iter()
        .any(|b| b["transitively_blocks"] == json!(true));
    assert!(
        has_transitive,
        "at least one blocker must be marked transitively_blocks=true"
    );
}

/// S004: Soft dependency warning object has the required shape fields.
#[test]
fn t017_soft_dep_warning_shape() {
    let warning = json!({
        "type": "soft_dependency_incomplete",
        "id": "task:soft-dep",
        "status": "todo",
    });

    assert_eq!(warning["type"], "soft_dependency_incomplete");
    assert!(
        warning["id"]
            .as_str()
            .expect("id must be string")
            .starts_with("task:"),
        "warning id must have task: prefix"
    );
    assert_eq!(warning["status"], "todo");
}

/// S006/S007/S008: Cyclic dependency errors use `CYCLIC_DEPENDENCY` (3003).
#[test]
fn t018_cyclic_dependency_error_code() {
    let err = EngramError::Task(TaskError::CyclicDependency);
    let response = err.to_response();

    assert_eq!(response.error.code, 3003, "CYCLIC_DEPENDENCY must be 3003");
    assert_eq!(response.error.name, "CyclicDependency");
}

/// S010: Multiple direct blockers all appear in the error response.
#[test]
fn t019_multiple_blockers_in_error() {
    let b1 = json!({ "id": "task:A1", "status": "todo", "dependency_type": "hard_blocker", "transitively_blocks": false });
    let b2 = json!({ "id": "task:A2", "status": "todo", "dependency_type": "hard_blocker", "transitively_blocks": false });
    let b3 = json!({ "id": "task:A3", "status": "todo", "dependency_type": "hard_blocker", "transitively_blocks": false });
    let err = make_blocked_error("task-B", vec![b1, b2, b3]);
    let response = err.to_response();

    let details = response.error.details.as_ref().expect("details present");
    let blockers = details["blockers"]
        .as_array()
        .expect("blockers must be array");
    assert_eq!(blockers.len(), 3, "all 3 blockers must appear in response");
}
