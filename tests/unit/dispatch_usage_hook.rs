//! Unit tests for the daemon dispatch correlation-id extraction hook
//! (067.002-T / t2).
//!
//! Covers the envelope-side extraction policy: read `_meta.correlation_id`,
//! sanitize-and-truncate (never fail a live tool call), and absence handling.

use engram::models::metrics::CORRELATION_ID_MAX_LEN;
use engram::services::policy::extract_correlation_id;
use serde_json::json;

/// Reads `_meta.correlation_id` when present.
#[test]
fn t067_002_extract_reads_meta_correlation_id() {
    let params = Some(json!({
        "query": "fn main",
        "_meta": { "correlation_id": "corr-xyz-42" }
    }));
    assert_eq!(
        extract_correlation_id(&params),
        Some("corr-xyz-42".to_string())
    );
}

/// Absent `_meta` (or absent field) yields `None`.
#[test]
fn t067_002_extract_absent_meta_is_none() {
    assert_eq!(extract_correlation_id(&None), None);
    assert_eq!(extract_correlation_id(&Some(json!({ "query": "x" }))), None);
    assert_eq!(
        extract_correlation_id(&Some(json!({ "_meta": { "agent_role": "planner" } }))),
        None
    );
}

/// Envelope policy: control characters / newlines are stripped rather than
/// rejected (a malformed id must not fail an otherwise-valid daemon call).
#[test]
fn t067_002_extract_sanitizes_control_chars() {
    let params = Some(json!({
        "_meta": { "correlation_id": "corr\n123\r\tok" }
    }));
    assert_eq!(
        extract_correlation_id(&params),
        Some("corr123ok".to_string())
    );
}

/// Envelope policy: an over-cap id is truncated (not rejected).
#[test]
fn t067_002_extract_truncates_overlong_id() {
    let long = "a".repeat(CORRELATION_ID_MAX_LEN + 40);
    let params = Some(json!({ "_meta": { "correlation_id": long } }));
    let extracted = extract_correlation_id(&params).expect("some");
    assert_eq!(extracted.chars().count(), CORRELATION_ID_MAX_LEN);
}

/// An all-control / empty id yields `None` (nothing usable remains).
#[test]
fn t067_002_extract_all_control_is_none() {
    let params = Some(json!({ "_meta": { "correlation_id": "\n\r\t" } }));
    assert_eq!(extract_correlation_id(&params), None);
}
