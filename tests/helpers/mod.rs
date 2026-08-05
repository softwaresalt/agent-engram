//! Daemon process test harness.
//!
//! This module bundles the [`DaemonHarness`] subprocess fixture for integration
//! tests.
//!
//! Provides [`DaemonHarness`] for spawning an `engram daemon` subprocess in
//! integration tests. Each instance gets its own [`TempDir`] workspace so
//! tests never share `CozoDB` state. The daemon is killed deterministically
//! when the harness is dropped.
//!
//! # Platform notes
//!
//! - **Unix / macOS**: IPC endpoint is a Unix domain socket at
//!   `{workspace}/.engram/run/engram.sock`. Ready detection polls via an IPC
//!   health-check request (more reliable than filesystem presence alone).
//! - **Windows**: IPC endpoint is a named pipe at `\\.\pipe\engram-{key}`,
//!   where `{key}` is resolved by the daemon's production endpoint helper.
//!   Ready detection uses an IPC health-check because `std::fs::metadata` does
//!   not detect named-pipe server readiness on Windows.
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
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

use engram::models::config::WorkspaceConfig;
use engram::server::state::{AppState, WorkspaceSnapshot};
use tempfile::TempDir;

const CHILD_REAP_LIMIT: Duration = Duration::from_secs(5);
#[cfg(all(unix, target_os = "macos"))]
const MAX_UNIX_SOCKET_PATH_LEN: usize = 103;
#[cfg(all(unix, not(target_os = "macos")))]
const MAX_UNIX_SOCKET_PATH_LEN: usize = 107;

fn paths_overlap(left: &Path, right: &Path) -> bool {
    left == right || left.starts_with(right) || right.starts_with(left)
}

/// Read the repository-owned IPC endpoint without creating workspace state.
///
/// On Windows the endpoint is UUID-based, so an absent persisted identity
/// yields `None` rather than calling the production create-on-read helper.
///
/// # Errors
///
/// Returns `Err` when the repository cannot be canonicalized, its persisted
/// identity cannot be read, or that identity is malformed.
pub fn repository_ipc_endpoint_if_known(
    repository: &Path,
) -> Result<Option<PathBuf>, Box<dyn std::error::Error>> {
    let repository = repository.canonicalize()?;

    #[cfg(windows)]
    {
        let id_path = repository.join(".engram").join(".workspace-id");
        if !id_path.is_file() {
            return Ok(None);
        }
        let raw = std::fs::read_to_string(id_path)?;
        let workspace_id = uuid::Uuid::parse_str(raw.trim())?;
        Ok(Some(PathBuf::from(format!(
            r"\\.\pipe\engram-{workspace_id}"
        ))))
    }

    #[cfg(unix)]
    {
        let local_endpoint = repository.join(".engram").join("run").join("engram.sock");
        let local_endpoint_text = local_endpoint.to_str().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "repository IPC endpoint is not valid UTF-8",
            )
        })?;
        if local_endpoint_text.len() <= MAX_UNIX_SOCKET_PATH_LEN {
            return Ok(Some(local_endpoint));
        }

        let id_path = repository.join(".engram").join(".workspace-id");
        if !id_path.is_file() {
            return Ok(None);
        }
        let raw = std::fs::read_to_string(id_path)?;
        let workspace_id = uuid::Uuid::parse_str(raw.trim())?;
        Ok(Some(PathBuf::from(format!(
            "/tmp/engram-{workspace_id}/engram.sock"
        ))))
    }

    #[cfg(not(any(unix, windows)))]
    {
        Ok(None)
    }
}

/// Verify that a test workspace and its owned identities are disjoint from
/// the repository daemon.
///
/// Both roots are canonicalized before containment checks. The derived IPC
/// endpoint and data directory are checked separately so a hash collision or
/// an accidentally shared data identity fails before a child is spawned.
///
/// # Errors
///
/// Returns `Err` when either path cannot be canonicalized, an endpoint cannot
/// be derived, or the workspace, endpoint, or data identity overlaps the
/// repository-owned identity.
pub fn verify_workspace_isolated_from_repository(
    workspace: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = workspace.canonicalize()?;
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).canonicalize()?;

    if paths_overlap(&workspace, &repository) {
        return Err(format!(
            "test workspace {} must not equal, contain, or be contained by repository root {}",
            workspace.display(),
            repository.display()
        )
        .into());
    }

    let workspace_endpoint = PathBuf::from(engram::daemon::ipc_server::ipc_endpoint(&workspace)?);
    if let Some(repository_endpoint) = repository_ipc_endpoint_if_known(&repository)? {
        if workspace_endpoint == repository_endpoint {
            return Err(format!(
                "test endpoint {} matches the repository-owned endpoint",
                workspace_endpoint.display()
            )
            .into());
        }
    }

    let workspace_data = workspace.join(".engram");
    let repository_data = repository.join(".engram");
    if paths_overlap(&workspace_data, &repository_data) {
        return Err(format!(
            "test data identity {} overlaps repository-owned data identity {}",
            workspace_data.display(),
            repository_data.display()
        )
        .into());
    }

    Ok(())
}

async fn wait_for_ipc_ready(path: &Path, timeout: Duration) -> Result<(), String> {
    let Some(deadline) = Instant::now().checked_add(timeout) else {
        return Err(format!(
            "cannot represent daemon readiness deadline for {timeout:?}"
        ));
    };
    let mut delay = Duration::from_millis(10);
    let mut attempt = 0_u32;

    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(format!(
                "daemon IPC endpoint did not become ready within {timeout:?} \
                 ({attempt} attempts); expected path: {}",
                path.display()
            ));
        }

        attempt = attempt.saturating_add(1);
        if tokio::time::timeout(remaining, ipc_ready(path))
            .await
            .unwrap_or(false)
        {
            return Ok(());
        }

        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            continue;
        }
        tokio::time::sleep(delay.min(remaining)).await;
        delay = (delay * 2).min(Duration::from_millis(500));
    }
}

fn terminate_and_reap_child(child: &mut Child, timeout: Duration) -> std::io::Result<ExitStatus> {
    if let Some(status) = child.try_wait()? {
        return Ok(status);
    }

    if let Err(kill_error) = child.kill() {
        if let Some(status) = child.try_wait()? {
            return Ok(status);
        }
        return Err(kill_error);
    }

    let deadline = Instant::now().checked_add(timeout).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("cannot represent child reap deadline for {timeout:?}"),
        )
    })?;
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(status);
        }
        if Instant::now() >= deadline {
            return Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                format!(
                    "owned daemon child {} was killed but not reaped within {timeout:?}",
                    child.id()
                ),
            ));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn readiness_failure(child: &mut Child, readiness_error: &str) -> Box<dyn std::error::Error> {
    let pid = child.id();
    match terminate_and_reap_child(child, CHILD_REAP_LIMIT) {
        Ok(status) => {
            format!("{readiness_error}; owned daemon child {pid} was reaped with status {status}")
                .into()
        }
        Err(cleanup_error) => {
            format!("{readiness_error}; failed to reap owned daemon child {pid}: {cleanup_error}")
                .into()
        }
    }
}

fn reap_child_on_drop(child: &mut Child, owner: &str) {
    if let Err(error) = terminate_and_reap_child(child, CHILD_REAP_LIMIT) {
        if std::thread::panicking() {
            eprintln!("{owner} cleanup failed during unwind: {error}");
        } else {
            panic!("{owner} cleanup failed: {error}");
        }
    }
}

/// Bind an in-process test state to workspace-local disposable storage.
///
/// Tests that dispatch library tools in-process cannot use
/// [`Command::env_remove`] to suppress an ambient `ENGRAM_DATA_DIR`. Building
/// the same explicit [`WorkspaceSnapshot`] used by other state-level fixtures
/// keeps their Cozo database inside the owning temporary workspace without
/// mutating process-global environment state.
///
/// # Panics
///
/// Panics if the isolated workspace snapshot cannot be bound to the test state.
pub async fn bind_isolated_workspace(
    state: &Arc<AppState>,
    workspace: &Path,
    branch: &str,
    config: WorkspaceConfig,
) {
    let path = workspace.display().to_string();
    let snapshot = WorkspaceSnapshot {
        workspace_id: format!("test:{path}"),
        workspace_uuid: format!("test:{path}"),
        branch: branch.to_owned(),
        data_dir: workspace.join(".engram"),
        path,
        last_flush: None,
        stale_files: false,
        connection_count: 0,
        file_mtimes: std::collections::HashMap::new(),
    };

    state
        .set_workspace_and_config(snapshot, Some(config))
        .await
        .expect("isolated test workspace must bind");
}

/// Compute the IPC endpoint path for a canonical workspace path.
///
/// - **Unix / macOS**: `{workspace}/.engram/run/engram.sock`
/// - **Windows**: `\\.\pipe\engram-{key}` using the daemon's production
///   endpoint derivation.
fn ipc_path_for_workspace(workspace: &Path) -> PathBuf {
    PathBuf::from(
        engram::daemon::ipc_server::ipc_endpoint(workspace)
            .expect("test workspace should produce a valid IPC endpoint"),
    )
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
    /// (starting at 10 ms, doubling each attempt, capped at 500 ms per step).
    /// Polling continues until the `timeout` wall-clock deadline is reached,
    /// at which point the child is killed and an `Err` is returned.
    ///
    /// # Errors
    ///
    /// Returns `Err` if:
    /// - The temporary directory cannot be created.
    /// - The workspace path cannot be canonicalized.
    /// - The `engram` binary cannot be spawned (e.g., not on `PATH`).
    /// - The IPC endpoint does not become ready within `timeout`.
    pub async fn spawn(timeout: Duration) -> Result<Self, Box<dyn std::error::Error>> {
        let workspace = TempDir::new()?;
        let workspace_path = workspace.path().canonicalize()?;

        // Create a minimal `.git` directory so the daemon accepts this as a workspace.
        // `canonicalize_workspace()` rejects paths where `.git` is not a directory.
        let git_dir = workspace_path.join(".git");
        std::fs::create_dir_all(&git_dir)?;
        std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n")?;
        verify_workspace_isolated_from_repository(&workspace_path)?;
        let ipc_path = ipc_path_for_workspace(&workspace_path);

        let workspace_str = workspace_path
            .to_str()
            .ok_or("workspace path contains non-UTF-8 characters")?;

        let child = Command::new(env!("CARGO_BIN_EXE_engram"))
            .args(["daemon", "--workspace", workspace_str])
            // Unset ENGRAM_DATA_DIR so the daemon uses the workspace-specific
            // data directory ({workspace}/.engram) rather than the developer's
            // production data directory.  Without this, every test daemon opens
            // the full production CozoDB (thousands of indexed files) and
            // hydrates the entire code graph, which takes minutes in debug mode
            // and causes the ready-timeout to fire (U016-HARNESS-2).
            .env_remove("ENGRAM_DATA_DIR")
            .spawn()?;

        // Establish RAII ownership before the first await so cancellation,
        // readiness failures, and panics all terminate the exact child.
        let mut harness = Self {
            workspace,
            child,
            ipc_path,
        };
        if let Err(error) = wait_for_ipc_ready(harness.ipc_path(), timeout).await {
            return Err(readiness_failure(&mut harness.child, &error));
        }
        Ok(harness)
    }

    /// Returns the path to the IPC endpoint for this workspace.
    #[must_use]
    pub fn ipc_path(&self) -> &Path {
        &self.ipc_path
    }

    // ── Lifecycle control ─────────────────────────────────────────────────────

    /// Poll for process exit without blocking.
    ///
    /// Returns `Ok(Some(status))` if the process has exited, `Ok(None)` if it
    /// is still running.
    ///
    /// # Errors
    ///
    /// Returns `Err` on OS error (e.g., the process handle is invalid).
    pub fn try_wait(&mut self) -> std::io::Result<Option<std::process::ExitStatus>> {
        self.child.try_wait()
    }

    // ── Alternative spawn constructors ────────────────────────────────────────

    /// Spawn a daemon for a specific, pre-existing workspace directory.
    ///
    /// The caller is responsible for creating `.git/HEAD` in the workspace
    /// before calling this function.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the workspace path cannot be canonicalized, the
    /// binary cannot be spawned, or the daemon does not become ready within
    /// `ready_timeout`.
    pub async fn spawn_for_workspace(
        workspace: &Path,
        ready_timeout: Duration,
    ) -> Result<HarnessWithoutOwnership, Box<dyn std::error::Error>> {
        let workspace_path = workspace.canonicalize()?;
        verify_workspace_isolated_from_repository(&workspace_path)?;
        let ipc_path = ipc_path_for_workspace(&workspace_path);

        let workspace_str = workspace_path
            .to_str()
            .ok_or("workspace path contains non-UTF-8 characters")?;

        let child = Command::new(env!("CARGO_BIN_EXE_engram"))
            .args(["daemon", "--workspace", workspace_str])
            .env_remove("ENGRAM_DATA_DIR")
            .spawn()?;

        let mut harness = HarnessWithoutOwnership { child, ipc_path };
        if let Err(error) = wait_for_ipc_ready(harness.ipc_path(), ready_timeout).await {
            return Err(readiness_failure(&mut harness.child, &error));
        }
        Ok(harness)
    }

    /// Spawn a daemon for an owned workspace while capturing diagnostics.
    ///
    /// This focused variant is intended for boundary-characterization tests
    /// that need to place server dispatch and response-frame events on one
    /// timeline. Standard input is closed, tracing stdout is written to
    /// `trace_log`, and stderr diagnostics are captured separately in
    /// `stderr_log`.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the workspace or either log cannot be opened, the
    /// binary cannot be spawned, or readiness is not reached before
    /// `ready_timeout`.
    pub async fn spawn_for_workspace_with_trace_log(
        workspace: &Path,
        trace_log: &Path,
        stderr_log: &Path,
        ready_timeout: Duration,
    ) -> Result<HarnessWithoutOwnership, Box<dyn std::error::Error>> {
        let workspace_path = workspace.canonicalize()?;
        verify_workspace_isolated_from_repository(&workspace_path)?;
        let ipc_path = ipc_path_for_workspace(&workspace_path);
        let workspace_str = workspace_path
            .to_str()
            .ok_or("workspace path contains non-UTF-8 characters")?;
        if trace_log == stderr_log {
            return Err("stdout trace and stderr diagnostic paths must differ".into());
        }
        let stdout = std::fs::File::create(trace_log)?;
        let stderr = std::fs::File::create(stderr_log)?;

        let child = Command::new(env!("CARGO_BIN_EXE_engram"))
            .args(["daemon", "--workspace", workspace_str])
            .stdin(Stdio::null())
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr))
            .env("RUST_LOG", "engram=debug,hyper=info")
            .env_remove("ENGRAM_DATA_DIR")
            .spawn()?;

        let mut harness = HarnessWithoutOwnership { child, ipc_path };
        if let Err(error) = wait_for_ipc_ready(harness.ipc_path(), ready_timeout).await {
            return Err(readiness_failure(&mut harness.child, &error));
        }
        Ok(harness)
    }

    /// Spawn a daemon with a short idle timeout (for TTL/lifecycle tests).
    ///
    /// Sets the `ENGRAM_IDLE_TIMEOUT_MS` environment variable so the daemon
    /// self-terminates after `timeout_ms` milliseconds of inactivity.
    ///
    /// # Errors
    ///
    /// Same as [`DaemonHarness::spawn`].
    pub async fn spawn_with_idle_timeout_ms(
        timeout_ms: u64,
        ready_timeout: Duration,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let workspace = TempDir::new()?;
        let workspace_path = workspace.path().canonicalize()?;

        let git_dir = workspace_path.join(".git");
        std::fs::create_dir_all(&git_dir)?;
        std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n")?;
        verify_workspace_isolated_from_repository(&workspace_path)?;
        let ipc_path = ipc_path_for_workspace(&workspace_path);

        let workspace_str = workspace_path
            .to_str()
            .ok_or("workspace path contains non-UTF-8 characters")?;

        let child = Command::new(env!("CARGO_BIN_EXE_engram"))
            .args(["daemon", "--workspace", workspace_str])
            .env("ENGRAM_IDLE_TIMEOUT_MS", timeout_ms.to_string())
            .env_remove("ENGRAM_DATA_DIR")
            .spawn()?;

        let mut harness = Self {
            workspace,
            child,
            ipc_path,
        };
        if let Err(error) = wait_for_ipc_ready(harness.ipc_path(), ready_timeout).await {
            return Err(readiness_failure(&mut harness.child, &error));
        }
        Ok(harness)
    }
}

impl Drop for DaemonHarness {
    fn drop(&mut self) {
        reap_child_on_drop(&mut self.child, "DaemonHarness");
    }
}

// ── HarnessWithoutOwnership ───────────────────────────────────────────────────

/// A daemon harness that does **not** own the workspace [`TempDir`].
///
/// Returned by [`DaemonHarness::spawn_for_workspace`] when the caller already
/// owns the workspace directory and must keep it alive independently.
pub struct HarnessWithoutOwnership {
    /// Child process handle; killed on drop.
    child: Child,
    /// IPC endpoint for this daemon.
    ipc_path: PathBuf,
}

impl HarnessWithoutOwnership {
    /// Returns the IPC endpoint path.
    #[must_use]
    pub fn ipc_path(&self) -> &Path {
        &self.ipc_path
    }

    /// Return the exact process identifier of the owned daemon child.
    #[must_use]
    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    /// Poll for owned-daemon exit without blocking.
    ///
    /// # Errors
    ///
    /// Returns `Err` when the operating system cannot query the child.
    pub fn try_wait(&mut self) -> std::io::Result<Option<std::process::ExitStatus>> {
        self.child.try_wait()
    }

    /// Kill the daemon, wait up to the harness reap limit, and return its exact
    /// process ID.
    ///
    /// The reaped [`Child`] handle remains owned by this harness.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the process cannot be killed or waited on.
    pub fn kill_and_wait(&mut self) -> std::io::Result<u32> {
        self.kill_and_wait_bounded(CHILD_REAP_LIMIT)
    }

    /// Kill the daemon, verify it is reaped within `timeout`, and return its
    /// exact process ID.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the process cannot be killed, queried, or reaped before
    /// the bounded deadline.
    pub fn kill_and_wait_bounded(&mut self, timeout: Duration) -> std::io::Result<u32> {
        let pid = self.child.id();
        terminate_and_reap_child(&mut self.child, timeout)?;
        Ok(pid)
    }
}

impl Drop for HarnessWithoutOwnership {
    fn drop(&mut self) {
        reap_child_on_drop(&mut self.child, "HarnessWithoutOwnership");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlap_detection_rejects_equal_parent_and_child_paths() {
        let repository = Path::new("C:/workspace/repository");
        assert!(paths_overlap(repository, repository));
        assert!(paths_overlap(Path::new("C:/workspace"), repository));
        assert!(paths_overlap(
            Path::new("C:/workspace/repository/tmp"),
            repository
        ));
        assert!(!paths_overlap(
            Path::new("C:/workspace/repository-sibling"),
            repository
        ));
    }

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
        let workspace = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(workspace.path().join(".git")).expect("create .git");
        std::fs::write(
            workspace.path().join(".git").join("HEAD"),
            "ref: refs/heads/main\n",
        )
        .expect("write HEAD");
        let path = ipc_path_for_workspace(workspace.path());
        let path_str = path.to_str().expect("pipe path is valid UTF-8");

        assert!(
            path_str.starts_with(r"\\.\pipe\engram-"),
            "Windows IPC path must start with {{pipe prefix}}, got: {path_str}"
        );

        let key_part = path_str
            .strip_prefix(r"\\.\pipe\engram-")
            .expect("prefix already verified");
        assert!(
            !key_part.is_empty(),
            "pipe suffix must not be empty, got: {key_part}"
        );
    }

    /// Verify that two different workspace paths produce different pipe names
    /// (collision resistance sanity check).
    #[test]
    #[cfg(windows)]
    fn ipc_path_windows_unique_per_workspace() {
        let workspace_a = tempfile::tempdir().expect("tempdir a");
        std::fs::create_dir(workspace_a.path().join(".git")).expect("create .git a");
        std::fs::write(
            workspace_a.path().join(".git").join("HEAD"),
            "ref: refs/heads/main\n",
        )
        .expect("write HEAD a");
        let workspace_b = tempfile::tempdir().expect("tempdir b");
        std::fs::create_dir(workspace_b.path().join(".git")).expect("create .git b");
        std::fs::write(
            workspace_b.path().join(".git").join("HEAD"),
            "ref: refs/heads/main\n",
        )
        .expect("write HEAD b");

        let a = ipc_path_for_workspace(workspace_a.path());
        let b = ipc_path_for_workspace(workspace_b.path());
        assert_ne!(a, b, "distinct workspaces must produce distinct pipe names");
    }
}
