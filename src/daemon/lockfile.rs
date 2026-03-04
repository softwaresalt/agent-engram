//! Daemon lockfile: PID file management via `fd-lock`.
//!
//! Acquires an exclusive lock on `.engram/run/engram.pid` to prevent multiple
//! daemon instances from binding the same IPC endpoint. Detects stale locks
//! via process liveness checks and cleans up dead daemon artifacts before
//! allowing a fresh daemon to start.

// TODO(T015): implement lockfile acquire/release/stale detection
// TODO(T053): implement crash recovery — detect stale lock, clean socket/pipe
