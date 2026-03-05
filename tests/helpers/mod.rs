//! Daemon process test harness.
//!
//! Provides [`DaemonHarness`] for spawning an `engram daemon` subprocess in
//! integration tests. Each instance gets its own [`TempDir`] workspace so
//! tests never share `SurrealDB` state. The daemon is killed deterministically
//! when the harness is dropped.
//!
//! # Platform notes
//!
//! - **Unix / macOS**: IPC endpoint is a Unix domain socket at
//!   `{workspace}/.engram/run/engram.sock`. Ready detection polls via an IPC
//!   health-check request (more reliable than filesystem presence alone).
//! - **Windows**: IPC endpoint is a named pipe at
//!   `\\.\pipe\engram-{sha256_prefix_16}`, where `sha256_prefix_16` is the
//!   first 16 hex characters of the SHA-256 hash of the **canonical** workspace
//!   path string, matching the daemon's own naming logic (ADR 0015). Ready
//!   detection uses an IPC health-check because `std::fs::metadata` does not
//!   detect named-pipe server readiness on Windows.
//!
//! # Usage (Phase 3+)
//!
//! ```rust,no_run
//! # use std::time::Duration;
//! # tokio_test::block_on(async {
//! // Requires the daemon to be implemented (Phase 2+).
//! // let harness = DaemonHarness::spawn(Duration::from_secs(5)).await.unwrap();
//! // let _ipc = harness.ipc_path();
//! # })
//! ```

// Allow dead code at the module level: the harness is infrastructure for
// Phase 3 tests (T020-T025) which do not exist yet.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::Duration;

use tempfile::TempDir;

/// Compute the IPC endpoint path for a canonical workspace path.
///
/// - **Unix / macOS**: `{workspace}/.engram/run/engram.sock`
/// - **Windows**: `\\.\pipe\engram-{sha256_prefix_16}` where
///   `sha256_prefix_16` is the first 16 hex characters (8 bytes) of the
///   SHA-256 hash of the canonical workspace path string, matching
///   [`src/daemon/lockfile.rs`] naming (ADR 0015).
fn ipc_path_for_workspace(workspace: &Path) -> PathBuf {
    #[cfg(not(windows))]
    {
        workspace.join(".engram").join("run").join("engram.sock")
    }

    #[cfg(windows)]
    {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        hasher.update(workspace.to_string_lossy().as_bytes());
        let digest = hasher.finalize();
        // First 8 bytes → 16 lowercase hex characters.
        let prefix = hex::encode(&digest[..8]);
        PathBuf::from(format!(r"\\.\pipe\engram-{prefix}"))
    }
}

/// Returns `true` if the IPC endpoint is accepting health-check requests.
///
/// Uses an actual `_health` IPC request instead of a filesystem probe because:
/// - On **Unix**, a socket file can exist before the daemon enters its accept
///   loop, causing false positives.
/// - On **Windows**, `std::fs::metadata` does not detect named-pipe server
///   readiness — the `\\.\pipe\*` namespace is not accessible via the normal
///   file-metadata API on all configurations.
///
/// A successful response (no error, `status == "ready"`) means the daemon is
/// fully initialized and ready to serve tool calls.
async fn ipc_ready(path: &Path) -> bool {
    let Some(endpoint) = path.to_str() else {
        return false;
    };
    engram::shim::lifecycle::check_health(endpoint).await
}

/// Test harness for spawning an `engram daemon` subprocess.
///
/// Starts the daemon with a temporary workspace directory, waits until the
/// IPC socket/pipe is ready (polling with exponential backoff), and ensures
/// the daemon process is killed when the harness is dropped.
///
/// Each call to [`DaemonHarness::spawn`] creates an isolated workspace so
/// concurrent tests cannot share or corrupt each other's state.
pub struct DaemonHarness {
    /// Temporary workspace directory (auto-cleaned on drop via [`TempDir`]).
    pub workspace: TempDir,
    /// Child process handle; killed synchronously in [`Drop::drop`].
    child: Child,
    /// Resolved IPC endpoint path for this workspace.
    ipc_path: PathBuf,
}

impl DaemonHarness {
    /// Spawn a daemon for a fresh temporary workspace and wait for IPC ready.
    ///
    /// Polls for the IPC socket/pipe path to appear with exponential backoff
    /// (starting at 10 ms, doubling each attempt, capped at 500 ms per step,
    /// for up to 30 attempts). Whichever limit is reached first — attempt cap
    /// or `timeout` wall-clock — triggers a `kill` of the child and an `Err`
    /// return.
    ///
    /// # Errors
    ///
    /// Returns `Err` if:
    /// - The temporary directory cannot be created.
    /// - The workspace path cannot be canonicalized.
    /// - The `engram` binary cannot be spawned (e.g., not on `PATH`).
    /// - The IPC endpoint does not become ready within `timeout` or 30
    ///   attempts.
    pub async fn spawn(timeout: Duration) -> Result<Self, Box<dyn std::error::Error>> {
        const MAX_ATTEMPTS: u32 = 30;

        let workspace = TempDir::new()?;
        let workspace_path = workspace.path().canonicalize()?;
        let ipc_path = ipc_path_for_workspace(&workspace_path);

        // Create a minimal `.git` directory so the daemon accepts this as a workspace.
        // `canonicalize_workspace()` rejects paths where `.git` is not a directory.
        let git_dir = workspace_path.join(".git");
        std::fs::create_dir_all(&git_dir)?;
        std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n")?;

        let workspace_str = workspace_path
            .to_str()
            .ok_or("workspace path contains non-UTF-8 characters")?;

        let child = Command::new(env!("CARGO_BIN_EXE_engram"))
            .args(["daemon", "--workspace", workspace_str])
            .spawn()?;

        let deadline = std::time::Instant::now() + timeout;
        let mut delay = Duration::from_millis(10);
        let mut attempt: u32 = 0;

        loop {
            if ipc_ready(&ipc_path).await {
                return Ok(Self {
                    workspace,
                    child,
                    ipc_path,
                });
            }

            attempt += 1;
            if attempt >= MAX_ATTEMPTS || std::time::Instant::now() >= deadline {
                let mut child = child;
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "daemon IPC endpoint did not become ready within {timeout:?} \
                     ({attempt} attempts); expected path: {}",
                    ipc_path.display()
                )
                .into());
            }

            tokio::time::sleep(delay).await;
            delay = (delay * 2).min(Duration::from_millis(500));
        }
    }

    /// Returns the path to the IPC endpoint for this workspace.
    #[must_use]
    pub fn ipc_path(&self) -> &Path {
        &self.ipc_path
    }
}

impl Drop for DaemonHarness {
    fn drop(&mut self) {
        // Best-effort cleanup: ignore errors so drop never panics.
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify that the Unix IPC path matches the expected socket location.
    #[test]
    #[cfg(not(windows))]
    fn ipc_path_unix_format() {
        let workspace = Path::new("/tmp/test-workspace");
        let path = ipc_path_for_workspace(workspace);
        assert_eq!(
            path,
            Path::new("/tmp/test-workspace/.engram/run/engram.sock"),
            "Unix IPC path must be {{workspace}}/.engram/run/engram.sock"
        );
    }

    /// Verify that the Windows IPC pipe name matches the expected format.
    #[test]
    #[cfg(windows)]
    fn ipc_path_windows_format() {
        let workspace = Path::new(r"C:\Users\test\workspace");
        let path = ipc_path_for_workspace(workspace);
        let path_str = path.to_str().expect("pipe path is valid UTF-8");

        assert!(
            path_str.starts_with(r"\\.\pipe\engram-"),
            "Windows IPC path must start with {{pipe prefix}}, got: {path_str}"
        );

        let hash_part = path_str
            .strip_prefix(r"\\.\pipe\engram-")
            .expect("prefix already verified");
        assert_eq!(
            hash_part.len(),
            16,
            "hash suffix must be exactly 16 hex characters, got: {hash_part}"
        );
        assert!(
            hash_part.chars().all(|c| c.is_ascii_hexdigit()),
            "hash suffix must be lowercase hex, got: {hash_part}"
        );
    }

    /// Verify that two different workspace paths produce different pipe names
    /// (collision resistance sanity check).
    #[test]
    #[cfg(windows)]
    fn ipc_path_windows_unique_per_workspace() {
        let a = ipc_path_for_workspace(Path::new(r"C:\workspace-a"));
        let b = ipc_path_for_workspace(Path::new(r"C:\workspace-b"));
        assert_ne!(a, b, "distinct workspaces must produce distinct pipe names");
    }
}
