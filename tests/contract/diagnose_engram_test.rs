//! Contract tests for `scripts/diagnose-engram.ps1`.
//!
//! The script advertises a read-only, non-mutating diagnostics contract but
//! CLI parity commands (`daemon-status`, `workspace-status`, `health`,
//! `search`) route through `run_tool`, which auto-spawns the daemon when
//! absent (`src/cli/runner.rs` `ensure_daemon_running`). These tests prove
//! two properties of the fixed script using a fake `engram` executable on
//! `PATH` that records every invocation, so a real daemon/binary is never
//! required:
//!
//! 1. When no daemon is running for the workspace, the script determines
//!    that fact itself (via `.engram\run\engram.pid`) without ever invoking
//!    the daemon-backed diagnostics, so no auto-spawn can occur.
//! 2. When the shim-startup-failures diagnostics log cannot be read (here:
//!    replaced by a directory to force a deterministic `Get-Content`
//!    failure), the script reports the failure and still reaches its final
//!    summary instead of aborting under `$ErrorActionPreference = "Stop"`.

#![cfg(windows)]

use std::fs;
use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

/// Fixture `engram.ps1`: records every invocation's arguments (one line per
/// call) to the file named by `ENGRAM_FIXTURE_LOG`, then returns a benign
/// success result. Never touches any real daemon state.
const FIXTURE_ENGRAM: &str = r#"param()
Add-Content -Path $env:ENGRAM_FIXTURE_LOG -Value ($args -join ' ')
if ($args -contains '--version') {
    Write-Output 'engram 0.0.0-fixture'
    exit 0
}
Write-Output "fixture-engram-called: $($args -join ' ')"
exit 0
"#;

fn diagnose_script_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/diagnose-engram.ps1")
}

/// Build a fixture directory containing `engram.ps1` and return
/// `(fixture_dir, invocation_log_path)`.
fn write_engram_fixture(fixture_dir: &Path) -> std::path::PathBuf {
    fs::write(fixture_dir.join("engram.ps1"), FIXTURE_ENGRAM).expect("write engram fixture");
    fixture_dir.join("invocations.log")
}

fn path_with_fixture_prepended(fixture_dir: &Path) -> std::ffi::OsString {
    std::env::join_paths(
        std::iter::once(fixture_dir.to_path_buf()).chain(std::env::split_paths(
            &std::env::var_os("PATH").unwrap_or_default(),
        )),
    )
    .expect("compose fixture PATH")
}

fn run_diagnose_script(
    workspace: &Path,
    fixture_dir: &Path,
    log_path: &Path,
) -> std::process::Output {
    Command::new("pwsh")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(diagnose_script_path())
        .arg("-Workspace")
        .arg(workspace)
        .env("PATH", path_with_fixture_prepended(fixture_dir))
        .env("ENGRAM_FIXTURE_LOG", log_path)
        .output()
        .expect("run diagnose-engram.ps1")
}

#[test]
fn diagnose_engram_skips_daemon_backed_probes_without_auto_spawn_when_daemon_absent() {
    let fixture = TempDir::new().expect("fixture dir");
    let log_path = write_engram_fixture(fixture.path());

    // Workspace has no `.engram/` at all, so the PID-file probe must report
    // "no daemon" -- this is the common customer-box state before any MCP
    // client has ever bound the workspace.
    let workspace = TempDir::new().expect("workspace dir");

    let output = run_diagnose_script(workspace.path(), fixture.path(), &log_path);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert_eq!(
        output.status.code(),
        Some(1),
        "script must report a failure exit code when daemon-backed diagnostics are skipped; \
         stdout: {stdout}; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("not running (no-auto-spawn probe found no live PID)"),
        "script must report the daemon as absent via its own no-auto-spawn probe; stdout: {stdout}"
    );
    assert!(
        stdout.contains("SKIPPED: no running daemon detected"),
        "daemon-backed diagnostics must report an explicit skip; stdout: {stdout}"
    );

    let invocation_log = fs::read_to_string(&log_path).unwrap_or_default();
    for forbidden in ["daemon-status", "workspace-status", "health", "search"] {
        assert!(
            !invocation_log.contains(forbidden),
            "the fixture `engram` executable must never be invoked with '{forbidden}' when no \
             daemon is running -- that would be exactly the auto-spawn this fix prevents; \
             invocation log: {invocation_log}"
        );
    }
    assert!(
        invocation_log.contains("--version"),
        "the read-only `--version` probe (no workspace/daemon interaction) must still run; \
         invocation log: {invocation_log}"
    );
}

#[test]
fn diagnose_engram_treats_pid_reuse_fingerprint_mismatch_as_no_daemon() {
    let fixture = TempDir::new().expect("fixture dir");
    let log_path = write_engram_fixture(fixture.path());

    let workspace = TempDir::new().expect("workspace dir");
    let run_dir = workspace.path().join(".engram").join("run");
    fs::create_dir_all(&run_dir).expect("create run dir");

    // This process's PID is genuinely live, but the recorded `start_time_unix`
    // fingerprint (2, well below any real Unix timestamp) deliberately does
    // not match this process's actual start time. A PID-reuse scenario looks
    // exactly like this: the OS has reassigned a previously-recorded PID to
    // an unrelated live process. The probe must treat the fingerprint
    // mismatch the same as "no daemon" rather than trusting bare liveness.
    let pid_record = format!(r#"{{"pid": {}, "start_time_unix": 2}}"#, std::process::id());
    fs::write(run_dir.join("engram.pid"), pid_record).expect("write mismatched pid record");

    let output = run_diagnose_script(workspace.path(), fixture.path(), &log_path);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert_eq!(
        output.status.code(),
        Some(1),
        "script must report a failure exit code when daemon-backed diagnostics are skipped; \
         stdout: {stdout}; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("not running (no-auto-spawn probe found no live PID)"),
        "a fingerprint mismatch must be reported identically to no daemon present; \
         stdout: {stdout}"
    );

    let invocation_log = fs::read_to_string(&log_path).unwrap_or_default();
    for forbidden in ["daemon-status", "workspace-status", "health", "search"] {
        assert!(
            !invocation_log.contains(forbidden),
            "a PID-reuse fingerprint mismatch must never be treated as a live daemon -- that \
             would violate the read-only contract; invocation log: {invocation_log}"
        );
    }
}

#[test]
fn diagnose_engram_treats_malformed_start_time_unix_as_no_daemon() {
    let fixture = TempDir::new().expect("fixture dir");
    let log_path = write_engram_fixture(fixture.path());

    let workspace = TempDir::new().expect("workspace dir");
    let run_dir = workspace.path().join(".engram").join("run");
    fs::create_dir_all(&run_dir).expect("create run dir");

    // This process's PID is genuinely live, but the recorded `start_time_unix`
    // fingerprint is negative -- a value Rust's `start_time_unix: u64` field
    // could never deserialize (it would instead fail to parse as structured
    // JSON entirely and fall back to legacy-numeric-or-absent handling). A
    // negative fingerprint is therefore malformed metadata, not a genuine
    // "no fingerprint recorded" legacy record, and must take the same safe
    // skip path as an unparseable one rather than being silently ignored and
    // falling through to a liveness-only match.
    let pid_record = format!(
        r#"{{"pid": {}, "start_time_unix": -5}}"#,
        std::process::id()
    );
    fs::write(run_dir.join("engram.pid"), pid_record).expect("write malformed pid record");

    let output = run_diagnose_script(workspace.path(), fixture.path(), &log_path);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert_eq!(
        output.status.code(),
        Some(1),
        "script must report a failure exit code when daemon-backed diagnostics are skipped; \
         stdout: {stdout}; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("not running (no-auto-spawn probe found no live PID)"),
        "a malformed start_time_unix fingerprint must be reported identically to no daemon \
         present; stdout: {stdout}"
    );

    let invocation_log = fs::read_to_string(&log_path).unwrap_or_default();
    for forbidden in ["daemon-status", "workspace-status", "health", "search"] {
        assert!(
            !invocation_log.contains(forbidden),
            "a malformed start_time_unix fingerprint must never be treated as a live daemon -- \
             that would violate the read-only contract; invocation log: {invocation_log}"
        );
    }
}

#[test]
fn diagnose_engram_continues_past_unreadable_diagnostics_log_to_summary() {
    let fixture = TempDir::new().expect("fixture dir");
    let log_path = write_engram_fixture(fixture.path());

    let workspace = TempDir::new().expect("workspace dir");
    let run_dir = workspace.path().join(".engram").join("run");
    let diagnostics_dir = workspace.path().join(".engram").join("diagnostics");
    fs::create_dir_all(&run_dir).expect("create run dir");
    fs::create_dir_all(&diagnostics_dir).expect("create diagnostics dir");

    // A live PID (this test process itself) so the daemon-backed diagnostics
    // are exercised too, isolating this test to the log-read failure alone.
    fs::write(run_dir.join("engram.pid"), std::process::id().to_string()).expect("write pid file");

    // Force a deterministic, portable `Get-Content` failure: a directory in
    // place of the expected log file raises a terminating error under
    // `$ErrorActionPreference = "Stop"` without relying on OS-specific ACLs
    // or a genuine concurrent-delete race.
    fs::create_dir_all(diagnostics_dir.join("shim-startup-failures.jsonl"))
        .expect("create directory standing in for the log file");

    let output = run_diagnose_script(workspace.path(), fixture.path(), &log_path);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert_eq!(
        output.status.code(),
        Some(1),
        "script must still exit nonzero when a diagnostic failed; stdout: {stdout}; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("FAILED to read"),
        "the tail-read failure must be reported explicitly instead of silently aborting; \
         stdout: {stdout}"
    );
    assert!(
        stdout.contains("One or more Engram diagnostics FAILED. See details above."),
        "the script must still reach and print its final summary after the tail-read failure \
         (proving it did not terminate early under $ErrorActionPreference = \"Stop\"); \
         stdout: {stdout}"
    );

    // With a live PID recorded, the daemon-backed diagnostics run normally --
    // proving the log-read failure is isolated and does not also suppress
    // the rest of the report.
    let invocation_log = fs::read_to_string(&log_path).unwrap_or_default();
    for expected in ["daemon-status", "workspace-status", "health", "search"] {
        assert!(
            invocation_log.contains(expected),
            "daemon-backed diagnostics must still run when a live daemon PID is recorded; \
             invocation log: {invocation_log}"
        );
    }
}
