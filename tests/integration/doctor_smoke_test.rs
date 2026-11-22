//! Integration test for the `doctor --smoke` full-handshake round-trip.
//!
//! Exercises [`engram::tools::doctor::run_smoke_test`] end-to-end.
//! A regression here means the daemon IPC handshake or smoke-test logic broke.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant, SystemTime};

use engram::daemon::ipc_server::ipc_endpoint;
use engram::daemon::protocol::IpcRequest;
use engram::shim::{ipc_client::send_request, lifecycle::check_health};
use engram::tools::doctor::run_smoke_test;
use serde_json::Value;

/// A full shim→daemon handshake smoke test must pass cleanly for a valid
/// git-backed workspace.
///
/// Daemon spawns, handshake succeeds, result.passed == true.
#[tokio::test]
async fn doctor_smoke_exercises_full_handshake() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");

    let result = run_smoke_test(workspace.path())
        .await
        .expect("smoke test should succeed without errors");

    assert!(
        result.passed,
        "smoke test reported failure: {}",
        result.message
    );
}

/// Smoke result latency field is present when the test completes.
///
/// Red phase: panics at `todo!()` before reaching this assertion.
#[tokio::test]
async fn doctor_smoke_result_includes_latency() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");

    let result = run_smoke_test(workspace.path())
        .await
        .expect("smoke test should succeed");

    assert!(
        result.latency_ms.is_some(),
        "smoke result should include latency measurement"
    );
}

/// A `read_server`-mode daemon's smoke run must parse the live status
/// response, pass, and leave the daemon running without issuing
/// `set_workspace` or `_shutdown` (138.008-T / 142.039-T safety contract).
///
/// This exercises the full live-mode branch end-to-end: a real daemon
/// process is spawned in `read_server` mode (via `.engram/config.toml`'s
/// persisted `mode` setting), `run_smoke_test` is run against it, and then a
/// direct follow-up IPC call proves the daemon is still alive -- which would
/// fail if `run_smoke_test` had sent `_shutdown` (the `managed`-mode
/// behavior). A regression that silently re-selected the `managed` smoke
/// sequence for a `read_server` daemon (e.g. by reverting to on-disk config
/// instead of the daemon's own observed live mode) would either corrupt the
/// live daemon's active workspace via an unwanted `set_workspace`, or shut
/// the daemon down -- either of which this test would catch.
#[tokio::test]
async fn doctor_smoke_leaves_read_server_daemon_running_without_binding_workspace() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");
    let engram_dir = workspace.path().join(".engram");
    fs::create_dir_all(&engram_dir).expect("create .engram");
    fs::write(engram_dir.join("config.toml"), "mode = \"read_server\"\n")
        .expect("write read_server config.toml");

    let result = run_smoke_test(workspace.path())
        .await
        .expect("read_server smoke test should succeed without errors");
    assert!(
        result.passed,
        "read_server smoke test reported failure: {}",
        result.message
    );

    // Prove the daemon was left running (not shut down) by sending a direct
    // follow-up get_daemon_status call. A `managed`-mode-style `_shutdown`
    // call during the smoke run would make this connection fail.
    let endpoint = ipc_endpoint(workspace.path()).expect("resolve ipc endpoint");
    let follow_up = IpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: Some(Value::Number(serde_json::Number::from(99))),
        method: "get_daemon_status".to_owned(),
        params: None,
    };
    let response = send_request(&endpoint, &follow_up, Duration::from_secs(10))
        .await
        .expect("follow-up get_daemon_status must reach a still-running read_server daemon");
    assert!(
        response.error.is_none(),
        "follow-up get_daemon_status returned an error, daemon may not have survived the smoke run: {:?}",
        response.error
    );
    let reported_mode = response
        .result
        .as_ref()
        .and_then(|result| result.get("mode"))
        .and_then(Value::as_str);
    assert_eq!(
        reported_mode,
        Some("read_server"),
        "daemon must still be running in read_server mode after the smoke run"
    );

    // Clean up: explicitly shut down the daemon this test spawned, since a
    // read_server smoke run intentionally never does so itself.
    let shutdown = IpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: Some(Value::Number(serde_json::Number::from(100))),
        method: "_shutdown".to_owned(),
        params: None,
    };
    let _ = send_request(&endpoint, &shutdown, Duration::from_secs(10)).await;
}

struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

type CliTestResult = Result<(), Box<dyn Error>>;

fn engram_command() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_engram"));
    command
        .env_remove("ENGRAM_DATA_DIR")
        .env_remove("ENGRAM_WORKSPACE")
        .env_remove("ENGRAM_DIRECT")
        .stdin(Stdio::null());
    command
}

fn snapshot_workspace(root: &Path) -> io::Result<BTreeMap<PathBuf, (u64, SystemTime)>> {
    fn visit(
        root: &Path,
        directory: &Path,
        snapshot: &mut BTreeMap<PathBuf, (u64, SystemTime)>,
    ) -> io::Result<()> {
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let path = entry.path();
            let relative = path
                .strip_prefix(root)
                .map_err(io::Error::other)?
                .to_path_buf();
            let metadata = fs::symlink_metadata(&path)?;
            snapshot.insert(relative, (metadata.len(), metadata.modified()?));
            if metadata.is_dir() {
                visit(root, &path, snapshot)?;
            }
        }
        Ok(())
    }

    let mut snapshot = BTreeMap::new();
    visit(root, root, &mut snapshot)?;
    Ok(snapshot)
}

async fn assert_daemon_ready(endpoint: &str) -> io::Result<()> {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !check_health(endpoint).await {
        if Instant::now() >= deadline {
            return Err(io::Error::other(
                "daemon did not become ready within 10 seconds",
            ));
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    Ok(())
}

/// `engram doctor --smoke` must probe a live `ReadServer` without provenance or side effects.
#[tokio::test]
async fn cli_doctor_smoke_read_server_succeeds_without_provenance_or_side_effects() -> CliTestResult
{
    let workspace = tempfile::tempdir()?;
    fs::create_dir(workspace.path().join(".git"))?;
    fs::write(
        workspace.path().join(".git").join("HEAD"),
        "ref: refs/heads/main\n",
    )?;
    fs::create_dir(workspace.path().join(".engram"))?;
    fs::write(
        workspace.path().join(".engram").join("config.toml"),
        "mode = \"read_server\"\n",
    )?;
    let canonical_workspace = workspace.path().canonicalize()?;
    let endpoint = ipc_endpoint(&canonical_workspace)?;

    let child = engram_command()
        .arg("daemon")
        .arg("--workspace")
        .arg(&canonical_workspace)
        .env("ENGRAM_IDLE_TIMEOUT_MS", "0")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    let _child_guard = ChildGuard(child);

    assert_daemon_ready(&endpoint).await?;
    let before = snapshot_workspace(&canonical_workspace)?;

    let output = engram_command()
        .arg("--workspace")
        .arg(&canonical_workspace)
        .args(["--json", "doctor", "--smoke"])
        .env("ENGRAM_IDLE_TIMEOUT_MS", "5000")
        .env("ENGRAM_READY_TIMEOUT_MS", "10000")
        .output()?;
    assert!(
        output.status.success(),
        "doctor --smoke failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let envelope: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(envelope["jsonrpc"], Value::String("2.0".to_owned()));
    assert_eq!(
        envelope["result"]["passed"],
        Value::Bool(true),
        "doctor --smoke did not pass: {envelope}"
    );
    let result = envelope
        .get("result")
        .ok_or_else(|| io::Error::other("doctor --smoke response omitted result"))?;
    assert!(
        result.get("provenance").is_none(),
        "CLI readiness must not report generation provenance"
    );

    let after = snapshot_workspace(&canonical_workspace)?;
    assert_eq!(
        before, after,
        "doctor --smoke changed the workspace recursively"
    );
    assert!(
        after.keys().all(|path| !path.starts_with(".engram/cozo")),
        "doctor --smoke created managed `.engram/cozo` state"
    );

    let request = IpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: Some(Value::Number(serde_json::Number::from(99_u8))),
        method: "get_daemon_status".to_owned(),
        params: None,
    };
    let response = send_request(&endpoint, &request, Duration::from_secs(10)).await?;
    assert!(
        response.error.is_none(),
        "follow-up get_daemon_status returned an error: {:?}",
        response.error
    );
    let reported_mode = response
        .result
        .as_ref()
        .and_then(|status| status.get("mode"))
        .and_then(Value::as_str);
    assert_eq!(reported_mode, Some("read_server"));

    Ok(())
}

/// Hidden `doctor` requires `--smoke` and remains absent from public command help.
#[test]
fn cli_doctor_without_smoke_flag_exits_two_requiring_smoke() -> CliTestResult {
    let output = engram_command()
        .arg("doctor")
        .env("ENGRAM_IDLE_TIMEOUT_MS", "2000")
        .output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2), "unexpected stderr: {stderr}");
    assert!(
        stderr.contains("--smoke"),
        "missing --smoke error was not reported: {stderr}"
    );

    let help = engram_command().arg("--help").output()?;
    assert!(
        help.status.success(),
        "engram --help failed: {}",
        String::from_utf8_lossy(&help.stderr)
    );
    let help = String::from_utf8(help.stdout)?;
    let (_, after_heading) = help
        .split_once("Commands:")
        .ok_or_else(|| io::Error::other("engram --help omitted Commands: section"))?;
    let commands_section = after_heading
        .split_once("\nOptions:")
        .map_or(after_heading, |(commands, _)| commands);
    let command_names: Vec<&str> = commands_section
        .lines()
        .filter_map(|line| {
            line.strip_prefix("  ")
                .and_then(|command| command.split_whitespace().next())
        })
        .collect();
    assert!(
        !command_names.contains(&"doctor"),
        "hidden doctor command leaked into --help: {help}"
    );

    Ok(())
}

/// A non-git workspace produces a structured `DaemonSpawnFailed` error, not a panic.
#[tokio::test]
async fn cli_doctor_smoke_non_git_workspace_returns_structured_error() -> CliTestResult {
    let workspace = tempfile::tempdir()?;
    let canonical_workspace = workspace.path().canonicalize()?;
    let output = engram_command()
        .arg("--workspace")
        .arg(&canonical_workspace)
        .args(["--json", "doctor", "--smoke"])
        .env("ENGRAM_IDLE_TIMEOUT_MS", "2000")
        .env("ENGRAM_READY_TIMEOUT_MS", "3000")
        .output()?;

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("panicked at"),
        "doctor --smoke panicked: {stderr}"
    );
    let envelope: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(envelope["jsonrpc"], Value::String("2.0".to_owned()));
    assert!(
        envelope["error"].is_object(),
        "doctor --smoke omitted a JSON-RPC error object: {envelope}"
    );
    assert_eq!(envelope["error"]["engram_code"], Value::from(8005_u64));
    assert_eq!(
        envelope["error"]["engram_name"],
        Value::String("DaemonSpawnFailed".to_owned())
    );

    Ok(())
}
