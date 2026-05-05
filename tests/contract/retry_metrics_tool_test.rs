//! Contract tests for the `get_mutable_script_retry_metrics` MCP tool (040.002-T).
//!
//! Verifies:
//! - Tool is registered in the catalog with the correct name.
//! - Tool responds with the expected schema (`retry_count`, `last_retry_at`).
//! - Initial `retry_count` value is a non-negative integer (zero or greater).

use std::sync::Arc;

use engram::server::state::AppState;
use engram::shim::tools_catalog;
use engram::tools;

/// AC: `get_mutable_script_retry_metrics` is present in the tool catalog.
#[test]
fn contract_get_mutable_script_retry_metrics_in_catalog() {
    let tools = tools_catalog::all_tools();
    let names: std::collections::HashSet<&str> = tools.iter().map(|t| t.name.as_ref()).collect();
    assert!(
        names.contains("get_mutable_script_retry_metrics"),
        "tool 'get_mutable_script_retry_metrics' must appear in all_tools() catalog"
    );
}

/// AC: Tool response includes a `retry_count` field with a u64-compatible value.
#[tokio::test]
async fn contract_get_mutable_script_retry_metrics_has_retry_count() {
    let state = Arc::new(AppState::new(10));

    let result = tools::dispatch(state, "get_mutable_script_retry_metrics", None)
        .await
        .expect("get_mutable_script_retry_metrics must succeed");

    assert!(
        result.get("retry_count").is_some(),
        "response must include 'retry_count'; got: {result:?}"
    );
    assert!(
        result["retry_count"].as_u64().is_some(),
        "'retry_count' must be a valid u64 integer; got: {}",
        result["retry_count"]
    );
}

/// AC: Tool response `last_retry_at` field is null or a string (RFC-3339 timestamp).
#[tokio::test]
async fn contract_get_mutable_script_retry_metrics_last_retry_at_schema() {
    let state = Arc::new(AppState::new(10));

    let result = tools::dispatch(state, "get_mutable_script_retry_metrics", None)
        .await
        .expect("get_mutable_script_retry_metrics must succeed");

    assert!(
        result.get("last_retry_at").is_some(),
        "response must include 'last_retry_at'; got: {result:?}"
    );
    let last_retry_at = &result["last_retry_at"];
    assert!(
        last_retry_at.is_null() || last_retry_at.is_string(),
        "'last_retry_at' must be null or a string; got: {last_retry_at}"
    );
}
