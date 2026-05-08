//! Direct (daemonless) runner for indexing and sync operations.
//!
//! When `--direct` is passed, the CLI acquires the daemon lock and calls the
//! service layer directly instead of routing through IPC. This allows
//! indexing to be run as a stand-alone process that exits on completion
//! without leaving a daemon alive. The daemon lock ensures at most one
//! writer is active at any time.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use tracing::warn;

use crate::cli::output::OutputFormatter;
use crate::daemon::lockfile::DaemonLock;
use crate::db::workspace::{canonicalize_workspace, resolve_data_dir, resolve_git_branch};
use crate::errors::{EngramError, LockError};
use crate::services::code_graph::{IndexResult, SyncResult, index_workspace, sync_workspace};
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
/// # Returns
/// - `0` — success
/// - `1` — tool error (DB or parse failure)
/// - `2` — invocation failure (bad workspace, lock held by daemon)
pub async fn run_direct_sync(workspace: &Path, full: bool, formatter: &OutputFormatter) -> i32 {
    let (ws_path, data_dir, branch, config) = match resolve_workspace_params(workspace, formatter) {
        Ok(params) => params,
        Err(code) => return code,
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

    if full {
        match index_workspace(&ws_path, &data_dir, &branch, &config.code_graph, false).await {
            Ok(result) => formatter.success(None, index_result_to_json(&result)),
            Err(e) => formatter.tool_error(None, 1, &e.to_string(), None),
        }
    } else {
        match sync_workspace(&ws_path, &data_dir, &branch, &config.code_graph).await {
            Ok(result) => formatter.success(None, sync_result_to_json(&result)),
            Err(e) => formatter.tool_error(None, 1, &e.to_string(), None),
        }
    }
}

/// Resolve workspace path, data directory, git branch, and config.
///
/// Returns `Ok((ws_path, data_dir, branch, config))` or `Err(exit_code)`.
fn resolve_workspace_params(
    workspace: &Path,
    formatter: &OutputFormatter,
) -> Result<
    (
        PathBuf,
        PathBuf,
        String,
        crate::models::config::WorkspaceConfig,
    ),
    i32,
> {
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

    let config = match parse_config(&ws_path) {
        Ok(cfg) => cfg,
        Err(e) => {
            warn!(error = %e, "failed to parse config, using defaults");
            crate::models::config::WorkspaceConfig::default()
        }
    };

    Ok((ws_path, data_dir, branch, config))
}

/// Convert [`IndexResult`] into a [`Value`] for the JSON-RPC 2.0 result field.
fn index_result_to_json(r: &IndexResult) -> Value {
    json!({
        "files_parsed": r.files_parsed,
        "files_skipped": r.files_skipped,
        "functions_indexed": r.functions_indexed,
        "classes_indexed": r.classes_indexed,
        "interfaces_indexed": r.interfaces_indexed,
        "edges_created": r.edges_created,
        "embeddings_generated": r.embeddings_generated,
        "duration_ms": r.duration_ms,
        "errors": r.errors.len(),
    })
}

/// Convert [`SyncResult`] into a [`Value`] for the JSON-RPC 2.0 result field.
fn sync_result_to_json(r: &SyncResult) -> Value {
    json!({
        "files_modified": r.files_modified,
        "files_added": r.files_added,
        "files_deleted": r.files_deleted,
        "files_unchanged": r.files_unchanged,
        "symbols_reembedded": r.symbols_reembedded,
        "duration_ms": r.duration_ms,
        "errors": r.errors.len(),
    })
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
    fn index_result_json_has_expected_keys() {
        let r = make_index_result();
        let v = index_result_to_json(&r);
        assert_eq!(v["files_parsed"], json!(10));
        assert_eq!(v["duration_ms"], json!(123_u64));
        assert_eq!(v["errors"], json!(0_usize));
    }

    #[test]
    fn sync_result_json_has_expected_keys() {
        let r = make_sync_result();
        let v = sync_result_to_json(&r);
        assert_eq!(v["files_modified"], json!(3));
        assert_eq!(v["files_unchanged"], json!(6));
        assert_eq!(v["errors"], json!(0_usize));
    }
}
