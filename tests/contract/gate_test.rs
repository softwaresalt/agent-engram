//! Contract tests for sandboxed query sanitizer (gate.rs).
//!
//! Tests validate the sanitize_query function that enforces read-only
//! query access for the query_graph tool.

/// S003: SELECT query is allowed (not rejected by sanitizer).
#[test]
fn t020_select_query_allowed() {
    let result = engram::services::gate::sanitize_query("SELECT * FROM code_file");
    assert!(result.is_ok(), "SELECT must be allowed");
}

/// SELECT with WHERE and ORDER is allowed.
#[test]
fn t021_select_with_where_allowed() {
    let result = engram::services::gate::sanitize_query(
        "SELECT * FROM `function` WHERE file_path = '/src/main.rs' ORDER BY name",
    );
    assert!(result.is_ok(), "SELECT with WHERE must be allowed");
}

/// Unterminated string literal is rejected to prevent keyword bypass.
#[test]
fn t022_unterminated_string_rejected() {
    let result = engram::services::gate::sanitize_query("SELECT * FROM code_file WHERE name = 'test");
    assert!(result.is_err(), "unterminated string must be rejected");
    let response = result.unwrap_err().to_response();
    assert_eq!(response.error.code, 4012, "QUERY_INVALID must be 4012");
}