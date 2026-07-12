//! `engram migrate-down <target>` — operator-invocable destructive migrations.
//!
//! A local, no-daemon maintenance command (a `verify`/`manifest` analog) that
//! opens the workspace CozoDB directly and invokes a reusable down-migration
//! entry point. It is a deliberate, destructive operation and must therefore be
//! run explicitly by an operator — it never auto-runs on startup and is not
//! exposed as a daemon MCP tool.
//!
//! The only target defined today is `calls-resolution`, which invokes
//! [`crate::db::queries::CodeGraphQueries::rollback_calls_resolution`]
//! (082.010-T): retract every `calls_resolved_singleton` edge, then drop the
//! `resolution` attribute — in that order, idempotently — so operators can run
//! the cleanup BEFORE deploying reverted code (plan §7). The CLI layer stays
//! thin: it only resolves the workspace, opens the DB, invokes the migration,
//! and prints a summary. All migration logic lives in 082.010-T.
//!
//! Exit codes:
//! - `0` — success (prints the retracted-edge count and column-drop status);
//! - `1` — tool error (DB open or migration failure);
//! - `2` — invocation error (unknown target, unresolvable/locked workspace).

use crate::cli::flags::GlobalFlags;
use crate::cli::output::OutputFormatter;
use crate::daemon::lockfile::DaemonLock;
use crate::db::connect_db;
use crate::db::queries::CodeGraphQueries;
use crate::db::workspace::{canonicalize_workspace, resolve_data_dir, resolve_git_branch};
use crate::errors::{EngramError, LockError};

/// Migration completed successfully.
const EXIT_OK: i32 = 0;
/// Usage or invocation error (unknown target, bad/locked workspace).
const EXIT_INVOCATION_ERROR: i32 = 2;

/// The single supported down-migration target.
const TARGET_CALLS_RESOLUTION: &str = "calls-resolution";

/// Run `engram migrate-down <target>`.
///
/// Dispatches the named down-migration against the workspace DB. Returns the
/// pinned exit code documented at the module level.
pub async fn run_migrate_down(target: String, flags: &GlobalFlags, fmt: &OutputFormatter) -> i32 {
    if target != TARGET_CALLS_RESOLUTION {
        fmt.cli_error(&format!(
            "unknown migrate-down target '{target}'; supported targets: {TARGET_CALLS_RESOLUTION}"
        ));
        return EXIT_INVOCATION_ERROR;
    }

    let workspace = match flags.resolve_workspace() {
        Ok(root) => root,
        Err(err) => return fmt.cli_error(&format!("cannot resolve workspace: {err}")),
    };
    let Some(ws_str) = workspace.to_str() else {
        return fmt.cli_error("workspace path contains invalid UTF-8");
    };
    let ws_path = match canonicalize_workspace(ws_str) {
        Ok(path) => path,
        Err(err) => return fmt.cli_error(&format!("invalid workspace: {err}")),
    };
    let branch = resolve_git_branch(&ws_path).unwrap_or_else(|_| "default".to_owned());
    let data_dir = resolve_data_dir(&ws_path);

    // Hold the daemon lock for the whole migration: this is a deliberate
    // destructive rewrite and must not race a running daemon's writers.
    let _lock = match DaemonLock::acquire(&ws_path) {
        Ok(lock) => lock,
        Err(EngramError::Lock(LockError::AlreadyHeld { pid })) => {
            return fmt.cli_error(&format!(
                "daemon is already running (pid {pid}); stop it before running migrate-down"
            ));
        }
        Err(err) => {
            return fmt.cli_error(&format!("failed to acquire daemon lock: {err}"));
        }
    };

    let db = match connect_db(&data_dir, &branch).await {
        Ok(db) => db,
        Err(err) => {
            let resp = err.to_response().error;
            return fmt.tool_error(
                flags.id_value(),
                i64::from(resp.code),
                &resp.message,
                resp.details,
            );
        }
    };
    let queries = CodeGraphQueries::new(db);

    match queries.rollback_calls_resolution().await {
        Ok(retracted) => {
            fmt.success(
                flags.id_value(),
                serde_json::json!({
                    "target": TARGET_CALLS_RESOLUTION,
                    "retracted_singleton_edges": retracted,
                    "resolution_column_dropped": true,
                }),
            );
            EXIT_OK
        }
        Err(err) => {
            let resp = err.to_response().error;
            fmt.tool_error(
                flags.id_value(),
                i64::from(resp.code),
                &resp.message,
                resp.details,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TARGET_CALLS_RESOLUTION;

    #[test]
    fn target_constant_is_the_canonical_calls_resolution_value() {
        assert_eq!(TARGET_CALLS_RESOLUTION, "calls-resolution");
    }
}
