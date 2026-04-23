use std::fs;
use std::sync::Arc;

use serde_json::{Value, json};

use engram::models::health::{HealthReport, HealthStatus};
use engram::server::state::AppState;
use engram::tools;

/// Assert `get_daemon_status` includes a `health` field covering all 8 failure modes.
#[tokio::test]
async fn contract_get_daemon_status_includes_health_field() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");
    let state = Arc::new(AppState::new(10));
    let path = workspace.path().to_string_lossy().to_string();

    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace should succeed");

    let status = tools::dispatch(state.clone(), "get_daemon_status", None)
        .await
        .expect("get_daemon_status should succeed");

    assert!(
        status.get("health").is_some(),
        "get_daemon_status should include a health field"
    );

    let health = status.get("health").expect("health field present");
    let checks = health
        .get("checks")
        .and_then(Value::as_array)
        .expect("health.checks should be an array");

    assert_eq!(
        checks.len(),
        8,
        "health report must cover all 8 failure modes: binary_version, pid_liveness, \
         workspace_identity, pipe_reachability, registry_validity, offline_scan, \
         session_resume, telemetry_health"
    );
}

/// Assert each required check name is present in the health report.
#[tokio::test]
async fn contract_health_report_covers_required_check_names() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");
    let state = Arc::new(AppState::new(10));
    let path = workspace.path().to_string_lossy().to_string();

    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace should succeed");

    let status = tools::dispatch(state.clone(), "get_daemon_status", None)
        .await
        .expect("get_daemon_status should succeed");

    let checks = status
        .get("health")
        .and_then(|h| h.get("checks"))
        .and_then(Value::as_array)
        .expect("health.checks array present");

    let names: Vec<&str> = checks
        .iter()
        .filter_map(|c| c.get("name").and_then(Value::as_str))
        .collect();

    for required in [
        "binary_version",
        "pid_liveness",
        "workspace_identity",
        "pipe_reachability",
        "registry_validity",
        "offline_scan",
        "session_resume",
        "telemetry_health",
    ] {
        assert!(
            names.contains(&required),
            "health report missing required check: {required}"
        );
    }
}

/// Assert each check has a valid `status` field.
#[tokio::test]
async fn contract_health_checks_have_valid_status_values() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");
    let state = Arc::new(AppState::new(10));
    let path = workspace.path().to_string_lossy().to_string();

    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace should succeed");

    let status = tools::dispatch(state.clone(), "get_daemon_status", None)
        .await
        .expect("get_daemon_status should succeed");

    let checks = status
        .get("health")
        .and_then(|h| h.get("checks"))
        .and_then(Value::as_array)
        .expect("health.checks array present");

    let valid_statuses = ["unknown", "green", "yellow", "red"];
    for check in checks {
        let check_name = check
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("(unknown)");
        let check_status = check
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("(missing)");
        assert!(
            valid_statuses.contains(&check_status),
            "check {check_name} has invalid status: {check_status}"
        );
    }
}

/// Unit test: `HealthReport` type structure compiles and has expected API.
#[test]
fn health_report_type_has_expected_structure() {
    let report = HealthReport::default();
    assert!(matches!(report.overall, HealthStatus::Unknown));
    assert!(report.checks.is_empty());
    assert!(report.find_check("binary_version").is_none());
}
