//! Integration test for the `doctor --smoke` full-handshake round-trip.
//!
//! Exercises [`engram::tools::doctor::run_smoke_test`] end-to-end.
//! A regression here means the daemon IPC handshake or smoke-test logic broke.

use std::fs;

use engram::tools::doctor::run_smoke_test;

/// A full shim→daemon handshake smoke test must pass cleanly for a valid
/// git-backed workspace.
///
/// Daemon spawns, handshake succeeds, result.passed == true.
#[tokio::test]
async fn doctor_smoke_exercises_full_handshake() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");

    let result = run_smoke_test(workspace.path())
        .await
        .expect("smoke test should succeed without errors");

    assert!(
        result.passed,
        "smoke test reported failure: {}",
        result.message
    );
}

/// Smoke result latency field is present when the test completes.
///
/// Red phase: panics at `todo!()` before reaching this assertion.
#[tokio::test]
async fn doctor_smoke_result_includes_latency() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");

    let result = run_smoke_test(workspace.path())
        .await
        .expect("smoke test should succeed");

    assert!(
        result.latency_ms.is_some(),
        "smoke result should include latency measurement"
    );
}
