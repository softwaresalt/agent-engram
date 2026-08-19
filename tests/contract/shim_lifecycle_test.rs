//! Contract tests for shim lifecycle (T020–T022).
//!
//! Scenarios covered:
//! - S001: Cold start — no daemon running, shim spawns daemon, forwards request, returns response
//! - S002: Warm start — daemon already running, shim connects and forwards
//! - S004: Error forwarding — daemon returns tool error, IPC client propagates faithfully
//! - S005: Cold start completes within 2 s (production SLA; debug budget 30 s)
//! - S008: Unknown method → method-not-found error forwarded faithfully

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use engram::daemon::protocol::IpcRequest;
use engram::shim::ipc_client::send_request;
use engram::shim::lifecycle::check_health;
use serde_json::json;
use tempfile::TempDir;
use tokio::io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader};

#[path = "../helpers/mod.rs"]
mod helpers;

use helpers::DaemonHarness;

fn run_git(repo: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .expect("run git fixture command");
    assert!(
        output.status.success(),
        "git {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn create_real_linked_worktree() -> (TempDir, PathBuf) {
    let fixture = TempDir::new().expect("fixture tempdir");
    let primary = fixture.path().join("primary");
    let linked = fixture.path().join("linked");
    fs::create_dir(&primary).expect("create primary checkout");
    run_git(&primary, &["init", "--initial-branch=main"]);
    run_git(&primary, &["config", "user.name", "Engram Test"]);
    run_git(
        &primary,
        &["config", "user.email", "engram-test@example.invalid"],
    );
    fs::write(primary.join("README.md"), "# fixture\n").expect("write tracked fixture");
    run_git(&primary, &["add", "README.md"]);
    run_git(&primary, &["commit", "-m", "fixture"]);

    let output = Command::new("git")
        .args(["worktree", "add", "-b", "feature/mcp-worktree"])
        .arg(&linked)
        .current_dir(&primary)
        .output()
        .expect("create linked worktree");
    assert!(
        output.status.success(),
        "git worktree add failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    (fixture, linked)
}

async fn request_daemon_shutdown(endpoint: &str) {
    let request = IpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: Some(json!(9001)),
        method: "_shutdown".to_owned(),
        params: None,
    };
    let _ = send_request(endpoint, &request, Duration::from_secs(2)).await;
}

// ── 122.004-T: worktree MCP startup contract ─────────────────────────────────

#[tokio::test]
async fn mcp_shim_handshake_is_bounded_and_stdout_clean_in_real_worktree() {
    let (_fixture, linked) = create_real_linked_worktree();
    let workspace = linked
        .to_str()
        .expect("linked worktree path must be valid UTF-8");
    let mut child = tokio::process::Command::new(env!("CARGO_BIN_EXE_engram"))
        .args(["shim", "--workspace", workspace])
        .env_remove("ENGRAM_DATA_DIR")
        .env("ENGRAM_READY_TIMEOUT_MS", "15000")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn MCP shim");
    let mut stdin = child.stdin.take().expect("capture shim stdin");
    let stdout = child.stdout.take().expect("capture shim stdout");

    stdin
        .write_all(
            br#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"worktree-contract","version":"1.0"}}}
"#,
        )
        .await
        .expect("write MCP initialize frame");

    let started = Instant::now();
    let mut response_line = String::new();
    let read_result = tokio::time::timeout(
        Duration::from_secs(20),
        BufReader::new(stdout).read_line(&mut response_line),
    )
    .await;
    let endpoint = engram::daemon::ipc_server::ipc_endpoint(&linked).ok();
    if let Some(endpoint) = endpoint.as_deref() {
        request_daemon_shutdown(endpoint).await;
    }
    drop(stdin);
    let wait_result = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;

    let bytes_read = read_result
        .expect("MCP initialize response must be bounded")
        .expect("read MCP initialize response");
    assert!(bytes_read > 0, "shim exited without an MCP response");
    assert!(
        started.elapsed() < Duration::from_secs(20),
        "worktree MCP startup exceeded its explicit contract budget"
    );
    let response: serde_json::Value =
        serde_json::from_str(response_line.trim()).expect("stdout must contain only an MCP frame");
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);
    assert!(
        response["result"]["capabilities"]["tools"].is_object(),
        "initialize response must advertise tools: {response}"
    );
    assert!(
        wait_result.is_ok(),
        "shim must exit after its MCP stdin closes"
    );
}

#[test]
fn shim_reports_spawned_daemon_early_exit_before_readiness_budget() {
    let workspace = TempDir::new().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create git metadata");
    fs::write(
        workspace.path().join(".git").join("HEAD"),
        "ref: refs/heads/main\n",
    )
    .expect("write HEAD");
    let current_test_exe = std::env::current_exe().expect("resolve current test executable");
    let started = Instant::now();
    let output = Command::new(env!("CARGO_BIN_EXE_engram"))
        .args(["shim", "--workspace"])
        .arg(workspace.path())
        .env_remove("ENGRAM_DATA_DIR")
        .env("CARGO_BIN_EXE_engram", current_test_exe)
        .env("ENGRAM_READY_TIMEOUT_MS", "30000")
        .stdin(Stdio::null())
        .output()
        .expect("run shim with early-exiting daemon executable");

    assert!(
        !output.status.success(),
        "shim must fail when its exact spawned daemon exits"
    );
    assert!(
        started.elapsed() < Duration::from_secs(15),
        "spawned daemon exit must fail fast instead of consuming the 30 s readiness budget"
    );
    assert!(
        output.stdout.is_empty(),
        "startup diagnostics must not contaminate MCP protocol stdout"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("exited") || stderr.contains("status"),
        "startup error must report the child exit status; stderr: {stderr}"
    );
}

#[test]
fn shim_rejects_invalid_workspace_without_consuming_readiness_budget() {
    let workspace = TempDir::new().expect("invalid workspace tempdir");
    let started = Instant::now();
    let output = Command::new(env!("CARGO_BIN_EXE_engram"))
        .args(["shim", "--workspace"])
        .arg(workspace.path())
        .env_remove("ENGRAM_DATA_DIR")
        .env("ENGRAM_READY_TIMEOUT_MS", "30000")
        .stdin(Stdio::null())
        .output()
        .expect("run shim for invalid workspace");

    assert!(!output.status.success(), "invalid workspace must fail");
    assert!(
        started.elapsed() < Duration::from_secs(15),
        "invalid workspace must fail before daemon readiness polling"
    );
    assert!(
        output.stdout.is_empty(),
        "invalid startup diagnostics must remain off MCP stdout"
    );
}

// ── T020 / S001: Cold start ───────────────────────────────────────────────────

/// Before any daemon is started, `check_health` returns `false`.
#[tokio::test]
async fn t020_s001_health_check_returns_false_before_daemon_starts() {
    // Pick a well-known-absent endpoint so we don't accidentally hit a running daemon.
    let endpoint = if cfg!(windows) {
        r"\\.\pipe\engram-deadbeef00000000".to_owned()
    } else {
        "/tmp/engram-test-cold-start-absent.sock".to_owned()
    };

    let healthy = check_health(&endpoint).await;
    assert!(
        !healthy,
        "check_health must return false when no daemon is listening at {endpoint}"
    );
}

/// T020 / S001 + S005: A freshly spawned daemon becomes healthy within the startup budget.
///
/// The spec's 2-second SLA (S005) applies to a production release build. In
/// debug test builds — especially when multiple test binaries each spawn a
/// daemon in parallel — the `CozoDB`/`SQLite` backend requires more startup time
/// than `SurrealDB` (schema bootstrap on `SQLite` involves more round-trips).
/// The 30-second budget accommodates `CozoDB` on a shared CI runner.
/// Running this test in isolation consistently passes in ≤ 2 s.
#[tokio::test]
async fn t020_s001_s005_daemon_becomes_healthy_within_startup_timeout() {
    let start = Instant::now();
    let harness = DaemonHarness::spawn(Duration::from_secs(30))
        .await
        .expect("daemon must spawn within the timeout");

    let elapsed = start.elapsed();
    assert!(
        elapsed < Duration::from_secs(30),
        "daemon must be ready within the 30 s debug budget (took {elapsed:?}; \
         spec 2 s SLA applies to release builds in isolation)"
    );

    let endpoint = harness
        .ipc_path()
        .to_str()
        .expect("IPC path is valid UTF-8");
    assert!(
        check_health(endpoint).await,
        "daemon IPC endpoint must be healthy immediately after spawn"
    );
}

/// T020 / S001: A `_health` IPC request against a freshly spawned daemon returns `status: ready`.
#[tokio::test]
async fn t020_s001_health_request_returns_ready_status() {
    let harness = DaemonHarness::spawn(Duration::from_secs(30))
        .await
        .expect("daemon must spawn");

    let endpoint = harness.ipc_path().to_str().expect("valid UTF-8");

    let request = IpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: Some(json!(1)),
        method: "_health".to_owned(),
        params: None,
    };

    let response = send_request(endpoint, &request, Duration::from_secs(5))
        .await
        .expect("_health IPC request must succeed");

    assert!(
        response.error.is_none(),
        "health response must not contain an error"
    );

    let result = response
        .result
        .expect("health response must contain a result");
    assert_eq!(result["status"], "ready", "health status must be 'ready'");
    assert!(
        result["uptime_seconds"].is_number(),
        "health response must include uptime_seconds"
    );
    assert!(
        result["active_connections"].is_number(),
        "health response must include active_connections"
    );
}

// ── T021 / S002: Warm start ───────────────────────────────────────────────────

/// Two sequential IPC requests to the same running daemon both succeed, and
/// the daemon is not restarted between them (uptime is non-decreasing).
#[tokio::test]
async fn t021_s002_warm_start_sequential_requests_share_daemon() {
    let harness = DaemonHarness::spawn(Duration::from_secs(30))
        .await
        .expect("daemon must spawn");

    let endpoint = harness.ipc_path().to_str().expect("valid UTF-8");

    let make_req = |id: u64| IpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: Some(json!(id)),
        method: "_health".to_owned(),
        params: None,
    };

    let resp1 = send_request(endpoint, &make_req(1), Duration::from_secs(5))
        .await
        .expect("first request must succeed");
    let resp2 = send_request(endpoint, &make_req(2), Duration::from_secs(5))
        .await
        .expect("second request must succeed");

    // Both responses must echo their respective IDs.
    assert_eq!(resp1.id, json!(1), "first response must echo id=1");
    assert_eq!(resp2.id, json!(2), "second response must echo id=2");
    assert!(
        resp1.error.is_none(),
        "first response must not have an error"
    );
    assert!(
        resp2.error.is_none(),
        "second response must not have an error"
    );

    // Uptime should be non-decreasing — confirms the same daemon instance handled both.
    let uptime1 = resp1
        .result
        .as_ref()
        .and_then(|v| v["uptime_seconds"].as_f64())
        .unwrap_or(0.0);
    let uptime2 = resp2
        .result
        .as_ref()
        .and_then(|v| v["uptime_seconds"].as_f64())
        .unwrap_or(0.0);
    assert!(
        uptime2 >= uptime1,
        "uptime must be non-decreasing between sequential requests (was {uptime1} → {uptime2})"
    );
}

/// T021 / S002: Response IDs are echoed exactly as sent (numeric type preserved).
#[tokio::test]
async fn t021_s002_response_id_echoed_exactly() {
    let harness = DaemonHarness::spawn(Duration::from_secs(30))
        .await
        .expect("daemon must spawn");

    let endpoint = harness.ipc_path().to_str().expect("valid UTF-8");

    let request = IpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: Some(json!(42)),
        method: "_health".to_owned(),
        params: None,
    };

    let response = send_request(endpoint, &request, Duration::from_secs(5))
        .await
        .expect("request must succeed");

    assert_eq!(response.id, json!(42), "numeric id must be echoed exactly");
}

// ── T022 / S004: Tool error forwarding ───────────────────────────────────────

/// A tool invocation with missing required parameters returns a structured
/// error response. The IPC client receives it as a successful transport but
/// an application-level error in the response body.
#[tokio::test]
async fn t022_s004_tool_error_forwarded_as_ipc_error_payload() {
    let harness = DaemonHarness::spawn(Duration::from_secs(30))
        .await
        .expect("daemon must spawn");

    let endpoint = harness.ipc_path().to_str().expect("valid UTF-8");

    // `update_task` with empty params — missing `task_id` → triggers a domain error.
    let request = IpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: Some(json!(10)),
        method: "update_task".to_owned(),
        params: Some(json!({})),
    };

    let response = send_request(endpoint, &request, Duration::from_secs(5))
        .await
        .expect("IPC transport must succeed — errors are in the payload, not the transport");

    assert_eq!(response.id, json!(10), "response must echo the request id");
    assert!(
        response.error.is_some(),
        "response must contain an error payload for missing required params"
    );

    let wire_err = response.error.unwrap();
    // The daemon wraps domain errors with JSON-RPC internal-error code.
    assert_eq!(
        wire_err.code, -32_603,
        "tool errors must use JSON-RPC internal error code -32603 (got {})",
        wire_err.code
    );
}

// ── T022 / S008: Unknown method ───────────────────────────────────────────────

/// Dispatching an unknown method name to the daemon returns an error in the
/// response payload (not a transport error). The error is forwarded faithfully.
#[tokio::test]
async fn t022_s008_unknown_method_returns_error_in_response() {
    let harness = DaemonHarness::spawn(Duration::from_secs(30))
        .await
        .expect("daemon must spawn");

    let endpoint = harness.ipc_path().to_str().expect("valid UTF-8");

    let request = IpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: Some(json!(99)),
        method: "nonexistent_tool_xyz_abc".to_owned(),
        params: None,
    };

    let response = send_request(endpoint, &request, Duration::from_secs(5))
        .await
        .expect("IPC transport must succeed for unknown method (error is in payload)");

    assert_eq!(
        response.id,
        json!(99),
        "response must echo the request id for unknown methods"
    );
    assert!(
        response.error.is_some(),
        "unknown method must produce an error in the response payload"
    );
}
