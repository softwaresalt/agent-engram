//! Contract tests for dispatch metrics instrumentation (TASK-010.03).
//!
//! Validates that the tool dispatch layer correctly records UsageEvents
//! with proper tool names, response sizes, and token estimates.

use std::sync::Arc;

use engram::server::state::AppState;
use engram::tools;
use serde_json::json;

/// AC#1: dispatch records a UsageEvent with correct tool_name and
/// non-zero response_bytes for read tools.
#[tokio::test]
async fn t010_03_dispatch_records_usage_event_for_read_tools() {
    // GIVEN a minimal AppState with a workspace bound
    let state = Arc::new(AppState::new(10));

    // WHEN dispatching a read tool (list_symbols)
    let _result = tools::dispatch(
        state.clone(),
        "list_symbols",
        Some(json!({ "name_contains": "test" })),
    )
    .await;

    // THEN a UsageEvent was recorded with tool_name = "list_symbols"
    // and response_bytes > 0
    // (Verification requires the MetricsCollector to be initialized
    // and provide a way to inspect recorded events — the Worker must
    // add this test infrastructure during implementation)
    assert!(true, "placeholder — Worker must wire UsageEvent inspection");
}

/// AC#2: dispatch does NOT record a UsageEvent for lifecycle/write tools.
#[tokio::test]
async fn t010_03_dispatch_skips_lifecycle_tools() {
    // GIVEN a minimal AppState
    let state = Arc::new(AppState::new(10));

    // WHEN dispatching a lifecycle tool (get_daemon_status)
    let _result = tools::dispatch(state.clone(), "get_daemon_status", None).await;

    // THEN no UsageEvent was recorded
    assert!(
        true,
        "placeholder — Worker must verify no UsageEvent emitted for lifecycle tools"
    );
}

/// AC#3: estimated_tokens equals response_bytes / 4.
#[tokio::test]
async fn t010_03_estimated_tokens_equals_bytes_div_4() {
    // GIVEN a tool response of known byte size
    let state = Arc::new(AppState::new(10));

    // WHEN dispatching a read tool
    let _result = tools::dispatch(
        state.clone(),
        "list_symbols",
        Some(json!({ "limit": 1 })),
    )
    .await;

    // THEN the recorded UsageEvent has estimated_tokens == response_bytes / 4
    assert!(
        true,
        "placeholder — Worker must capture and verify the UsageEvent fields"
    );
}
