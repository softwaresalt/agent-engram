//! IPC runner: routes CLI commands through the daemon via IPC.
//!
//! Resolves the workspace, ensures the daemon is running, builds an
//! [`IpcRequest`], calls the daemon, and maps the response to an exit code.

use std::time::Duration;

use serde_json::Value;

use crate::cli::flags::GlobalFlags;
use crate::cli::output::OutputFormatter;
use crate::daemon::ipc_server::ipc_endpoint;
use crate::daemon::protocol::{IpcError, IpcRequest, IpcResponse};
use crate::errors::EngramError;
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

/// Poll interval for text-mode indexing heartbeats.
const INDEXING_PROGRESS_POLL_SECS: u64 = 5;

#[derive(Debug, Clone, PartialEq, Eq)]
struct IndexingProgress {
    running: bool,
    files_scanned: u64,
    files_total: u64,
}

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

fn is_indexing_method(method: &str) -> bool {
    method == "index_workspace"
}

fn extract_indexing_progress(result: &Value) -> Option<IndexingProgress> {
    let scan_status = result.get("scan_status")?;
    Some(IndexingProgress {
        running: scan_status.get("running")?.as_bool()?,
        files_scanned: scan_status
            .get("files_scanned")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        files_total: scan_status
            .get("files_total")
            .and_then(Value::as_u64)
            .unwrap_or(0),
    })
}

fn render_indexing_progress(progress: Option<IndexingProgress>, elapsed: Duration) -> String {
    let elapsed_secs = elapsed.as_secs();
    match progress {
        Some(progress) if progress.files_total > 0 => format!(
            "Indexing workspace... {}/{} files ({elapsed_secs}s elapsed)",
            progress.files_scanned, progress.files_total
        ),
        Some(progress) if progress.running => {
            format!("Indexing workspace... working ({elapsed_secs}s elapsed)")
        }
        _ => format!("Indexing workspace... {elapsed_secs}s elapsed"),
    }
}

async fn fetch_indexing_progress(
    endpoint: &str,
    timeout: Duration,
) -> Result<Option<IndexingProgress>, EngramError> {
    let response = ipc_client::send_request(
        endpoint,
        &IpcRequest {
            jsonrpc: "2.0".to_owned(),
            id: Some(Value::from(0_u64)),
            method: "get_workspace_status".to_owned(),
            params: None,
        },
        timeout,
    )
    .await?;

    Ok(response.result.as_ref().and_then(extract_indexing_progress))
}

async fn send_request_with_optional_progress(
    endpoint: &str,
    request: &IpcRequest,
    timeout: Duration,
    formatter: &OutputFormatter,
    method: &str,
) -> Result<IpcResponse, EngramError> {
    if !is_indexing_method(method) || !formatter.shows_progress() {
        return ipc_client::send_request(endpoint, request, timeout).await;
    }

    formatter.progress_hint("Indexing workspace...");

    let request_future = ipc_client::send_request(endpoint, request, timeout);
    tokio::pin!(request_future);

    let heartbeat_start = tokio::time::Instant::now();
    let mut heartbeat = tokio::time::interval(Duration::from_secs(INDEXING_PROGRESS_POLL_SECS));
    heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    heartbeat.tick().await;

    let mut last_message = String::from("Indexing workspace...");

    loop {
        tokio::select! {
            response = &mut request_future => return response,
            _ = heartbeat.tick() => {
                let progress = fetch_indexing_progress(endpoint, Duration::from_secs(INDEXING_PROGRESS_POLL_SECS))
                    .await
                    .ok()
                    .flatten();
                let message = render_indexing_progress(progress, heartbeat_start.elapsed());
                if message != last_message {
                    formatter.progress_hint(&message);
                    last_message = message;
                }
            }
        }
    }
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
    match send_request_with_optional_progress(&endpoint, &request, timeout, formatter, method).await
    {
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

    use super::{
        INDEX_IN_PROGRESS_CODE, JSONRPC_INTERNAL_ERROR_CODE, extract_indexing_progress,
        friendly_error_message, render_indexing_progress,
    };

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

    #[test]
    fn extract_indexing_progress_reads_scan_status_snapshot() {
        let snapshot = json!({
            "scan_status": {
                "running": true,
                "files_scanned": 12,
                "files_total": 48
            }
        });

        let progress =
            extract_indexing_progress(&snapshot).expect("scan_status should parse into progress");
        assert!(progress.running, "running flag should be preserved");
        assert_eq!(progress.files_scanned, 12);
        assert_eq!(progress.files_total, 48);
    }

    #[test]
    fn render_indexing_progress_prefers_file_counts_when_available() {
        let message = render_indexing_progress(
            extract_indexing_progress(&json!({
                "scan_status": {
                    "running": true,
                    "files_scanned": 3,
                    "files_total": 10
                }
            })),
            Duration::from_secs(9),
        );

        assert!(
            message.contains("3/10 files"),
            "expected file-count progress, got: {message}"
        );
    }

    #[test]
    fn render_indexing_progress_degrades_to_elapsed_time_without_totals() {
        let message = render_indexing_progress(None, Duration::from_secs(11));
        assert!(
            message.contains("11s elapsed"),
            "expected elapsed-time heartbeat, got: {message}"
        );
    }
}
