//! Contract tests for daemon performance observability (User Story 2).
//!
//! Scenarios: S056–S060 from SCENARIOS.md.

use std::sync::Arc;

use engram::server::state::AppState;

// ── T027: S057 — latency tracking ────────────────────────────────────────────

/// S057: Tool call latency is recorded and percentiles are computable.
#[tokio::test]
async fn t027_latency_tracking() {
    let state = Arc::new(AppState::new(1));

    // Record 10 synthetic latency samples (microseconds).
    for micros in [100_u64, 200, 300, 400, 500, 600, 700, 800, 900, 1_000] {
        state.record_tool_latency(micros).await;
    }

    let (p50, p95, p99) = state.latency_percentiles().await;
    assert!(p50 > 0, "p50 must be > 0 after recording latencies");
    assert!(p95 >= p50, "p95 must be >= p50");
    assert!(p99 >= p95, "p99 must be >= p95");
    assert_eq!(
        state.tool_call_count(),
        10,
        "tool_call_count must equal the number of recorded latency samples"
    );
}

// ── T028: S056 — health report shape ─────────────────────────────────────────

/// S056: `get_health_report` returns all expected metrics fields.
#[tokio::test]
async fn t028_health_report_has_required_fields() {
    let state = Arc::new(AppState::new(1));

    let result = engram::tools::read::get_health_report(state, None)
        .await
        .expect("get_health_report must not fail");

    assert!(result["version"].is_string(), "version must be a string");
    assert!(
        result["uptime_seconds"].is_number(),
        "uptime_seconds must be a number"
    );
    assert!(
        result["active_connections"].is_number(),
        "active_connections must be a number"
    );
    assert!(
        result["tool_call_count"].is_number(),
        "tool_call_count must be a number"
    );
    assert!(
        result["latency_us"].is_object(),
        "latency_us must be an object"
    );
    assert!(
        result["latency_us"]["p50"].is_number(),
        "p50 must be a number"
    );
    assert!(
        result["latency_us"]["p95"].is_number(),
        "p95 must be a number"
    );
    assert!(
        result["latency_us"]["p99"].is_number(),
        "p99 must be a number"
    );
    assert!(
        result["memory_mb"].is_number(),
        "memory_mb must be a number"
    );
    assert!(
        result["watcher_events"].is_number(),
        "watcher_events must be a number"
    );
}

// ── T029: S060 — health report without workspace ─────────────────────────────

/// S060: `get_health_report` works without workspace binding.
#[tokio::test]
async fn t029_health_report_no_workspace_required() {
    // No `set_workspace` called — must still succeed.
    let state = Arc::new(AppState::new(1));

    let result = engram::tools::read::get_health_report(state, None).await;
    assert!(
        result.is_ok(),
        "get_health_report must succeed even without a bound workspace"
    );

    let report = result.expect("already asserted ok");
    assert!(
        report["workspace_id"].is_null(),
        "workspace_id must be null when no workspace is set"
    );
}
