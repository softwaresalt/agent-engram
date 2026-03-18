//! Daemon lockfile: PID file management via `fd-lock`.
//!
//! Uses two files in `.engram/run/`:
//!
//! | File | Purpose |
//! |------|---------|
//! | `engram.lock` | Exclusive OS-level write lock target (via `fd-lock`) |
//! | `engram.pid`  | Plain text file containing the daemon's PID (never locked) |
//!
//! Separating the lock target from the PID record is required on Windows where
//! `fd-lock` uses `LockFileEx`, which creates a **mandatory** byte-range lock.
//! A mandatory lock prevents even *read* operations by other processes, so
//! `read_pid()` would fail with `ERROR_LOCK_VIOLATION` if both were the same
//! file. By keeping `engram.pid` unlocked, any process (including the shim's
//! stale-PID check) can always read the holder's PID.
//!
//! When the holding process exits (normally or via crash) the OS automatically
//! releases the lock on `engram.lock`, allowing a subsequent `acquire()` to
//! succeed. `engram.pid` is overwritten on each successful acquisition.
//!
//! # Memory model
//!
//! [`fd_lock::RwLock`] requires that the guard lifetime is bounded by the lock's
//! lifetime. To store both inside [`DaemonLock`] we `Box::leak` the lock to
//! obtain a `'static` mutable reference. The leaked allocation is intentional:
//! a daemon holds exactly one lock for its entire lifetime, and the OS reclaims
//! the file handle on process exit.

use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};

use fd_lock::RwLock;
use sysinfo::{Pid, System};
use tracing::warn;

use crate::errors::{EngramError, LockError};

/// An acquired exclusive lock on the daemon lock file.
///
/// Dropping this value releases the OS-level file lock on `engram.lock` so
/// that another daemon instance can acquire it. `engram.pid` is not deleted on
/// drop; the next successful `acquire()` overwrites the PID.
pub struct DaemonLock {
    /// Holds the OS-level write lock on `engram.lock` for its lifetime.
    _guard: fd_lock::RwLockWriteGuard<'static, File>,
    /// Path to the `.engram/run/engram.pid` file (plain, never locked).
    path: PathBuf,
    /// The process ID written into the PID file on successful acquisition.
    pid: u32,
}

impl DaemonLock {
    /// Acquire an exclusive lock on `.engram/run/engram.lock` inside `workspace`.
    ///
    /// Creates `.engram/run/` if it does not exist. On success the current
    /// process ID is written to `.engram/run/engram.pid` (a plain, unlocked
    /// file that any process can read).
    ///
    /// If `try_write()` fails with `WouldBlock` and the recorded PID in
    /// `engram.pid` belongs to a dead process (stale lockfile), both
    /// `engram.lock` and `engram.pid` are removed and acquisition is retried
    /// once. If the second attempt also fails, or if the recorded process is
    /// still alive, [`LockError::AlreadyHeld`] is returned without a retry.
    ///
    /// # Errors
    ///
    /// - [`LockError::AlreadyHeld`] — a live daemon process holds the lock.
    /// - [`LockError::AcquisitionFailed`] — directory or file creation failed
    ///   (e.g. permission denied).
    pub fn acquire(workspace: &Path) -> Result<Self, EngramError> {
        acquire_inner(workspace, true)
    }

    /// Returns the path of the acquired PID file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the process ID written into the PID file.
    pub fn pid(&self) -> u32 {
        self.pid
    }
}

/// Inner lock acquisition implementation.
///
/// When `allow_retry` is `true` and a `WouldBlock` result is found alongside
/// a stale (dead) PID, both `engram.lock` and `engram.pid` are removed and
/// `acquire_inner` is called once more with `allow_retry = false` to prevent
/// unbounded recursion.
fn acquire_inner(workspace: &Path, allow_retry: bool) -> Result<DaemonLock, EngramError> {
    let run_dir = workspace.join(".engram").join("run");

    std::fs::create_dir_all(&run_dir).map_err(|e| {
        EngramError::Lock(LockError::AcquisitionFailed {
            path: run_dir.display().to_string(),
            reason: e.to_string(),
        })
    })?;

    // Lock target: engram.lock (exclusively locked by fd-lock).
    // PID record:  engram.pid (plain file, never locked — always readable).
    // Keeping them separate avoids Windows ERROR_LOCK_VIOLATION when other
    // processes try to read the PID from the locked file.
    let lock_path = run_dir.join("engram.lock");
    let pid_path = run_dir.join("engram.pid");

    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)
        .map_err(|e| {
            EngramError::Lock(LockError::AcquisitionFailed {
                path: lock_path.display().to_string(),
                reason: e.to_string(),
            })
        })?;

    // Leak the RwLock to obtain a `'static` reference for the guard.
    // Safety contract: the OS releases the file lock when the guard drops
    // (triggered by DaemonLock::drop) or when the process exits.
    let rw_lock: &'static mut RwLock<File> = Box::leak(Box::new(RwLock::new(file)));

    match rw_lock.try_write() {
        Ok(guard) => {
            // Write PID to the plain engram.pid file.  std::fs::write truncates
            // before writing so stale bytes from a longer previous PID cannot
            // survive and produce a corrupted read on the next acquire.
            let pid = std::process::id();
            let pid_str = pid.to_string();
            std::fs::write(&pid_path, pid_str.as_bytes()).map_err(|e| {
                EngramError::Lock(LockError::AcquisitionFailed {
                    path: pid_path.display().to_string(),
                    reason: format!("write PID file failed: {e}"),
                })
            })?;

            // T053: clean up any stale Unix socket left behind by a crashed
            // daemon so the next `bind_listener` call succeeds (S039-S040).
            // On Windows, named pipes are cleaned up by the OS automatically.
            clean_stale_socket(&run_dir);

            Ok(DaemonLock {
                _guard: guard,
                path: pid_path,
                pid,
            })
        }
        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
            // engram.pid is a plain unlocked file — read_pid always succeeds
            // on all platforms, including Windows.
            match read_pid(&pid_path) {
                Some(pid) if is_process_alive(pid) => {
                    warn!(pid, "daemon lock held by live process, cannot start");
                    Err(EngramError::Lock(LockError::AlreadyHeld { pid }))
                }
                Some(pid) if allow_retry => {
                    warn!(pid, "found stale lockfile, cleaning up");
                    // The holding process is dead; remove both stale files and
                    // retry once. On most OSes the OS lock is already released when
                    // the holding process died, so the retry should succeed.
                    if let Err(e) = std::fs::remove_file(&lock_path) {
                        if e.kind() != std::io::ErrorKind::NotFound {
                            warn!(
                                path = %lock_path.display(),
                                error = %e,
                                "failed to remove stale lock file"
                            );
                        }
                    }
                    if let Err(e) = std::fs::remove_file(&pid_path) {
                        if e.kind() != std::io::ErrorKind::NotFound {
                            warn!(
                                path = %pid_path.display(),
                                error = %e,
                                "failed to remove stale PID file"
                            );
                        }
                    }
                    acquire_inner(workspace, false)
                }
                Some(pid) => {
                    // Second attempt still blocked — return AlreadyHeld.
                    warn!(pid, "stale lockfile cleanup retry failed; lock still held");
                    Err(EngramError::Lock(LockError::AlreadyHeld { pid }))
                }
                None => {
                    warn!("lockfile held but PID unreadable");
                    Err(EngramError::Lock(LockError::AlreadyHeld { pid: 0 }))
                }
            }
        }
        Err(e) => Err(EngramError::Lock(LockError::AcquisitionFailed {
            path: lock_path.display().to_string(),
            reason: e.to_string(),
        })),
    }
}

/// Check whether a process with the given PID is currently running.
///
/// Uses [`sysinfo`] to query the OS process table. Returns `false` if the
/// process is not found (dead, zombie, or never existed) or if the PID is 0.
fn is_process_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    let mut sys = System::new();
    sys.refresh_process(Pid::from_u32(pid))
}

/// Read a PID from `path`, returning `None` if the file is missing, empty, or
/// contains non-numeric content.
fn read_pid(path: &Path) -> Option<u32> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

/// Remove a stale Unix domain socket file if it exists in `run_dir`.
///
/// On Unix, a crashed daemon may leave behind `.engram/run/engram.sock`. The
/// OS does not auto-clean socket files the way it cleans file-descriptor locks,
/// so a subsequent `bind()` call on the same path would fail with `EADDRINUSE`
/// unless the file is removed first.
///
/// On Windows this is a no-op — named pipes are automatically cleaned up by
/// the OS when the server process dies.
fn clean_stale_socket(run_dir: &Path) {
    #[cfg(unix)]
    {
        let sock_path = run_dir.join("engram.sock");
        match std::fs::remove_file(&sock_path) {
            Ok(()) => {
                tracing::info!(
                    path = %sock_path.display(),
                    "removed stale IPC socket from previous daemon run"
                );
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // Nothing to clean up; this is the normal case for a clean start.
            }
            Err(e) => {
                // Warn but don't fail — `bind_listener` in `ipc_server` also
                // removes the socket, so this is defence-in-depth only.
                tracing::warn!(
                    path = %sock_path.display(),
                    error = %e,
                    "failed to remove stale IPC socket; bind_listener will retry"
                );
            }
        }
    }

    // On non-Unix platforms the socket concept doesn't apply; suppress the
    // unused-variable warning.
    #[cfg(not(unix))]
    let _ = run_dir;
}

#[cfg(test)]
mod tests {
    use super::is_process_alive;

    /// Verifies the live-process branch of `is_process_alive`.
    ///
    /// `std::process::id()` returns this process's own PID, which is guaranteed
    /// to be present in the OS process table for the entire duration of the test.
    /// sysinfo 0.30: `System::new()` + `refresh_process(pid)` probes the OS for
    /// exactly one PID without loading the full process table.
    #[test]
    fn is_process_alive_returns_true_for_live_process() {
        let pid = std::process::id();
        assert!(
            is_process_alive(pid),
            "is_process_alive({pid}) must return true for the running test process"
        );
    }

    /// Verifies the PID-0 guard branch of `is_process_alive`.
    #[test]
    fn is_process_alive_returns_false_for_pid_zero() {
        assert!(
            !is_process_alive(0),
            "is_process_alive(0) must always return false (PID-0 guard)"
        );
    }

    /// Verifies the dead/nonexistent-PID branch of `is_process_alive`.
    ///
    /// PID 99_999_999 cannot exist on any real OS:
    /// - Linux: `PID_MAX` ≤ 4_194_304
    /// - Windows: PIDs are multiples of 4, max ~4 million
    #[test]
    fn is_process_alive_returns_false_for_nonexistent_pid() {
        assert!(
            !is_process_alive(99_999_999),
            "is_process_alive(99_999_999) must return false — no real process has this PID"
        );
    }
}
