//! Contract tests for the `engram eval` CLI subcommand (081.003-T).
//!
//! Validates the CLI + JSON-stdout contract by invoking the `engram` binary as
//! a subprocess. A [`DaemonHarness`] pre-spawns the daemon so the CLI targets a
//! live workspace.
//!
//! Scenarios:
//! 1. `eval` is registered in the CLI help surface.
//! 2. `engram eval` exits 0 on an empty/disabled run.
//! 3. `engram --json eval` emits a JSON envelope to stdout (`enabled:false`).
//! 4. `engram --quiet eval` suppresses stdout (exit code only).

#[path = "../helpers/mod.rs"]
mod helpers;

use std::process::Command;
use std::time::Duration;

const READY_TIMEOUT: Duration = Duration::from_secs(20);

/// Run the `engram` binary with `args` and capture (`exit_code`, stdout, stderr).
fn run_cli(args: &[&str]) -> (i32, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_engram"))
        .args(args)
        .env_remove("ENGRAM_DATA_DIR")
        .output()
        .expect("engram CLI must execute");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

// ── Scenario 1: subcommand registered in help ────────────────────────────────

#[test]
fn eval_subcommand_registered_in_help() {
    // `engram eval --help` parses and exits 0 (no daemon required).
    let (code, _out, _err) = run_cli(&["eval", "--help"]);
    assert_eq!(code, 0, "engram eval --help must exit 0");

    // Top-level help lists the `eval` subcommand.
    let (code, stdout, _err) = run_cli(&["--help"]);
    assert_eq!(code, 0, "engram --help must exit 0");
    assert!(
        stdout.contains("eval"),
        "top-level help must list the eval subcommand; got:\n{stdout}"
    );
}

// ── Scenarios 2–4: empty/disabled run, JSON stdout, quiet suppression ─────────

#[tokio::test]
async fn eval_empty_run_json_and_quiet_contract() {
    let harness = helpers::DaemonHarness::spawn(READY_TIMEOUT)
        .await
        .expect("daemon must spawn");
    let ws = harness
        .workspace
        .path()
        .to_str()
        .expect("workspace path must be UTF-8");

    // Scenario 2 + 3: exit 0 and a JSON envelope on stdout with enabled:false.
    let (code, stdout, stderr) = run_cli(&["--workspace", ws, "--json", "eval"]);
    assert_eq!(
        code, 0,
        "engram eval must exit 0 on an empty/disabled run; stderr:\n{stderr}"
    );
    let envelope: serde_json::Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("engram eval must emit JSON to stdout: {e}; stdout:\n{stdout}"));
    let report = &envelope["result"];
    assert_eq!(
        report["enabled"],
        serde_json::json!(false),
        "disabled workspace must report enabled:false"
    );
    assert!(
        report.get("semantic").is_some() && report.get("graph").is_some(),
        "report must carry semantic and graph sections; got:\n{report}"
    );

    // Scenario 4: --quiet suppresses stdout but still exits 0.
    let (code, stdout, stderr) = run_cli(&["--workspace", ws, "--quiet", "eval"]);
    assert_eq!(
        code, 0,
        "engram eval --quiet must exit 0; stderr:\n{stderr}"
    );
    assert!(
        stdout.trim().is_empty(),
        "engram eval --quiet must suppress stdout; got:\n{stdout}"
    );
}
