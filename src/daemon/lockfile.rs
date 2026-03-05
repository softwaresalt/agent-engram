//! Daemon lockfile: PID file management via `fd-lock`.
//!
//! Acquires an exclusive OS-level write lock on `.engram/run/engram.pid` to
//! prevent multiple daemon instances from serving the same workspace
//! simultaneously. When the holding process exits (normally or via crash) the
//! OS automatically releases the lock, allowing a subsequent `acquire()` to
//! succeed even if the PID file still contains the old process ID.
//!
//! # Memory model
//!
//! [`fd_lock::RwLock`] requires that the guard lifetime is bounded by the lock's
//! lifetime. To store both inside [`DaemonLock`] we `Box::leak` the lock to
//! obtain a `'static` mutable reference. The leaked allocation is intentional:
//! a daemon holds exactly one lock for its entire lifetime, and the OS reclaims
//! the file handle on process exit.

use std::fs::{File, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use fd_lock::RwLock;

use crate::errors::{EngramError, LockError};

/// An acquired exclusive lock on the daemon PID file.
///
/// Dropping this value releases the OS-level file lock so that another daemon
/// instance can acquire it. The PID file itself is not deleted on drop; the
/// next successful `acquire()` overwrites the PID.
pub struct DaemonLock {
    /// Holds the OS-level write lock for its lifetime.
    _guard: fd_lock::RwLockWriteGuard<'static, File>,
    /// Path to the `.engram/run/engram.pid` file.
    path: PathBuf,
    /// The process ID written into the PID file on successful acquisition.
    pid: u32,
}

impl DaemonLock {
    /// Acquire an exclusive lock on `.engram/run/engram.pid` inside `workspace`.
    ///
    /// Creates `.engram/run/` if it does not exist. On success the current
    /// process ID is written to the PID file.
    ///
    /// If the file exists but the owning process is dead (stale lock), the OS
    /// already released the lock, so `try_write()` succeeds and we overwrite
    /// the stale PID.
    ///
    /// # Errors
    ///
    /// - [`LockError::AlreadyHeld`] — a live daemon process holds the lock.
    /// - [`LockError::AcquisitionFailed`] — directory or file creation failed
    ///   (e.g. permission denied).
    pub fn acquire(workspace: &Path) -> Result<Self, EngramError> {
        let run_dir = workspace.join(".engram").join("run");

        std::fs::create_dir_all(&run_dir).map_err(|e| {
            EngramError::Lock(LockError::AcquisitionFailed {
                path: run_dir.display().to_string(),
                reason: e.to_string(),
            })
        })?;

        let pid_path = run_dir.join("engram.pid");

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&pid_path)
            .map_err(|e| {
                EngramError::Lock(LockError::AcquisitionFailed {
                    path: pid_path.display().to_string(),
                    reason: e.to_string(),
                })
            })?;

        // Leak the RwLock to obtain a `'static` reference for the guard.
        // Safety contract: the OS releases the file lock when the guard drops
        // (triggered by DaemonLock::drop) or when the process exits.
        let rw_lock: &'static mut RwLock<File> = Box::leak(Box::new(RwLock::new(file)));

        match rw_lock.try_write() {
            Ok(mut guard) => {
                // Truncate to zero before writing our PID so that stale bytes
                // from a longer previous PID (e.g. "12345678" → "99") do not
                // survive and produce a corrupted PID on the next read.
                guard.set_len(0).map_err(|e| {
                    EngramError::Lock(LockError::AcquisitionFailed {
                        path: pid_path.display().to_string(),
                        reason: format!("truncate PID file failed: {e}"),
                    })
                })?;
                guard.seek(SeekFrom::Start(0)).map_err(|e| {
                    EngramError::Lock(LockError::AcquisitionFailed {
                        path: pid_path.display().to_string(),
                        reason: e.to_string(),
                    })
                })?;

                let pid = std::process::id();
                let pid_str = pid.to_string();
                guard.write_all(pid_str.as_bytes()).map_err(|e| {
                    EngramError::Lock(LockError::AcquisitionFailed {
                        path: pid_path.display().to_string(),
                        reason: e.to_string(),
                    })
                })?;

                guard.flush().map_err(|e| {
                    EngramError::Lock(LockError::AcquisitionFailed {
                        path: pid_path.display().to_string(),
                        reason: e.to_string(),
                    })
                })?;

                // T053: clean up any stale Unix socket left behind by a crashed
                // daemon so the next `bind_listener` call succeeds (S039-S040).
                // On Windows, named pipes are cleaned up by the OS automatically.
                clean_stale_socket(&run_dir);

                Ok(Self {
                    _guard: guard,
                    path: pid_path,
                    pid,
                })
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                let pid = read_pid(&pid_path).unwrap_or(0);
                Err(EngramError::Lock(LockError::AlreadyHeld { pid }))
            }
            Err(e) => Err(EngramError::Lock(LockError::AcquisitionFailed {
                path: pid_path.display().to_string(),
                reason: e.to_string(),
            })),
        }
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
