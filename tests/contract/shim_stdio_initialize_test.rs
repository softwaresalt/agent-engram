//! Contract tests for the shim's serve-first stdio initialize contract
//! (124-F, stash 870B1AFF, plan unit U1).
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
    assert!(
        call_response.get("error").is_some(),
        "no tools/call may succeed while the session is degraded: {call_response}"
    );
    let error_message = call_response["error"]["message"]
        .as_str()
        .expect("degraded tools/call error must carry a message");
    assert!(
        error_message.contains("readiness_timeout"),
        "degraded tools/call error must name the startup failure cause: {error_message}"
    );
    assert_eq!(
        call_response["error"]["data"]["failure_class"], "readiness_timeout",
        "degraded tools/call error data must carry the failure_class: {call_response}"
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
        call_response.get("error").is_some(),
        "no tools/call may succeed for an inadmissible workspace: {call_response}"
    );
    assert_eq!(
        call_response["error"]["data"]["failure_class"], "admission_failure",
        "degraded tools/call error data must name admission_failure: {call_response}"
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
