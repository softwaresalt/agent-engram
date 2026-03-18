//! Unit tests for the daemon lockfile (`DaemonLock`) (T011).
//!
//! Scenarios covered:
//! - S027: `acquire()` on a fresh directory → success; PID file contains our PID
//! - S029: `acquire()` when PID file has a dead PID but no OS lock → success (stale)
//! - S030: `acquire()` when `.engram/run/` is read-only → returns `LockError`
//! - S032: `acquire()` → drop → `acquire()` again → second acquire succeeds

use std::fs;
use std::io::Write;

use tempfile::TempDir;

use engram::daemon::lockfile::DaemonLock;
// EngramError is only used in the Unix-only S030 test; import it there locally.

#[test]
fn s027_acquire_on_fresh_workspace_succeeds_and_writes_pid() {
    let dir = TempDir::new().expect("tempdir");
    let workspace = dir.path();

    let lock = DaemonLock::acquire(workspace).expect("should acquire lock");

    // PID file must exist at the expected path (engram.pid — plain, unlocked)
    let pid_path = workspace.join(".engram").join("run").join("engram.pid");
    assert!(pid_path.exists(), "PID file must be created");
    assert_eq!(lock.path(), pid_path);

    // Lock file must also exist (engram.lock — fd-lock target)
    let lock_path = workspace.join(".engram").join("run").join("engram.lock");
    assert!(lock_path.exists(), "lock file must be created");

    // PID must be the current process ID.  Because engram.pid is a plain
    // unlocked file on all platforms we can also verify the written value.
    assert_eq!(
        lock.pid(),
        std::process::id(),
        "lock must report current process PID"
    );
    let written: u32 = fs::read_to_string(&pid_path)
        .expect("engram.pid must be readable")
        .trim()
        .parse()
        .expect("engram.pid must contain a numeric PID");
    assert_eq!(
        written,
        std::process::id(),
        "engram.pid must match current PID"
    );
}

// ── S029: stale lock (dead PID, no OS lock) ───────────────────────────────────

#[test]
fn s029_acquire_with_stale_pid_file_succeeds() {
    let dir = TempDir::new().expect("tempdir");
    let workspace = dir.path();

    // Pre-create the run directory and write a "dead" PID (astronomically high
    // PID that can't correspond to a real running process on any sane system).
    let run_dir = workspace.join(".engram").join("run");
    fs::create_dir_all(&run_dir).expect("create run dir");
    let pid_path = run_dir.join("engram.pid");
    let mut f = fs::File::create(&pid_path).expect("create pid file");
    // Write a PID with no OS lock on it (no fd-lock held on engram.lock)
    writeln!(f, "99999999").expect("write fake pid");
    drop(f); // close the file — no lock held

    // Now acquire should succeed because no fd-lock is held on engram.lock
    let lock = DaemonLock::acquire(workspace).expect("should acquire stale lock");
    assert_eq!(
        lock.pid(),
        std::process::id(),
        "should overwrite stale PID with ours"
    );
}

// ── S030: read-only directory (Unix only) ────────────────────────────────────

#[cfg(not(windows))]
#[test]
fn s030_acquire_with_readonly_run_dir_returns_lock_error() {
    use std::os::unix::fs::PermissionsExt;

    use engram::errors::EngramError;

    let dir = TempDir::new().expect("tempdir");
    let workspace = dir.path();

    // Create the run directory then make it read-only (no write or exec for owner)
    let run_dir = workspace.join(".engram").join("run");
    fs::create_dir_all(&run_dir).expect("create run dir");
    fs::set_permissions(&run_dir, fs::Permissions::from_mode(0o444)).expect("set read-only");

    let result = DaemonLock::acquire(workspace);

    // Restore permissions so TempDir can clean up
    fs::set_permissions(&run_dir, fs::Permissions::from_mode(0o755)).ok();

    match result {
        Err(EngramError::Lock(_)) => {} // expected
        Err(other) => panic!("expected LockError, got: {other:?}"),
        Ok(_) => panic!("expected an error for read-only directory"),
    }
}

// ── S032: acquire → drop → re-acquire ────────────────────────────────────────

#[test]
fn s032_acquire_drop_acquire_again_succeeds() {
    let dir = TempDir::new().expect("tempdir");
    let workspace = dir.path();

    // First acquire
    let lock1 = DaemonLock::acquire(workspace).expect("first acquire");
    let pid_path = lock1.path().to_path_buf();

    // Release the lock by dropping
    drop(lock1);

    // Second acquire on the same workspace should succeed
    let lock2 = DaemonLock::acquire(workspace).expect("second acquire after drop");
    assert_eq!(lock2.path(), pid_path, "same PID file path");
    assert_eq!(lock2.pid(), std::process::id());
}
