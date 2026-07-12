//! Integration test for the `engram migrate-down calls-resolution` operator
//! subcommand (082.013-T rollback trigger).
//!
//! Runs the built `engram` binary as a subprocess against a temporary workspace
//! so the actual clap parse + dispatch path is exercised end-to-end. The
//! workspace DB is pre-populated through the public `CodeGraphQueries` surface,
//! then the CLI entry point is invoked to drive the 082.010-T
//! `rollback_calls_resolution` down-migration.
//!
//! Scenarios (3):
//!   1. the subcommand parses and dispatches, exiting 0 and printing the
//!      retracted-edge count;
//!   2. on a workspace with tagged edges it retracts every
//!      `calls_resolved_singleton` edge (direct preserved), verified via a fresh
//!      DB open;
//!   3. a second invocation is idempotent — exit 0, zero further retractions.

#![allow(clippy::needless_raw_string_hashes)]
#![allow(clippy::doc_markdown)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;
use tokio::test;

use engram::db::connect_db;
use engram::db::queries::CodeGraphQueries;

/// Branch name written into `.git/HEAD`; both the subprocess and the in-test DB
/// opens resolve to this branch.
const BRANCH: &str = "main";

fn engram_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_engram"))
}

/// Create a minimal git workspace (`.git/HEAD` only) so branch resolution yields
/// a stable `main` branch.
fn init_git(dir: &Path) {
    let git_dir = dir.join(".git");
    fs::create_dir_all(&git_dir).expect("create .git");
    fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").expect("write HEAD");
}

/// Run `engram migrate-down calls-resolution --json` against `workspace` with an
/// isolated data dir. Returns `(exit_code, stdout, stderr)`.
fn run_migrate_down(workspace: &Path, data_dir: &Path) -> (i32, String, String) {
    let output = Command::new(engram_bin())
        .args(["migrate-down", "calls-resolution", "--json"])
        .current_dir(workspace)
        .env_remove("ENGRAM_DIRECT")
        .env_remove("ENGRAM_DATA_DIR")
        .env("ENGRAM_DATA_DIR", data_dir)
        .output()
        .expect("failed to run engram migrate-down");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let code = output.status.code().unwrap_or(-1);
    (code, stdout, stderr)
}

/// Parse `result.retracted_singleton_edges` out of a JSON-RPC success envelope.
fn retracted_count(stdout: &str) -> u64 {
    let parsed: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("migrate-down output must be valid JSON");
    assert_eq!(parsed["jsonrpc"], "2.0", "must be a JSON-RPC envelope");
    parsed["result"]["retracted_singleton_edges"]
        .as_u64()
        .expect("retracted_singleton_edges must be a number")
}

#[test]
async fn migrate_down_retracts_singletons_then_is_idempotent() {
    let tmp = TempDir::new().expect("tempdir");
    let ws = tmp.path().canonicalize().expect("canonicalize");
    init_git(&ws);
    let data_dir = ws.join(".engram-test-data");

    // Pre-populate: one direct edge and two tagged singleton edges. The handle
    // is dropped before invoking the subprocess so the CozoDB lock is released.
    {
        let db = connect_db(&data_dir, BRANCH).await.expect("connect_db");
        let q = CodeGraphQueries::new(db);
        q.create_calls_edge("fn:a", "fn:b")
            .await
            .expect("direct edge");
        q.create_calls_edge_with_resolution("fn:c", "fn:d", "calls_resolved_singleton")
            .await
            .expect("singleton edge 1");
        q.create_calls_edge_with_resolution("fn:e", "fn:f", "calls_resolved_singleton")
            .await
            .expect("singleton edge 2");
        drop(q);
    }

    // Scenario 1 + 2: the subcommand parses, dispatches, and retracts both
    // tagged edges.
    let (code, stdout, stderr) = run_migrate_down(&ws, &data_dir);
    assert_eq!(code, 0, "migrate-down must exit 0; stderr: {stderr}");
    assert_eq!(
        retracted_count(&stdout),
        2,
        "both singleton edges must be retracted; stdout: {stdout}"
    );

    // Scenario 2 (end-state): a fresh open confirms the singletons are gone and
    // the direct edge survived. (Re-opening re-runs the up-migration, so the
    // resolution column is present again; the column-drop itself is pinned by
    // the schema-layer unit test in src/db/cozo_backend/schema.rs.)
    {
        let db = connect_db(&data_dir, BRANCH).await.expect("reopen db");
        let q = CodeGraphQueries::new(db);
        let counts = q
            .count_calls_edges_by_resolution()
            .await
            .expect("count by resolution");
        assert_eq!(
            counts.get("calls_resolved_singleton"),
            None,
            "no singleton edges may survive rollback; {counts:?}"
        );
        assert_eq!(
            counts.get("direct"),
            Some(&1),
            "the direct edge must survive rollback; {counts:?}"
        );
        drop(q);
    }

    // Scenario 3: a second invocation is idempotent — exit 0, zero retractions.
    let (code2, stdout2, stderr2) = run_migrate_down(&ws, &data_dir);
    assert_eq!(
        code2, 0,
        "second migrate-down must exit 0; stderr: {stderr2}"
    );
    assert_eq!(
        retracted_count(&stdout2),
        0,
        "a re-run must retract nothing; stdout: {stdout2}"
    );
}

#[test]
async fn migrate_down_rejects_unknown_target() {
    let tmp = TempDir::new().expect("tempdir");
    let ws = tmp.path().canonicalize().expect("canonicalize");
    init_git(&ws);
    let data_dir = ws.join(".engram-test-data");

    let output = Command::new(engram_bin())
        .args(["migrate-down", "no-such-target", "--json"])
        .current_dir(&ws)
        .env_remove("ENGRAM_DATA_DIR")
        .env("ENGRAM_DATA_DIR", &data_dir)
        .output()
        .expect("failed to run engram migrate-down");
    let code = output.status.code().unwrap_or(-1);
    assert_eq!(code, 2, "an unknown target must exit 2 (invocation error)");
}
