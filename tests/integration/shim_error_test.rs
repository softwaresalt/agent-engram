//! Integration tests for shim error scenarios (T023).
//!
//! Scenarios covered:
//! - S009: Daemon timeout — request to unreachable endpoint returns `IpcError::Timeout`
//!   or `IpcError::ConnectionFailed`.
//! - S010: Daemon crash — killing the daemon process mid-session causes subsequent
//!   IPC requests to fail with `IpcError::ConnectionFailed`.

use std::time::Duration;

use engram::daemon::protocol::IpcRequest;
use engram::errors::{EngramError, IpcError};
use engram::shim::ipc_client::send_request;
use engram::shim::lifecycle::check_health;
use serde_json::json;

#[path = "../helpers/mod.rs"]
mod helpers;

use helpers::DaemonHarness;

// ── T023 / S009: Timeout ──────────────────────────────────────────────────────

/// A request to an endpoint where nothing is listening fails with either
/// `IpcError::Timeout` (if the OS connection attempt hangs) or
/// `IpcError::ConnectionFailed` (if the OS rejects it immediately).
/// Both outcomes satisfy the S009 contract: the shim must not hang forever.
#[tokio::test]
async fn t023_s009_request_to_unreachable_endpoint_fails_promptly() {
    let endpoint = if cfg!(windows) {
        r"\\.\pipe\engram-deadbeef99999999".to_owned()
    } else {
        "/tmp/engram-test-s009-unreachable.sock".to_owned()
    };

    let request = IpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: Some(json!(1)),
        method: "_health".to_owned(),
        params: None,
    };

    // Use a 200 ms timeout — the call must fail, not hang.
    let start = std::time::Instant::now();
    let result = send_request(&endpoint, &request, Duration::from_millis(200)).await;
    let elapsed = start.elapsed();

    assert!(result.is_err(), "request to unreachable endpoint must fail");
    assert!(
        elapsed < Duration::from_secs(5),
        "request must fail promptly (took {elapsed:?}); check timeout logic"
    );

    match result.unwrap_err() {
        EngramError::Ipc(IpcError::Timeout { .. } | IpcError::ConnectionFailed { .. }) => {
            // Expected: Timeout if connect blocks until deadline;
            // ConnectionFailed if the OS rejects immediately (ECONNREFUSED / ERROR_FILE_NOT_FOUND).
        }
        other => panic!("expected Timeout or ConnectionFailed, got: {other}"),
    }
}

/// A request with a very tight deadline (1 ms) must fail with Timeout, not hang.
#[tokio::test]
async fn t023_s009_extremely_short_timeout_returns_timeout_error() {
    let endpoint = if cfg!(windows) {
        r"\\.\pipe\engram-deadbeef11111111".to_owned()
    } else {
        "/tmp/engram-test-s009-short-timeout.sock".to_owned()
    };

    let request = IpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: Some(json!(2)),
        method: "_health".to_owned(),
        params: None,
    };

    let result = send_request(&endpoint, &request, Duration::from_millis(1)).await;

    assert!(result.is_err(), "request with 1 ms timeout must fail");
    // We accept Timeout or ConnectionFailed — both satisfy "not hanging".
    assert!(
        matches!(
            result.unwrap_err(),
            EngramError::Ipc(IpcError::Timeout { .. } | IpcError::ConnectionFailed { .. })
        ),
        "error must be Timeout or ConnectionFailed"
    );
}

// ── T023 / S010: Daemon crash ─────────────────────────────────────────────────

/// After the daemon process is killed, `check_health` returns `false`.
#[tokio::test]
async fn t023_s010_check_health_returns_false_after_daemon_killed() {
    let harness = DaemonHarness::spawn(Duration::from_secs(10))
        .await
        .expect("daemon must spawn");

    let endpoint = harness.ipc_path().to_str().expect("valid UTF-8").to_owned();

    // Daemon is healthy before we kill it.
    assert!(
        check_health(&endpoint).await,
        "daemon must be healthy before being killed"
    );

    // Kill the daemon by dropping the harness.
    drop(harness);

    // Brief pause to let the OS reap the child process and clean up the socket/pipe.
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Health check must now return false.
    assert!(
        !check_health(&endpoint).await,
        "check_health must return false after daemon process is killed"
    );
}

/// After the daemon is killed, subsequent IPC requests fail with a transport error.
#[tokio::test]
async fn t023_s010_ipc_request_fails_after_daemon_crash() {
    let harness = DaemonHarness::spawn(Duration::from_secs(10))
        .await
        .expect("daemon must spawn");

    let endpoint = harness.ipc_path().to_str().expect("valid UTF-8").to_owned();

    // Verify the daemon is healthy before we kill it.
    let healthy_before = check_health(&endpoint).await;
    assert!(healthy_before, "daemon must be healthy before crash");

    // Kill the daemon.
    drop(harness);

    // Give the OS a moment to clean up.
    tokio::time::sleep(Duration::from_millis(150)).await;

    // Now a request must fail.
    let request = IpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: Some(json!(1)),
        method: "_health".to_owned(),
        params: None,
    };

    let result = send_request(&endpoint, &request, Duration::from_millis(500)).await;

    assert!(result.is_err(), "IPC request must fail after daemon crash");
    match result.unwrap_err() {
        EngramError::Ipc(IpcError::ConnectionFailed { .. } | IpcError::Timeout { .. }) => {
            // Expected: ConnectionFailed when nothing is listening; Timeout on platforms
            // where the OS queues the connection briefly after process death.
        }
        other => panic!("expected ConnectionFailed or Timeout after daemon crash, got: {other}"),
    }
}
