//! Direct (daemonless) runner for indexing and sync operations.
//!
//! When `--direct` is passed, the CLI acquires the daemon lock and calls the
//! service layer directly instead of routing through IPC. This allows
//! indexing to be run as a stand-alone process that exits on completion
//! without leaving a daemon alive. The daemon lock ensures at most one
//! writer is active at any time.

use std::path::{Path, PathBuf};

use fd_lock::RwLock as FdRwLock;
use serde_json::{Value, json};
use tracing::warn;

use crate::cli::output::OutputFormatter;
use crate::daemon::lockfile::DaemonLock;
use crate::db::workspace::{canonicalize_workspace, resolve_data_dir, resolve_git_branch};
use crate::errors::{EngramError, LockError};
use crate::services::code_graph::{
    IndexResult, ProgressCallback, SyncResult, index_workspace_with_progress,
    sync_workspace_with_progress,
};
use crate::services::config::parse_config;

/// Run an incremental sync (or full re-index) in direct mode.
///
/// 1. Canonicalises `workspace` and checks it is a valid git root.
/// 2. Tries to acquire the daemon lock; returns exit code 2 if a daemon is
///    already running.
/// 3. Resolves the data directory, branch, and config.
/// 4. Calls [`sync_workspace`] (incremental) or [`index_workspace`] (full).
/// 5. Prints a JSON-RPC 2.0 success envelope and returns exit code 0.
///
/// The `id` parameter is echoed in the JSON-RPC envelope so scripts can
/// correlate responses (same behaviour as the IPC path). Pass
/// [`GlobalFlags::id_value()`] or `None` to use the default id `1`, matching
/// the IPC runner behaviour (see `src/cli/runner.rs`).
///
/// # Returns
/// - `0` — success
/// - `1` — tool error (DB, parse failure, or invalid config)
/// - `2` — invocation failure (bad workspace, lock held by daemon)
pub async fn run_direct_sync(
    workspace: &Path,
    full: bool,
    id: Option<serde_json::Value>,
    formatter: &OutputFormatter,
) -> i32 {
    // Default to `1` when no --id provided; matches the IPC runner path (runner.rs).
    // JSON-RPC 2.0 requires a non-null id for request-correlated responses.
    let effective_id = id.unwrap_or_else(|| Value::from(1_u64));

    let (ws_path, data_dir, branch) = match resolve_workspace_params(workspace, formatter) {
        Ok(params) => params,
        Err(code) => return code,
    };

    let config = match parse_config(&ws_path) {
        Ok(cfg) => cfg,
        Err(e) => {
            let resp = e.to_response().error;
            return formatter.tool_error(
                Some(effective_id),
                i64::from(resp.code),
                &resp.message,
                resp.details,
            );
        }
    };

    let _lock = match DaemonLock::acquire(&ws_path) {
        Ok(lock) => lock,
        Err(EngramError::Lock(LockError::AlreadyHeld { pid })) => {
            return formatter.cli_error(&format!(
                "daemon is already running (pid {pid}); \
                 stop it before using --direct mode"
            ));
        }
        Err(e) => {
            return formatter.cli_error(&format!("failed to acquire daemon lock: {e}"));
        }
    };

    // Probe the CozoDB advisory lock before connect_db's 30-second polling loop.
    // When ENGRAM_DATA_DIR is shared across workspaces, two processes targeting
    // different workspace roots (different DaemonLock paths) may race on the
    // same CozoDB database file. This pre-check returns a clear exit-2 error
    // immediately instead of surfacing a generic 30-second timeout message.
    //
    // TOCTOU note: the probe and connect_db's own try_write loop are not atomic.
    // Another process could acquire the lock between the probe and connect_db.
    // The connect_db timeout covers that residual race and will also return a
    // clean error; this probe only improves the UX for the common case.
    {
        let branch_safe = branch.replace(['/', '\\', ':'], "_");
        let db_lock_path = data_dir
            .join("cozo")
            .join(&branch_safe)
            .join("engram.db.lock");
        if db_lock_path.exists() {
            match std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(false)
                .open(&db_lock_path)
            {
                Ok(f) => {
                    let mut rw = FdRwLock::new(f);
                    if rw.try_write().is_err() {
                        return formatter.cli_error(
                            "workspace database is locked by another process; \
                             stop the daemon first or use IPC mode (omit --direct)",
                        );
                    }
                }
                Err(e) => {
                    // Cannot open the lock file — permissions or sharing violation.
                    // Return exit 2 rather than falling through to connect_db's
                    // 30-second polling loop with an opaque timeout message.
                    return formatter.cli_error(&format!(
                        "cannot open workspace database lock file: {e}; \
                         check permissions or stop any running daemon"
                    ));
                }
            }
        }
    }

    let started_at = std::time::Instant::now();
    let mut last_message = String::new();
    let mut progress_callback = |completed: u64, total: u64| {
        if !formatter.shows_progress() {
            return;
        }

        let message = render_direct_progress(completed, total, started_at.elapsed().as_secs());
        if message != last_message {
            formatter.progress_hint(&message);
            last_message = message;
        }
    };
    let progress: Option<&mut ProgressCallback<'_>> = if formatter.shows_progress() {
        Some(&mut progress_callback)
    } else {
        None
    };

    if full {
        match index_workspace_with_progress(
            &ws_path,
            &data_dir,
            &branch,
            &config.code_graph,
            false,
            progress,
        )
        .await
        {
            Ok(result) => formatter.success(Some(effective_id), index_result_to_json(&result)),
            Err(e) => {
                let resp = e.to_response().error;
                formatter.tool_error(
                    Some(effective_id),
                    i64::from(resp.code),
                    &resp.message,
                    resp.details,
                )
            }
        }
    } else {
        match sync_workspace_with_progress(
            &ws_path,
            &data_dir,
            &branch,
            &config.code_graph,
            progress,
        )
        .await
        {
            Ok(result) => formatter.success(Some(effective_id), sync_result_to_json(&result)),
            Err(e) => {
                let resp = e.to_response().error;
                formatter.tool_error(
                    Some(effective_id),
                    i64::from(resp.code),
                    &resp.message,
                    resp.details,
                )
            }
        }
    }
}

fn render_direct_progress(completed: u64, total: u64, elapsed_secs: u64) -> String {
    if total == 0 {
        return format!("Pre-warm workspace... {elapsed_secs}s elapsed");
    }

    let percent = completed.saturating_mul(100) / total;
    format!("Pre-warm workspace... {completed}/{total} files ({percent}%, {elapsed_secs}s elapsed)")
}

/// Resolve workspace path, data directory, and git branch.
///
/// Returns `Ok((ws_path, data_dir, branch))`.
///
/// The `Err(i32)` variant carries the CLI exit code. Errors are formatted
/// and emitted to stderr via `formatter.cli_error` as a side effect before
/// the exit code is returned — this is intentional for CLI dispatch.
/// Config parsing is intentionally excluded here so callers can report
/// config errors at the appropriate severity (exit 1 tool error vs
/// exit 2 invocation error).
fn resolve_workspace_params(
    workspace: &Path,
    formatter: &OutputFormatter,
) -> Result<(PathBuf, PathBuf, String), i32> {
    let ws_str = workspace
        .to_str()
        .ok_or_else(|| formatter.cli_error("workspace path contains invalid UTF-8"))?;

    let ws_path = canonicalize_workspace(ws_str)
        .map_err(|e| formatter.cli_error(&format!("invalid workspace: {e}")))?;

    let branch = resolve_git_branch(&ws_path).unwrap_or_else(|e| {
        warn!(error = %e, "could not resolve git branch; using 'default'");
        "default".to_owned()
    });

    let data_dir = resolve_data_dir(&ws_path);

    Ok((ws_path, data_dir, branch))
}

/// Convert [`IndexResult`] into a [`Value`] for the JSON-RPC 2.0 result field.
///
/// Serialises the full struct so the direct-mode output matches the daemon
/// IPC path field-for-field (`IndexResult` derives [`serde::Serialize`]).
fn index_result_to_json(r: &IndexResult) -> Value {
    serde_json::to_value(r).unwrap_or_else(|_| json!({}))
}

/// Convert [`SyncResult`] into a [`Value`] for the JSON-RPC 2.0 result field.
///
/// Serialises the full struct so the direct-mode output matches the daemon
/// IPC path field-for-field (`SyncResult` derives [`serde::Serialize`]).
fn sync_result_to_json(r: &SyncResult) -> Value {
    serde_json::to_value(r).unwrap_or_else(|_| json!({}))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{index_result_to_json, sync_result_to_json};
    use crate::services::code_graph::{IndexResult, SyncResult};

    fn make_index_result() -> IndexResult {
        IndexResult {
            files_parsed: 10,
            files_skipped: 2,
            functions_indexed: 50,
            classes_indexed: 5,
            interfaces_indexed: 3,
            edges_created: 20,
            embeddings_generated: 55,
            tier1_count: 40,
            tier2_count: 15,
            cross_file_edges_dropped: 0,
            errors: vec![],
            duration_ms: 123,
        }
    }

    fn make_sync_result() -> SyncResult {
        SyncResult {
            files_modified: 3,
            files_added: 1,
            files_deleted: 0,
            files_unchanged: 6,
            symbols_reembedded: 10,
            symbols_reused: 30,
            concerns_relinked: 0,
            concerns_orphaned: 0,
            edges_created: 5,
            cross_file_edges_dropped: 0,
            errors: vec![],
            duration_ms: 45,
        }
    }

    #[test]
    fn index_result_json_has_full_schema() {
        let r = make_index_result();
        let v = index_result_to_json(&r);
        // Core fields
        assert_eq!(v["files_parsed"], json!(10));
        assert_eq!(v["duration_ms"], json!(123_u64));
        // Previously omitted fields — full struct serialisation ensures parity
        // with the daemon IPC path
        assert_eq!(v["tier1_count"], json!(40_usize));
        assert_eq!(v["tier2_count"], json!(15_usize));
        assert_eq!(v["cross_file_edges_dropped"], json!(0_usize));
        // errors is the full list, not just a count
        assert_eq!(v["errors"], json!([]));
    }

    #[test]
    fn sync_result_json_has_full_schema() {
        let r = make_sync_result();
        let v = sync_result_to_json(&r);
        // Core fields
        assert_eq!(v["files_modified"], json!(3));
        assert_eq!(v["files_unchanged"], json!(6));
        // Previously omitted fields — full struct serialisation ensures parity
        // with the daemon IPC path
        assert_eq!(v["symbols_reused"], json!(30_usize));
        assert_eq!(v["concerns_relinked"], json!(0_usize));
        assert_eq!(v["edges_created"], json!(5_usize));
        // errors is the full list, not just a count
        assert_eq!(v["errors"], json!([]));
    }

    /// Verify that fd_lock detects a held write lock from a separate file handle.
    ///
    /// This test documents the cross-handle locking guarantee that the db-lock
    /// probe in `run_direct_sync` relies on. On Windows, `LockFileEx` locks are
    /// per-file-handle even within the same process. On Linux/macOS, `flock` /
    /// `fcntl` may coalesce per-process — but the probe is specifically for the
    /// multi-process scenario (daemon holds the lock, direct mode probes it).
    #[test]
    fn fd_lock_try_write_conflicts_with_held_write_lock() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let lock_path = tmp.path().join("engram.db.lock");
        std::fs::write(&lock_path, b"").expect("create lock file");

        // Handle A acquires an exclusive write lock.
        let file_a = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&lock_path)
            .expect("open a");
        let mut rw_a = fd_lock::RwLock::new(file_a);
        let _guard_a = rw_a.try_write().expect("first acquisition must succeed");

        // Handle B's try_write should fail while A holds the lock.
        let file_b = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&lock_path)
            .expect("open b");
        let mut rw_b = fd_lock::RwLock::new(file_b);

        // On Windows, LockFileEx enforces per-handle exclusivity.
        // On Linux/macOS with BSD flock, try_write may succeed for same-process
        // handles. The integration test (cli_direct_test.rs) covers cross-process.
        #[cfg(target_os = "windows")]
        assert!(
            rw_b.try_write().is_err(),
            "second try_write must fail while first handle holds the lock (Windows)"
        );

        // On non-Windows: just assert the call doesn't panic.
        #[cfg(not(target_os = "windows"))]
        let _ = rw_b.try_write();
    }
}
