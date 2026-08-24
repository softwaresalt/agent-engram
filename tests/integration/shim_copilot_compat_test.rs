//! Integration coverage for the exact Copilot CLI `1.0.81-8` stdio ordering
//! (130-F, shipment 124-S, plan unit U3).
//!
//! This drives one MCP session end-to-end in the order Copilot actually
//! produced in `.copilot/logs/process-1787528861694-24588.log`:
//!
//! ```text
//! server/discover (id 0)  ->  initialize  ->  tools/list  ->  tools/call
//! ```
//!
//! and proves the spike's success criteria: the pre-handshake probe is
//! refused with `-32601` instead of terminating the process, and the SAME
//! session then completes a standards-compliant handshake, serves the full
//! tool catalog, and routes a `tools/call`.
//!
//! # Catalog oracle delegation (plan review finding F5)
//!
//! Catalog integrity is delegated to the existing independent MCP catalog
//! oracle fixture authored for 123-S and hardened by 129-F. This test does
//! not re-derive expectations from the production catalog module, and it
//! never treats a hardcoded tool count as the oracle — the count is a smoke
//! assertion only.
//!
//! # Why `tools/call` is asserted as *routed*, not *daemon-backed*
//!
//! Standing up a live daemon here would pull the ~7.5 minute Cozo cold start
//! into this suite. That defect is tracked independently and is explicitly
//! out of scope for this shipment, so the harness uses the same fast,
//! deterministic degraded-daemon fixture the other shim contract tests use.
//! The invariant under test is that the probe does not break the session:
//! `tools/call` must return a well-formed JSON-RPC response correlated to its
//! request id rather than a broken pipe. Daemon-backed `tools/call` success
//! is already covered by the daemon integration suites.

use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use serde_json::Value;
use tempfile::TempDir;
use tokio::io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader};

/// Location of the declarative, human-authored catalog expectation fixture
/// owned by the independent MCP catalog oracle.
const CATALOG_FIXTURE_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/mcp_tool_catalog.expected.json"
);

/// Bounded budget for any single MCP frame read.
const FRAME_BUDGET: Duration = Duration::from_secs(20);

/// Create a workspace whose `.git` entry satisfies workspace admission.
fn workspace_with_valid_git_root() -> TempDir {
    let workspace = TempDir::new().expect("workspace tempdir");
    std::fs::create_dir(workspace.path().join(".git")).expect("create .git directory");
    std::fs::write(
        workspace.path().join(".git").join("HEAD"),
        "ref: refs/heads/main\n",
    )
    .expect("write HEAD");
    workspace
}

/// Tool names the independent oracle fixture declares.
fn expected_tool_names() -> std::collections::BTreeSet<String> {
    let raw = std::fs::read_to_string(CATALOG_FIXTURE_PATH).unwrap_or_else(|error| {
        panic!("catalog oracle fixture must exist at {CATALOG_FIXTURE_PATH}: {error}")
    });
    let doc: Value =
        serde_json::from_str(&raw).expect("catalog oracle fixture must be well-formed JSON");
    doc["tools"]
        .as_array()
        .expect("catalog oracle fixture must carry a `tools` array")
        .iter()
        .map(|tool| {
            tool["name"]
                .as_str()
                .expect("each fixture tool must carry a name")
                .to_owned()
        })
        .collect()
}

/// Spawn `engram shim` with a deliberately fast-failing child daemon.
fn spawn_shim(workspace: &Path) -> tokio::process::Child {
    let current_test_exe = std::env::current_exe().expect("resolve current test executable");
    tokio::process::Command::new(env!("CARGO_BIN_EXE_engram"))
        .args(["shim", "--workspace"])
        .arg(workspace)
        .env_remove("ENGRAM_DATA_DIR")
        .env_remove("ENGRAM_MCP_PREINIT_COMPAT")
        .env("CARGO_BIN_EXE_engram", current_test_exe)
        .env("ENGRAM_READY_TIMEOUT_MS", "30000")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .expect("spawn engram shim")
}

/// Read newline-delimited frames until one carrying `id` is observed.
///
/// Bounded and event-driven: never sleeps, and fails loudly if the shim
/// closes stdout (which is exactly the pre-fix broken-pipe symptom).
async fn read_frame_with_id(
    stdout: &mut BufReader<tokio::process::ChildStdout>,
    id: i64,
    context: &str,
) -> Value {
    loop {
        let mut line = String::new();
        let bytes_read = tokio::time::timeout(FRAME_BUDGET, stdout.read_line(&mut line))
            .await
            .unwrap_or_else(|_| panic!("{context} exceeded {FRAME_BUDGET:?}"))
            .unwrap_or_else(|error| panic!("failed to read {context}: {error}"));
        assert!(
            bytes_read > 0,
            "shim exited (stdout EOF) before {context} — the session did not survive"
        );
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let frame: Value = serde_json::from_str(trimmed).unwrap_or_else(|error| {
            panic!("stdout must carry only JSON-RPC frames, got {trimmed:?}: {error}")
        });
        if frame.get("id") == Some(&Value::from(id)) {
            return frame;
        }
    }
}

/// The full Copilot ordering completes end-to-end in a single session.
#[tokio::test]
async fn copilot_probe_then_handshake_completes_catalog_and_tool_call() {
    let workspace = workspace_with_valid_git_root();
    let mut child = spawn_shim(workspace.path());
    let mut stdin = child.stdin.take().expect("capture shim stdin");
    let mut stdout = BufReader::new(child.stdout.take().expect("capture shim stdout"));

    // ── Step 1: the pre-initialize probe Copilot 1.0.81-8 actually sends ──
    stdin
        .write_all(
            br#"{"jsonrpc":"2.0","id":0,"method":"server/discover","params":{}}
"#,
        )
        .await
        .expect("write the Copilot server/discover probe");
    let probe = read_frame_with_id(&mut stdout, 0, "server/discover probe response").await;
    assert_eq!(
        probe["error"]["code"], -32601,
        "the pre-initialize probe must be refused with method-not-found: {probe}"
    );
    assert!(
        probe["id"].is_number(),
        "the probe response must echo id 0 as a JSON number: {probe}"
    );

    // ── Step 2: the standards-compliant handshake still completes ─────────
    stdin
        .write_all(
            br#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"copilot-compat-integration","version":"1.0"}}}
"#,
        )
        .await
        .expect("write MCP initialize frame");
    let initialize = read_frame_with_id(&mut stdout, 1, "MCP initialize response").await;
    assert!(
        initialize.get("error").is_none(),
        "initialize must succeed after the probe: {initialize}"
    );
    assert_eq!(initialize["result"]["serverInfo"]["name"], "engram-shim");

    // ── Step 3: the full catalog is served, verified by the oracle ────────
    stdin
        .write_all(
            br#"{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}
{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}
"#,
        )
        .await
        .expect("complete MCP initialization and request tools/list");
    let tools = read_frame_with_id(&mut stdout, 2, "tools/list response").await;
    let observed: std::collections::BTreeSet<String> = tools["result"]["tools"]
        .as_array()
        .expect("tools/list must return an array after the probe")
        .iter()
        .map(|tool| {
            tool["name"]
                .as_str()
                .expect("each served tool must carry a name")
                .to_owned()
        })
        .collect();
    assert_eq!(
        observed,
        expected_tool_names(),
        "the catalog served after a tolerated probe must match the independent oracle fixture"
    );
    // Smoke assertion only — the oracle above is authoritative (review F5).
    assert!(
        observed.len() >= 20,
        "smoke check: the catalog should still be substantial, saw {}",
        observed.len()
    );

    // ── Step 4: tools/call is routed and answered, not broken-piped ───────
    stdin
        .write_all(
            br#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"get_daemon_status","arguments":{}}}
"#,
        )
        .await
        .expect("request get_daemon_status");
    let call = read_frame_with_id(&mut stdout, 3, "tools/call response").await;
    assert_eq!(call["jsonrpc"], "2.0");
    assert!(
        call.get("result").is_some(),
        "tools/call must return a correlated JSON-RPC result after the probe: {call}"
    );

    // ── Clean disconnect ends the session normally ────────────────────────
    stdin.shutdown().await.ok();
    drop(stdin);
    let exit_status = tokio::time::timeout(Duration::from_secs(10), child.wait())
        .await
        .expect("shim must exit within 10s of stdin closing")
        .expect("wait for shim");
    assert_ne!(
        exit_status.code(),
        Some(13),
        "a probe-then-handshake session must never be classified as a TransportFailure (13)"
    );
}
