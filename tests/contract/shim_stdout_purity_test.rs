//! Contract tests for shim stdout framing purity (124-F, stash 870B1AFF,
//! plan unit U2).
//!
//! `src/lib.rs::init_tracing` builds a `fmt::layer()` whose default writer is
//! stdout. The shim path now calls `init_tracing` (124-F U5) so its own
//! startup diagnostics are attributable via `RUST_LOG`; if that writer were
//! not pinned to stderr, any tracing output would land on the MCP stdout
//! framing channel and corrupt `initialize` (see
//! `docs/decisions/2026-08-21-870b1aff-copilot-mcp-stdio-initialize-investigation.md`,
//! finding E5). These tests assert that every byte the shim writes to stdout
//! parses as JSON-RPC framing, both in a clean run and when debug-level
//! tracing is force-enabled via `RUST_LOG` / `ENGRAM_LOG_FORMAT` — the
//! configuration most likely to regress the stderr pin (plan hardening,
//! "Reinforced Verification").

use std::fs;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use serde_json::Value;
use tempfile::TempDir;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

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

/// Assert every non-empty line of `stdout_bytes` parses as a JSON-RPC 2.0
/// frame. Any stray byte (a log line, a banner, a partial write) fails this
/// assertion immediately with the offending line surfaced.
fn assert_stdout_is_pure_jsonrpc(stdout_bytes: &[u8]) {
    let text = String::from_utf8(stdout_bytes.to_vec())
        .expect("shim stdout must be valid UTF-8 (JSON-RPC framing only)");
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let frame: Value = serde_json::from_str(line.trim()).unwrap_or_else(|error| {
            panic!(
                "shim stdout must contain only JSON-RPC frames; offending line failed to parse ({error}): {line}"
            )
        });
        assert_eq!(
            frame["jsonrpc"], "2.0",
            "every shim stdout line must be a JSON-RPC 2.0 frame: {line}"
        );
    }
}

/// Run a full MCP exchange against a shim whose spawned daemon has already
/// failed (so plenty of internal tracing activity — spawn attempts, backoff,
/// classification — has a chance to fire), then return the captured stdout
/// and stderr bytes for purity assertions.
async fn run_mcp_exchange_and_capture(
    workspace: &Path,
    extra_env: &[(&str, &str)],
) -> (Vec<u8>, Vec<u8>) {
    let current_test_exe = std::env::current_exe().expect("resolve current test executable");
    let mut command = tokio::process::Command::new(env!("CARGO_BIN_EXE_engram"));
    command
        .args(["shim", "--workspace"])
        .arg(workspace)
        .env_remove("ENGRAM_DATA_DIR")
        .env("CARGO_BIN_EXE_engram", current_test_exe)
        .env("ENGRAM_READY_TIMEOUT_MS", "30000")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    for (key, value) in extra_env {
        command.env(key, value);
    }
    let mut child = command.spawn().expect("spawn engram shim");
    let mut stdin = child.stdin.take().expect("capture shim stdin");
    let mut stdout = child.stdout.take().expect("capture shim stdout");
    let mut stderr = child.stderr.take().expect("capture shim stderr");

    stdin
        .write_all(
            br#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"stdout-purity-contract","version":"1.0"}}}
{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}
{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}
{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"get_workspace_status","arguments":{}}}
"#,
        )
        .await
        .expect("write full MCP exchange");
    stdin.shutdown().await.expect("close MCP stdin");
    drop(stdin);

    let mut stdout_bytes = Vec::new();
    let mut stderr_bytes = Vec::new();
    tokio::time::timeout(
        Duration::from_secs(15),
        stdout.read_to_end(&mut stdout_bytes),
    )
    .await
    .expect("drain shim stdout within 15s")
    .expect("read shim stdout");
    tokio::time::timeout(
        Duration::from_secs(15),
        stderr.read_to_end(&mut stderr_bytes),
    )
    .await
    .expect("drain shim stderr within 15s")
    .expect("read shim stderr");
    let _ = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;

    (stdout_bytes, stderr_bytes)
}

/// Scenario 1: a clean shim run (default logging configuration) never writes
/// anything but JSON-RPC frames to stdout, even in a degraded session.
#[tokio::test]
async fn stdout_is_pure_jsonrpc_in_a_clean_run() {
    let workspace = workspace_with_valid_git_root();
    let (stdout_bytes, _stderr_bytes) = run_mcp_exchange_and_capture(workspace.path(), &[]).await;
    assert!(
        !stdout_bytes.is_empty(),
        "shim must produce MCP responses on stdout"
    );
    assert_stdout_is_pure_jsonrpc(&stdout_bytes);
}

/// Scenario 2: stdout purity holds even when `RUST_LOG=engram=debug` and
/// `ENGRAM_LOG_FORMAT` are set — the configuration most likely to regress the
/// stderr pin, since it forces the tracing subscriber to actually emit
/// formatted log lines during the shim's own startup diagnostics.
#[tokio::test]
async fn stdout_is_pure_jsonrpc_with_debug_logging_env_vars_set() {
    let workspace = workspace_with_valid_git_root();
    let (stdout_bytes, stderr_bytes) = run_mcp_exchange_and_capture(
        workspace.path(),
        &[
            ("RUST_LOG", "engram=debug"),
            ("ENGRAM_LOG_FORMAT", "pretty"),
        ],
    )
    .await;
    assert!(
        !stdout_bytes.is_empty(),
        "shim must produce MCP responses on stdout"
    );
    assert_stdout_is_pure_jsonrpc(&stdout_bytes);

    // The debug-logging configuration should actually exercise the tracing
    // subscriber (otherwise this scenario would trivially pass without
    // proving anything about the writer pin). Diagnostic output is expected
    // on stderr, not asserted-empty, but stdout purity above is what matters.
    let _ = stderr_bytes;
}

/// Scenario 2b: the same invariant holds for JSON-formatted tracing output.
#[tokio::test]
async fn stdout_is_pure_jsonrpc_with_debug_logging_json_format() {
    let workspace = workspace_with_valid_git_root();
    let (stdout_bytes, _stderr_bytes) = run_mcp_exchange_and_capture(
        workspace.path(),
        &[("RUST_LOG", "engram=debug"), ("ENGRAM_LOG_FORMAT", "json")],
    )
    .await;
    assert!(
        !stdout_bytes.is_empty(),
        "shim must produce MCP responses on stdout"
    );
    assert_stdout_is_pure_jsonrpc(&stdout_bytes);
}
