//! IPC runner: routes CLI commands through the daemon via IPC.
//!
//! Resolves the workspace, ensures the daemon is running, builds an
//! [`IpcRequest`], calls the daemon, and maps the response to an exit code.

use std::time::Duration;

use serde_json::Value;

use crate::cli::flags::GlobalFlags;
use crate::cli::output::OutputFormatter;
use crate::daemon::ipc_server::ipc_endpoint;
use crate::daemon::protocol::{IpcError, IpcRequest};
use crate::shim::ipc_client;
use crate::shim::lifecycle::{check_health, ensure_daemon_running};

/// Default IPC request timeout for short-lived CLI commands (30 s).
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// IPC request timeout for long-running indexing commands (300 s).
pub const INDEXING_TIMEOUT_SECS: u64 = 300;

/// Engram-level error code for IndexInProgress, embedded in wire `data.engram_code`.
const INDEX_IN_PROGRESS_CODE: u16 = 7003;

/// JSON-RPC internal-error code (`-32603`) used as a secondary heuristic for
/// IndexInProgress detection when `engram_code` is absent.
const JSONRPC_INTERNAL_ERROR_CODE: i32 = -32_603;

/// Translate a wire-format [`IpcError`] into a user-facing message.
///
/// Primary path: `data.engram_code == 7003` → fixed friendly string.
/// Fallback path: code `-32603` whose message contains both "index" and "progress"
/// (case-insensitive) → same friendly string. This covers daemon versions that
/// return the `IndexInProgress` payload without an explicit `engram_code` field.
fn friendly_error_message(err: &IpcError) -> String {
    const INDEX_IN_PROGRESS_MSG: &str = "Indexing is in progress. \
         This command will be available once indexing completes. \
         Try again shortly.";

    // Primary: explicit engram_code in the data envelope.
    let primary_match = err
        .data
        .as_ref()
        .and_then(|d| d.get("engram_code"))
        .and_then(serde_json::Value::as_u64)
        .map(|code| code == u64::from(INDEX_IN_PROGRESS_CODE))
        .unwrap_or(false);

    if primary_match {
        return INDEX_IN_PROGRESS_MSG.to_owned();
    }

    // Fallback: -32603 internal error whose message text mentions indexing.
    let msg_lower = err.message.to_lowercase();
    if err.code == JSONRPC_INTERNAL_ERROR_CODE
        && msg_lower.contains("index")
        && msg_lower.contains("progress")
    {
        return INDEX_IN_PROGRESS_MSG.to_owned();
    }

    err.message.clone()
}

/// Run a single tool call through the daemon IPC and print the result.
///
/// `command_default_secs` is the per-command timeout default, which may be
/// overridden by `flags.timeout` / `ENGRAM_CLI_TIMEOUT`. Pass
/// [`DEFAULT_TIMEOUT_SECS`] for most commands, [`INDEXING_TIMEOUT_SECS`] for
/// `index` / `sync --full`.
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
    run_tool_timed(method, params, flags, formatter, DEFAULT_TIMEOUT_SECS).await
}

/// Like [`run_tool`] but with an explicit per-command timeout default.
pub async fn run_tool_timed(
    method: &str,
    params: Option<Value>,
    flags: &GlobalFlags,
    formatter: &OutputFormatter,
    command_default_secs: u64,
) -> i32 {
    let timeout: Duration = flags.ipc_timeout(command_default_secs);

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

    // Compute IPC endpoint before spawning so we can probe daemon liveness
    // and emit a progress hint when the daemon is not yet running.
    let endpoint = match ipc_endpoint(&workspace_path) {
        Ok(ep) => ep,
        Err(e) => return formatter.cli_error(&format!("cannot compute IPC endpoint: {e}")),
    };

    // Emit a progress hint when the daemon is not yet reachable so the
    // terminal does not appear frozen during the auto-spawn delay.
    if !check_health(&endpoint).await {
        formatter.progress_hint("Starting engram daemon...");
    }

    // Ensure daemon is running (auto-spawn if needed).
    if let Err(e) = ensure_daemon_running(&workspace_path).await {
        return formatter.cli_error(&format!("daemon unavailable: {e}"));
    }

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
    match ipc_client::send_request(&endpoint, &request, timeout).await {
        Ok(response) => {
            if let Some(result) = response.result {
                formatter.success(Some(id), result)
            } else if let Some(err) = response.error {
                let message = friendly_error_message(&err);
                formatter.tool_error(Some(id), i64::from(err.code), &message, err.data)
            } else {
                formatter.cli_error("daemon returned empty response")
            }
        }
        Err(e) => formatter.cli_error(&format!("IPC failure: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use serde_json::json;

    use crate::cli::flags::GlobalFlags;
    use crate::cli::output::{OutputFormatter, OutputMode};
    use crate::daemon::protocol::IpcError;

    use super::{INDEX_IN_PROGRESS_CODE, JSONRPC_INTERNAL_ERROR_CODE, friendly_error_message};

    fn make_formatter() -> OutputFormatter {
        OutputFormatter::new(OutputMode::Json)
    }

    fn make_flags(timeout: Option<u64>) -> GlobalFlags {
        GlobalFlags {
            workspace: None,
            id: None,
            json: false,
            format: None,
            quiet: false,
            timeout,
        }
    }

    fn make_ipc_error(code: i32, message: &str, data: Option<serde_json::Value>) -> IpcError {
        IpcError {
            code,
            message: message.to_owned(),
            data,
        }
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

    #[test]
    fn ipc_timeout_uses_explicit_flag_over_default() {
        let flags = make_flags(Some(120));
        assert_eq!(flags.ipc_timeout(30), Duration::from_secs(120));
    }

    #[test]
    fn ipc_timeout_falls_back_to_command_default() {
        let flags = make_flags(None);
        assert_eq!(flags.ipc_timeout(30), Duration::from_secs(30));
        assert_eq!(flags.ipc_timeout(300), Duration::from_secs(300));
    }

    #[test]
    fn index_in_progress_code_is_7003() {
        // Wire-format engram_code 7003 must trigger friendly message.
        let data = json!({ "engram_code": 7003_u64 });
        let code = data.get("engram_code").and_then(serde_json::Value::as_u64);
        assert_eq!(code, Some(7003));
    }

    #[test]
    fn friendly_message_primary_path_engram_code_7003() {
        let err = make_ipc_error(
            JSONRPC_INTERNAL_ERROR_CODE,
            "index operation in progress",
            Some(json!({ "engram_code": u64::from(INDEX_IN_PROGRESS_CODE) })),
        );
        let msg = friendly_error_message(&err);
        assert!(
            msg.contains("Indexing is in progress"),
            "expected friendly message, got: {msg}"
        );
    }

    #[test]
    fn friendly_message_fallback_path_no_engram_code() {
        // No engram_code in data — fall through to the -32603 + message heuristic.
        let err = make_ipc_error(
            JSONRPC_INTERNAL_ERROR_CODE,
            "Index operation in progress on this workspace",
            Some(json!({ "unrelated": true })),
        );
        let msg = friendly_error_message(&err);
        assert!(
            msg.contains("Indexing is in progress"),
            "expected friendly message from fallback, got: {msg}"
        );
    }

    #[test]
    fn friendly_message_primary_wins_when_both_signals_present() {
        // Both engram_code 7003 AND a matching message — primary path wins (same result).
        let err = make_ipc_error(
            JSONRPC_INTERNAL_ERROR_CODE,
            "Index operation in progress",
            Some(json!({ "engram_code": u64::from(INDEX_IN_PROGRESS_CODE) })),
        );
        let msg = friendly_error_message(&err);
        assert!(
            msg.contains("Indexing is in progress"),
            "expected friendly message, got: {msg}"
        );
    }

    #[test]
    fn friendly_message_no_false_positive_on_unrelated_32603() {
        // -32603 with unrelated message → original message preserved.
        let original = "internal server fault";
        let err = make_ipc_error(JSONRPC_INTERNAL_ERROR_CODE, original, None);
        let msg = friendly_error_message(&err);
        assert_eq!(msg, original, "unrelated -32603 must not be rewritten");
    }

    #[test]
    fn friendly_message_no_false_positive_wrong_code() {
        // Wrong JSON-RPC code with IndexInProgress-like message → original preserved.
        let err = make_ipc_error(-32_601, "index in progress", None);
        let msg = friendly_error_message(&err);
        assert!(
            !msg.contains("Indexing is in progress"),
            "wrong code must not trigger friendly message; got: {msg}"
        );
    }
}
