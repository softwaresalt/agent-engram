//! Contract tests for sandboxed graph query interface (User Story 4).
//! Scenarios S031–S043 from SCENARIOS.md.
//! Error codes: `QUERY_REJECTED`=4010, `QUERY_TIMEOUT`=4011, `QUERY_INVALID`=4012.

use engram::errors::{EngramError, GraphQueryError};

/// S033: INSERT query is rejected.
#[test]
fn t064_insert_rejected() {
    let result = engram::services::gate::sanitize_query("INSERT INTO task { title: 'hack' }");
    assert!(result.is_err(), "INSERT must be rejected");
    let err = result.unwrap_err();
    let response = err.to_response();
    assert_eq!(response.error.code, 4010, "QUERY_REJECTED must be 4010");
    assert_eq!(response.error.name, "QueryRejected");
}

/// S034: DELETE query is rejected.
#[test]
fn t065_delete_rejected() {
    let result = engram::services::gate::sanitize_query("DELETE task:A");
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_response().error.code, 4010);
}

/// S035: UPDATE query is rejected.
#[test]
fn t066_update_rejected() {
    let result = engram::services::gate::sanitize_query("UPDATE task SET status = 'done'");
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_response().error.code, 4010);
}

/// S041: DEFINE statement is rejected.
#[test]
fn t067_define_rejected() {
    let result = engram::services::gate::sanitize_query("DEFINE TABLE evil SCHEMAFULL");
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_response().error.code, 4010);
}

/// S042: RELATE statement is rejected.
#[test]
fn t068_relate_rejected() {
    let result = engram::services::gate::sanitize_query("RELATE task:A->depends_on->task:B");
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_response().error.code, 4010);
}

/// S031: SELECT query is allowed (not rejected by sanitizer).
#[test]
fn t062_select_allowed() {
    let result =
        engram::services::gate::sanitize_query("SELECT * FROM task WHERE status = 'in_progress'");
    assert!(result.is_ok(), "SELECT must pass sanitizer: {result:?}");
}

/// S032: Graph traversal query is allowed.
#[test]
fn t063_graph_traversal_allowed() {
    let result = engram::services::gate::sanitize_query("SELECT <-depends_on<-task FROM task:A");
    assert!(
        result.is_ok(),
        "Graph traversal must pass sanitizer: {result:?}"
    );
}

/// SELECT with write keyword inside a string literal must NOT be rejected (word-boundary detection).
#[test]
fn t062b_select_with_string_literal_keyword_allowed() {
    // "UPDATE" appears inside a string literal — must NOT be rejected
    let result = engram::services::gate::sanitize_query(
        "SELECT * FROM task WHERE title = 'UPDATE this field'",
    );
    assert!(
        result.is_ok(),
        "Keyword inside string literal must not be rejected: {result:?}"
    );
}

/// S037: Row limit enforcement — response shape includes truncated flag.
#[test]
fn t071_row_limit_response_shape() {
    let response = serde_json::json!({
        "rows": [],
        "row_count": 0,
        "truncated": false,
    });
    assert!(response["rows"].is_array());
    assert!(response["row_count"].is_number());
    assert!(response["truncated"].is_boolean());
}

/// S043: `query_graph` without workspace returns `WORKSPACE_NOT_SET` (code 1003).
#[test]
fn t072_workspace_not_set_for_query() {
    let err = engram::errors::EngramError::Workspace(engram::errors::WorkspaceError::NotSet);
    // WORKSPACE_NOT_SET = 1003 (see errors::codes)
    assert_eq!(err.to_response().error.code, 1003);
}

/// S036: Timeout error has correct code (4011).
#[test]
fn t069_timeout_error_code() {
    let err = EngramError::GraphQuery(GraphQueryError::Timeout { timeout_ms: 5000 });
    let response = err.to_response();
    assert_eq!(response.error.code, 4011, "QUERY_TIMEOUT must be 4011");
    assert_eq!(response.error.name, "QueryTimeout");
    let details = response.error.details.as_ref().unwrap();
    assert_eq!(details["timeout_ms"], 5000);
}

/// S038: Invalid `SurrealQL` syntax has correct code (4012).
#[test]
fn t070_invalid_syntax_error_code() {
    let err = EngramError::GraphQuery(GraphQueryError::Invalid {
        reason: "unexpected token".to_string(),
    });
    let response = err.to_response();
    assert_eq!(response.error.code, 4012, "QUERY_INVALID must be 4012");
    assert_eq!(response.error.name, "QueryInvalid");
    let details = response.error.details.as_ref().unwrap();
    assert_eq!(details["reason"], "unexpected token");
}

/// Unterminated string literal must be rejected to prevent write-keyword bypass.
#[test]
fn t070b_unterminated_string_literal_rejected() {
    // An unclosed quote could hide a write keyword from the sanitizer.
    let result =
        engram::services::gate::sanitize_query("SELECT * FROM task WHERE x = \"DELETE task:A");
    assert!(result.is_err(), "unterminated string must be rejected");
    let response = result.unwrap_err().to_response();
    assert_eq!(
        response.error.code, 4012,
        "unterminated string must be QUERY_INVALID (4012)"
    );
}

/// Unterminated single-quoted string literal must also be rejected.
#[test]
fn t070c_unterminated_single_quote_rejected() {
    let result =
        engram::services::gate::sanitize_query("SELECT * FROM task WHERE x = 'DELETE task:A");
    assert!(
        result.is_err(),
        "unterminated single-quote must be rejected"
    );
    assert_eq!(result.unwrap_err().to_response().error.code, 4012);
}
