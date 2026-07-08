//! Indexing subcommands: sync and index (the critical preloading commands).

use serde_json::json;

use crate::cli::direct::run_direct_sync;
use crate::cli::flags::GlobalFlags;
use crate::cli::output::OutputFormatter;
use crate::cli::runner::{INDEXING_TIMEOUT_SECS, run_tool, run_tool_timed};

/// `engram sync [--full] [--force] [--direct]` — incremental or full workspace index.
///
/// Without `--direct`, routes through the IPC daemon (auto-spawned if needed).
/// With `--direct`, acquires the daemon lock and runs service functions in-process,
/// then exits when complete. Useful for pre-loading the index from a startup script
/// before launching an MCP host.
///
/// `--force` re-parses and re-embeds all discovered files (bypassing the
/// content-hash skip) and implies the full-scan path.
pub async fn run_sync(
    full: bool,
    force: bool,
    direct: bool,
    flags: &GlobalFlags,
    formatter: &OutputFormatter,
) -> i32 {
    if direct {
        let workspace = match flags.resolve_workspace() {
            Ok(p) => p,
            Err(e) => return formatter.cli_error(&e),
        };
        let correlation_id = match flags.resolve_correlation_id() {
            Ok(v) => v,
            Err(e) => return formatter.cli_error(&format!("invalid --correlation-id: {e}")),
        };
        return run_direct_sync(
            &workspace,
            full,
            force,
            flags.id_value(),
            correlation_id,
            formatter,
        )
        .await;
    }
    if full || force {
        // Full re-index can take minutes on large workspaces — use extended timeout.
        run_tool_timed(
            "index_workspace",
            force_params(force),
            flags,
            formatter,
            INDEXING_TIMEOUT_SECS,
        )
        .await
    } else {
        run_tool("sync_workspace", None, flags, formatter).await
    }
}

/// `engram index [--force] [--direct]` — full scan; alias for `engram sync --full`.
pub async fn run_index(
    force: bool,
    direct: bool,
    flags: &GlobalFlags,
    formatter: &OutputFormatter,
) -> i32 {
    if direct {
        let workspace = match flags.resolve_workspace() {
            Ok(p) => p,
            Err(e) => return formatter.cli_error(&e),
        };
        let correlation_id = match flags.resolve_correlation_id() {
            Ok(v) => v,
            Err(e) => return formatter.cli_error(&format!("invalid --correlation-id: {e}")),
        };
        return run_direct_sync(
            &workspace,
            true,
            force,
            flags.id_value(),
            correlation_id,
            formatter,
        )
        .await;
    }
    // Full re-index can take minutes on large workspaces — use extended timeout.
    run_tool_timed(
        "index_workspace",
        force_params(force),
        flags,
        formatter,
        INDEXING_TIMEOUT_SECS,
    )
    .await
}

/// Build `index_workspace` params for the `force` flag: `Some({"force": true})`
/// when forcing a re-parse, `None` otherwise (preserving the default fast path).
fn force_params(force: bool) -> Option<serde_json::Value> {
    if force {
        Some(json!({ "force": true }))
    } else {
        None
    }
}

// Routing behaviour is covered by tests/integration/cli_direct_test.rs which
// runs the binary as a subprocess and verifies the actual dispatch paths.
