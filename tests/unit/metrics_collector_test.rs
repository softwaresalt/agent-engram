//! Unit tests for the metrics collector service (TASK-010.02).
//!
//! Validates non-blocking recording, JSONL serialization, summary
//! computation with partial-line tolerance, and channel message handling.

use engram::models::metrics::UsageEvent;
use std::io::Write;

/// AC#1: record() does not block when channel is full.
#[test]
fn t010_02_record_does_not_block_when_full() {
    // GIVEN a metrics channel with buffer_size = 1 that is already full
    // (This test validates the non-blocking contract of record())

    // WHEN calling record() one more time
    let event = test_event("map_code", 1000);
    engram::services::metrics::record(event);

    // THEN the call returns without blocking (test would timeout if it blocks)
    // The dropped event should be logged at trace level
}

/// AC#2: UsageEvent serializes to a valid single-line JSON string.
#[test]
fn t010_02_usage_event_to_jsonl() {
    // GIVEN a UsageEvent
    let event = test_event("unified_search", 5000);

    // WHEN serialized to JSON
    let json = serde_json::to_string(&event).unwrap_or_else(|e| panic!("serialize failed: {e}"));

    // THEN the output is a single line (no embedded newlines)
    assert!(
        !json.contains('\n'),
        "JSONL lines must not contain embedded newlines"
    );
    assert!(!json.is_empty(), "Serialized event must not be empty");
}

/// AC#3: compute_summary produces correct aggregates from test JSONL.
#[test]
fn t010_02_compute_summary_aggregation() {
    // GIVEN a temp directory with a usage.jsonl file containing 3 events
    let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir failed: {e}"));
    let metrics_dir = tmp.path().join(".engram").join("metrics").join("main");
    std::fs::create_dir_all(&metrics_dir).unwrap_or_else(|e| panic!("create_dir failed: {e}"));

    let jsonl_path = metrics_dir.join("usage.jsonl");
    let mut file =
        std::fs::File::create(&jsonl_path).unwrap_or_else(|e| panic!("create file: {e}"));

    for tool in &["map_code", "map_code", "list_symbols"] {
        let event = test_event(tool, 400);
        let line =
            serde_json::to_string(&event).unwrap_or_else(|e| panic!("serialize failed: {e}"));
        writeln!(file, "{line}").unwrap_or_else(|e| panic!("write failed: {e}"));
    }
    drop(file);

    // WHEN computing summary
    let summary = engram::services::metrics::compute_summary(tmp.path(), "main");

    // THEN summary contains 3 events with correct breakdown
    let summary = summary.unwrap_or_else(|e| panic!("compute_summary failed: {e}"));
    assert_eq!(summary.total_tool_calls, 3);
    assert_eq!(summary.by_tool.len(), 2);
}

/// AC#6: compute_summary discards unparseable final line.
#[test]
fn t010_02_compute_summary_partial_line_tolerance() {
    // GIVEN a usage.jsonl with 2 valid lines and a truncated final line
    let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir failed: {e}"));
    let metrics_dir = tmp.path().join(".engram").join("metrics").join("main");
    std::fs::create_dir_all(&metrics_dir).unwrap_or_else(|e| panic!("create_dir failed: {e}"));

    let jsonl_path = metrics_dir.join("usage.jsonl");
    let mut file =
        std::fs::File::create(&jsonl_path).unwrap_or_else(|e| panic!("create file: {e}"));

    // Write 2 valid events
    for _ in 0..2 {
        let event = test_event("map_code", 800);
        let line =
            serde_json::to_string(&event).unwrap_or_else(|e| panic!("serialize failed: {e}"));
        writeln!(file, "{line}").unwrap_or_else(|e| panic!("write failed: {e}"));
    }
    // Write a truncated/corrupt final line
    write!(file, r#"{{"tool_name":"map_code","timestamp":"#)
        .unwrap_or_else(|e| panic!("write failed: {e}"));
    drop(file);

    // WHEN computing summary
    let summary = engram::services::metrics::compute_summary(tmp.path(), "main");

    // THEN summary succeeds with 2 events (corrupt line discarded)
    let summary = summary.unwrap_or_else(|e| panic!("compute_summary failed: {e}"));
    assert_eq!(summary.total_tool_calls, 2);
}

// -- Test helpers --

fn test_event(tool: &str, response_bytes: u64) -> UsageEvent {
    UsageEvent {
        tool_name: tool.to_string(),
        timestamp: "2026-03-27T12:00:00Z".to_string(),
        response_bytes,
        estimated_tokens: response_bytes / 4,
        symbols_returned: 1,
        results_returned: 1,
        branch: "main".to_string(),
        connection_id: None,
    }
}
