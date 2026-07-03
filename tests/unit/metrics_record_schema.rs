//! Unit tests for the usage-telemetry record + metrics-config schema extension
//! (067.001-T / t1).
//!
//! Covers: serde round-trip with the new autoharness-facing fields, back-compat
//! deserialization of a v1 record (missing new fields), omission of optional
//! fields when absent, `MetricsConfig` defaults, correlation-id validation /
//! sanitization (dual-source policy split), stable hashing, and `CoarseParams`.

use engram::models::metrics::{
    CORRELATION_ID_MAX_LEN, CoarseParams, MetricsConfig, USAGE_SCHEMA_VERSION, UsageEvent,
    sanitize_correlation_id, stable_hash_hex, validate_correlation_id,
};

/// AC#1: a fully populated `UsageEvent` (incl. new fields) round-trips via serde.
#[test]
fn t067_001_usage_event_round_trip_with_new_fields() {
    let event = UsageEvent {
        tool_name: "unified_search".to_string(),
        timestamp: "2026-07-03T12:00:00+00:00".to_string(),
        branch: "main".to_string(),
        workspace: "/home/dev/proj".to_string(),
        latency_ms: 42,
        correlation_id: Some("corr-abc-123".to_string()),
        params_summary: CoarseParams::from_parts(Some("fn dispatch"), Some(10)),
        ..Default::default()
    };
    assert_eq!(event.schema_version, USAGE_SCHEMA_VERSION);

    let json = serde_json::to_string(&event).expect("serialize");
    let round_tripped: UsageEvent = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(event, round_tripped);
    // Pinned contract fields must be present in the wire form.
    assert!(json.contains("\"schema_version\":2"));
    assert!(json.contains("\"correlation_id\":\"corr-abc-123\""));
    assert!(json.contains("\"latency_ms\":42"));
    assert!(json.contains("\"workspace\":\"/home/dev/proj\""));
    assert!(json.contains("\"params_summary\""));
}

/// AC#2: a v1 record (without any of the new fields) still deserializes and the
/// new fields take their additive defaults (`schema_version` = 2, others empty).
#[test]
fn t067_001_v1_record_back_compat_deserializes() {
    // A pre-067 record: no schema_version / correlation_id / latency_ms /
    // workspace / params_summary.
    let v1 = r#"{
        "tool_name": "map_code",
        "timestamp": "2026-03-27T12:00:00Z",
        "response_bytes": 1000,
        "estimated_tokens": 250,
        "symbols_returned": 3,
        "results_returned": 3,
        "branch": "main"
    }"#;

    let event: UsageEvent = serde_json::from_str(v1).expect("v1 record must still deserialize");
    assert_eq!(event.tool_name, "map_code");
    assert_eq!(event.schema_version, USAGE_SCHEMA_VERSION);
    assert_eq!(event.correlation_id, None);
    assert_eq!(event.latency_ms, 0);
    assert!(event.workspace.is_empty());
    assert_eq!(event.params_summary, None);
}

/// AC#3: optional fields (`correlation_id`, `params_summary`) are omitted from
/// the JSON when absent.
#[test]
fn t067_001_optional_fields_omitted_when_absent() {
    let event = UsageEvent {
        tool_name: "list_symbols".to_string(),
        timestamp: "2026-07-03T12:00:00+00:00".to_string(),
        branch: "main".to_string(),
        ..Default::default()
    };
    let json = serde_json::to_string(&event).expect("serialize");
    assert!(
        !json.contains("correlation_id"),
        "correlation_id must be omitted when None; got {json}"
    );
    assert!(
        !json.contains("params_summary"),
        "params_summary must be omitted when None; got {json}"
    );
    // schema_version is NOT optional — always present.
    assert!(json.contains("\"schema_version\":2"));
}

/// AC#4: `MetricsConfig` defaults include the new rotation / path-override fields.
#[test]
fn t067_001_metrics_config_defaults() {
    let cfg = MetricsConfig::default();
    assert!(cfg.enabled);
    assert_eq!(cfg.buffer_size, 1024);
    assert_eq!(cfg.usage_path_override, None);
    assert_eq!(cfg.max_file_bytes, 10 * 1024 * 1024);
    assert_eq!(cfg.max_rotated_files, 5);
}

/// Envelope policy: `sanitize_correlation_id` strips control chars/newlines and
/// truncates to the cap (never fails).
#[test]
fn t067_001_sanitize_strips_control_and_truncates() {
    // Newline / carriage-return / tab are removed to protect JSONL line integrity.
    assert_eq!(
        sanitize_correlation_id("abc\ndef\r\tghi"),
        Some("abcdefghi".to_string())
    );
    // Over-cap input is truncated to CORRELATION_ID_MAX_LEN chars.
    let long = "x".repeat(CORRELATION_ID_MAX_LEN + 50);
    let sanitized = sanitize_correlation_id(&long).expect("non-empty");
    assert_eq!(sanitized.chars().count(), CORRELATION_ID_MAX_LEN);
    // All-control / empty input yields None (treated as not supplied).
    assert_eq!(sanitize_correlation_id("\n\r\t"), None);
    assert_eq!(sanitize_correlation_id(""), None);
}

/// CLI/direct policy: `validate_correlation_id` rejects invalid/over-cap ids.
#[test]
fn t067_001_validate_rejects_invalid_and_overlong() {
    // Valid id passes through unchanged.
    assert_eq!(
        validate_correlation_id("corr-123"),
        Ok(Some("corr-123".to_string()))
    );
    // Empty id is "not supplied".
    assert_eq!(validate_correlation_id(""), Ok(None));
    // Control characters are rejected.
    assert!(validate_correlation_id("bad\nid").is_err());
    // Over-cap ids are rejected.
    let long = "y".repeat(CORRELATION_ID_MAX_LEN + 1);
    assert!(validate_correlation_id(&long).is_err());
    // Exactly at cap is accepted.
    let at_cap = "z".repeat(CORRELATION_ID_MAX_LEN);
    assert!(validate_correlation_id(&at_cap).is_ok());
}

/// `stable_hash_hex` is deterministic and collision-distinct for different input.
#[test]
fn t067_001_stable_hash_is_deterministic() {
    let a = stable_hash_hex("select * from foo");
    let b = stable_hash_hex("select * from foo");
    let c = stable_hash_hex("select * from bar");
    assert_eq!(a, b, "same input must hash identically");
    assert_ne!(a, c, "different input should hash differently");
    assert_eq!(a.len(), 16, "hash is 16 hex chars (64-bit)");
}

/// `CoarseParams::from_parts` never stores raw query text and omits itself when
/// neither a query nor a limit is present.
#[test]
fn t067_001_coarse_params_from_parts() {
    // No query, no limit → None (field omitted).
    assert_eq!(CoarseParams::from_parts(None, None), None);

    // Query present → hash + length recorded, raw text never stored.
    let raw = "sensitive query text";
    let cp = CoarseParams::from_parts(Some(raw), Some(25)).expect("some");
    assert_eq!(cp.query_hash, Some(stable_hash_hex(raw)));
    assert_eq!(cp.query_len, Some(20));
    assert_eq!(cp.limit, Some(25));
    let json = serde_json::to_string(&cp).expect("serialize");
    assert!(
        !json.contains(raw),
        "raw query text must never be persisted; got {json}"
    );

    // Limit only → query fields omitted.
    let limit_only = CoarseParams::from_parts(None, Some(5)).expect("some");
    assert_eq!(limit_only.query_hash, None);
    assert_eq!(limit_only.query_len, None);
    assert_eq!(limit_only.limit, Some(5));
}
