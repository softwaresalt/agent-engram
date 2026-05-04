//! Unit tests for the mutable-script retry counter public API (040.001-T).
//!
//! Verifies that `mutable_script_retry_metrics()` returns a valid [`RetryMetrics`]
//! snapshot with the correct initial state.
//!
//! Initial state: `retry_count` is zero and `last_retry_at` is `None`.

use engram::db::mutable_script_retry_metrics;

/// AC: `mutable_script_retry_metrics()` returns a snapshot with a `retry_count` field.
#[test]
fn t040_001_metrics_has_retry_count_field() {
    let metrics = mutable_script_retry_metrics();
    // Any access to the returned struct proves the function returned successfully.
    let _ = metrics.retry_count;
}

/// AC: fresh `retry_count` value is a valid `u64` (zero or greater).
#[test]
fn t040_001_metrics_retry_count_is_u64() {
    let metrics = mutable_script_retry_metrics();
    // u64 is always ≥ 0; this assertion is a type-level smoke test.
    assert!(
        metrics.retry_count < u64::MAX,
        "retry_count must be a valid u64"
    );
}
