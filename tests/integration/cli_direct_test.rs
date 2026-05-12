//! Integration tests for `engram sync --direct` and `engram index --direct`.
//!
//! These tests run the built binary as a subprocess against a temporary
//! workspace. They verify:
//! 1. `sync --direct` on an empty workspace exits 0 and produces a valid
//!    JSON-RPC 2.0 result envelope.
//! 2. `index --direct` on an empty workspace exits 0.
//! 3. `ENGRAM_DIRECT=1` activates direct mode without the flag.
//! 4. Non-git directories are rejected with exit 2.
//!
//! Lock-contention (daemon holds lock → direct mode returns exit 2) and
//! hydration fast-path verification are tracked as follow-up backlog items.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

// ── Helpers ───────────────────────────────────────────────────────────────────

fn engram_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_engram"))
}

/// Create a minimal git workspace in `dir` (`.git/HEAD` only).
fn init_git(dir: &Path) {
    let git_dir = dir.join(".git");
    fs::create_dir_all(&git_dir).expect("create .git");
    fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").expect("write HEAD");
}

/// Run `engram <args>` with isolated data dir. Returns `(exit_code, stdout, stderr)`.
fn run_direct(workspace: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let bin = engram_bin();
    let data_dir = workspace.join(".engram-test-data");
    let output = Command::new(&bin)
        .args(extra_args)
        .current_dir(workspace)
        .env_remove("ENGRAM_DATA_DIR")
        .env("ENGRAM_DATA_DIR", &data_dir)
        .output()
        .unwrap_or_else(|e| panic!("failed to run {}: {e}", bin.display()));
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let code = output.status.code().unwrap_or(-1);
    (code, stdout, stderr)
}

/// Run `engram <args>` with `ENGRAM_DIRECT=1` and an isolated data dir.
fn run_with_env_direct(workspace: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let bin = engram_bin();
    let data_dir = workspace.join(".engram-test-data");
    let output = Command::new(&bin)
        .args(extra_args)
        .current_dir(workspace)
        .env_remove("ENGRAM_DATA_DIR")
        .env("ENGRAM_DATA_DIR", &data_dir)
        .env("ENGRAM_DIRECT", "1")
        .output()
        .unwrap_or_else(|e| panic!("failed to run {}: {e}", bin.display()));
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let code = output.status.code().unwrap_or(-1);
    (code, stdout, stderr)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// `engram sync --direct --json` exits 0 on a valid empty workspace.
#[test]
fn direct_sync_exits_zero() {
    let tmp = TempDir::new().expect("tempdir");
    let ws = tmp.path().canonicalize().expect("canonicalize");
    init_git(&ws);

    let (code, _stdout, stderr) = run_direct(&ws, &["sync", "--direct", "--json"]);
    assert_eq!(
        code, 0,
        "engram sync --direct must exit 0 on a valid workspace; stderr: {stderr}"
    );
}

/// `engram sync --direct --json` produces a JSON-RPC 2.0 result envelope.
#[test]
fn direct_sync_emits_jsonrpc_envelope() {
    let tmp = TempDir::new().expect("tempdir");
    let ws = tmp.path().canonicalize().expect("canonicalize");
    init_git(&ws);

    let (code, stdout, stderr) = run_direct(&ws, &["sync", "--direct", "--json"]);
    assert_eq!(code, 0, "sync --direct must exit 0; stderr: {stderr}");

    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("sync --direct output must be valid JSON");
    assert_eq!(
        parsed["jsonrpc"], "2.0",
        "envelope must contain jsonrpc: '2.0'"
    );
    assert!(
        parsed["result"].is_object(),
        "response must have a result field; got: {parsed}"
    );
}

/// `engram index --direct --json` exits 0 on a valid empty workspace.
#[test]
fn direct_index_exits_zero() {
    let tmp = TempDir::new().expect("tempdir");
    let ws = tmp.path().canonicalize().expect("canonicalize");
    init_git(&ws);

    let (code, _stdout, stderr) = run_direct(&ws, &["index", "--direct", "--json"]);
    assert_eq!(
        code, 0,
        "engram index --direct must exit 0 on a valid workspace; stderr: {stderr}"
    );
}

/// `ENGRAM_DIRECT=1 engram sync --json` activates direct mode without --direct flag.
#[test]
fn env_var_activates_direct_mode() {
    let tmp = TempDir::new().expect("tempdir");
    let ws = tmp.path().canonicalize().expect("canonicalize");
    init_git(&ws);

    let (code, stdout, stderr) = run_with_env_direct(&ws, &["sync", "--json"]);
    assert_eq!(
        code, 0,
        "ENGRAM_DIRECT=1 engram sync must exit 0; stderr: {stderr}"
    );

    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("env-var direct output must be valid JSON");
    assert_eq!(
        parsed["jsonrpc"], "2.0",
        "envelope must have jsonrpc: '2.0'"
    );
}

/// `engram sync --direct` returns exit 2 when the workspace path is not a git root.
#[test]
fn direct_sync_rejects_non_git_workspace() {
    let tmp = TempDir::new().expect("tempdir");
    let ws = tmp.path().canonicalize().expect("canonicalize");
    // Intentionally omit `.git` directory.

    let (code, _stdout, stderr) = run_direct(&ws, &["sync", "--direct", "--json"]);
    assert_eq!(
        code, 2,
        "sync --direct must exit 2 for a non-git workspace; stderr: {stderr}"
    );
}

/// S081: `engram sync --direct` returns exit 2 immediately when the `CozoDB`
/// database is locked by another process.
///
/// The test holds the `engram.db.lock` advisory lock from a background thread,
/// then runs the binary. Because the lock is held by a separate OS-level file
/// handle, the binary's `fd_lock::RwLock::try_write()` probe sees the file as
/// locked and must return exit 2 before attempting the 30-second `connect_db`
/// polling loop.
///
/// On Windows `LockFileEx` enforces per-handle exclusivity even within the same
/// process. On Linux/macOS, advisory `flock`/`fcntl` locks do not conflict
/// across handles in the same process, so this test is restricted to Windows.
/// Cross-process lock contention is the real production scenario and is covered
/// by the daemon-held lock in actual operation.
#[test]
#[cfg(target_os = "windows")]
fn direct_sync_detects_locked_database() {
    use std::sync::mpsc;

    let tmp = TempDir::new().expect("tempdir");
    let ws = tmp.path().canonicalize().expect("canonicalize");
    init_git(&ws);

    // Use a dedicated, separate data dir (not the one run_direct builds).
    let data_dir = tmp.path().join("locked-data");
    let cozo_dir = data_dir.join("cozo").join("main");
    fs::create_dir_all(&cozo_dir).expect("create cozo dir");
    let db_lock_path = cozo_dir.join("engram.db.lock");
    fs::write(&db_lock_path, b"").expect("create lock file");

    // Acquire the write lock from a background thread to hold it while
    // the binary runs. The thread signals readiness via a channel.
    let lock_path_clone = db_lock_path.clone();
    let (tx, rx) = mpsc::channel::<()>();
    let (done_tx, done_rx) = mpsc::channel::<()>();
    let _holder = std::thread::spawn(move || {
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&lock_path_clone)
            .expect("open lock file in holder thread");
        let mut rw = fd_lock::RwLock::new(file);
        let _guard = rw.try_write().expect("holder must acquire lock first");
        tx.send(()).expect("send ready");
        // Keep the guard alive until the test signals done.
        let _ = done_rx.recv_timeout(std::time::Duration::from_secs(15));
    });

    // Wait until the background thread holds the lock.
    rx.recv_timeout(std::time::Duration::from_secs(5))
        .expect("lock holder did not signal ready in time");

    // Run the binary; it must detect the locked DB and exit 2 quickly.
    let bin = engram_bin();
    let output = Command::new(&bin)
        .args(["sync", "--direct", "--json"])
        .current_dir(&ws)
        .env_remove("ENGRAM_DATA_DIR")
        .env("ENGRAM_DATA_DIR", &data_dir)
        .output()
        .expect("run engram binary");

    // Signal holder to release the lock (test is done regardless).
    let _ = done_tx.send(());

    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let code = output.status.code().unwrap_or(-1);

    assert_eq!(
        code, 2,
        "must exit 2 when DB is locked by another process; stderr: {stderr}"
    );
    assert!(
        stderr.contains("locked by another process"),
        "stderr must mention 'locked by another process'; got: {stderr}"
    );
}
