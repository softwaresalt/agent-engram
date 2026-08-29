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
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use engram::daemon::protocol::IpcRequest;
use engram::shim::ipc_client::{probe, send_request};
use engram::shim::lifecycle::check_health;
use engram::shim::pidfile::PidFile;
use engram::shim::tools_catalog;
use serde_json::json;
use tempfile::TempDir;
use tokio::io::{AsyncBufReadExt as _, AsyncReadExt as _, AsyncWriteExt as _, BufReader};

#[path = "../helpers/fake_health_responder.rs"]
#[allow(dead_code)]
mod fake_health_responder;
#[path = "../helpers/mod.rs"]
mod helpers;

use fake_health_responder::{FakeHealthResponder, HealthScript};
use helpers::DaemonHarness;

#[derive(Clone, Default)]
struct CapturedTrace(Arc<Mutex<Vec<u8>>>);

impl std::io::Write for CapturedTrace {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .expect("lock trace capture")
            .extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'writer> tracing_subscriber::fmt::MakeWriter<'writer> for CapturedTrace {
    type Writer = Self;

    fn make_writer(&'writer self) -> Self::Writer {
        self.clone()
    }
}

#[tokio::test(flavor = "current_thread")]
async fn ready_health_probe_emits_diagnostic_version_fields() {
    let workspace = TempDir::new().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create git metadata");
    let endpoint =
        engram::daemon::ipc_server::ipc_endpoint(workspace.path()).expect("derive IPC endpoint");
    let _fake = FakeHealthResponder::spawn(&endpoint, HealthScript::Ready);
    tokio::time::sleep(Duration::from_millis(50)).await;

    let captured = CapturedTrace::default();
    let subscriber = tracing_subscriber::fmt()
        .json()
        .with_max_level(tracing::Level::DEBUG)
        .with_writer(captured.clone())
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);

    assert!(check_health(&endpoint).await);

    let bytes = captured.0.lock().expect("lock trace capture").clone();
    let ready_event = String::from_utf8(bytes)
        .expect("trace capture is UTF-8")
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).expect("trace line is JSON"))
        .find(|record| record["fields"]["message"] == "health check returned")
        .expect("ready health probe must emit its diagnostic event");

    assert_eq!(ready_event["fields"]["ready"], true);
    assert_eq!(
        ready_event["fields"]["protocol_version"],
        engram::shim::version::ENGRAM_PROTOCOL_VERSION
    );
    assert_eq!(ready_event["fields"]["build_hash"], "test-fake");
}

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

async fn request_daemon_shutdown(endpoint: &str) -> Result<(), String> {
    let request = IpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: Some(json!(9001)),
        method: "_shutdown".to_owned(),
        params: None,
    };
    let response = send_request(endpoint, &request, Duration::from_secs(2))
        .await
        .map_err(|error| format!("send daemon shutdown request: {error}"))?;
    if let Some(error) = response.error {
        return Err(format!(
            "daemon rejected shutdown request with {}: {}",
            error.code, error.message
        ));
    }
    Ok(())
}

async fn wait_for_daemon_cleanup(
    endpoint: &str,
    pid_hint: Option<&PidFile>,
) -> Result<String, String> {
    let timeout = Duration::from_secs(5);
    let deadline = tokio::time::Instant::now() + timeout;

    loop {
        let endpoint_reachable = probe(endpoint, Duration::from_millis(100)).await.is_ok();
        let pid_alive = pid_hint
            .map(PidFile::verify_alive)
            .transpose()
            .map_err(|error| format!("verify captured daemon PID: {error}"))?
            .unwrap_or(false);

        if !endpoint_reachable && !pid_alive {
            return Ok(format!(
                "endpoint_reachable=false, captured_pid_alive=false, captured_pid={:?}",
                pid_hint.map(|pid_file| pid_file.pid)
            ));
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "daemon remained after {timeout:?}: endpoint_reachable={endpoint_reachable}, \
                 captured_pid_alive={pid_alive}, captured_pid={:?}",
                pid_hint.map(|pid_file| pid_file.pid)
            ));
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

async fn read_bounded_mcp_line(
    stdout: &mut BufReader<tokio::process::ChildStdout>,
    timeout: Duration,
    context: &str,
) -> Result<String, String> {
    let mut line = String::new();
    let bytes_read = tokio::time::timeout(timeout, stdout.read_line(&mut line))
        .await
        .map_err(|_| format!("{context} exceeded {timeout:?}"))?
        .map_err(|error| format!("failed to read {context}: {error}"))?;
    if bytes_read == 0 {
        return Err(format!("shim exited before {context}"));
    }
    Ok(line)
}

async fn complete_mcp_exchange(
    stdin: &mut tokio::process::ChildStdin,
    stdout: &mut BufReader<tokio::process::ChildStdout>,
) -> Result<(String, Duration, String, String), String> {
    let started = Instant::now();
    stdin
        .write_all(
            br#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"worktree-contract","version":"1.0"}}}
"#,
        )
        .await
        .map_err(|error| format!("write MCP initialize frame: {error}"))?;
    let initialize_line =
        read_bounded_mcp_line(stdout, Duration::from_secs(20), "MCP initialize response").await?;
    let initialize_elapsed = started.elapsed();

    stdin
        .write_all(
            br#"{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}
{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}
"#,
        )
        .await
        .map_err(|error| format!("complete MCP initialization and request tools/list: {error}"))?;
    let tools_line =
        read_bounded_mcp_line(stdout, Duration::from_secs(10), "tools/list response").await?;

    stdin
        .write_all(
            br#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"get_workspace_status","arguments":{}}}
"#,
        )
        .await
        .map_err(|error| format!("request get_workspace_status: {error}"))?;
    let status_line = read_bounded_mcp_line(
        stdout,
        Duration::from_secs(10),
        "get_workspace_status response",
    )
    .await?;

    Ok((initialize_line, initialize_elapsed, tools_line, status_line))
}

fn captured_stderr(capture: &Mutex<Vec<u8>>) -> String {
    let bytes = capture.lock().expect("lock captured shim stderr");
    String::from_utf8_lossy(&bytes).into_owned()
}

struct StderrDrain {
    capture: Arc<Mutex<Vec<u8>>>,
    task: tokio::task::JoinHandle<std::io::Result<()>>,
}

fn drain_stderr(mut stderr: tokio::process::ChildStderr) -> StderrDrain {
    let capture = Arc::new(Mutex::new(Vec::new()));
    let task_capture = Arc::clone(&capture);
    let task = tokio::spawn(async move {
        let mut chunk = [0_u8; 4096];
        loop {
            let bytes_read = stderr.read(&mut chunk).await?;
            if bytes_read == 0 {
                return Ok(());
            }
            task_capture
                .lock()
                .expect("lock shim stderr drain buffer")
                .extend_from_slice(&chunk[..bytes_read]);
        }
    });
    StderrDrain { capture, task }
}

async fn cleanup_linked_daemon(linked: &Path, validation_errors: &mut Vec<String>) -> String {
    let pid_hint = PidFile::read(linked);
    if pid_hint.is_none() {
        validation_errors.push("daemon PID metadata was not visible before shutdown".to_owned());
    }

    let endpoint = match engram::daemon::ipc_server::ipc_endpoint(linked) {
        Ok(endpoint) => endpoint,
        Err(error) => {
            let state = format!("failed to resolve linked-worktree daemon endpoint: {error}");
            validation_errors.push(state.clone());
            return state;
        }
    };

    let shutdown_result = request_daemon_shutdown(&endpoint).await;
    let verification_result = wait_for_daemon_cleanup(&endpoint, pid_hint.as_ref()).await;
    if let Err(error) = &shutdown_result {
        validation_errors.push(format!("daemon shutdown was not acknowledged: {error}"));
    }
    if let Err(error) = &verification_result {
        validation_errors.push(format!("daemon cleanup verification failed: {error}"));
    }

    format!(
        "shutdown_acknowledged={}, captured_pid={:?}, verification={verification_result:?}",
        shutdown_result.is_ok(),
        pid_hint.as_ref().map(|pid_file| pid_file.pid)
    )
}

async fn finish_mcp_session(
    mut stdin: tokio::process::ChildStdin,
    mut stdout: BufReader<tokio::process::ChildStdout>,
    mut child: tokio::process::Child,
    stderr_drain: StderrDrain,
    linked: &Path,
) -> Result<String, String> {
    let mut validation_errors = Vec::new();
    if let Err(error) = stdin.shutdown().await {
        validation_errors.push(format!("close MCP stdin to deliver EOF: {error}"));
    }
    drop(stdin);
    let mut stdout_task = tokio::spawn(async move {
        let mut trailing = Vec::new();
        let read_result = stdout.read_to_end(&mut trailing).await;
        (read_result, trailing)
    });

    match tokio::time::timeout(Duration::from_secs(5), child.wait()).await {
        Ok(Ok(exit_status)) if !exit_status.success() => validation_errors.push(format!(
            "shim must exit successfully after its MCP stdin closes; status: {exit_status}"
        )),
        Ok(Ok(_)) => {}
        Ok(Err(error)) => validation_errors.push(format!("failed to wait for shim: {error}")),
        Err(_) => {
            validation_errors.push("shim did not exit within 5s after MCP stdin closed".to_owned());
            if let Err(error) = child.kill().await {
                validation_errors.push(format!("failed to kill timed-out shim: {error}"));
            }
        }
    }

    let stderr_before_cleanup = captured_stderr(&stderr_drain.capture);
    let cleanup_state = cleanup_linked_daemon(linked, &mut validation_errors).await;

    let trailing_stdout = match tokio::time::timeout(Duration::from_secs(5), &mut stdout_task).await
    {
        Ok(Ok((Ok(bytes_read), trailing_stdout))) => {
            if bytes_read != 0 {
                validation_errors.push(format!(
                    "shim emitted {bytes_read} trailing stdout bytes: {}",
                    String::from_utf8_lossy(&trailing_stdout)
                ));
            }
            trailing_stdout
        }
        Ok(Ok((Err(error), trailing_stdout))) => {
            validation_errors.push(format!("failed to drain shim stdout: {error}"));
            trailing_stdout
        }
        Ok(Err(error)) => {
            validation_errors.push(format!("join shim stdout drain task: {error}"));
            Vec::new()
        }
        Err(_) => {
            stdout_task.abort();
            validation_errors.push("shim stdout drain did not finish within 5s".to_owned());
            Vec::new()
        }
    };

    let mut stderr_task = stderr_drain.task;
    match tokio::time::timeout(Duration::from_secs(5), &mut stderr_task).await {
        Ok(Ok(Ok(()))) => {}
        Ok(Ok(Err(error))) => {
            validation_errors.push(format!("failed to drain shim stderr: {error}"));
        }
        Ok(Err(error)) => {
            validation_errors.push(format!("join shim stderr drain task: {error}"));
        }
        Err(_) => {
            stderr_task.abort();
            validation_errors.push("shim stderr drain did not finish within 5s".to_owned());
        }
    }

    let stderr = captured_stderr(&stderr_drain.capture);
    if !trailing_stdout.is_empty() {
        validation_errors.push(format!(
            "shim stdout contained trailing protocol contamination: {}",
            String::from_utf8_lossy(&trailing_stdout)
        ));
    }

    if validation_errors.is_empty() {
        Ok(format!("cleanup state: {cleanup_state}; stderr: {stderr}"))
    } else {
        Err(format!(
            "{}; cleanup state: {cleanup_state}; stderr before cleanup: \
             {stderr_before_cleanup}; final stderr: {stderr}",
            validation_errors.join("; ")
        ))
    }
}

fn assert_initialize_response(initialize_line: &str, initialize_elapsed: Duration) {
    assert!(
        initialize_elapsed < Duration::from_secs(20),
        "worktree MCP startup exceeded its explicit contract budget"
    );
    let response: serde_json::Value = serde_json::from_str(initialize_line.trim())
        .expect("stdout must contain only an MCP initialize frame");
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);
    assert!(
        response.get("error").is_none(),
        "initialize must return a successful JSON-RPC response: {response}"
    );
    assert_eq!(
        response["result"]["protocolVersion"], "2024-11-05",
        "initialize must negotiate the requested MCP protocol version"
    );
    assert_eq!(
        response["result"]["serverInfo"]["name"], "engram-shim",
        "initialize must identify the Engram shim server"
    );
    assert_eq!(
        response["result"]["serverInfo"]["version"],
        env!("CARGO_PKG_VERSION"),
        "initialize must report the Engram package version"
    );
    assert!(
        response["result"]["capabilities"]["tools"].is_object(),
        "initialize response must advertise tools: {response}"
    );
}

fn assert_tools_response(tools_line: &str) {
    let response: serde_json::Value =
        serde_json::from_str(tools_line.trim()).expect("tools/list stdout must be one MCP frame");
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 2);
    let tool_names: std::collections::BTreeSet<String> = response["result"]["tools"]
        .as_array()
        .expect("tools/list must return an array")
        .iter()
        .map(|tool| {
            tool["name"]
                .as_str()
                .expect("every listed tool must have a string name")
                .to_owned()
        })
        .collect();
    let expected_tool_names: std::collections::BTreeSet<String> = tools_catalog::all_tools()
        .into_iter()
        .map(|tool| tool.name.to_string())
        .collect();
    assert_eq!(
        tool_names, expected_tool_names,
        "tools/list must expose the expected Engram catalog"
    );
    assert_eq!(tool_names.len(), tools_catalog::TOOL_COUNT);
}

fn assert_status_response(status_line: &str, linked: &Path) {
    let response: serde_json::Value =
        serde_json::from_str(status_line.trim()).expect("tools/call stdout must be one MCP frame");
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 3);
    assert!(
        response.get("error").is_none(),
        "get_workspace_status must return a successful JSON-RPC response: {response}"
    );
    let status_text = response["result"]["content"][0]["text"]
        .as_str()
        .expect("get_workspace_status must return text content");
    let status: serde_json::Value =
        serde_json::from_str(status_text).expect("workspace status content must be valid JSON");
    assert!(status.is_object(), "workspace status must be a JSON object");
    assert_eq!(
        status["branch"], "feature__mcp-worktree",
        "workspace status must retain linked-worktree branch identity"
    );
    let reported_workspace = Path::new(
        status["path"]
            .as_str()
            .expect("workspace status must report its bound path"),
    )
    .canonicalize()
    .expect("canonicalize reported workspace");
    assert_eq!(
        reported_workspace,
        linked.canonicalize().expect("canonicalize linked worktree"),
        "workspace status must remain bound to the linked worktree"
    );
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
        .kill_on_drop(true)
        .spawn()
        .expect("spawn MCP shim");
    let mut stdin = child.stdin.take().expect("capture shim stdin");
    let stdout = child.stdout.take().expect("capture shim stdout");
    let mut stdout = BufReader::new(stdout);
    let stderr = child.stderr.take().expect("capture shim stderr");
    let stderr_drain = drain_stderr(stderr);

    let exchange_result = complete_mcp_exchange(&mut stdin, &mut stdout).await;
    let finish_result = finish_mcp_session(stdin, stdout, child, stderr_drain, &linked).await;
    let (initialize_line, initialize_elapsed, tools_line, status_line) =
        match (exchange_result, finish_result) {
            (Ok(exchange), Ok(_diagnostics)) => exchange,
            (Err(exchange_error), Ok(diagnostics)) => {
                panic!("MCP exchange failed: {exchange_error}; {diagnostics}")
            }
            (Ok(_), Err(finish_error)) => panic!("MCP session cleanup failed: {finish_error}"),
            (Err(exchange_error), Err(finish_error)) => {
                panic!(
                    "MCP exchange failed: {exchange_error}; MCP session cleanup failed: \
                     {finish_error}"
                )
            }
        };
    assert_initialize_response(&initialize_line, initialize_elapsed);
    assert_tools_response(&tools_line);
    assert_status_response(&status_line, &linked);
}

#[test]
fn shim_reports_transport_failure_fast_when_client_disconnects_before_initialize() {
    // 124-F (870B1AFF) serve-first contract: the shim binds the MCP stdio
    // transport unconditionally and no longer fails fast on daemon-readiness
    // preconditions before initialize (see `shim_stdio_initialize_test.rs`
    // for the degraded-session, daemon-readiness-failure contract with a
    // real MCP handshake). With `Stdio::null()` on stdin, no client ever
    // connects to send `initialize`, so the *transport* itself reports a
    // closed connection — a distinct, still fast, still attributable
    // failure (`ShimFailureClass::TransportFailure`, exit code 13).
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
        .expect("run shim with a disconnected MCP client");

    assert!(
        !output.status.success(),
        "shim must fail when no client ever sends initialize"
    );
    assert_eq!(
        output.status.code(),
        Some(13),
        "shim must exit with the documented transport-failure code (13)"
    );
    assert!(
        started.elapsed() < Duration::from_secs(15),
        "a disconnected client must fail fast instead of consuming the 30 s readiness budget"
    );
    assert!(
        output.stdout.is_empty(),
        "startup diagnostics must not contaminate MCP protocol stdout"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("transport_failure"),
        "startup error must name the transport_failure class; stderr: {stderr}"
    );
}

#[test]
fn shim_rejects_invalid_workspace_without_consuming_readiness_budget() {
    // Same serve-first rationale as above: with no client connected, the
    // transport itself fails fast rather than the (now-deferred and
    // never-reached, since no client sends initialize) workspace-admission
    // precondition. See `shim_stdio_initialize_test.rs` for the
    // admission-failure-with-a-real-handshake contract.
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
