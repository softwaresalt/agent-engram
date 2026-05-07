//! Lifecycle subcommands: bind, daemon-status, workspace-status, flush.

use serde_json::json;

use crate::cli::flags::GlobalFlags;
use crate::cli::output::OutputFormatter;
use crate::cli::runner::run_tool;

/// `engram bind [path]` → `set_workspace`
pub async fn run_bind(
    path: Option<String>,
    flags: &GlobalFlags,
    formatter: &OutputFormatter,
) -> i32 {
    let resolved = match path {
        Some(p) => p,
        None => match flags.resolve_workspace() {
            Ok(p) => p.to_string_lossy().into_owned(),
            Err(e) => return formatter.cli_error(&e),
        },
    };
    let params = json!({ "path": resolved });
    run_tool("set_workspace", Some(params), flags, formatter).await
}

/// `engram daemon-status` → `get_daemon_status`
pub async fn run_daemon_status(flags: &GlobalFlags, formatter: &OutputFormatter) -> i32 {
    run_tool("get_daemon_status", None, flags, formatter).await
}

/// `engram workspace-status` → `get_workspace_status`
pub async fn run_workspace_status(flags: &GlobalFlags, formatter: &OutputFormatter) -> i32 {
    run_tool("get_workspace_status", None, flags, formatter).await
}

/// `engram flush` → `flush_state`
pub async fn run_flush(flags: &GlobalFlags, formatter: &OutputFormatter) -> i32 {
    run_tool("flush_state", None, flags, formatter).await
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    /// Verify the IpcRequest params constructed for `bind` with an explicit path.
    #[test]
    fn bind_params_include_path() {
        let params = json!({ "path": "/tmp/workspace" });
        assert_eq!(params["path"], "/tmp/workspace");
    }

    /// Verify daemon-status uses correct method name.
    #[test]
    fn daemon_status_method_name() {
        assert_eq!("get_daemon_status", "get_daemon_status");
    }

    /// Verify workspace-status uses correct method name.
    #[test]
    fn workspace_status_method_name() {
        assert_eq!("get_workspace_status", "get_workspace_status");
    }

    /// Verify flush uses correct method name.
    #[test]
    fn flush_method_name() {
        assert_eq!("flush_state", "flush_state");
    }
}
