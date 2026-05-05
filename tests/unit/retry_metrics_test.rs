//! Unit tests for the mutable-script retry counter public API (040.001-T).
//!
//! Verifies structural and behavioral invariants of the public
//! [`mutable_script_retry_metrics()`] API: field accessibility and monotonicity.
//! Tests do not assert initial-state values because the counters are process-global
//! atomics and may have been incremented by other tests in the same test run.

use engram::db::mutable_script_retry_metrics;

/// AC: `mutable_script_retry_metrics()` returns a snapshot with a `retry_count` field.
#[test]
fn t040_001_metrics_has_retry_count_field() {
    let metrics = mutable_script_retry_metrics();
    // Any access to the returned struct proves the function returned successfully.
    let _ = metrics.retry_count;
}

/// AC: `retry_count` is monotonically non-decreasing across consecutive reads.
///
/// Process-global atomics only increment; a second snapshot must report a count
/// that is greater than or equal to the first.
#[test]
fn t040_001_metrics_retry_count_is_u64() {
    let first = mutable_script_retry_metrics();
    let second = mutable_script_retry_metrics();
    assert!(
        second.retry_count >= first.retry_count,
        "retry_count must be non-decreasing: first={}, second={}",
        first.retry_count,
        second.retry_count,
    );
}
