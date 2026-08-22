//! Agent-visible MCP `tools/list` capture harness (Feature 127-F, unit U2).
//!
//! INDEPENDENCE INVARIANT — do not violate:
//!
//! This helper obtains the serialized `tools/list` catalog EXACTLY as an MCP
//! client receives it: it spawns `engram shim` as a subprocess, drives the MCP
//! stdio surface (`initialize` -> `notifications/initialized` -> `tools/list`),
//! and returns the newline-delimited JSON-RPC response. It observes the true
//! agent-visible bytes past the serialization boundary.
//!
//! It MUST NOT read the in-process Rust catalog. Do NOT add an import of the
//! production catalog module, and do NOT call its enumeration constructor from
//! this file. The oracle's whole purpose is to observe the agent-visible
//! contract independently of the artifact under test. The independence guard
//! (`scripts/check-oracle-independence.*`) and an in-test assertion both scan
//! this file for the forbidden tokens.

#![allow(dead_code)]

use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use serde_json::Value;
use tempfile::TempDir;
use tokio::io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader};

/// Create a workspace directory whose `.git` entry satisfies workspace
/// admission (`canonicalize_workspace` requires a real directory) without a
/// full `git init`.
pub fn workspace_with_valid_git_root() -> TempDir {
    let workspace = TempDir::new().expect("workspace tempdir");
    std::fs::create_dir(workspace.path().join(".git")).expect("create .git directory");
    std::fs::write(
        workspace.path().join(".git").join("HEAD"),
        "ref: refs/heads/main\n",
    )
    .expect("write HEAD");
    workspace
}

/// Drive the shim's MCP stdio surface against a fresh temp workspace and return
/// the full JSON-RPC `tools/list` response `Value`, exactly as an MCP client
/// receives it.
pub async fn capture_tools_list_response() -> Value {
    let workspace = workspace_with_valid_git_root();
    capture_tools_list_response_in(workspace.path()).await
}

/// Drive the shim's MCP stdio surface against `workspace` and return the full
/// JSON-RPC `tools/list` response `Value`.
///
/// The spawned shim points `CARGO_BIN_EXE_engram` at the current test
/// executable so its child "daemon" exits immediately: `tools/list` is served
/// from the static catalog regardless of daemon readiness, so the capture is
/// fast and deterministic and never waits out the readiness budget.
pub async fn capture_tools_list_response_in(workspace: &Path) -> Value {
    let current_test_exe = std::env::current_exe().expect("resolve current test executable");
    let mut child = tokio::process::Command::new(env!("CARGO_BIN_EXE_engram"))
        .args(["shim", "--workspace"])
        .arg(workspace)
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
            br#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"mcp-catalog-oracle","version":"1.0"}}}
{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}
{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}
"#,
        )
        .await
        .expect("write MCP initialize + tools/list frames");

    let response = read_tools_list_frame(&mut stdout).await;

    stdin.shutdown().await.ok();
    drop(stdin);
    let _ = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;

    response
}

/// Read newline-delimited MCP frames from `stdout` until the `tools/list`
/// response (JSON-RPC id `2` carrying a `result`) is observed, then return it.
async fn read_tools_list_frame(stdout: &mut BufReader<tokio::process::ChildStdout>) -> Value {
    let read_budget = Duration::from_secs(30);
    loop {
        let mut line = String::new();
        let bytes_read = tokio::time::timeout(read_budget, stdout.read_line(&mut line))
            .await
            .expect("tools/list read timed out")
            .expect("read line from shim stdout");
        assert!(bytes_read > 0, "shim exited before returning tools/list");
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Ok(value) = serde_json::from_str::<Value>(trimmed) else {
            continue;
        };
        if value.get("id") == Some(&Value::from(2)) && value.get("result").is_some() {
            return value;
        }
    }
}
