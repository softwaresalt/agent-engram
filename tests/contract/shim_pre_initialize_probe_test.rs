//! Contract tests for the shim's pre-`initialize` compatibility window
//! (130-F, shipment 124-S, plan units U1 and U4).
//!
//! GitHub Copilot CLI `1.0.81-8` (prerelease) sends a JSON-RPC request with
//! id `0` and method `server/discover` **before** the MCP `initialize`
//! request. rmcp's server handshake rejects that ordering with
//! `expect initialized request` and terminates, which the client observes as
//! a broken pipe; the shim exits with `ShimFailureClass::TransportFailure`
//! (exit code 13). See
//! `docs/decisions/2026-08-23-copilot-prerelease-server-discover-mcp-compatibility-spike.md`.
//!
//! These tests assert the compatibility contract:
//!
//! * U1 — a `server/discover` probe sent as the FIRST frame is answered with
//!   JSON-RPC `-32601` (`Method not found`) echoing the request id
//!   **type-preserving** (id `0` stays the JSON number `0`), and the shim
//!   stays alive to complete a standards-compliant `initialize`.
//! * U4 — the preservation invariants: an `initialize`-first session is
//!   unchanged, stdout carries only well-formed JSON-RPC frames, a degraded
//!   session's `tools/call` still returns a structured error rather than
//!   exiting, a probe-then-`initialize` session never yields exit 13,
//!   an id-less pre-initialize probe produces no response frame, and
//!   `ENGRAM_MCP_PREINIT_COMPAT=0` restores the strict pre-change behavior.
//!
//! Every assertion is event-driven within a bounded timeout. No test in this
//! file uses a bare sleep to infer liveness (plan review finding F3).

use std::fs;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use serde_json::Value;
use tempfile::TempDir;
use tokio::io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader};

/// Bounded budget for any single MCP frame read. Generous enough to absorb
/// process startup on a loaded CI worker, short enough to fail a hung shim
/// well inside the harness timeout.
const FRAME_BUDGET: Duration = Duration::from_secs(20);

/// The exact frame Copilot CLI `1.0.81-8` emits before `initialize`:
/// request id `0` as a JSON **number**, method `server/discover`.
const COPILOT_PROBE: &[u8] = br#"{"jsonrpc":"2.0","id":0,"method":"server/discover","params":{}}
"#;

const INITIALIZE_FRAME: &[u8] =
    br#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"copilot-preinit-contract","version":"1.0"}}}
"#;

/// Create a workspace directory whose `.git` entry satisfies workspace
/// admission without requiring a real `git init`.
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

/// Spawn `engram shim` against `workspace`.
///
/// `CARGO_BIN_EXE_engram` is pointed at the current test executable so the
/// shim's spawned "daemon" exits immediately. Startup therefore resolves to a
/// degraded outcome fast and deterministically — which is exactly the
/// serve-first surface these tests need: `initialize` and `tools/list` are
/// still served, and `tools/call` returns a structured error.
///
/// `compat` sets `ENGRAM_MCP_PREINIT_COMPAT` when `Some`; when `None` the
/// variable is removed so the test observes the shipped default (enabled).
fn spawn_shim(workspace: &Path, compat: Option<&str>) -> tokio::process::Child {
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
    match compat {
        Some(value) => command.env("ENGRAM_MCP_PREINIT_COMPAT", value),
        None => command.env_remove("ENGRAM_MCP_PREINIT_COMPAT"),
    };
    command.spawn().expect("spawn engram shim")
}

/// Read one newline-delimited frame from `stdout` within [`FRAME_BUDGET`].
///
/// Returns `None` when the shim closed stdout (EOF) — the signal that the
/// process terminated instead of answering.
async fn read_frame(
    stdout: &mut BufReader<tokio::process::ChildStdout>,
    context: &str,
) -> Option<String> {
    let mut line = String::new();
    let bytes_read = tokio::time::timeout(FRAME_BUDGET, stdout.read_line(&mut line))
        .await
        .unwrap_or_else(|_| panic!("{context} exceeded {FRAME_BUDGET:?}"))
        .unwrap_or_else(|error| panic!("failed to read {context}: {error}"));
    if bytes_read == 0 {
        return None;
    }
    Some(line)
}

/// Read one frame and parse it as JSON, asserting the shim did not exit.
async fn read_json_frame(
    stdout: &mut BufReader<tokio::process::ChildStdout>,
    context: &str,
) -> Value {
    let line = read_frame(stdout, context)
        .await
        .unwrap_or_else(|| panic!("shim exited (stdout EOF) before {context}"));
    serde_json::from_str(line.trim())
        .unwrap_or_else(|error| panic!("{context} must be a well-formed JSON-RPC frame: {error}"))
}

// ── U1 — RED: the reproduced Copilot ordering ────────────────────────────────

/// U1: `server/discover` (id `0`) as the FIRST frame is answered with
/// `-32601`, the id round-trips as the JSON number `0`, and the SAME session
/// then completes a standards-compliant `initialize`.
///
/// The `initialize` success is the deterministic positive liveness signal
/// required by plan review finding F3 — it proves the shim did not exit
/// without asserting a non-event.
#[tokio::test]
async fn server_discover_probe_before_initialize_is_answered_and_session_survives() {
    let workspace = workspace_with_valid_git_root();
    let mut child = spawn_shim(workspace.path(), None);
    let mut stdin = child.stdin.take().expect("capture shim stdin");
    let mut stdout = BufReader::new(child.stdout.take().expect("capture shim stdout"));

    stdin
        .write_all(COPILOT_PROBE)
        .await
        .expect("write the Copilot server/discover probe");

    let probe_response = read_json_frame(&mut stdout, "server/discover probe response").await;
    assert_eq!(probe_response["jsonrpc"], "2.0");
    assert_eq!(
        probe_response["error"]["code"], -32601,
        "a pre-initialize server/discover probe must be refused with JSON-RPC \
         method-not-found, not terminate the session: {probe_response}"
    );

    // Review finding F4: id `0` is the classic falsy-id serialization bug.
    // Assert both the numeric TYPE and the value so a coercion to
    // null/absent/"0" cannot pass.
    let id = &probe_response["id"];
    assert!(
        id.is_number(),
        "the -32601 response must echo request id 0 as a JSON number so the \
         client can correlate it; got {id:?} in {probe_response}"
    );
    assert_eq!(
        id.as_i64(),
        Some(0),
        "the -32601 response must echo the exact request id 0: {probe_response}"
    );

    // Positive liveness signal: the same session completes initialize.
    stdin
        .write_all(INITIALIZE_FRAME)
        .await
        .expect("write MCP initialize frame after the probe");
    let initialize_response = read_json_frame(&mut stdout, "MCP initialize response").await;
    assert_eq!(initialize_response["id"], 1);
    assert!(
        initialize_response.get("error").is_none(),
        "initialize must succeed after a tolerated pre-initialize probe: {initialize_response}"
    );
    assert_eq!(
        initialize_response["result"]["serverInfo"]["name"], "engram-shim",
        "initialize must identify the Engram shim server: {initialize_response}"
    );
}

// ── U4 — Regression guards: preservation invariants ──────────────────────────

/// U4(e)+(f): an id-less `server/discover` frame is a JSON-RPC notification.
/// Responding to a notification violates JSON-RPC, so the shim must emit NO
/// frame for it; the next frame on stdout must be the `initialize` result.
#[tokio::test]
async fn id_less_pre_initialize_probe_produces_no_response_frame() {
    let workspace = workspace_with_valid_git_root();
    let mut child = spawn_shim(workspace.path(), None);
    let mut stdin = child.stdin.take().expect("capture shim stdin");
    let mut stdout = BufReader::new(child.stdout.take().expect("capture shim stdout"));

    stdin
        .write_all(
            br#"{"jsonrpc":"2.0","method":"server/discover","params":{}}
"#,
        )
        .await
        .expect("write id-less server/discover notification");
    stdin
        .write_all(INITIALIZE_FRAME)
        .await
        .expect("write MCP initialize frame");

    // The FIRST frame on stdout must be the initialize result. If the shim
    // wrongly answered the notification, this frame would be the -32601.
    let first = read_json_frame(&mut stdout, "first frame after id-less probe").await;
    assert!(
        first.get("error").is_none(),
        "an id-less pre-initialize probe is a notification and must draw no \
         response frame; got {first}"
    );
    assert_eq!(
        first["id"], 1,
        "the first stdout frame must be the initialize response, proving the \
         notification was dropped silently: {first}"
    );
}

/// U4(a)+(b)+(c)+(d): a probe-then-`initialize` session preserves every
/// documented invariant — full catalog, structured degraded `tools/call`,
/// stdout JSON-RPC purity, and the exit-code taxonomy (never 13).
#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn probe_then_initialize_session_preserves_catalog_degradation_and_exit_code() {
    let workspace = workspace_with_valid_git_root();
    let mut child = spawn_shim(workspace.path(), None);
    let mut stdin = child.stdin.take().expect("capture shim stdin");
    let mut stdout = BufReader::new(child.stdout.take().expect("capture shim stdout"));

    stdin
        .write_all(COPILOT_PROBE)
        .await
        .expect("write the Copilot server/discover probe");
    let probe_response = read_json_frame(&mut stdout, "server/discover probe response").await;
    assert_eq!(probe_response["error"]["code"], -32601);

    stdin
        .write_all(INITIALIZE_FRAME)
        .await
        .expect("write MCP initialize frame");
    let initialize_response = read_json_frame(&mut stdout, "MCP initialize response").await;
    assert!(initialize_response.get("error").is_none());

    // (a) A post-probe session serves the FULL static catalog, unchanged.
    stdin
        .write_all(
            br#"{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}
{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}
"#,
        )
        .await
        .expect("complete MCP initialization and request tools/list");
    let tools_response = read_json_frame(&mut stdout, "tools/list response").await;
    let tool_names: std::collections::BTreeSet<String> = tools_response["result"]["tools"]
        .as_array()
        .expect("tools/list must return an array after a tolerated probe")
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
        "the pre-initialize compatibility window must not alter the tool catalog"
    );

    // (c) Serve-first / degraded-session behavior is preserved: tools/call
    // returns a structured tool-level error instead of exiting.
    stdin
        .write_all(
            br#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"get_daemon_status","arguments":{}}}
"#,
        )
        .await
        .expect("request get_daemon_status");
    let call_response = read_json_frame(&mut stdout, "tools/call response").await;
    assert_eq!(call_response["id"], 3);
    assert!(
        call_response.get("error").is_none(),
        "a degraded tools/call must remain a successful JSON-RPC response \
         carrying result.isError=true: {call_response}"
    );
    assert_eq!(
        call_response["result"]["isError"], true,
        "no tools/call may succeed while the session is degraded: {call_response}"
    );
    assert!(
        call_response["result"]["structuredContent"]["failure_class"].is_string(),
        "degraded tools/call must carry a structured failure_class: {call_response}"
    );

    // (d) Close the session and assert the exit-code taxonomy is untouched.
    stdin.shutdown().await.expect("close MCP stdin");
    drop(stdin);

    // (b) stdout purity: drain every remaining frame and assert each one is a
    // well-formed JSON-RPC object. Any diagnostic text leaking to stdout
    // would fail to parse here.
    while let Some(line) = read_frame(&mut stdout, "trailing stdout frame").await {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let frame: Value = serde_json::from_str(trimmed).unwrap_or_else(|error| {
            panic!("stdout must carry only JSON-RPC frames, got {trimmed:?}: {error}")
        });
        assert_eq!(
            frame["jsonrpc"], "2.0",
            "every stdout frame must be JSON-RPC 2.0: {frame}"
        );
    }

    let exit_status = tokio::time::timeout(Duration::from_secs(10), child.wait())
        .await
        .expect("shim must exit within 10s of stdin closing")
        .expect("wait for shim");
    assert_ne!(
        exit_status.code(),
        Some(13),
        "a probe-then-initialize session must never be classified as a \
         TransportFailure (exit 13); the probe is a tolerated compatibility \
         case, not a transport fault"
    );
    // The daemon deliberately fails in this harness, so the session is
    // degraded and the readiness-timeout code (11) is the correct outcome.
    // Asserting it pins the taxonomy: the compatibility window must not
    // change HOW a session is classified, only that the probe is tolerated.
    assert_eq!(
        exit_status.code(),
        Some(11),
        "the compatibility window must leave the exit-code taxonomy unchanged"
    );
}

/// U4(g): with `ENGRAM_MCP_PREINIT_COMPAT=0` the kill-switch restores the
/// strict pre-change rmcp ordering — the probe is NOT answered and the
/// session terminates with the documented transport-failure exit code (13).
///
/// This is the runtime rollback contract from the plan: operators can restore
/// the previous behavior with no redeploy.
#[tokio::test]
async fn kill_switch_restores_strict_pre_initialize_ordering() {
    let workspace = workspace_with_valid_git_root();
    let mut child = spawn_shim(workspace.path(), Some("0"));
    let mut stdin = child.stdin.take().expect("capture shim stdin");
    let mut stdout = BufReader::new(child.stdout.take().expect("capture shim stdout"));

    stdin
        .write_all(COPILOT_PROBE)
        .await
        .expect("write the Copilot server/discover probe");

    // Strict mode: no -32601 is produced. Whatever rmcp does, it must NOT be
    // the compatibility response, and the session must end.
    if let Some(line) = read_frame(&mut stdout, "strict-mode frame").await {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            if let Ok(frame) = serde_json::from_str::<Value>(trimmed) {
                assert_ne!(
                    frame["error"]["code"], -32601,
                    "with the kill-switch set the shim must not synthesize a \
                     compatibility response: {frame}"
                );
            }
        }
    }

    stdin.shutdown().await.ok();
    drop(stdin);
    let exit_status = tokio::time::timeout(Duration::from_secs(20), child.wait())
        .await
        .expect("shim must exit within 20s with the kill-switch set")
        .expect("wait for shim");
    assert_eq!(
        exit_status.code(),
        Some(13),
        "ENGRAM_MCP_PREINIT_COMPAT=0 must restore the strict rmcp ordering, \
         which classifies the rejected handshake as a TransportFailure (13)"
    );
}

/// U4(b): rmcp and the compatibility filter can write to stdout in the same
/// armed window, so every frame must stay on its own line.
///
/// An unparseable frame makes rmcp emit a `-32700` parse-error response and
/// **keep waiting** for `initialize`, leaving the filter armed. A probe sent
/// immediately after therefore has two independent writers on stdout. This
/// asserts every emitted line is an intact, separately parseable JSON-RPC
/// frame — the failure mode being guarded against is two JSON objects spliced
/// onto one line.
#[tokio::test]
async fn concurrent_rmcp_and_filter_writes_stay_on_separate_lines() {
    let workspace = workspace_with_valid_git_root();
    let mut child = spawn_shim(workspace.path(), None);
    let mut stdin = child.stdin.take().expect("capture shim stdin");
    let mut stdout = BufReader::new(child.stdout.take().expect("capture shim stdout"));

    // Interleave frames rmcp answers itself with probes the filter answers.
    for _ in 0..8_u32 {
        stdin
            .write_all(b"{not valid json at all}\n")
            .await
            .expect("write an unparseable frame rmcp must answer");
        stdin
            .write_all(COPILOT_PROBE)
            .await
            .expect("write the Copilot server/discover probe");
    }
    stdin
        .write_all(INITIALIZE_FRAME)
        .await
        .expect("write MCP initialize frame");

    let mut saw_method_not_found = false;
    loop {
        let line = read_frame(&mut stdout, "interleaved stdout frame")
            .await
            .expect("shim exited before completing initialize");
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        // The load-bearing assertion: each LINE must parse as exactly one
        // JSON-RPC frame. Two spliced objects fail here.
        let frame: Value = serde_json::from_str(trimmed).unwrap_or_else(|error| {
            panic!("each stdout line must be exactly one JSON-RPC frame, got {trimmed:?}: {error}")
        });
        assert_eq!(frame["jsonrpc"], "2.0");
        if frame["error"]["code"] == -32601 {
            saw_method_not_found = true;
            assert!(
                frame["id"].is_number(),
                "the probe response must keep id 0 numeric even under \
                 concurrent writes: {frame}"
            );
        }
        if frame["id"].as_i64() == Some(1) {
            assert!(
                frame.get("error").is_none(),
                "initialize must still succeed after interleaved traffic: {frame}"
            );
            break;
        }
    }
    assert!(
        saw_method_not_found,
        "the probes must still have been answered with -32601"
    );

    stdin.shutdown().await.ok();
    drop(stdin);
    let _ = tokio::time::timeout(Duration::from_secs(10), child.wait()).await;
}

/// U4(a): an `initialize`-first session — the overwhelmingly common case —
/// is completely unaffected by the compatibility window.
#[tokio::test]
async fn initialize_first_session_is_unchanged() {
    let workspace = workspace_with_valid_git_root();
    let mut child = spawn_shim(workspace.path(), None);
    let mut stdin = child.stdin.take().expect("capture shim stdin");
    let mut stdout = BufReader::new(child.stdout.take().expect("capture shim stdout"));

    stdin
        .write_all(INITIALIZE_FRAME)
        .await
        .expect("write MCP initialize frame");
    let initialize_response = read_json_frame(&mut stdout, "MCP initialize response").await;
    assert_eq!(initialize_response["id"], 1);
    assert!(
        initialize_response.get("error").is_none(),
        "an initialize-first session must be unaffected: {initialize_response}"
    );
    assert_eq!(
        initialize_response["result"]["serverInfo"]["name"],
        "engram-shim"
    );

    // A `server/discover` arriving AFTER initialize is outside the
    // compatibility window and must fall through to rmcp's own semantics —
    // the filter disarms permanently on the first initialize.
    stdin
        .write_all(
            br#"{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}
{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}
"#,
        )
        .await
        .expect("complete MCP initialization and request tools/list");
    let tools_response = read_json_frame(&mut stdout, "tools/list response").await;
    assert!(
        tools_response["result"]["tools"].is_array(),
        "a normal session must serve the catalog: {tools_response}"
    );

    stdin.shutdown().await.ok();
    drop(stdin);
    let _ = tokio::time::timeout(Duration::from_secs(10), child.wait()).await;
}
