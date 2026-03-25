//! Integration tests for embedding status in health report and workspace statistics (dxo.4.3).
//!
//! Verifies that `get_health_report` and `get_workspace_statistics` both expose
//! an `embedding_status` section so agents can see at a glance whether semantic
//! search is functional.

use std::time::Duration;

use engram::daemon::protocol::IpcRequest;
use engram::shim::ipc_client::send_request;
use serde_json::{Value, json};

#[path = "../helpers/mod.rs"]
mod helpers;

use helpers::DaemonHarness;

fn make_request(id: i64, method: &str, params: Option<Value>) -> IpcRequest {
    IpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: Some(Value::Number(serde_json::Number::from(id))),
        method: method.to_owned(),
        params,
    }
}

// ── get_health_report ─────────────────────────────────────────────────

/// Health report must include an `embedding_status` section with the
/// required fields, even before any workspace is bound.
#[tokio::test]
async fn health_report_includes_embedding_status() {
    // GIVEN a running daemon (no workspace required for health report)
    let harness = DaemonHarness::spawn(Duration::from_secs(15))
        .await
        .expect("daemon must spawn");
    let endpoint = harness.ipc_path().to_str().expect("UTF-8").to_owned();

    // WHEN we call get_health_report
    let request = make_request(1, "get_health_report", Some(json!({})));
    let response = send_request(&endpoint, &request, Duration::from_secs(10))
        .await
        .expect("IPC call must succeed");

    assert!(response.error.is_none(), "get_health_report must not error");
    let result = response
        .result
        .expect("get_health_report must return a result");

    // THEN an `embedding_status` section must be present
    assert!(
        result["embedding_status"].is_object(),
        "health report must include embedding_status object; got: {result}"
    );
}

/// The `embedding_status` in the health report must include the core fields
/// that agents need to determine whether semantic search is functional.
#[tokio::test]
async fn health_report_embedding_status_has_required_fields() {
    // GIVEN a running daemon
    let harness = DaemonHarness::spawn(Duration::from_secs(15))
        .await
        .expect("daemon must spawn");
    let endpoint = harness.ipc_path().to_str().expect("UTF-8").to_owned();

    // WHEN we call get_health_report
    let request = make_request(1, "get_health_report", Some(json!({})));
    let response = send_request(&endpoint, &request, Duration::from_secs(10))
        .await
        .expect("IPC call must succeed");

    let result = response
        .result
        .expect("get_health_report must return a result");
    let es = &result["embedding_status"];

    // THEN the section must have all required fields
    assert!(
        es["enabled"].is_boolean(),
        "embedding_status.enabled must be a boolean; got: {es}"
    );
    assert!(
        es["model_loaded"].is_boolean(),
        "embedding_status.model_loaded must be a boolean; got: {es}"
    );
    assert!(
        es["coverage_percent"].is_number(),
        "embedding_status.coverage_percent must be a number; got: {es}"
    );
}

/// When embeddings are disabled at compile time, the health report must
/// clearly indicate that semantic search is not functional.
#[cfg(not(feature = "embeddings"))]
#[tokio::test]
async fn health_report_embedding_status_disabled_when_feature_off() {
    // GIVEN a daemon compiled without the embeddings feature
    let harness = DaemonHarness::spawn(Duration::from_secs(15))
        .await
        .expect("daemon must spawn");
    let endpoint = harness.ipc_path().to_str().expect("UTF-8").to_owned();

    // WHEN we call get_health_report
    let request = make_request(1, "get_health_report", Some(json!({})));
    let response = send_request(&endpoint, &request, Duration::from_secs(10))
        .await
        .expect("IPC call must succeed");

    let result = response
        .result
        .expect("get_health_report must return a result");
    let es = &result["embedding_status"];

    // THEN enabled and model_loaded must both be false
    assert_eq!(
        es["enabled"], false,
        "embedding_status.enabled must be false without feature flag; got: {es}"
    );
    assert_eq!(
        es["model_loaded"], false,
        "embedding_status.model_loaded must be false without feature flag; got: {es}"
    );
}

// ── get_workspace_statistics ──────────────────────────────────────────

/// Workspace statistics must include an `embedding_status` section that
/// reports symbol coverage with the live embedding data from the DB.
#[tokio::test]
async fn workspace_statistics_includes_embedding_status() {
    // GIVEN a daemon with a workspace auto-bound on startup
    let harness = DaemonHarness::spawn(Duration::from_secs(15))
        .await
        .expect("daemon must spawn");
    let endpoint = harness.ipc_path().to_str().expect("UTF-8").to_owned();

    // WHEN we call get_workspace_statistics
    let request = make_request(1, "get_workspace_statistics", None);
    let response = send_request(&endpoint, &request, Duration::from_secs(10))
        .await
        .expect("IPC call must succeed");

    assert!(
        response.error.is_none(),
        "get_workspace_statistics must not error; got: {:?}",
        response.error
    );
    let result = response
        .result
        .expect("get_workspace_statistics must return a result");

    // THEN an `embedding_status` section must be present
    assert!(
        result["embedding_status"].is_object(),
        "workspace statistics must include embedding_status object; got: {result}"
    );
}

/// The `embedding_status` in workspace statistics must expose `coverage_percent`
/// derived from the actual symbol counts in the database.
#[tokio::test]
async fn workspace_statistics_embedding_status_has_coverage_field() {
    // GIVEN a daemon with a bound workspace
    let harness = DaemonHarness::spawn(Duration::from_secs(15))
        .await
        .expect("daemon must spawn");
    let endpoint = harness.ipc_path().to_str().expect("UTF-8").to_owned();

    // WHEN we call get_workspace_statistics
    let request = make_request(1, "get_workspace_statistics", None);
    let response = send_request(&endpoint, &request, Duration::from_secs(10))
        .await
        .expect("IPC call must succeed");

    let result = response
        .result
        .expect("get_workspace_statistics must return a result");
    let es = &result["embedding_status"];

    // THEN coverage_percent must be a numeric value
    assert!(
        es["coverage_percent"].is_number(),
        "embedding_status.coverage_percent must be a number in workspace statistics; got: {es}"
    );

    // AND total_symbols must be a numeric value
    assert!(
        es["total_symbols"].is_number(),
        "embedding_status.total_symbols must be a number; got: {es}"
    );
}
