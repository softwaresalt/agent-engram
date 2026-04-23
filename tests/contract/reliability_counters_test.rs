//! Contract tests for reliability telemetry counters (029-F WS-8).
//!
//! Asserts:
//! - `get_daemon_status` exposes a `telemetry` object.
//! - `telemetry` contains the four reliability counter fields.
//! - All counter values start at zero for a fresh daemon state.
//!
//! Red-phase: these tests fail until `ReliabilityCounters` is wired into
//! `DaemonStatus` and returned from `get_daemon_status`.

use std::sync::Arc;

use engram::server::state::AppState;
use engram::tools;
use serde_json::json;

/// `get_daemon_status` response must include a `telemetry` object.
#[tokio::test]
async fn contract_get_daemon_status_has_telemetry_object() {
    let state = Arc::new(AppState::new(10));

    let result = tools::dispatch(state, "get_daemon_status", None)
        .await
        .expect("get_daemon_status must succeed");

    assert!(
        result.get("telemetry").is_some(),
        "get_daemon_status must include a 'telemetry' object; got: {result:?}"
    );
    assert!(
        result["telemetry"].is_object(),
        "'telemetry' must be a JSON object; got: {}",
        result["telemetry"]
    );
}

/// `telemetry` must contain `stale_pid_recovered` counter starting at 0.
#[tokio::test]
async fn contract_telemetry_has_stale_pid_recovered_counter() {
    let state = Arc::new(AppState::new(10));

    let result = tools::dispatch(state, "get_daemon_status", None)
        .await
        .expect("get_daemon_status must succeed");

    let telemetry = result["telemetry"].as_object().expect("telemetry must be object");
    let counter = telemetry
        .get("stale_pid_recovered")
        .expect("telemetry must have stale_pid_recovered field");
    assert_eq!(
        counter.as_u64(),
        Some(0),
        "stale_pid_recovered must start at 0, got: {counter}"
    );
}

/// `telemetry` must contain `version_mismatch_respawn` counter starting at 0.
#[tokio::test]
async fn contract_telemetry_has_version_mismatch_respawn_counter() {
    let state = Arc::new(AppState::new(10));

    let result = tools::dispatch(state, "get_daemon_status", None)
        .await
        .expect("get_daemon_status must succeed");

    let telemetry = result["telemetry"].as_object().expect("telemetry must be object");
    let counter = telemetry
        .get("version_mismatch_respawn")
        .expect("telemetry must have version_mismatch_respawn field");
    assert_eq!(
        counter.as_u64(),
        Some(0),
        "version_mismatch_respawn must start at 0, got: {counter}"
    );
}

/// `telemetry` must contain `registry_validation_failures` counter starting at 0.
#[tokio::test]
async fn contract_telemetry_has_registry_validation_failures_counter() {
    let state = Arc::new(AppState::new(10));

    let result = tools::dispatch(state, "get_daemon_status", None)
        .await
        .expect("get_daemon_status must succeed");

    let telemetry = result["telemetry"].as_object().expect("telemetry must be object");
    let counter = telemetry
        .get("registry_validation_failures")
        .expect("telemetry must have registry_validation_failures field");
    assert_eq!(
        counter.as_u64(),
        Some(0),
        "registry_validation_failures must start at 0, got: {counter}"
    );
}

/// `telemetry` must contain `duplicate_daemon_detected` counter starting at 0.
#[tokio::test]
async fn contract_telemetry_has_duplicate_daemon_detected_counter() {
    let state = Arc::new(AppState::new(10));

    let result = tools::dispatch(state, "get_daemon_status", None)
        .await
        .expect("get_daemon_status must succeed");

    let telemetry = result["telemetry"].as_object().expect("telemetry must be object");
    let counter = telemetry
        .get("duplicate_daemon_detected")
        .expect("telemetry must have duplicate_daemon_detected field");
    assert_eq!(
        counter.as_u64(),
        Some(0),
        "duplicate_daemon_detected must start at 0, got: {counter}"
    );
}

/// Counter must reflect increments after a `set_workspace` call with a broken
/// registry (which should trigger a `registry_validation_failures` increment).
///
/// Green-phase requirement: `set_workspace` MUST call `validate_sources_strict`
/// and increment the counter on failure.
#[tokio::test]
async fn contract_broken_registry_increments_validation_failure_counter() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    let workspace_path = workspace.path().to_str().expect("UTF-8 path").to_owned();

    let engram_dir = workspace.path().join(".engram");
    std::fs::create_dir_all(&engram_dir).expect("create .engram");
    std::fs::write(
        engram_dir.join("registry.yaml"),
        "version: \"1\"\nsources:\n  - path: does_not_exist\n    type: directory\n    optional: false\n",
    )
    .expect("write registry.yaml");

    let state = Arc::new(AppState::new(10));

    let _ = tools::dispatch(
        Arc::clone(&state),
        "set_workspace",
        Some(json!({ "path": workspace_path })),
    )
    .await;

    let result = tools::dispatch(Arc::clone(&state), "get_daemon_status", None)
        .await
        .expect("get_daemon_status must succeed");

    let telemetry = result["telemetry"].as_object().expect("telemetry must be object");
    let counter = telemetry
        .get("registry_validation_failures")
        .expect("telemetry must have registry_validation_failures field");
    assert!(
        counter.as_u64().unwrap_or(0) >= 1,
        "registry_validation_failures must be >= 1 after broken-registry set_workspace, got: {counter}"
    );
}
