//! Integration tests for CLI-direct (daemonless) usage-telemetry emission
//! (067.006-T / t6).
//!
//! `run_direct_sync` bypasses `tools::dispatch`, so it carries its own in-process
//! emit hook. These tests run the built binary as a subprocess against a fresh
//! git workspace and assert that `.engram/metrics/<branch>/usage.jsonl` gains a
//! record that carries the caller-supplied correlation id (flag AND env) plus a
//! pinned ISO-8601-UTC timestamp, and that the id is omitted when unsupplied.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

fn engram_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_engram"))
}

/// Create a minimal git workspace (`.git/HEAD` → `main`).
fn init_git(dir: &Path) {
    let git_dir = dir.join(".git");
    fs::create_dir_all(&git_dir).expect("create .git");
    fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").expect("write HEAD");
}

/// Run `engram <args>` with an isolated data dir and optional extra env pairs.
fn run(workspace: &Path, extra_args: &[&str], env: &[(&str, &str)]) -> (i32, String) {
    let bin = engram_bin();
    let data_dir = workspace.join(".engram-test-data");
    let mut cmd = Command::new(&bin);
    cmd.args(extra_args)
        .current_dir(workspace)
        .env_remove("ENGRAM_DATA_DIR")
        .env_remove("ENGRAM_CORRELATION_ID")
        .env("ENGRAM_DATA_DIR", &data_dir);
    for (k, v) in env {
        cmd.env(k, v);
    }
    let output = cmd
        .output()
        .unwrap_or_else(|e| panic!("failed to run {}: {e}", bin.display()));
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (output.status.code().unwrap_or(-1), stderr)
}

fn usage_path(workspace: &Path) -> PathBuf {
    workspace
        .join(".engram")
        .join("metrics")
        .join("main")
        .join("usage.jsonl")
}

/// Read and parse every non-empty JSONL record from the usage file.
fn read_records(workspace: &Path) -> Vec<Value> {
    let path = usage_path(workspace);
    let content = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("usage.jsonl must exist at {}: {e}", path.display()));
    content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str::<Value>(l).expect("each line is valid JSON"))
        .collect()
}

/// A flag-supplied correlation id lands on the emitted direct-mode record with a
/// pinned ISO-8601-UTC timestamp and schema_version 2.
#[test]
fn direct_sync_flag_correlation_id_recorded() {
    let tmp = TempDir::new().expect("tempdir");
    let ws = tmp.path().canonicalize().expect("canonicalize");
    init_git(&ws);

    let (code, stderr) = run(
        &ws,
        &["sync", "--direct", "--json", "--correlation-id", "corr-direct-1"],
        &[],
    );
    assert_eq!(code, 0, "sync --direct must exit 0; stderr: {stderr}");

    let records = read_records(&ws);
    assert_eq!(records.len(), 1, "exactly one usage record expected");
    let rec = &records[0];

    assert_eq!(rec["correlation_id"], Value::String("corr-direct-1".into()));
    assert_eq!(rec["tool_name"], Value::String("sync_workspace".into()));
    assert_eq!(rec["schema_version"], Value::from(2));

    let ts = rec["timestamp"].as_str().expect("timestamp present");
    let parsed = chrono::DateTime::parse_from_rfc3339(ts).expect("timestamp is RFC3339");
    assert_eq!(
        parsed.offset().local_minus_utc(),
        0,
        "timestamp must be pinned to UTC"
    );
}

/// The `ENGRAM_CORRELATION_ID` env var is honored on the direct path.
#[test]
fn direct_sync_env_correlation_id_recorded() {
    let tmp = TempDir::new().expect("tempdir");
    let ws = tmp.path().canonicalize().expect("canonicalize");
    init_git(&ws);

    let (code, stderr) = run(
        &ws,
        &["sync", "--direct", "--json"],
        &[("ENGRAM_CORRELATION_ID", "env-corr-9")],
    );
    assert_eq!(code, 0, "sync --direct must exit 0; stderr: {stderr}");

    let records = read_records(&ws);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["correlation_id"], Value::String("env-corr-9".into()));
}

/// When no correlation id is supplied the field is omitted from the record.
#[test]
fn direct_sync_omits_correlation_id_when_absent() {
    let tmp = TempDir::new().expect("tempdir");
    let ws = tmp.path().canonicalize().expect("canonicalize");
    init_git(&ws);

    let (code, stderr) = run(&ws, &["sync", "--direct", "--json"], &[]);
    assert_eq!(code, 0, "sync --direct must exit 0; stderr: {stderr}");

    let records = read_records(&ws);
    assert_eq!(records.len(), 1);
    assert!(
        records[0].get("correlation_id").is_none(),
        "correlation_id must be omitted when unsupplied, got: {}",
        records[0]
    );
    // The pinned timestamp is present regardless of correlation id.
    assert!(records[0]["timestamp"].as_str().is_some_and(|t| !t.is_empty()));
}

/// `index --direct` records the `index_workspace` tool name.
#[test]
fn direct_index_records_index_workspace() {
    let tmp = TempDir::new().expect("tempdir");
    let ws = tmp.path().canonicalize().expect("canonicalize");
    init_git(&ws);

    let (code, stderr) = run(
        &ws,
        &["index", "--direct", "--json", "--correlation-id", "corr-idx"],
        &[],
    );
    assert_eq!(code, 0, "index --direct must exit 0; stderr: {stderr}");

    let records = read_records(&ws);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["tool_name"], Value::String("index_workspace".into()));
    assert_eq!(records[0]["correlation_id"], Value::String("corr-idx".into()));
}

/// An invalid `--correlation-id` (over the 128-char cap) is rejected with exit 2
/// and emits no record.
#[test]
fn direct_sync_rejects_invalid_correlation_id() {
    let tmp = TempDir::new().expect("tempdir");
    let ws = tmp.path().canonicalize().expect("canonicalize");
    init_git(&ws);

    // An over-cap id (JSONL line-integrity / bounded-size guard); the CLI must
    // reject it rather than truncate on the human-driven surface.
    let over_cap = "a".repeat(200);
    let (code, _stderr) = run(
        &ws,
        &["sync", "--direct", "--json", "--correlation-id", &over_cap],
        &[],
    );
    assert_eq!(code, 2, "an invalid correlation id must exit 2");
    assert!(
        !usage_path(&ws).exists(),
        "no usage record may be written when the id is rejected"
    );
}
