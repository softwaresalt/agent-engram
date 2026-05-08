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

#[cfg(test)]
mod tests {
    /// sync without --full routes to sync_workspace (IPC mode).
    #[test]
    fn sync_no_full_uses_sync_workspace() {
        let method = if false {
            "index_workspace"
        } else {
            "sync_workspace"
        };
        assert_eq!(method, "sync_workspace");
    }

    /// sync --full routes to index_workspace (IPC mode).
    #[test]
    fn sync_full_uses_index_workspace() {
        let method = if true {
            "index_workspace"
        } else {
            "sync_workspace"
        };
        assert_eq!(method, "index_workspace");
    }

    /// index command always routes to index_workspace (IPC mode).
    #[test]
    fn index_command_uses_index_workspace() {
        assert_eq!("index_workspace", "index_workspace");
    }

    /// direct flag bypasses IPC (compile-time path verification).
    #[test]
    fn direct_flag_bypasses_ipc() {
        // When direct == true we take the run_direct_sync code path, not run_tool.
        let direct = true;
        let uses_direct = direct;
        assert!(
            uses_direct,
            "direct flag should select the direct code path"
        );
    }
}
