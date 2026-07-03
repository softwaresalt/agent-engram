//! Contract tests locking the `usage.jsonl` record shape and back-compat
//! semantics for the usage-telemetry EMIT feature (067.004-T / t4).
//!
//! These are pure model-level tests (no daemon, no writer singleton): they pin
//! the serialized wire contract that downstream tooling (autoharness) consumes.

use engram::models::metrics::{
    CORRELATION_ID_MAX_LEN, CoarseParams, USAGE_SCHEMA_VERSION, UsageEvent,
    sanitize_correlation_id, validate_correlation_id,
};

fn base_event() -> UsageEvent {
    UsageEvent {
        tool_name: "unified_search".to_owned(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        ..Default::default()
    }
}

/// A pre-schema (v1) record — one that predates `schema_version` / `correlation_id` /
/// `latency_ms` / `workspace` / `params_summary` — must still deserialize, with the new
/// fields taking their documented defaults (`schema_version` = 2, `correlation_id`
/// absent).
#[test]
fn t067_004_v1_record_back_compat_deserializes() {
    let v1 = r#"{
        "tool_name": "map_code",
        "timestamp": "2026-01-01T00:00:00+00:00",
        "response_bytes": 512,
        "estimated_tokens": 128,
        "symbols_returned": 4,
        "results_returned": 4,
        "branch": "main"
    }"#;

    let event: UsageEvent = serde_json::from_str(v1).expect("v1 record must deserialize");
    assert_eq!(event.schema_version, USAGE_SCHEMA_VERSION);
    assert_eq!(event.schema_version, 2);
    assert_eq!(event.correlation_id, None);
    assert_eq!(event.latency_ms, 0);
    assert!(event.workspace.is_empty());
    assert!(event.params_summary.is_none());
}

/// The default schema version is pinned at 2.
#[test]
fn t067_004_schema_version_is_pinned_at_2() {
    let event = base_event();
    assert_eq!(event.schema_version, 2);
}

/// `correlation_id` is omitted from the serialized record when unset.
#[test]
fn t067_004_correlation_id_omitted_when_none() {
    let event = base_event();
    let json = serde_json::to_string(&event).expect("serialize");
    assert!(
        !json.contains("correlation_id"),
        "correlation_id must be omitted when None; got: {json}"
    );
}

/// `correlation_id` is present in the serialized record when set.
#[test]
fn t067_004_correlation_id_present_when_some() {
    let mut event = base_event();
    event.correlation_id = Some("corr-contract-1".to_owned());
    let json = serde_json::to_string(&event).expect("serialize");
    assert!(json.contains("\"correlation_id\":\"corr-contract-1\""));
}

/// The coarse params summary never leaks raw query text — only a stable hash,
/// the length, and any caller-supplied limit.
#[test]
fn t067_004_params_summary_is_coarse_no_raw_text() {
    let summary = CoarseParams::from_parts(Some("fn secret_query"), Some(10))
        .expect("summary present when query/limit supplied");

    assert_eq!(summary.query_len, Some(15));
    assert_eq!(summary.limit, Some(10));
    let hash = summary.query_hash.as_deref().expect("query_hash present");
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));

    let json = serde_json::to_string(&summary).expect("serialize");
    assert!(
        !json.contains("secret"),
        "coarse summary must not contain raw query text; got: {json}"
    );
}

/// Neither a query nor a limit → no summary (field omitted).
#[test]
fn t067_004_params_summary_absent_when_nothing_supplied() {
    assert!(CoarseParams::from_parts(None, None).is_none());
    let event = base_event();
    let json = serde_json::to_string(&event).expect("serialize");
    assert!(!json.contains("params_summary"));
}

/// Every record carries an ISO-8601 timestamp pinned to UTC.
#[test]
fn t067_004_timestamp_is_iso8601_utc() {
    let event = base_event();
    let parsed = chrono::DateTime::parse_from_rfc3339(&event.timestamp)
        .expect("timestamp must be RFC3339 / ISO-8601");
    assert_eq!(
        parsed.offset().local_minus_utc(),
        0,
        "timestamp must be pinned to UTC (+00:00)"
    );
}

/// A fully-populated record round-trips through serialize → deserialize.
#[test]
fn t067_004_record_round_trips() {
    let mut event = base_event();
    event.correlation_id = Some("corr-rt".to_owned());
    event.latency_ms = 42;
    event.workspace = "/tmp/ws".to_owned();
    event.params_summary = CoarseParams::from_parts(Some("query"), Some(5));

    let json = serde_json::to_string(&event).expect("serialize");
    let back: UsageEvent = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(event, back);
}

/// The dual-source validation policy is locked: the CLI/direct surface rejects
/// control chars / over-cap, while the envelope surface sanitizes-and-truncates.
#[test]
fn t067_004_correlation_id_dual_policy() {
    // CLI/direct policy: reject.
    assert!(validate_correlation_id("bad\nid").is_err());
    assert!(validate_correlation_id(&"a".repeat(CORRELATION_ID_MAX_LEN + 1)).is_err());
    assert_eq!(validate_correlation_id("").expect("empty ok"), None);
    assert_eq!(
        validate_correlation_id("ok-id").expect("valid"),
        Some("ok-id".to_owned())
    );

    // Envelope policy: sanitize + truncate, never fail.
    assert_eq!(
        sanitize_correlation_id("keep\nme"),
        Some("keepme".to_owned())
    );
    let truncated = sanitize_correlation_id(&"a".repeat(CORRELATION_ID_MAX_LEN + 50))
        .expect("non-empty after sanitize");
    assert_eq!(truncated.chars().count(), CORRELATION_ID_MAX_LEN);
    assert_eq!(sanitize_correlation_id("\n\t\r"), None);
}
