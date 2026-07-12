//! CLI exit-code surfacing for retrieval-eval thresholds (084.007-T / 14B33F9F).
//!
//! Exercises the full `engram eval` process boundary against a live daemon so
//! the threshold outcome recorded by `run_retrieval_eval` (084.006-T) is
//! observed through the real CLI, not just the pure `eval_exit_code` mapping
//! unit-tested in `src/cli/commands/eval.rs`.
//!
//! Scenarios (mirroring the plan's ≤3 verification points):
//!   1. breach → dedicated non-zero exit (`3`): an impossible recall floor
//!      (`min_resolution_recall = 2.0`) is breached once the corpus is
//!      non-empty;
//!   2. enabled + thresholds met → exit `0`: the permissive default thresholds
//!      cannot be breached by a real run.
//!
//! The disabled/empty-run contract (exit `0`) is covered at the same process
//! boundary by `contract_cli_eval::eval_empty_run_json_and_quiet_contract`.
//!
//! Isolation: both `run_cli` and the daemon clear `ENGRAM_DATA_DIR` so the
//! workspace-local `.engram` data directory is used, never the developer DB.

#[path = "../helpers/mod.rs"]
mod helpers;

use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

const READY_TIMEOUT: Duration = Duration::from_secs(20);
/// Upper bound for driving the fixture into the corpus before asserting.
const INDEX_DEADLINE: Duration = Duration::from_secs(90);
/// Upper bound for the eval to settle on a terminal threshold verdict.
const EVAL_DEADLINE: Duration = Duration::from_secs(30);

/// A single in-file call site (`alpha` → `beta`) so the eval sees a non-empty
/// call-site inventory (the threshold empty-guard passes) and a measurable
/// resolution recall (`<= 1.0`, so a floor of `2.0` is always breached).
const FIXTURE_SRC: &str =
    "pub fn alpha() {\n    beta();\n}\n\npub fn beta() {\n    let _ = 1;\n}\n";

/// Run the `engram` binary with `args`, isolated from the developer data dir.
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

/// Prepare a git-backed workspace with the given `[retrieval_eval]` config body
/// and the single-call-site fixture, returning the owning temp dir.
fn prepare_workspace(config_toml: &str) -> tempfile::TempDir {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    // `.git/HEAD` so the daemon accepts the workspace as a repo root.
    let git = ws.join(".git");
    std::fs::create_dir_all(&git).expect("create .git");
    std::fs::write(git.join("HEAD"), "ref: refs/heads/main\n").expect("write HEAD");
    // Workspace config enabling retrieval-eval with the caller's thresholds.
    let engram_dir = ws.join(".engram");
    std::fs::create_dir_all(&engram_dir).expect("create .engram");
    std::fs::write(engram_dir.join("config.toml"), config_toml).expect("write config.toml");
    // A source file with one resolvable in-file call.
    let src = ws.join("src");
    std::fs::create_dir_all(&src).expect("create src");
    std::fs::write(src.join("a.rs"), FIXTURE_SRC).expect("write fixture");
    tmp
}

/// Drive an incremental index to completion, tolerating the transient
/// `IndexInProgress` the daemon's background bind-scan can raise. Returns once
/// `engram sync` reports success (exit `0`) — at which point the fixture is in
/// the corpus (`list_code_files` non-empty) — or panics on deadline.
fn ensure_indexed(ws: &str) {
    let deadline = Instant::now() + INDEX_DEADLINE;
    loop {
        let (code, _out, err) = run_cli(&["--workspace", ws, "sync"]);
        if code == 0 {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "workspace did not finish indexing within {INDEX_DEADLINE:?}; last stderr:\n{err}"
        );
        std::thread::sleep(Duration::from_millis(400));
    }
}

/// Poll `engram --quiet eval` until it returns a terminal threshold verdict —
/// `0` (met / empty) or `3` (breached) — tolerating the transient tool error
/// (exit `1`, e.g. a `SQLITE_BUSY` while the daemon's post-index coalesced sync
/// writes) or connection hiccup (exit `2`) that can occur immediately after
/// indexing. The verdict is deterministic once the corpus is settled; only
/// non-terminal codes are retried, so a genuine pass/breach is never masked.
fn eval_until_settled(ws: &str) -> i32 {
    let deadline = Instant::now() + EVAL_DEADLINE;
    loop {
        let (code, _out, err) = run_cli(&["--workspace", ws, "--quiet", "eval"]);
        if code == 0 || code == 3 {
            return code;
        }
        assert!(
            Instant::now() < deadline,
            "eval never returned a terminal verdict within {EVAL_DEADLINE:?} \
             (last exit {code}); stderr:\n{err}"
        );
        std::thread::sleep(Duration::from_millis(400));
    }
}

/// Spawn a daemon for the prepared workspace, drive indexing, and return the
/// settled `engram --quiet eval` exit code.
async fn eval_exit_for(ws_path: &Path) -> i32 {
    // Keep the harness bound for the whole test; it kills the daemon on drop.
    let _harness = helpers::DaemonHarness::spawn_for_workspace(ws_path, READY_TIMEOUT)
        .await
        .expect("daemon must spawn for the prepared workspace");
    let ws = ws_path.to_str().expect("workspace path must be UTF-8");
    ensure_indexed(ws);
    eval_until_settled(ws)
}

// ── Verification 1: breach → dedicated non-zero exit ─────────────────────────

#[tokio::test]
async fn breached_recall_floor_exits_nonzero() {
    // An impossible recall floor (2.0 exceeds any achievable recall) guarantees
    // a breach once the corpus is non-empty.
    let tmp = prepare_workspace(
        "[retrieval_eval]\nenabled = true\n\n\
         [retrieval_eval.thresholds]\nmin_resolution_recall = 2.0\n",
    );
    let code = eval_exit_for(tmp.path()).await;
    assert_ne!(
        code, 0,
        "a breached recall floor must surface as a non-zero exit"
    );
    assert_eq!(
        code, 3,
        "a breach maps to the dedicated EXIT_THRESHOLDS_BREACHED code (3)"
    );
}

// ── Verification 2: enabled + thresholds met → exit 0 ────────────────────────

#[tokio::test]
async fn met_thresholds_exit_zero() {
    // Enabled, but with the permissive default thresholds (recall floor 0.0,
    // false-edge ceiling 1.0): a real run cannot breach them.
    let tmp = prepare_workspace("[retrieval_eval]\nenabled = true\n");
    let code = eval_exit_for(tmp.path()).await;
    assert_eq!(
        code, 0,
        "an enabled run that meets its thresholds must exit 0"
    );
}
