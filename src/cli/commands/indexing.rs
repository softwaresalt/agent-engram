//! Indexing subcommands: sync and index (the critical preloading commands).

use crate::cli::direct::run_direct_sync;
use crate::cli::flags::GlobalFlags;
use crate::cli::output::OutputFormatter;
use crate::cli::runner::run_tool;

/// `engram sync [--full] [--direct]` — incremental or full workspace index.
///
/// Without `--direct`, routes through the IPC daemon (auto-spawned if needed).
/// With `--direct`, acquires the daemon lock and runs service functions in-process,
/// then exits when complete. Useful for pre-loading the index from a startup script
/// before launching an MCP host.
pub async fn run_sync(
    full: bool,
    direct: bool,
    flags: &GlobalFlags,
    formatter: &OutputFormatter,
) -> i32 {
    if direct {
        let workspace = match flags.resolve_workspace() {
            Ok(p) => p,
            Err(e) => return formatter.cli_error(&e),
        };
        return run_direct_sync(&workspace, full, formatter).await;
    }
    if full {
        run_tool("index_workspace", None, flags, formatter).await
    } else {
        run_tool("sync_workspace", None, flags, formatter).await
    }
}

/// `engram index [--direct]` — alias for `engram sync --full [--direct]`.
pub async fn run_index(direct: bool, flags: &GlobalFlags, formatter: &OutputFormatter) -> i32 {
    if direct {
        let workspace = match flags.resolve_workspace() {
            Ok(p) => p,
            Err(e) => return formatter.cli_error(&e),
        };
        return run_direct_sync(&workspace, true, formatter).await;
    }
    run_tool("index_workspace", None, flags, formatter).await
}

// Routing behaviour is covered by tests/integration/cli_direct_test.rs which
// runs the binary as a subprocess and verifies the actual dispatch paths.
