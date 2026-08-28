//! Contract tests for the shim's serve-first stdio initialize contract
//! (124-F, stash 870B1AFF, plan unit U1).
#![allow(clippy::doc_markdown)]
//!
//! Historically, `engram shim` evaluated workspace admission, daemon
//! readiness, and IPC endpoint derivation *before* binding the MCP stdio
//! transport. Any failure among those three preconditions terminated the
//! process while an MCP client was mid-`initialize`, which a client observes
//! as a closed pipe (Windows `os error 232`) rather than an attributable
//! failure (see `docs/decisions/2026-08-21-870b1aff-copilot-mcp-stdio-initialize-investigation.md`).
//!
//! These tests assert the new contract: the shim binds the transport and
//! answers `initialize` unconditionally, still serves the static `tools/list`
//! catalog, but fails every `tools/call` with a structured error naming the
//! precondition failure. They also assert the documented distinct exit code,
//! an attributable stderr line, and the absence of sensitive fields in the
//! durable startup-failure record.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};

use serde_json::Value;
use tempfile::TempDir;
use tokio::io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader};

use engram::daemon::protocol::IpcRequest;
use engram::shim::ipc_client::{probe, send_request};
use engram::shim::lifecycle::check_health;

#[path = "../helpers/fake_health_responder.rs"]
#[allow(dead_code)]
mod fake_health_responder;
use fake_health_responder::{FakeHealthResponder, HealthScript};

/// A marker value planted in the environment to prove it is never echoed
/// into stdout, stderr, or the durable startup-failure record.
const FAKE_SECRET_MARKER: &str = "engram-test-secret-marker-4f1c9b7e";

/// Create a workspace directory whose `.git` entry satisfies
/// `canonicalize_workspace` (a directory, not a symlink) without requiring a
/// real `git init`.
fn workspace_with_valid_git_root() -> TempDir {
    let workspace = TempDir::new().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git directory");
    fs::write(
        workspace.path().join(".git").join("HEAD"),
        "ref: refs/heads/main\n",
    )
    .expect("write HEAD");
    workspace
}

async fn read_bounded_mcp_line(
    stdout: &mut BufReader<tokio::process::ChildStdout>,
    timeout: Duration,
    context: &str,
) -> String {
    let mut line = String::new();
    let bytes_read = tokio::time::timeout(timeout, stdout.read_line(&mut line))
        .await
        .unwrap_or_else(|_| panic!("{context} exceeded {timeout:?}"))
        .unwrap_or_else(|error| panic!("failed to read {context}: {error}"));
    assert!(bytes_read > 0, "shim exited before {context}");
    line
}

/// Spawn `engram shim` against `workspace` with its own spawned "daemon"
/// engineered to exit immediately, so `ensure_daemon_running` fails fast and
/// deterministically (readiness-timeout classification) without waiting out
/// the readiness budget.
fn spawn_shim_with_failing_daemon(workspace: &Path) -> tokio::process::Child {
    let current_test_exe = std::env::current_exe().expect("resolve current test executable");
    tokio::process::Command::new(env!("CARGO_BIN_EXE_engram"))
        .args(["shim", "--workspace"])
        .arg(workspace)
        .env_remove("ENGRAM_DATA_DIR")
        .env("CARGO_BIN_EXE_engram", current_test_exe)
        .env("ENGRAM_READY_TIMEOUT_MS", "30000")
        .env("ENGRAM_TEST_FAKE_SECRET", FAKE_SECRET_MARKER)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .expect("spawn engram shim")
}

fn spawn_shim_with_readiness_budget(workspace: &Path, timeout_ms: u64) -> tokio::process::Child {
    tokio::process::Command::new(env!("CARGO_BIN_EXE_engram"))
        .args(["shim", "--workspace"])
        .arg(workspace)
        .env_remove("ENGRAM_DATA_DIR")
        .env("CARGO_BIN_EXE_engram", env!("CARGO_BIN_EXE_engram"))
        .env("ENGRAM_READY_TIMEOUT_MS", timeout_ms.to_string())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .expect("spawn engram shim")
}

/// Spawn the shim with a `FakeHealthResponder` already listening on the
/// workspace's IPC endpoint. Returns the child, the fake responder (kept
/// alive), and the endpoint string.
///
/// For non-`VersionMismatch` scripts, a PID file pointing to the current test
/// process is planted so that the shim's startup path finds a "live daemon",
/// probes the fake's endpoint successfully, and enters
/// `poll_until_ready(None)` — which has no child-exit detection. This lets the
/// startup timeout naturally into `NotReady` / `WaitingForReadiness` rather
/// than short-circuiting into `SpawnFailed`.
///
/// For `VersionMismatch` scripts the startup catches the error before any
/// timeout, so no PID file is needed (and writing one would cause the respawn
/// path to attempt to kill the test process).
async fn spawn_shim_with_fake_health(
    workspace: &Path,
    script: HealthScript,
    ready_timeout_ms: u64,
) -> (tokio::process::Child, FakeHealthResponder, String) {
    let endpoint =
        engram::daemon::ipc_server::ipc_endpoint(workspace).expect("derive daemon endpoint");
    let fake = FakeHealthResponder::spawn(&endpoint, script.clone());
    // Brief pause to let the fake bind the listener.
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Plant a PID file for non-VersionMismatch scripts so
    // ensure_daemon_running skips spawn_daemon and enters
    // poll_until_ready(None) (no child-exit tracking).
    let needs_pid_file = !matches!(script, HealthScript::VersionMismatch { .. });
    if needs_pid_file {
        let pid_dir = workspace.join(".engram").join("run");
        fs::create_dir_all(&pid_dir).expect("create .engram/run");
        let pid_json = format!(r#"{{"pid":{},"start_time_unix":1}}"#, std::process::id());
        fs::write(pid_dir.join("engram.pid"), &pid_json).expect("write fake PID file");
    }

    let current_test_exe = std::env::current_exe().expect("resolve current test executable");
    let child = tokio::process::Command::new(env!("CARGO_BIN_EXE_engram"))
        .args(["shim", "--workspace"])
        .arg(workspace)
        .env_remove("ENGRAM_DATA_DIR")
        .env("CARGO_BIN_EXE_engram", current_test_exe)
        .env("ENGRAM_READY_TIMEOUT_MS", ready_timeout_ms.to_string())
        .env("ENGRAM_TEST_FAKE_SECRET", FAKE_SECRET_MARKER)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .expect("spawn engram shim with fake health");
    (child, fake, endpoint)
}

async fn spawn_delayed_daemon(workspace: &Path, delay_ms: u64) -> (tokio::process::Child, String) {
    let endpoint =
        engram::daemon::ipc_server::ipc_endpoint(workspace).expect("derive daemon endpoint");
    let mut child = tokio::process::Command::new(env!("CARGO_BIN_EXE_engram"))
        .args(["daemon", "--workspace"])
        .arg(workspace)
        .env_remove("ENGRAM_DATA_DIR")
        .env("ENGRAM_TEST_STARTUP_DELAY_MS", delay_ms.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .expect("spawn delayed daemon");

    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    loop {
        if probe(&endpoint, Duration::from_millis(100)).await.is_ok() {
            return (child, endpoint);
        }
        assert!(
            child.try_wait().expect("inspect delayed daemon").is_none(),
            "delayed daemon exited before binding its endpoint"
        );
        assert!(
            tokio::time::Instant::now() < deadline,
            "delayed daemon did not bind within 10s"
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

async fn wait_for_daemon_ready(endpoint: &str) {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    while !check_health(endpoint).await {
        assert!(
            tokio::time::Instant::now() < deadline,
            "delayed daemon did not become ready within 30s"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

async fn shutdown_daemon(child: &mut tokio::process::Child, endpoint: &str) {
    let request = IpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: Some(Value::from(9001)),
        method: "_shutdown".to_owned(),
        params: None,
    };
    let response = send_request(endpoint, &request, Duration::from_secs(2))
        .await
        .expect("request delayed daemon shutdown");
    assert!(
        response.error.is_none(),
        "delayed daemon rejected shutdown: {response:?}"
    );
    let status = tokio::time::timeout(Duration::from_secs(5), child.wait())
        .await
        .expect("delayed daemon must exit within 5s")
        .expect("wait for delayed daemon");
    assert!(status.success(), "delayed daemon exited unsuccessfully");
}

/// Scenarios (a), (b), (c): `initialize` completes, `tools/list` returns the
/// static catalog, and `tools/call` fails with a structured error — all while
/// the shim's own spawned daemon has already failed. Also asserts the
/// documented distinct exit code, an attributable stderr line, and the
/// absence of sensitive fields in the durable startup-failure record.
#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn shim_serves_initialize_and_tools_list_then_degrades_tool_calls_on_daemon_failure() {
    let workspace = workspace_with_valid_git_root();
    let mut child = spawn_shim_with_failing_daemon(workspace.path());
    let mut stdin = child.stdin.take().expect("capture shim stdin");
    let stdout = child.stdout.take().expect("capture shim stdout");
    let mut stdout = BufReader::new(stdout);

    // ── Scenario (a): initialize completes despite failed daemon readiness ──
    let started = Instant::now();
    stdin
        .write_all(
            br#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"stdio-initialize-contract","version":"1.0"}}}
"#,
        )
        .await
        .expect("write MCP initialize frame");
    let initialize_line = read_bounded_mcp_line(
        &mut stdout,
        Duration::from_secs(20),
        "MCP initialize response",
    )
    .await;
    let initialize_elapsed = started.elapsed();
    assert!(
        initialize_elapsed < Duration::from_secs(20),
        "initialize must complete without waiting out daemon readiness; took {initialize_elapsed:?}"
    );
    let initialize_response: Value = serde_json::from_str(initialize_line.trim())
        .expect("stdout must contain only an MCP initialize frame");
    assert_eq!(initialize_response["jsonrpc"], "2.0");
    assert_eq!(initialize_response["id"], 1);
    assert!(
        initialize_response.get("error").is_none(),
        "initialize must succeed even when daemon readiness has already failed: {initialize_response}"
    );
    assert_eq!(
        initialize_response["result"]["serverInfo"]["name"], "engram-shim",
        "initialize must identify the Engram shim server"
    );

    // ── Scenario (b): tools/list still returns the static catalog ──────────
    stdin
        .write_all(
            br#"{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}
{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}
"#,
        )
        .await
        .expect("complete MCP initialization and request tools/list");
    let tools_line =
        read_bounded_mcp_line(&mut stdout, Duration::from_secs(10), "tools/list response").await;
    let tools_response: Value =
        serde_json::from_str(tools_line.trim()).expect("tools/list stdout must be one MCP frame");
    let tool_names: std::collections::BTreeSet<String> = tools_response["result"]["tools"]
        .as_array()
        .expect("tools/list must return an array in a degraded session")
        .iter()
        .map(|tool| tool["name"].as_str().expect("tool name").to_owned())
        .collect();
    let expected_tool_names: std::collections::BTreeSet<String> =
        engram::shim::tools_catalog::all_tools()
            .into_iter()
            .map(|tool| tool.name.to_string())
            .collect();
    assert_eq!(
        tool_names, expected_tool_names,
        "tools/list must expose the full Engram catalog even in a degraded session"
    );

    // ── Scenario (c): tools/call fails with a structured, attributable error ──
    stdin
        .write_all(
            br#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"get_workspace_status","arguments":{}}}
"#,
        )
        .await
        .expect("request get_workspace_status");
    let call_line =
        read_bounded_mcp_line(&mut stdout, Duration::from_secs(20), "tools/call response").await;
    let call_response: Value =
        serde_json::from_str(call_line.trim()).expect("tools/call stdout must be one MCP frame");
    assert_eq!(call_response["id"], 3);
    // A degraded startup precondition is a tool-level failure (the request
    // was valid and routed, but the shim cannot execute it), so it is
    // reported as `result.isError == true` with structured content — NOT a
    // protocol-level JSON-RPC `error`, which MCP clients typically render
    // opaquely without surfacing the message (`rmcp::model::CallToolResult`
    // "When to use this vs `Err(ErrorData)`" doc comment).
    assert!(
        call_response.get("error").is_none(),
        "a degraded tools/call must be a successful JSON-RPC response carrying \
         result.isError=true, not a protocol-level error: {call_response}"
    );
    assert_eq!(
        call_response["result"]["isError"], true,
        "no tools/call may succeed while the session is degraded: {call_response}"
    );
    let structured = &call_response["result"]["structuredContent"];
    assert_eq!(
        structured["failure_class"], "readiness_timeout",
        "degraded tools/call structured content must carry the failure_class: {call_response}"
    );
    assert_eq!(
        structured["recoverable"], false,
        "a daemon process that exited during startup must remain terminal: {call_response}"
    );
    let content_text = call_response["result"]["content"][0]["text"]
        .as_str()
        .expect("degraded tools/call must carry visible content text");
    assert!(
        content_text.contains("readiness_timeout"),
        "degraded tools/call content must name the startup failure cause: {content_text}"
    );

    // ── Close the session and assert the documented exit-code taxonomy ─────
    stdin.shutdown().await.expect("close MCP stdin");
    drop(stdin);
    let mut stderr_bytes = Vec::new();
    {
        use tokio::io::AsyncReadExt as _;
        let mut stderr = child.stderr.take().expect("capture shim stderr");
        tokio::time::timeout(
            Duration::from_secs(5),
            stderr.read_to_end(&mut stderr_bytes),
        )
        .await
        .expect("drain shim stderr within 5s")
        .expect("read shim stderr");
    }
    let exit_status = tokio::time::timeout(Duration::from_secs(5), child.wait())
        .await
        .expect("shim must exit within 5s of stdin closing")
        .expect("wait for shim");
    assert!(
        !exit_status.success(),
        "shim must exit non-zero when the session was ever degraded"
    );
    assert_eq!(
        exit_status.code(),
        Some(11),
        "shim must exit with the documented readiness-timeout exit code (11)"
    );
    let stderr_text = String::from_utf8_lossy(&stderr_bytes);
    assert!(
        stderr_text.contains("readiness_timeout"),
        "shim must write an attributable stderr line naming the failure class: {stderr_text}"
    );
    assert!(
        !stderr_text.contains(FAKE_SECRET_MARKER),
        "stderr must never contain environment variable values: {stderr_text}"
    );

    // ── Absence of sensitive fields in the durable startup-failure record ──
    let record_path = workspace
        .path()
        .join(".engram")
        .join("diagnostics")
        .join("shim-startup-failures.jsonl");
    let record_contents = fs::read_to_string(&record_path).unwrap_or_else(|error| {
        panic!("durable startup-failure record must exist at {record_path:?}: {error}")
    });
    let mut saw_record = false;
    for line in record_contents
        .lines()
        .filter(|line| !line.trim().is_empty())
    {
        saw_record = true;
        let record: Value = serde_json::from_str(line).expect("record line must be valid JSON");
        let object = record.as_object().expect("record must be a JSON object");
        let mut keys: Vec<&String> = object.keys().collect();
        keys.sort();
        assert_eq!(
            keys,
            vec!["binary_version", "failure_class", "message", "timestamp"],
            "startup-failure record must contain exactly the documented fields: {record}"
        );
        assert_eq!(record["failure_class"], "readiness_timeout");
        assert!(
            record["timestamp"].as_str().is_some(),
            "record must carry a timestamp: {record}"
        );
        let serialized = record.to_string();
        for forbidden in ["token", "credential", "password", "secret", "Bearer "] {
            assert!(
                !serialized
                    .to_ascii_lowercase()
                    .contains(&forbidden.to_ascii_lowercase()),
                "startup-failure record must not contain '{forbidden}': {record}"
            );
        }
        assert!(
            !serialized.contains(FAKE_SECRET_MARKER),
            "startup-failure record must never contain environment variable values: {record}"
        );
        // This failure originates from `ensure_daemon_running` (readiness
        // timeout), whose message reports the spawned child's exit status —
        // it never names a filesystem path at all, in-workspace or
        // otherwise. Assert that directly rather than with a brittle
        // path-token heuristic that misclassifies ordinary punctuation
        // (e.g. "status exit code: 101") as a path.
        let message = record["message"]
            .as_str()
            .expect("record message must be a string");
        assert!(
            !message.contains(":\\") && !message.contains(":/"),
            "readiness-timeout startup-failure message must not reference any filesystem path: {message}"
        );
    }
    assert!(
        saw_record,
        "durable startup-failure record must contain at least one entry"
    );
}

/// A readiness timeout is an attribution deadline, not a permanent session
/// failure. The same long-lived stdio shim must recover when the workspace
/// daemon later becomes ready at the named-pipe endpoint.
#[tokio::test]
async fn shim_recovers_after_timed_out_daemon_later_becomes_ready() {
    let workspace = workspace_with_valid_git_root();
    let (mut daemon, endpoint) = spawn_delayed_daemon(workspace.path(), 1000).await;
    let mut child = spawn_shim_with_readiness_budget(workspace.path(), 1);
    let mut stdin = child.stdin.take().expect("capture shim stdin");
    let stdout = child.stdout.take().expect("capture shim stdout");
    let mut stdout = BufReader::new(stdout);

    stdin
        .write_all(
            br#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"late-readiness-recovery-contract","version":"1.0"}}}
{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"get_workspace_status","arguments":{}}}
"#,
        )
        .await
        .expect("initialize shim and request status before daemon readiness");

    let initialize_line = read_bounded_mcp_line(
        &mut stdout,
        Duration::from_secs(20),
        "MCP initialize response",
    )
    .await;
    let initialize_response: Value = serde_json::from_str(initialize_line.trim())
        .expect("initialize stdout must contain one MCP frame");
    assert!(
        initialize_response.get("error").is_none(),
        "initialize must succeed before daemon readiness: {initialize_response}"
    );

    let degraded_line = read_bounded_mcp_line(
        &mut stdout,
        Duration::from_secs(20),
        "initial degraded tools/call response",
    )
    .await;
    let degraded_response: Value = serde_json::from_str(degraded_line.trim())
        .expect("degraded tools/call stdout must contain one MCP frame");
    assert_eq!(
        degraded_response["result"]["structuredContent"]["failure_class"], "readiness_timeout",
        "the first call must observe the intentional readiness timeout: {degraded_response}"
    );
    assert_eq!(
        degraded_response["result"]["structuredContent"]["recoverable"], true,
        "late daemon readiness must be explicitly retryable for MCP agents: {degraded_response}"
    );
    assert!(
        degraded_response["result"]["structuredContent"]["retry_after_ms"]
            .as_u64()
            .is_some_and(|retry_after_ms| retry_after_ms > 0),
        "a recoverable error must tell agents when to retry: {degraded_response}"
    );

    wait_for_daemon_ready(&endpoint).await;

    stdin
        .write_all(
            br#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"get_workspace_status","arguments":{}}}
"#,
        )
        .await
        .expect("request status through the same shim after daemon readiness");
    let recovered_line = read_bounded_mcp_line(
        &mut stdout,
        Duration::from_secs(10),
        "recovered tools/call response",
    )
    .await;
    let recovered_response: Value = serde_json::from_str(recovered_line.trim())
        .expect("recovered tools/call stdout must contain one MCP frame");
    assert!(
        recovered_response.get("error").is_none(),
        "the recovered request must not return a JSON-RPC error: {recovered_response}"
    );
    assert_ne!(
        recovered_response["result"]["isError"], true,
        "the same shim must forward calls after late daemon readiness: {recovered_response}"
    );

    shutdown_daemon(&mut daemon, &endpoint).await;
    stdin.shutdown().await.expect("close MCP stdin");
    drop(stdin);
    let exit_status = tokio::time::timeout(Duration::from_secs(5), child.wait())
        .await
        .expect("recovered shim must exit within 5s of stdin closing")
        .expect("wait for recovered shim");
    assert!(
        exit_status.success(),
        "a shim that recovered before session end must exit successfully: {exit_status}"
    );
}

/// A client disconnect must cancel unresolved startup work instead of keeping
/// the stdio process alive for the daemon's full readiness budget.
#[tokio::test]
async fn shim_aborts_unresolved_startup_after_client_disconnects() {
    let workspace = workspace_with_valid_git_root();
    let (mut daemon, endpoint) = spawn_delayed_daemon(workspace.path(), 5000).await;
    let mut child = spawn_shim_with_readiness_budget(workspace.path(), 30_000);
    let mut stdin = child.stdin.take().expect("capture shim stdin");
    let stdout = child.stdout.take().expect("capture shim stdout");
    let mut stdout = BufReader::new(stdout);

    stdin
        .write_all(
            br#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"startup-cancellation-contract","version":"1.0"}}}
"#,
        )
        .await
        .expect("initialize shim while daemon startup remains unresolved");
    let initialize_line = read_bounded_mcp_line(
        &mut stdout,
        Duration::from_secs(20),
        "MCP initialize response",
    )
    .await;
    let initialize_response: Value = serde_json::from_str(initialize_line.trim())
        .expect("initialize stdout must contain one MCP frame");
    assert!(
        initialize_response.get("error").is_none(),
        "initialize must not wait for daemon readiness: {initialize_response}"
    );

    let disconnected_at = Instant::now();
    stdin.shutdown().await.expect("close MCP stdin");
    drop(stdin);
    let exit_status = tokio::time::timeout(Duration::from_secs(5), child.wait())
        .await
        .expect("shim must cancel unresolved startup within 5s")
        .expect("wait for cancelled shim");
    shutdown_daemon(&mut daemon, &endpoint).await;

    assert!(
        disconnected_at.elapsed() < Duration::from_secs(5),
        "shim waited too long after client disconnect"
    );
    assert_eq!(
        exit_status.code(),
        Some(13),
        "unresolved startup teardown must report transport_failure"
    );
}

/// When workspace admission itself fails (no `.git` at all), the session
/// still serves `initialize`/`tools/list`, and the final exit code reflects
/// the admission-failure classification rather than the readiness-timeout
/// classification used above.
#[tokio::test]
async fn shim_degrades_tools_call_and_exits_with_admission_failure_code_for_invalid_workspace() {
    let workspace = TempDir::new().expect("invalid workspace tempdir (no .git)");
    let current_test_exe = std::env::current_exe().expect("resolve current test executable");
    let mut child = tokio::process::Command::new(env!("CARGO_BIN_EXE_engram"))
        .args(["shim", "--workspace"])
        .arg(workspace.path())
        .env_remove("ENGRAM_DATA_DIR")
        .env("CARGO_BIN_EXE_engram", current_test_exe)
        .env("ENGRAM_READY_TIMEOUT_MS", "30000")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .expect("spawn engram shim");
    let mut stdin = child.stdin.take().expect("capture shim stdin");
    let stdout = child.stdout.take().expect("capture shim stdout");
    let mut stdout = BufReader::new(stdout);

    stdin
        .write_all(
            br#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"stdio-initialize-contract","version":"1.0"}}}
"#,
        )
        .await
        .expect("write MCP initialize frame");
    let initialize_line = read_bounded_mcp_line(
        &mut stdout,
        Duration::from_secs(20),
        "MCP initialize response",
    )
    .await;
    let initialize_response: Value = serde_json::from_str(initialize_line.trim())
        .expect("stdout must contain only an MCP initialize frame");
    assert!(
        initialize_response.get("error").is_none(),
        "initialize must succeed even when workspace admission has already failed: {initialize_response}"
    );

    stdin
        .write_all(
            br#"{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"get_workspace_status","arguments":{}}}
"#,
        )
        .await
        .expect("complete MCP initialization and request tools/call");
    let call_line =
        read_bounded_mcp_line(&mut stdout, Duration::from_secs(20), "tools/call response").await;
    let call_response: Value =
        serde_json::from_str(call_line.trim()).expect("tools/call stdout must be one MCP frame");
    assert!(
        call_response.get("error").is_none(),
        "a degraded tools/call must be a successful JSON-RPC response carrying \
         result.isError=true, not a protocol-level error: {call_response}"
    );
    assert_eq!(
        call_response["result"]["isError"], true,
        "no tools/call may succeed for an inadmissible workspace: {call_response}"
    );
    assert_eq!(
        call_response["result"]["structuredContent"]["failure_class"], "admission_failure",
        "degraded tools/call structured content must name admission_failure: {call_response}"
    );

    stdin.shutdown().await.expect("close MCP stdin");
    drop(stdin);
    let exit_status = tokio::time::timeout(Duration::from_secs(5), child.wait())
        .await
        .expect("shim must exit within 5s of stdin closing")
        .expect("wait for shim");
    assert!(
        !exit_status.success(),
        "shim must exit non-zero for an admission failure"
    );
    assert_eq!(
        exit_status.code(),
        Some(10),
        "shim must exit with the documented admission-failure exit code (10)"
    );
}

/// Documents the shim's stdio initialize contract at the type level: the
/// path referenced above must exist so the record location is discoverable
/// without re-deriving it from prose.
#[test]
fn startup_failure_record_relative_path_is_documented() {
    let expected: PathBuf = [".engram", "diagnostics", "shim-startup-failures.jsonl"]
        .iter()
        .collect();
    assert_eq!(
        expected,
        Path::new(".engram/diagnostics/shim-startup-failures.jsonl"),
        "durable startup-failure record path convention must stay stable for docs/troubleshooting.md"
    );
}

// ── 138-F Terminal Classification Matrix (T1–T4) ─────────────────────────────
//
// NEW-RED: These tests scaffold the plan's terminal classification scenarios.
// They assert `protocol_incompatible` / 15005 / recoverable==false which does
// not exist yet (current code returns readiness_timeout / 15002). When behavior
// tasks 138.001-T and 138.004-T land, the shim will classify protocol-level
// incompatibility as terminal and these tests will turn GREEN.
//
// The setup currently uses `spawn_shim_with_failing_daemon` as the simplest
// harness that gets the shim into a degraded state. When 138.004-T lands, the
// setup will be updated to use the 138.002-T H1 `FakeHealthResponder` with
// scenario-specific scripts (VersionMismatch, JsonRpcError -32601, etc.) so
// the shim can exercise the terminal classification path end-to-end.

/// Shared assertion for T1–T4 terminal classification contract tests.
///
/// Asserts: `failure_class == "protocol_incompatible"`, `engram_code == 15005`,
/// `recoverable == false`, no `retry_after_ms`, content text contains
/// "protocol_incompatible".
fn assert_terminal_protocol_incompatible(call_response: &Value, context: &str) {
    assert!(
        call_response.get("error").is_none(),
        "{context}: terminal tools/call must be a successful JSON-RPC response \
         carrying result.isError=true: {call_response}"
    );
    assert_eq!(
        call_response["result"]["isError"], true,
        "{context}: terminal tools/call must have isError=true: {call_response}"
    );
    let structured = &call_response["result"]["structuredContent"];
    assert_eq!(
        structured["failure_class"], "protocol_incompatible",
        "{context}: terminal failure_class must be protocol_incompatible: {call_response}"
    );
    assert_eq!(
        structured["engram_code"], 15005,
        "{context}: terminal engram_code must be 15005: {call_response}"
    );
    assert_eq!(
        structured["recoverable"], false,
        "{context}: terminal must be non-recoverable: {call_response}"
    );
    assert!(
        structured.get("retry_after_ms").is_none(),
        "{context}: terminal must NOT carry a retry_after_ms key at all (agents branch on key \
         presence, not truthiness — an explicit null would still be wrong): {call_response}"
    );
    let content_text = call_response["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or("");
    assert!(
        content_text.contains("protocol_incompatible"),
        "{context}: content text must name protocol_incompatible: {content_text}"
    );
}

/// Helper: initialize the shim over MCP stdio and return stdin/stdout handles
/// ready for tools/call requests.
async fn initialize_shim_mcp(
    child: &mut tokio::process::Child,
) -> (
    tokio::process::ChildStdin,
    BufReader<tokio::process::ChildStdout>,
) {
    let mut stdin = child.stdin.take().expect("capture shim stdin");
    let stdout = child.stdout.take().expect("capture shim stdout");
    let mut stdout = BufReader::new(stdout);

    stdin
        .write_all(
            br#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"t-matrix-contract","version":"1.0"}}}
"#,
        )
        .await
        .expect("write MCP initialize");
    let _ =
        read_bounded_mcp_line(&mut stdout, Duration::from_secs(20), "initialize response").await;
    (stdin, stdout)
}

/// Helper: send tools/call and parse the JSON response.
async fn send_tools_call(
    stdin: &mut tokio::process::ChildStdin,
    stdout: &mut BufReader<tokio::process::ChildStdout>,
    id: u64,
) -> Value {
    let frame = format!(
        r#"{{"jsonrpc":"2.0","id":{id},"method":"tools/call","params":{{"name":"get_workspace_status","arguments":{{}}}}}}"#,
    );
    stdin
        .write_all(format!("{frame}\n").as_bytes())
        .await
        .expect("write tools/call");
    let line = read_bounded_mcp_line(stdout, Duration::from_secs(30), "tools/call response").await;
    serde_json::from_str(line.trim()).expect("parse tools/call response")
}

/// T1 — daemon replies with wrong `protocol_version` (138.008-T).
///
/// When the daemon's `_health` response carries a `protocol_version` that
/// differs from `ENGRAM_PROTOCOL_VERSION`, the shim must classify this as
/// terminal protocol incompatibility and never retry.
#[tokio::test]
async fn t1_wrong_protocol_version_is_terminal() {
    let workspace = workspace_with_valid_git_root();
    // Sequence: first 5 probes return NotReady (startup uses ~3), then switch
    // to VersionMismatch for the recovery probe.
    let (mut child, fake, _ep) = spawn_shim_with_fake_health(
        workspace.path(),
        HealthScript::Sequence {
            initial: Box::new(HealthScript::NotReady {
                status: "starting".into(),
            }),
            switch_after: 5,
            then: Box::new(HealthScript::VersionMismatch { version: 999 }),
        },
        200,
    )
    .await;
    let (mut stdin, mut stdout) = initialize_shim_mcp(&mut child).await;

    // Wait for the monitor/forwarding probe to fire and latch terminal.
    tokio::time::sleep(Duration::from_millis(800)).await;

    // First tools/call: should get terminal protocol_incompatible
    let resp = send_tools_call(&mut stdin, &mut stdout, 10).await;
    assert_terminal_protocol_incompatible(&resp, "T1 first call");

    // Three further tools/call: identical terminal payload, 0 additional probes
    let count_before_followups = fake.count();
    for id in 11..=13 {
        let resp = send_tools_call(&mut stdin, &mut stdout, id).await;
        assert_terminal_protocol_incompatible(&resp, &format!("T1 followup call {id}"));
    }
    assert_eq!(
        fake.count(),
        count_before_followups,
        "T1: terminal-latch followup tools/call must issue zero additional _health probes"
    );
}

/// T2 — daemon returns JSON-RPC error -32601 Method Not Found (138.008-T).
///
/// A `-32601` response to `_health` proves the daemon does not implement the
/// health protocol at all. The shim must classify this as terminal.
///
/// NEW-RED: currently the shim returns `readiness_timeout`, not
/// `protocol_incompatible`.
#[tokio::test]
async fn t2_jsonrpc_method_not_found_is_terminal() {
    let workspace = workspace_with_valid_git_root();
    let (mut child, fake, _ep) = spawn_shim_with_fake_health(
        workspace.path(),
        HealthScript::JsonRpcError {
            code: -32601,
            message: "Method not found".into(),
        },
        500,
    )
    .await;
    let (mut stdin, mut stdout) = initialize_shim_mcp(&mut child).await;

    let resp = send_tools_call(&mut stdin, &mut stdout, 10).await;
    assert_terminal_protocol_incompatible(&resp, "T2 first call");

    let count_before_followups = fake.count();
    for id in 11..=13 {
        let resp = send_tools_call(&mut stdin, &mut stdout, id).await;
        assert_terminal_protocol_incompatible(&resp, &format!("T2 followup call {id}"));
    }
    assert_eq!(
        fake.count(),
        count_before_followups,
        "T2: terminal-latch followup tools/call must issue zero additional _health probes"
    );
}

/// T3 — daemon returns valid JSON-RPC with missing `result` (138.008-T).
///
/// A response with no `result` key and no `error` key is a protocol violation.
/// The shim must classify this as terminal protocol incompatibility.
///
/// NEW-RED: currently the shim returns `readiness_timeout`, not
/// `protocol_incompatible`.
#[tokio::test]
async fn t3_missing_result_is_terminal() {
    let workspace = workspace_with_valid_git_root();
    let (mut child, fake, _ep) =
        spawn_shim_with_fake_health(workspace.path(), HealthScript::MissingResult, 500).await;
    let (mut stdin, mut stdout) = initialize_shim_mcp(&mut child).await;

    let resp = send_tools_call(&mut stdin, &mut stdout, 10).await;
    assert_terminal_protocol_incompatible(&resp, "T3 first call");

    let count_before_followups = fake.count();
    for id in 11..=13 {
        let resp = send_tools_call(&mut stdin, &mut stdout, id).await;
        assert_terminal_protocol_incompatible(&resp, &format!("T3 followup call {id}"));
    }
    assert_eq!(
        fake.count(),
        count_before_followups,
        "T3: terminal-latch followup tools/call must issue zero additional _health probes"
    );
}

/// T4 — daemon returns valid JSON-RPC with undecodable `result` (138.008-T).
///
/// A response where `result` is present but cannot be deserialized as
/// `HealthCheckResult` indicates a structurally incompatible daemon. The shim
/// must classify this as terminal.
///
/// NEW-RED: currently the shim returns `readiness_timeout`, not
/// `protocol_incompatible`.
#[tokio::test]
async fn t4_undecodable_result_is_terminal() {
    let workspace = workspace_with_valid_git_root();
    let (mut child, fake, _ep) =
        spawn_shim_with_fake_health(workspace.path(), HealthScript::UndecodableResult, 500).await;
    let (mut stdin, mut stdout) = initialize_shim_mcp(&mut child).await;

    let resp = send_tools_call(&mut stdin, &mut stdout, 10).await;
    assert_terminal_protocol_incompatible(&resp, "T4 first call");

    let count_before_followups = fake.count();
    for id in 11..=13 {
        let resp = send_tools_call(&mut stdin, &mut stdout, id).await;
        assert_terminal_protocol_incompatible(&resp, &format!("T4 followup call {id}"));
    }
    assert_eq!(
        fake.count(),
        count_before_followups,
        "T4: terminal-latch followup tools/call must issue zero additional _health probes"
    );
}

// ── 138-F Terminal Side-Effects (T5, T7) ─────────────────────────────────────
//
// NEW-RED: Test durable startup-failure record and message hygiene for terminal
// protocol incompatibility. The monitor currently writes only readiness_timeout
// records. When behavior tasks land (138.005-T), the monitor will write
// protocol_incompatible records and sanitize daemon-supplied free-form text.

/// T5 — durable protocol_incompatible record written by monitor (138.009-T).
///
/// After a terminal classification: exactly ONE additional
/// `protocol_incompatible` startup-failure record appears in the diagnostic
/// JSONL file beyond the pre-existing `readiness_timeout` record. The new
/// record carries `failure_class == "protocol_incompatible"`, the fixed
/// `record_message()` string, and no filesystem path or environment variable.
///
/// NEW-RED: the monitor currently does not write protocol_incompatible records.
#[tokio::test]
async fn t5_durable_protocol_incompatible_record() {
    let workspace = workspace_with_valid_git_root();
    let (mut child, _fake, _ep) = spawn_shim_with_fake_health(
        workspace.path(),
        HealthScript::Sequence {
            initial: Box::new(HealthScript::NotReady {
                status: "starting".into(),
            }),
            switch_after: 8,
            then: Box::new(HealthScript::VersionMismatch { version: 999 }),
        },
        200,
    )
    .await;
    let (stdin, _stdout) = initialize_shim_mcp(&mut child).await;

    // Wait for the monitor to latch terminal and write the record.
    tokio::time::sleep(Duration::from_millis(1200)).await;

    // Let the shim settle into its degraded state and write its records.
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Close stdin so the shim sees EOF and exits.
    drop(stdin);
    let status = tokio::time::timeout(Duration::from_secs(15), child.wait())
        .await
        .expect("shim must exit within timeout")
        .expect("wait for shim");
    let _ = status;

    // Read the durable startup-failure record file.
    let record_path = workspace
        .path()
        .join(".engram")
        .join("diagnostics")
        .join("shim-startup-failures.jsonl");
    let content = fs::read_to_string(&record_path).unwrap_or_default();
    let lines: Vec<&str> = content.lines().collect();
    let all_records: Vec<Value> = lines
        .iter()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .collect();

    // Baseline: the pre-existing readiness_timeout record (written at
    // src/shim/mod.rs:218-221 when the initial readiness deadline expires,
    // before the monitor is ever spawned) must be present and unaffected.
    let readiness_timeout_records: Vec<&Value> = all_records
        .iter()
        .filter(|r| r["failure_class"] == "readiness_timeout")
        .collect();
    assert_eq!(
        readiness_timeout_records.len(),
        1,
        "T5: the pre-existing readiness_timeout record must be present exactly once, \
         byte-unchanged by this feature; found {}: {content}",
        readiness_timeout_records.len()
    );

    // Assert at least one protocol_incompatible record exists.
    let protocol_records: Vec<&Value> = all_records
        .iter()
        .filter(|r| r["failure_class"] == "protocol_incompatible")
        .collect();

    assert!(
        !protocol_records.is_empty(),
        "T5: must have at least one protocol_incompatible record; \
         found only: {content}"
    );

    // Exact schema and content contract for every protocol_incompatible
    // record: the fixed four-field key set, the fixed record_message()
    // string, and no filesystem path, workspace path, or environment value.
    let ws_path = workspace.path().to_str().unwrap_or("");
    for record in &protocol_records {
        let record_str = record.to_string();
        let obj = record.as_object().unwrap_or_else(|| {
            panic!("T5: protocol_incompatible record must be a JSON object: {record_str}")
        });
        let mut keys: Vec<&str> = obj.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            vec!["binary_version", "failure_class", "message", "timestamp"],
            "T5: protocol_incompatible record must have exactly the documented four-field \
             schema, no more and no fewer: {record_str}"
        );
        assert_eq!(
            record["message"], "daemon protocol or _health contract is incompatible with this shim",
            "T5: protocol_incompatible record message must be the fixed, variable-free \
             ShimFailureClass::ProtocolIncompatible::record_message() string: {record_str}"
        );
        assert!(
            !record_str.contains(ws_path),
            "T5: protocol_incompatible record must not contain workspace path: {record_str}"
        );
        assert!(
            !record_str.contains(FAKE_SECRET_MARKER),
            "T5: protocol_incompatible record must not contain any environment value: {record_str}"
        );
    }

    // Exactly ONE protocol_incompatible record, not more.
    assert_eq!(
        protocol_records.len(),
        1,
        "T5: must have exactly 1 protocol_incompatible record; found {}",
        protocol_records.len()
    );
}

/// T7 — message hygiene: daemon-supplied text must not leak (138.009-T).
///
/// Two cases: (a) `_health` JSON-RPC error message embedding a path and env
/// value; (b) undecodable result payload embedding a path via a serde "invalid
/// type" error (which echoes the received value verbatim in its message).
/// Neither the path nor the environment variable must appear in the tools/call
/// response content or structuredContent.
#[tokio::test]
async fn t7a_daemon_text_does_not_leak_into_responses_jsonrpc_error() {
    let workspace = workspace_with_valid_git_root();
    let ws_path_str = workspace.path().to_str().unwrap_or("").to_owned();
    // Embed the workspace path AND the fake secret in the error message so we
    // can verify neither leaks into the client-visible response.
    let poisoned_message = format!("internal error at {ws_path_str} env={FAKE_SECRET_MARKER}");
    let (mut child, _fake, _ep) = spawn_shim_with_fake_health(
        workspace.path(),
        HealthScript::JsonRpcError {
            code: -32601,
            message: poisoned_message,
        },
        500,
    )
    .await;
    let (mut stdin, mut stdout) = initialize_shim_mcp(&mut child).await;

    let resp = send_tools_call(&mut stdin, &mut stdout, 10).await;

    assert_terminal_protocol_incompatible(&resp, "T7a");

    // Hygiene: the workspace path must not appear in the response.
    let resp_str = resp.to_string();
    assert!(
        !resp_str.contains(&ws_path_str),
        "T7a: workspace path must not appear in response: {resp_str}"
    );

    // Hygiene: FAKE_SECRET_MARKER must not appear in the response.
    assert!(
        !resp_str.contains(FAKE_SECRET_MARKER),
        "T7a: sensitive env marker must not appear in response: {resp_str}"
    );
}

/// T7(b) — the second daemon-controlled text source: an undecodable `result`
/// payload whose wrong-typed `protocol_version` field is a poisoned string
/// (workspace path + fake secret). Serde's "invalid type" error text echoes
/// the received value verbatim (`src/shim/lifecycle.rs`'s
/// `format!("invalid _health payload: {e}")`), so this proves the decode-
/// failure path is sanitized identically to the JSON-RPC error-message path
/// covered by T7(a) — a hygiene test that exercised only (a) would leave (b)
/// unguarded (Copilot review finding on PR #366).
#[tokio::test]
async fn t7b_daemon_text_does_not_leak_into_responses_undecodable_payload() {
    let workspace = workspace_with_valid_git_root();
    let ws_path_str = workspace.path().to_str().unwrap_or("").to_owned();
    let poisoned = format!("{ws_path_str}/env={FAKE_SECRET_MARKER}");
    let (mut child, _fake, _ep) = spawn_shim_with_fake_health(
        workspace.path(),
        HealthScript::UndecodableResultWithPoisonedText(poisoned.clone()),
        500,
    )
    .await;
    let (mut stdin, mut stdout) = initialize_shim_mcp(&mut child).await;

    let resp = send_tools_call(&mut stdin, &mut stdout, 10).await;

    assert_terminal_protocol_incompatible(&resp, "T7b");

    let resp_str = resp.to_string();
    assert!(
        !resp_str.contains(&ws_path_str),
        "T7b: workspace path must not appear in response: {resp_str}"
    );
    assert!(
        !resp_str.contains(FAKE_SECRET_MARKER),
        "T7b: sensitive env marker must not appear in response: {resp_str}"
    );
    assert!(
        !resp_str.contains(&poisoned),
        "T7b: the poisoned serde-error-echoed value must not appear in response: {resp_str}"
    );
}

// ── 138-F Transient Over-Terminalization Guards (R1, R2, R4, R5) ─────────────
//
// NEW-RED: These are the PRIMARY guards for the dominant risk:
// over-terminalization is worse than under-classification. Each asserts that a
// specific transient error scenario produces `recoverable == true` and
// `retry_after_ms == 250` (the WaitingForReadiness late-recovery behavior).
//
// Currently RED because the test setup uses `spawn_shim_with_failing_daemon`
// which produces a `Degraded` (non-recoverable) response rather than the
// `WaitingForReadiness` (recoverable) response. When behavior tasks land and
// the FakeHealthResponder is properly wired, these will use scenario-specific
// scripts and the shim will enter WaitingForReadiness → recoverable → GREEN.

/// Shared assertion for R1/R2/R4/R5: response must be transient/recoverable.
fn assert_transient_recoverable(call_response: &Value, context: &str) {
    assert!(
        call_response.get("error").is_none(),
        "{context}: transient tools/call must be a successful JSON-RPC response: {call_response}"
    );
    assert_eq!(
        call_response["result"]["isError"], true,
        "{context}: transient tools/call must have isError=true: {call_response}"
    );
    let structured = &call_response["result"]["structuredContent"];
    assert_eq!(
        structured["failure_class"], "readiness_timeout",
        "{context}: transient failure_class must be readiness_timeout: {call_response}"
    );
    assert_eq!(
        structured["recoverable"], true,
        "{context}: transient errors MUST be recoverable: {call_response}"
    );
    assert_eq!(
        structured["retry_after_ms"], 250,
        "{context}: retry_after_ms must be 250 (RECOVERY_PROBE_COOLDOWN): {call_response}"
    );
}

/// R1 — version-compatible `{status: "starting"}` → transient (138.010-T).
///
/// A daemon that replies with the correct protocol version but is not yet
/// ready (status != "ready") must remain transient — the shim stays in
/// WaitingForReadiness and tools/call returns recoverable=true.
///
/// NEW-RED: current test setup produces Degraded (recoverable=false) because
/// the daemon exits immediately; correct setup requires FakeHealthResponder
/// with `HealthScript::NotReady { status: "starting" }`.
#[tokio::test]
async fn r1_version_compatible_not_ready_is_transient() {
    let workspace = workspace_with_valid_git_root();
    let (mut child, _fake, _ep) = spawn_shim_with_fake_health(
        workspace.path(),
        HealthScript::NotReady {
            status: "starting".into(),
        },
        200,
    )
    .await;
    let (mut stdin, mut stdout) = initialize_shim_mcp(&mut child).await;

    let resp = send_tools_call(&mut stdin, &mut stdout, 10).await;
    assert_transient_recoverable(&resp, "R1");
}

/// R2 — no responder bound (connect refused) → transient (138.010-T).
///
/// When no daemon is listening on the IPC endpoint (connection refused), the
/// error originates inside `ipc_client::send_request` and MUST be classified
/// as transient — never terminal.
#[tokio::test]
async fn r2_connect_refused_is_transient() {
    let workspace = workspace_with_valid_git_root();
    // Start with a bound NotReady responder so the shim's startup path finds
    // a "live daemon", reaches poll_until_ready(None), and times out into
    // WaitingForReadiness (ensure_daemon_running's own readiness budget,
    // including any respawn-ladder logic, runs and fully completes exactly
    // once here — before the fake is ever touched).
    let (mut child, fake, ep) = spawn_shim_with_fake_health(
        workspace.path(),
        HealthScript::NotReady {
            status: "initializing".into(),
        },
        200,
    )
    .await;
    let (mut stdin, mut stdout) = initialize_shim_mcp(&mut child).await;

    // First tools/call: blocks on await_startup_outcome until
    // compute_startup_outcome publishes WaitingForReadiness (its own
    // ensure_daemon_running budget has now fully elapsed), then the
    // request-triggered recovery probe sees the still-bound NotReady
    // responder and returns Recoverable. This confirms ensure_daemon_running
    // has already resolved and will not run again for the rest of this test.
    let warmup_resp = send_tools_call(&mut stdin, &mut stdout, 1).await;
    assert_transient_recoverable(&warmup_resp, "R2 warmup (bound NotReady responder)");

    // NOW genuinely unbind the responder (via its `_shutdown` handling,
    // which stops its accept loop and drops the listener), so the NEXT
    // recovery probe — routed exclusively through
    // transport::ShimHandler::forwarding_endpoint, never back through
    // ensure_daemon_running or its respawn ladder — hits a real connection
    // refusal originating inside ipc_client::send_request.
    let shutdown_request = IpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: Some(Value::from(9001)),
        method: "_shutdown".to_owned(),
        params: None,
    };
    send_request(&ep, &shutdown_request, Duration::from_secs(2))
        .await
        .expect("request fake responder shutdown");
    drop(fake);
    // Give the accept loop a moment to actually return and drop the
    // listener, unbinding the platform endpoint.
    tokio::time::sleep(Duration::from_millis(100)).await;

    let resp = send_tools_call(&mut stdin, &mut stdout, 10).await;
    assert_transient_recoverable(&resp, "R2");
}

/// R4 — JSON-RPC -32603 Internal Error → transient (138.010-T).
///
/// A `-32603` error from the daemon's `_health` handler indicates a temporary
/// internal fault, NOT protocol incompatibility. ONLY `-32601` Method Not
/// Found proves the daemon doesn't implement `_health`. `-32603` MUST remain
/// transient.
///
/// NEW-RED: current test setup produces Degraded (recoverable=false).
#[tokio::test]
async fn r4_jsonrpc_internal_error_is_transient() {
    let workspace = workspace_with_valid_git_root();
    let (mut child, _fake, _ep) = spawn_shim_with_fake_health(
        workspace.path(),
        HealthScript::JsonRpcError {
            code: -32603,
            message: "Internal error".into(),
        },
        500,
    )
    .await;
    let (mut stdin, mut stdout) = initialize_shim_mcp(&mut child).await;

    let resp = send_tools_call(&mut stdin, &mut stdout, 10).await;
    assert_transient_recoverable(&resp, "R4");
}

/// R5 — partial JSON line then close → transient (138.010-T).
///
/// A daemon that writes a truncated response and drops the connection produces
/// an error inside `ipc_client::send_request` (transport layer). Transport
/// errors MUST always be transient — they never prove protocol incompatibility.
///
/// NEW-RED: current test setup produces Degraded (recoverable=false).
#[tokio::test]
async fn r5_truncated_response_is_transient() {
    let workspace = workspace_with_valid_git_root();
    let (mut child, _fake, _ep) =
        spawn_shim_with_fake_health(workspace.path(), HealthScript::TruncatedThenClose, 500).await;
    let (mut stdin, mut stdout) = initialize_shim_mcp(&mut child).await;

    let resp = send_tools_call(&mut stdin, &mut stdout, 10).await;
    assert_transient_recoverable(&resp, "R5");
}

// ── C4 Teardown Neutrality (138.011-T) ───────────────────────────────────────

/// C4 teardown — after terminal latch, client disconnect terminates promptly
/// (138.011-T).
///
/// Pre-existing half: `shim_aborts_unresolved_startup_after_client_disconnects`
/// remains GREEN and byte-unmodified (verified by running it, not by copying).
///
/// NEW-RED half: after a terminal latch, disconnecting the MCP client (closing
/// stdin) must still cause the shim to exit within a bounded time. The monitor's
/// `outcome_tx.closed()` remains the sole non-probe exit path. Currently RED
/// because terminal latching does not exist — the shim times out on daemon
/// readiness and the exit may take the full timeout budget.
#[tokio::test]
async fn c4_terminal_latch_client_disconnect_terminates_promptly() {
    let workspace = workspace_with_valid_git_root();
    let (mut child, _fake, _ep) = spawn_shim_with_fake_health(
        workspace.path(),
        HealthScript::Sequence {
            initial: Box::new(HealthScript::NotReady {
                status: "starting".into(),
            }),
            switch_after: 5,
            then: Box::new(HealthScript::VersionMismatch { version: 999 }),
        },
        200,
    )
    .await;
    let (mut stdin, mut stdout) = initialize_shim_mcp(&mut child).await;

    // Wait for terminal latch.
    tokio::time::sleep(Duration::from_millis(800)).await;

    // Get the first degraded tools/call so we know the session is active.
    let resp = send_tools_call(&mut stdin, &mut stdout, 10).await;
    // Assert terminal (protocol_incompatible) — RED because current code
    // returns readiness_timeout.
    assert_terminal_protocol_incompatible(&resp, "C4 pre-disconnect");

    // Close stdin to simulate client disconnect.
    stdin.shutdown().await.expect("close stdin");
    drop(stdin);
    drop(stdout);

    // The shim must exit promptly (within 2 seconds) after terminal + disconnect.
    let exit_result = tokio::time::timeout(Duration::from_secs(2), child.wait()).await;
    assert!(
        exit_result.is_ok(),
        "C4: after terminal latch + client disconnect, shim must exit within 2s"
    );
}
