//! Unit tests for metrics model types (TASK-010.01).
//!
//! Validates serde round-trip, summary aggregation, config defaults,
//! and `BTreeMap` deterministic ordering.

use engram::models::metrics::{MetricsConfig, MetricsSummary, TimeRange, ToolMetrics, UsageEvent};
use std::collections::BTreeMap;

/// AC#1: `UsageEvent` serializes to JSON and round-trips via `serde_json`.
#[test]
fn t010_01_usage_event_serde_round_trip() {
    // GIVEN a fully populated UsageEvent
    let event = UsageEvent {
        tool_name: "map_code".to_string(),
        timestamp: "2026-03-27T12:00:00Z".to_string(),
        response_bytes: 4800,
        estimated_tokens: 1200,
        symbols_returned: 5,
        results_returned: 5,
        branch: "main".to_string(),
        connection_id: Some("uuid-1234".to_string()),
    };

    // WHEN serialized to JSON and deserialized back
    let json = serde_json::to_string(&event).unwrap_or_else(|e| panic!("serialize failed: {e}"));
    let round_tripped: UsageEvent =
        serde_json::from_str(&json).unwrap_or_else(|e| panic!("deserialize failed: {e}"));

    // THEN the round-tripped value equals the original
    assert_eq!(event, round_tripped);
}

/// AC#1 variant: `UsageEvent` with `connection_id = None` omits the field.
#[test]
fn t010_01_usage_event_none_connection_id_omitted() {
    // GIVEN a UsageEvent without connection_id
    let event = UsageEvent {
        tool_name: "list_symbols".to_string(),
        timestamp: "2026-03-27T12:00:00Z".to_string(),
        response_bytes: 200,
        estimated_tokens: 50,
        symbols_returned: 10,
        results_returned: 10,
        branch: "main".to_string(),
        connection_id: None,
    };

    // WHEN serialized to JSON
    let json = serde_json::to_string(&event).unwrap_or_else(|e| panic!("serialize failed: {e}"));

    // THEN the JSON does not contain "connection_id"
    assert!(
        !json.contains("connection_id"),
        "connection_id should be omitted when None"
    );
}

/// AC#2: `MetricsSummary::from_events` computes correct aggregates.
#[test]
fn t010_01_metrics_summary_from_events() {
    // GIVEN 5 UsageEvents across 2 different tools
    let events = vec![
        usage_event("map_code", 1000, "2026-03-27T10:00:00Z"),
        usage_event("map_code", 2000, "2026-03-27T10:01:00Z"),
        usage_event("list_symbols", 500, "2026-03-27T10:02:00Z"),
        usage_event("map_code", 1500, "2026-03-27T10:03:00Z"),
        usage_event("list_symbols", 300, "2026-03-27T10:04:00Z"),
    ];

    // WHEN computing a summary
    let summary = MetricsSummary::from_events(&events);

    // THEN totals are correct
    assert_eq!(summary.total_tool_calls, 5);
    assert_eq!(summary.total_tokens, 1325); // (1000+2000+500+1500+300)/4
    assert_eq!(summary.by_tool.len(), 2);
    assert!(summary.by_tool.contains_key("map_code"));
    assert!(summary.by_tool.contains_key("list_symbols"));
}

/// AC#3: `MetricsConfig` defaults to `enabled=true`, `buffer_size=1024`.
#[test]
fn t010_01_metrics_config_defaults() {
    // GIVEN default MetricsConfig
    let config = MetricsConfig::default();

    // THEN defaults are as specified
    assert!(config.enabled);
    assert_eq!(config.buffer_size, 1024);
}

/// AC#4: `MetricsConfig` deserializes from partial TOML.
#[test]
fn t010_01_metrics_config_partial_toml() {
    // GIVEN TOML with only enabled field
    let toml_str = r"enabled = false";

    // WHEN deserialized
    let config: MetricsConfig =
        toml::from_str(toml_str).unwrap_or_else(|e| panic!("TOML parse failed: {e}"));

    // THEN enabled is overridden but buffer_size gets default
    assert!(!config.enabled);
    assert_eq!(config.buffer_size, 1024);
}

/// AC#5: `BTreeMap` produces deterministic key ordering in serialized summary.
#[test]
fn t010_01_btreemap_deterministic_ordering() {
    // GIVEN a MetricsSummary with tools in non-alphabetical insert order
    let mut by_tool = BTreeMap::new();
    by_tool.insert(
        "zebra_tool".to_string(),
        ToolMetrics {
            call_count: 1,
            total_tokens: 100,
            avg_tokens: 100.0,
        },
    );
    by_tool.insert(
        "alpha_tool".to_string(),
        ToolMetrics {
            call_count: 2,
            total_tokens: 200,
            avg_tokens: 100.0,
        },
    );

    let summary = MetricsSummary {
        total_tool_calls: 3,
        total_tokens: 300,
        by_tool,
        top_symbols: vec![],
        time_range: TimeRange {
            start: "2026-01-01T00:00:00Z".to_string(),
            end: "2026-03-27T00:00:00Z".to_string(),
        },
        session_count: 1,
    };

    // WHEN serialized to JSON
    let json = serde_json::to_string(&summary).unwrap_or_else(|e| panic!("serialize failed: {e}"));

    // THEN "alpha_tool" appears before "zebra_tool" (sorted keys)
    let alpha_pos = json.find("alpha_tool");
    let zebra_pos = json.find("zebra_tool");
    assert!(
        alpha_pos < zebra_pos,
        "BTreeMap should produce alphabetical key ordering: alpha_pos={alpha_pos:?}, zebra_pos={zebra_pos:?}"
    );
}

// -- Test helpers --

fn usage_event(tool: &str, response_bytes: u64, timestamp: &str) -> UsageEvent {
    UsageEvent {
        tool_name: tool.to_string(),
        timestamp: timestamp.to_string(),
        response_bytes,
        estimated_tokens: response_bytes / 4,
        symbols_returned: 1,
        results_returned: 1,
        branch: "main".to_string(),
        connection_id: None,
    }
}
