//! Integration test for the `doctor --smoke` full-handshake round-trip.
//!
//! Exercises [`engram::tools::doctor::run_smoke_test`] end-to-end.
//! A regression here means the daemon IPC handshake or smoke-test logic broke.

use std::fs;
use std::time::Duration;

use engram::daemon::ipc_server::ipc_endpoint;
use engram::daemon::protocol::IpcRequest;
use engram::shim::ipc_client::send_request;
use engram::tools::doctor::run_smoke_test;
use serde_json::Value;

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

/// A `read_server`-mode daemon's smoke run must parse the live status
/// response, pass, and leave the daemon running without issuing
/// `set_workspace` or `_shutdown` (138.008-T / 142.039-T safety contract).
///
/// This exercises the full live-mode branch end-to-end: a real daemon
/// process is spawned in `read_server` mode (via `.engram/config.toml`'s
/// persisted `mode` setting), `run_smoke_test` is run against it, and then a
/// direct follow-up IPC call proves the daemon is still alive -- which would
/// fail if `run_smoke_test` had sent `_shutdown` (the `managed`-mode
/// behavior). A regression that silently re-selected the `managed` smoke
/// sequence for a `read_server` daemon (e.g. by reverting to on-disk config
/// instead of the daemon's own observed live mode) would either corrupt the
/// live daemon's active workspace via an unwanted `set_workspace`, or shut
/// the daemon down -- either of which this test would catch.
#[tokio::test]
async fn doctor_smoke_leaves_read_server_daemon_running_without_binding_workspace() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");
    let engram_dir = workspace.path().join(".engram");
    fs::create_dir_all(&engram_dir).expect("create .engram");
    fs::write(engram_dir.join("config.toml"), "mode = \"read_server\"\n")
        .expect("write read_server config.toml");

    let result = run_smoke_test(workspace.path())
        .await
        .expect("read_server smoke test should succeed without errors");
    assert!(
        result.passed,
        "read_server smoke test reported failure: {}",
        result.message
    );

    // Prove the daemon was left running (not shut down) by sending a direct
    // follow-up get_daemon_status call. A `managed`-mode-style `_shutdown`
    // call during the smoke run would make this connection fail.
    let endpoint = ipc_endpoint(workspace.path()).expect("resolve ipc endpoint");
    let follow_up = IpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: Some(Value::Number(serde_json::Number::from(99))),
        method: "get_daemon_status".to_owned(),
        params: None,
    };
    let response = send_request(&endpoint, &follow_up, Duration::from_secs(10))
        .await
        .expect("follow-up get_daemon_status must reach a still-running read_server daemon");
    assert!(
        response.error.is_none(),
        "follow-up get_daemon_status returned an error, daemon may not have survived the smoke run: {:?}",
        response.error
    );
    let reported_mode = response
        .result
        .as_ref()
        .and_then(|result| result.get("mode"))
        .and_then(Value::as_str);
    assert_eq!(
        reported_mode,
        Some("read_server"),
        "daemon must still be running in read_server mode after the smoke run"
    );

    // Clean up: explicitly shut down the daemon this test spawned, since a
    // read_server smoke run intentionally never does so itself.
    let shutdown = IpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: Some(Value::Number(serde_json::Number::from(100))),
        method: "_shutdown".to_owned(),
        params: None,
    };
    let _ = send_request(&endpoint, &shutdown, Duration::from_secs(10)).await;
}
