//! IPC runner: routes CLI commands through the daemon via IPC.
//!
//! Resolves the workspace, ensures the daemon is running, builds an
//! [`IpcRequest`], calls the daemon, and maps the response to an exit code.

use std::time::Duration;

use serde_json::Value;

use crate::cli::flags::GlobalFlags;
use crate::cli::output::OutputFormatter;
use crate::daemon::ipc_server::ipc_endpoint;
use crate::daemon::protocol::IpcRequest;
use crate::shim::ipc_client;
use crate::shim::lifecycle::ensure_daemon_running;

/// Default IPC request timeout for CLI invocations.
const CLI_IPC_TIMEOUT: Duration = Duration::from_secs(30);

/// Run a single tool call through the daemon IPC and print the result.
///
/// Returns the process exit code:
/// - `0` — success (result present)
/// - `1` — tool error (error present in response)
/// - `2` — connection / invocation failure (no response reached daemon)
pub async fn run_tool(
    method: &str,
    params: Option<Value>,
    flags: &GlobalFlags,
    formatter: &OutputFormatter,
) -> i32 {
    // Resolve workspace.
    let workspace_path = match flags.resolve_workspace() {
        Ok(p) => p,
        Err(e) => return formatter.cli_error(&e),
    };

    // Canonicalize only when the path exists; propagate permission errors.
    let workspace_path = match std::fs::canonicalize(&workspace_path) {
        Ok(p) => p,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => workspace_path,
        Err(e) => return formatter.cli_error(&format!("workspace path error: {e}")),
    };

    // Ensure daemon is running (auto-spawn if needed).
    if let Err(e) = ensure_daemon_running(&workspace_path).await {
        return formatter.cli_error(&format!("daemon unavailable: {e}"));
    }

    // Compute IPC endpoint.
    let endpoint = match ipc_endpoint(&workspace_path) {
        Ok(ep) => ep,
        Err(e) => return formatter.cli_error(&format!("cannot compute IPC endpoint: {e}")),
    };

    // Default to `1` when the caller does not supply an explicit request ID.
    // JSON-RPC 2.0 requires a non-null id for requests that expect a response;
    // `null` is rejected by the daemon validator.
    let id = flags.id_value().unwrap_or_else(|| Value::from(1_u64));

    let request = IpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: Some(id.clone()),
        method: method.to_owned(),
        params,
    };

    // Send request.
    match ipc_client::send_request(&endpoint, &request, CLI_IPC_TIMEOUT).await {
        Ok(response) => {
            if let Some(result) = response.result {
                formatter.success(Some(id), result)
            } else if let Some(err) = response.error {
                formatter.tool_error(Some(id), i64::from(err.code), &err.message, err.data)
            } else {
                formatter.cli_error("daemon returned empty response")
            }
        }
        Err(e) => formatter.cli_error(&format!("IPC failure: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use crate::cli::output::{OutputFormatter, OutputMode};

    fn make_formatter() -> OutputFormatter {
        OutputFormatter::new(OutputMode::Json)
    }

    #[test]
    fn exit_code_constants_are_correct() {
        // Verify the OutputFormatter exit code contract (unit-tested in output.rs).
        // This test documents the exit code meanings expected by run_tool callers.
        let f = make_formatter();
        assert_eq!(f.success(None, serde_json::json!({})), 0);
        assert_eq!(f.tool_error(None, 1, "err", None), 1);
        assert_eq!(f.cli_error("bad"), 2);
    }
}
