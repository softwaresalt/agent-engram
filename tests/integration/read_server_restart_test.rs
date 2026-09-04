//! End-to-end coverage for plan unit F05 (shim mode propagation, 142.010-T).
//!
//! The shim never carries a `--mode` flag: [`spawn_daemon`] re-launches
//! `engram daemon --workspace <path>` and the spawned daemon independently
//! resolves its own [`DaemonMode`] from that workspace's persisted
//! `.engram/config.toml` (`crate::daemon::run` → `DaemonMode::resolve` →
//! `AppState::with_mode`). These tests prove the persisted mode actually
//! survives that indirection for both shim entry points that start a daemon:
//!
//! * cold auto-spawn (`ensure_daemon_running` → `spawn_daemon`), and
//! * one bounded restart (`ensure_daemon_running` → `respawn_daemon` →
//!   `spawn_daemon`) driven by the version-mismatch path.
//!
//! Observation channel: the daemon's own startup event
//! `daemon mode resolved` (emitted by `crate::daemon::run` immediately after
//! `DaemonMode::resolve`). The shim's debug-only capture seam
//! (`ENGRAM_TEST_CAPTURE_AUTOSPAWN_TRACE=1`) redirects the spawned daemon's
//! stderr into `.engram/test-autospawn.stderr.log` and switches the daemon to
//! JSON logging, so the resolved mode is machine-readable from the test side.
//! This is the same seam `integration_cold_cli_request_frame_correlation`
//! already uses to observe a shim-spawned daemon.
//!
//! The shim is exercised through a real `engram` CLI child process rather than
//! an in-process `ensure_daemon_running` call because the capture seam is read
//! from the *spawning* process's environment, and this repository does not
//! mutate environment variables inside test processes.
//!
//! See docs/exec-plans/2026-09-02-separate-indexer-read-server-plan.md.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use engram::daemon::ipc_server::{ipc_endpoint, resolve_daemon_mode};
use engram::daemon::protocol::{HealthCheckResult, IpcRequest, IpcResponse};
use engram::models::config::DaemonMode;
use engram::shim::ipc_client::probe;
use engram::shim::version::ENGRAM_PROTOCOL_VERSION;
use interprocess::local_socket::ListenerOptions;
use interprocess::local_socket::tokio::Listener;
use interprocess::local_socket::tokio::prelude::*;
use serde_json::{Value, json};
use tempfile::TempDir;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

/// Debug-only shim switch that redirects a spawned daemon's stdio into the
/// workspace and forces JSON daemon logging.
const CAPTURE_SWITCH: &str = "ENGRAM_TEST_CAPTURE_AUTOSPAWN_TRACE";
const TRACE_STDOUT: &str = "test-autospawn.stdout.log";
const TRACE_STDERR: &str = "test-autospawn.stderr.log";
/// Startup event emitted by `engram::daemon::run` after `DaemonMode::resolve`.
const MODE_EVENT: &str = "daemon mode resolved";
/// Startup event emitted by `PluginConfig::load`, carrying the config path.
const CONFIG_EVENT: &str = "loaded plugin config from config.toml";
const CLI_TIMEOUT: Duration = Duration::from_secs(240);
const TRACE_TIMEOUT: Duration = Duration::from_secs(30);
/// Shim-spawned daemons are ephemeral for the CLI cold path: the CLI child does
/// not return until the daemon it started has exited, so the idle TTL bounds
/// how long each auto-spawn costs. `integration_cold_cli_request_frame_correlation`
/// uses the same convention.
const IDLE_TIMEOUT_MS_NUM: u64 = 20_000;
const IDLE_TIMEOUT_MS: &str = "20000";

// ── Workspace + CLI harness ───────────────────────────────────────────────────

/// Create an isolated workspace whose `.engram/config.toml` carries
/// `mode_setting` (omitted entirely when `None`).
fn prepare_workspace(mode_setting: Option<&str>) -> (TempDir, PathBuf) {
    let workspace = tempfile::Builder::new()
        .prefix("read-server-restart-")
        .tempdir()
        .expect("workspace tempdir");
    let root = workspace
        .path()
        .canonicalize()
        .expect("canonicalize workspace");

    let git_dir = root.join(".git");
    fs::create_dir_all(&git_dir).expect("create .git");
    fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").expect("write HEAD");

    let engram_dir = root.join(".engram");
    fs::create_dir_all(&engram_dir).expect("create .engram");
    let mut config = String::from("log_level = \"debug\"\nlog_format = \"json\"\n");
    if let Some(mode) = mode_setting {
        writeln!(config, "mode = \"{mode}\"").expect("write mode setting");
    }
    fs::write(engram_dir.join("config.toml"), config).expect("write config.toml");

    (workspace, root)
}

/// Drive the shim's auto-spawn path through a real `engram` CLI child process.
///
/// `daemon-status` is the cheapest daemon-dependent CLI command: it routes
/// through `cli::runner::run_tool_dispatch`, which calls
/// `shim::lifecycle::ensure_daemon_running` before issuing its IPC request.
async fn run_cli_autospawn(root: &Path) -> std::process::Output {
    tokio::process::Command::new(env!("CARGO_BIN_EXE_engram"))
        .arg("--workspace")
        .arg(root)
        .args(["--json", "--timeout", "120", "daemon-status"])
        .current_dir(root)
        .env_remove("ENGRAM_DATA_DIR")
        .env_remove("ENGRAM_DIRECT")
        .env_remove("ENGRAM_WORKSPACE")
        .env("ENGRAM_IDLE_TIMEOUT_MS", IDLE_TIMEOUT_MS)
        .env("ENGRAM_READY_TIMEOUT_MS", "180000")
        .env("RUST_LOG", "engram=debug")
        .env(CAPTURE_SWITCH, "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .expect("launch real engram CLI")
}

/// Run the CLI auto-spawn path and fail loudly with captured diagnostics.
///
/// Returns the CLI's JSON-RPC response, which is itself proof that a real
/// daemon answered on the IPC endpoint the shim brought up.
async fn autospawn_or_panic(root: &Path, phase: &str) -> Value {
    let output = tokio::time::timeout(CLI_TIMEOUT, run_cli_autospawn(root))
        .await
        .unwrap_or_else(|_| panic!("{phase}: engram CLI did not exit before timeout"));

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(
        output.status.success(),
        "{phase}: engram CLI failed ({status:?})\nstdout:\n{stdout}\nstderr:\n{stderr}\ntrace:\n{trace}",
        status = output.status,
        trace = read_trace_text(root),
    );

    let response: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|error| panic!("{phase}: CLI response is not JSON ({error}): {stdout}"));
    assert!(
        response["result"]["version"].is_string(),
        "{phase}: CLI response must carry a live daemon-status result: {response}"
    );
    response
}

// ── Trace observation ─────────────────────────────────────────────────────────

fn trace_paths(root: &Path) -> [PathBuf; 2] {
    [
        root.join(".engram").join(TRACE_STDERR),
        root.join(".engram").join(TRACE_STDOUT),
    ]
}

fn read_trace_text(root: &Path) -> String {
    trace_paths(root)
        .iter()
        .filter_map(|path| fs::read_to_string(path).ok())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Extract the `path` field of the daemon's `loaded plugin config from
/// config.toml` event.
///
/// This is the shim's own contribution to mode propagation: the daemon can only
/// resolve the right mode if the shim handed `--workspace` the exact directory
/// whose config carries the setting.
fn config_source_from_trace(root: &Path) -> Option<String> {
    trace_records(root).into_iter().find_map(|record| {
        if record["fields"]["message"] == json!(CONFIG_EVENT) {
            record["fields"]["path"].as_str().map(ToOwned::to_owned)
        } else {
            None
        }
    })
}

/// Extract the `mode` field of the daemon's `daemon mode resolved` event.
fn resolved_mode_from_trace(root: &Path) -> Option<String> {
    trace_records(root).into_iter().find_map(|record| {
        if record["fields"]["message"] == json!(MODE_EVENT) {
            record["fields"]["mode"].as_str().map(ToOwned::to_owned)
        } else {
            None
        }
    })
}

/// Parse every complete JSON line captured from a shim-spawned daemon.
///
/// Only complete lines are considered so a partially-flushed tail cannot be
/// mistaken for a malformed record.
fn trace_records(root: &Path) -> Vec<Value> {
    let mut records = Vec::new();
    for path in trace_paths(root) {
        let Ok(content) = fs::read(&path) else {
            continue;
        };
        let complete_len = content
            .iter()
            .rposition(|byte| *byte == b'\n')
            .map_or(0, |index| index + 1);
        for line in content[..complete_len].split(|byte| *byte == b'\n') {
            let Ok(text) = std::str::from_utf8(line) else {
                continue;
            };
            if text.trim().is_empty() {
                continue;
            }
            if let Ok(record) = serde_json::from_str::<Value>(text) {
                records.push(record);
            }
        }
    }
    records
}

/// Poll the capture files until the spawned daemon has reported its mode.
async fn await_resolved_mode(root: &Path, phase: &str) -> String {
    let deadline = tokio::time::Instant::now() + TRACE_TIMEOUT;
    loop {
        if let Some(mode) = resolved_mode_from_trace(root) {
            return mode;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "{phase}: shim-spawned daemon never reported {MODE_EVENT}\ntrace:\n{trace}",
            trace = read_trace_text(root),
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

/// Move the capture files aside so the next shim spawn can create them again.
///
/// `spawn_daemon` opens the capture files with `create_new`, so a second spawn
/// in the same workspace fails unless the previous traces are rotated away.
fn rotate_trace(root: &Path, tag: &str) {
    for path in trace_paths(root) {
        if path.exists() {
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .expect("trace file name");
            let rotated = path.with_file_name(format!("{tag}.{name}"));
            fs::rename(&path, &rotated).expect("rotate captured daemon trace");
        }
    }
}

// ── Daemon control ────────────────────────────────────────────────────────────

/// Wait until the shim-spawned daemon has released `endpoint`.
///
/// The CLI child is expected not to return until the daemon it spawned has
/// exited (idle TTL), so this is normally already true within a second or two
/// of the poll starting. The deadline is nonetheless bounded generously beyond
/// the daemon's own idle timeout (`IDLE_TIMEOUT_MS_NUM`): a first CI failure
/// at a `+30s` margin (finishing at 50.26s against a 50s budget) showed that
/// socket-teardown latency under CI's shared, heavily parallel `cargo test`
/// load can itself run into tens of seconds after the daemon process has
/// otherwise exited, so a "generous" margin close to the raw timeout is not
/// generous enough. The margin here is `+90s` (3x the original), and the
/// panic message reports the actual elapsed wait so a future flake carries
/// its own timing evidence instead of just the budget that was exceeded.
async fn await_endpoint_released(endpoint: &str, phase: &str) {
    let budget = Duration::from_millis(IDLE_TIMEOUT_MS_NUM) + Duration::from_secs(90);
    let started = tokio::time::Instant::now();
    let deadline = started + budget;
    while endpoint_reachable(endpoint).await {
        assert!(
            tokio::time::Instant::now() < deadline,
            "{phase}: endpoint {endpoint} was never released by the auto-spawned \
             daemon after waiting {elapsed:?} (budget {budget:?})",
            elapsed = started.elapsed()
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// Connection-level reachability probe.
///
/// `check_health` is deliberately not used here: it rejects a protocol-version
/// mismatch, which is exactly the condition the fake stale daemon reports.
async fn endpoint_reachable(endpoint: &str) -> bool {
    probe(endpoint, Duration::from_millis(500)).await.is_ok()
}

// ── Fake stale daemon (drives the shim's bounded-restart path) ────────────────

fn is_expected_stale_daemon_shutdown(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        std::io::ErrorKind::NotFound
            | std::io::ErrorKind::BrokenPipe
            | std::io::ErrorKind::ConnectionReset
            | std::io::ErrorKind::UnexpectedEof
    )
}

#[cfg(unix)]
fn bind_fake_listener(
    endpoint: &str,
) -> Result<Listener, Box<dyn std::error::Error + Send + Sync>> {
    use interprocess::local_socket::{GenericFilePath, ToFsName};

    if let Some(parent) = Path::new(endpoint).parent() {
        fs::create_dir_all(parent)?;
    }
    if fs::metadata(endpoint).is_ok() {
        fs::remove_file(endpoint)?;
    }

    let name = endpoint.to_fs_name::<GenericFilePath>()?;
    Ok(ListenerOptions::new().name(name).create_tokio()?)
}

#[cfg(windows)]
fn bind_fake_listener(
    endpoint: &str,
) -> Result<Listener, Box<dyn std::error::Error + Send + Sync>> {
    use interprocess::local_socket::{GenericNamespaced, ToNsName};

    let pipe_name = endpoint.strip_prefix(r"\\.\pipe\").unwrap_or(endpoint);
    let name = pipe_name.to_ns_name::<GenericNamespaced>()?;
    Ok(ListenerOptions::new().name(name).create_tokio()?)
}

/// Serve `_health` with an incompatible protocol version until the shim asks
/// for shutdown, then release the endpoint.
///
/// This is what forces `ensure_daemon_running` down the bounded-restart branch
/// (`respawn_daemon`) rather than a plain cold spawn. `saw_shutdown` records
/// that the shim really did drive the respawn path, so the restart assertion
/// cannot silently degrade into a second cold spawn.
///
/// Connections that close without sending a request (the reachability probe
/// does exactly that) are ignored rather than treated as end-of-life: the fake
/// must stay bound until the shim shuts it down.
async fn run_stale_daemon(
    endpoint: String,
    saw_shutdown: Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let listener = bind_fake_listener(&endpoint)?;

    loop {
        let stream = match listener.accept().await {
            Ok(stream) => stream,
            Err(error) if is_expected_stale_daemon_shutdown(&error) => break,
            Err(error) => return Err(Box::new(error)),
        };
        let (recv_half, mut send_half) = stream.split();
        let mut reader = BufReader::new(recv_half);
        let mut request_line = String::new();
        match reader.read_line(&mut request_line).await {
            Ok(0) => continue,
            Ok(_) => {}
            Err(error) if is_expected_stale_daemon_shutdown(&error) => continue,
            Err(error) => return Err(Box::new(error)),
        }
        if request_line.trim().is_empty() {
            continue;
        }
        let request = IpcRequest::from_line(&request_line).map_err(|response| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                response
                    .error
                    .map_or_else(|| "invalid request".to_owned(), |error| error.message),
            )
        })?;
        let request_id = request.id.unwrap_or(Value::Null);

        let (response, should_exit) = match request.method.as_str() {
            "_health" => (
                IpcResponse::success(
                    request_id,
                    json!(HealthCheckResult {
                        status: "ready".to_owned(),
                        uptime_seconds: 0,
                        workspace: Some(endpoint.clone()),
                        active_connections: 1,
                        protocol_version: ENGRAM_PROTOCOL_VERSION.saturating_add(1),
                        build_hash: "stale-build".to_owned(),
                    }),
                ),
                false,
            ),
            "_shutdown" => {
                saw_shutdown.store(true, Ordering::SeqCst);
                (
                    IpcResponse::success(
                        request_id,
                        json!({ "status": "shutting_down", "flush_started": true }),
                    ),
                    true,
                )
            }
            _ => (
                IpcResponse::error(
                    request_id,
                    engram::daemon::protocol::IpcError {
                        code: -32_601,
                        message: "method not supported by fake stale daemon".to_owned(),
                        data: None,
                    },
                ),
                false,
            ),
        };

        let response_line = response.to_line()?;
        match send_half.write_all(response_line.as_bytes()).await {
            Ok(()) => {}
            Err(error) if is_expected_stale_daemon_shutdown(&error) => break,
            Err(error) => return Err(Box::new(error)),
        }
        match send_half.flush().await {
            Ok(()) => {}
            Err(error) if is_expected_stale_daemon_shutdown(&error) => break,
            Err(error) => return Err(Box::new(error)),
        }

        if should_exit {
            break;
        }
    }

    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// A persisted `read_server` mode must survive shim auto-spawn *and* the shim's
/// one bounded restart — a respawned read-server daemon never resumes managed.
#[tokio::test]
async fn read_server_mode_survives_auto_spawn_and_bounded_restart() {
    let (_workspace, root) = prepare_workspace(Some("read_server"));

    // The persisted setting resolves to ReadServer for the exact workspace path
    // the shim hands to `engram daemon --workspace <path>`.
    assert_eq!(
        resolve_daemon_mode(&root).expect("persisted mode must resolve"),
        DaemonMode::ReadServer,
    );

    // ── Phase 1: cold auto-spawn (no daemon running) ─────────────────────────
    autospawn_or_panic(&root, "auto-spawn").await;

    assert_eq!(
        config_source_from_trace(&root).as_deref(),
        Some(
            root.join(".engram")
                .join("config.toml")
                .display()
                .to_string()
                .as_str()
        ),
        "shim auto-spawn must point the daemon at this workspace's own config",
    );

    let mode = await_resolved_mode(&root, "auto-spawn").await;
    assert_eq!(
        mode,
        DaemonMode::ReadServer.as_str(),
        "shim auto-spawn must preserve the persisted read-server mode",
    );

    // ── Phase 2: one bounded restart via the version-mismatch respawn path ───
    let endpoint = ipc_endpoint(&root).expect("endpoint must resolve");
    await_endpoint_released(&endpoint, "auto-spawn").await;

    rotate_trace(&root, "phase1");
    assert!(
        resolved_mode_from_trace(&root).is_none(),
        "rotated traces must not leak phase-1 evidence into the restart assertion"
    );

    // A stale daemon on the discovery endpoint forces `ensure_daemon_running`
    // down the bounded-restart branch (`respawn_daemon`) instead of a plain
    // cold spawn.
    let saw_shutdown = Arc::new(AtomicBool::new(false));
    let stale = tokio::spawn(run_stale_daemon(
        endpoint.clone(),
        Arc::clone(&saw_shutdown),
    ));
    let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
    while !endpoint_reachable(&endpoint).await {
        assert!(
            tokio::time::Instant::now() < deadline,
            "fake stale daemon never became reachable on {endpoint}"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    autospawn_or_panic(&root, "bounded-restart").await;

    tokio::time::timeout(Duration::from_secs(30), stale)
        .await
        .expect("fake stale daemon task should finish once the shim shuts it down")
        .expect("fake stale daemon task should join cleanly")
        .expect("fake stale daemon task should exit cleanly");
    assert!(
        saw_shutdown.load(Ordering::SeqCst),
        "the restart phase must drive the bounded-restart path (`respawn_daemon` \
         shuts the stale daemon down); without it this case degrades into a \
         second cold spawn and proves nothing about restarts",
    );

    let restarted_mode = await_resolved_mode(&root, "bounded-restart").await;
    assert_eq!(
        restarted_mode,
        DaemonMode::ReadServer.as_str(),
        "bounded restart must preserve the persisted read-server mode",
    );
    assert_ne!(
        restarted_mode,
        DaemonMode::Managed.as_str(),
        "a respawned read-server daemon must never resume in managed mode",
    );
}

/// An explicit `managed` mode still auto-spawns a managed daemon.
#[tokio::test]
async fn explicit_managed_mode_auto_spawn_is_unchanged() {
    let (_workspace, root) = prepare_workspace(Some("managed"));

    assert_eq!(
        resolve_daemon_mode(&root).expect("persisted mode must resolve"),
        DaemonMode::Managed,
    );

    autospawn_or_panic(&root, "managed auto-spawn").await;

    let mode = await_resolved_mode(&root, "managed auto-spawn").await;
    assert_eq!(
        mode,
        DaemonMode::Managed.as_str(),
        "explicit managed mode must survive shim auto-spawn unchanged",
    );
}

/// An absent mode setting keeps the pre-142-F default: managed.
#[tokio::test]
async fn absent_mode_setting_auto_spawns_managed_daemon() {
    let (_workspace, root) = prepare_workspace(None);

    assert_eq!(
        resolve_daemon_mode(&root).expect("absent mode must resolve to the default"),
        DaemonMode::Managed,
    );

    autospawn_or_panic(&root, "default auto-spawn").await;

    let mode = await_resolved_mode(&root, "default auto-spawn").await;
    assert_eq!(
        mode,
        DaemonMode::Managed.as_str(),
        "an absent mode setting must still auto-spawn a managed daemon",
    );
}
