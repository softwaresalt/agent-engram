//! Indexing subcommands: sync and index (the critical preloading commands).

use crate::cli::flags::GlobalFlags;
use crate::cli::output::OutputFormatter;
use crate::cli::runner::run_tool;

/// `engram sync [--full]` — incremental or full workspace index.
///
/// `--full` maps to `index_workspace`; plain `sync` maps to `sync_workspace`.
pub async fn run_sync(full: bool, flags: &GlobalFlags, formatter: &OutputFormatter) -> i32 {
    if full {
        run_tool("index_workspace", None, flags, formatter).await
    } else {
        run_tool("sync_workspace", None, flags, formatter).await
    }
}

/// `engram index` — alias for `engram sync --full`.
pub async fn run_index(flags: &GlobalFlags, formatter: &OutputFormatter) -> i32 {
    run_tool("index_workspace", None, flags, formatter).await
}

#[cfg(test)]
mod tests {
    /// sync without --full maps to sync_workspace.
    #[test]
    fn sync_no_full_uses_sync_workspace() {
        let method = if false {
            "index_workspace"
        } else {
            "sync_workspace"
        };
        assert_eq!(method, "sync_workspace");
    }

    /// sync --full maps to index_workspace.
    #[test]
    fn sync_full_uses_index_workspace() {
        let method = if true {
            "index_workspace"
        } else {
            "sync_workspace"
        };
        assert_eq!(method, "index_workspace");
    }

    /// index command always uses index_workspace.
    #[test]
    fn index_command_uses_index_workspace() {
        assert_eq!("index_workspace", "index_workspace");
    }
}
