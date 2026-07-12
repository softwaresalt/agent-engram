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
    method == "index_workspace" || method == "sync_workspace"
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

/// Inject a validated correlation id into a request's `_meta.correlation_id`
/// envelope field, preserving any existing params and `_meta` entries.
///
/// Returns `params` unchanged when `correlation_id` is `None`. When present and
/// `params` is absent, a fresh `{ "_meta": { "correlation_id": id } }` object is
/// created; when `params` is a non-object value it is left untouched (the
/// daemon rejects malformed params on its own terms).
///
/// Exposed (crate-public API) so the dual-source correlation-id contract can be
/// proven end-to-end through the real daemon dispatch path.
#[must_use]
pub fn inject_correlation_id(params: Option<Value>, correlation_id: Option<&str>) -> Option<Value> {
    let Some(correlation_id) = correlation_id else {
        return params;
    };

    let mut params = params.unwrap_or_else(|| Value::Object(serde_json::Map::new()));
    let Some(object) = params.as_object_mut() else {
        return Some(params);
    };

    let meta = object
        .entry("_meta")
        .or_insert_with(|| Value::Object(serde_json::Map::new()));
    if let Some(meta_object) = meta.as_object_mut() {
        meta_object.insert(
            "correlation_id".to_owned(),
            Value::String(correlation_id.to_owned()),
        );
    } else {
        // Existing `_meta` was not an object — replace it with a valid envelope.
        *meta = serde_json::json!({ "correlation_id": correlation_id });
    }

    Some(params)
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
    // capture = false: the successful result is printed by move, never cloned,
    // so non-capturing callers (the common path: `unified_search`, `map_code`,
    // …) pay nothing for the capture feature (084.007-T / Thread-4).
    run_tool_dispatch(
        method,
        params,
        flags,
        formatter,
        command_default_secs,
        false,
    )
    .await
    .0
}

/// Like [`run_tool_timed`], but also returns the tool's successful result JSON
/// (when present) so a caller can post-process it — e.g. map a report field to a
/// domain exit code (084.007-T). Printed output and the base exit code are
/// identical to [`run_tool_timed`]; the returned `Option<Value>` is `Some` only
/// on a successful tool result and `None` on tool/connection errors.
pub async fn run_tool_timed_capture(
    method: &str,
    params: Option<Value>,
    flags: &GlobalFlags,
    formatter: &OutputFormatter,
    command_default_secs: u64,
) -> (i32, Option<Value>) {
    run_tool_dispatch(method, params, flags, formatter, command_default_secs, true).await
}

/// Shared dispatch core for [`run_tool_timed`] and [`run_tool_timed_capture`].
///
/// When `capture` is `false` the successful result JSON is handed to the
/// formatter by move (no clone) and `None` is returned; only when `capture` is
/// `true` — the CLI exit-code path that must post-process the report — is the
/// value cloned so it can be both printed and returned. This keeps large
/// `unified_search` / `map_code` responses off the clone path unless a caller
/// actually requests capture (084.007-T / Thread-4).
async fn run_tool_dispatch(
    method: &str,
    params: Option<Value>,
    flags: &GlobalFlags,
    formatter: &OutputFormatter,
    command_default_secs: u64,
    capture: bool,
) -> (i32, Option<Value>) {
    let timeout: Duration = flags.ipc_timeout(command_default_secs);

    // Resolve workspace.
    let workspace_path = match flags.resolve_workspace() {
        Ok(p) => p,
        Err(e) => return (formatter.cli_error(&e), None),
    };

    // Canonicalize only when the path exists; propagate permission errors.
    let workspace_path = match std::fs::canonicalize(&workspace_path) {
        Ok(p) => p,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => workspace_path,
        Err(e) => {
            return (
                formatter.cli_error(&format!("workspace path error: {e}")),
                None,
            );
        }
    };

    // Compute IPC endpoint before spawning so we can probe daemon liveness
    // and emit a progress hint when the daemon is not yet running.
    let endpoint = match ipc_endpoint(&workspace_path) {
        Ok(ep) => ep,
        Err(e) => {
            return (
                formatter.cli_error(&format!("cannot compute IPC endpoint: {e}")),
                None,
            );
        }
    };

    // Emit a progress hint when the daemon is not yet reachable so the
    // terminal does not appear frozen during the auto-spawn delay.
    if !check_health(&endpoint).await {
        formatter.progress_hint("Starting engram daemon...");
    }

    // Ensure daemon is running (auto-spawn if needed).
    if let Err(e) = ensure_daemon_running(&workspace_path).await {
        return (
            formatter.cli_error(&format!("daemon unavailable: {e}")),
            None,
        );
    }

    // Default to `1` when the caller does not supply an explicit request ID.
    // JSON-RPC 2.0 requires a non-null id for requests that expect a response;
    // `null` is rejected by the daemon validator.
    let id = flags.id_value().unwrap_or_else(|| Value::from(1_u64));

    // Resolve + validate the caller-supplied correlation id and inject it into
    // the request envelope (`params._meta.correlation_id`) so the daemon
    // dispatch choke point stamps it onto the emitted usage record. No per-tool
    // schema change — this is envelope-level metadata.
    let correlation_id = match flags.resolve_correlation_id() {
        Ok(value) => value,
        Err(e) => {
            return (
                formatter.cli_error(&format!("invalid --correlation-id: {e}")),
                None,
            );
        }
    };
    let params = inject_correlation_id(params, correlation_id.as_deref());

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
                if capture {
                    // Clone once so the value can be both printed and returned
                    // to the post-processing caller (084.007-T).
                    let code = formatter.success(Some(id), result.clone());
                    (code, Some(result))
                } else {
                    // Move the value into the formatter — no clone on the
                    // common non-capturing path (Thread-4).
                    let code = formatter.success(Some(id), result);
                    (code, None)
                }
            } else if let Some(err) = response.error {
                let message = friendly_error_message(&err);
                (
                    formatter.tool_error(Some(id), i64::from(err.code), &message, err.data),
                    None,
                )
            } else {
                (formatter.cli_error("daemon returned empty response"), None)
            }
        }
        Err(e) => (formatter.cli_error(&format!("IPC failure: {e}")), None),
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
        friendly_error_message, inject_correlation_id, render_indexing_progress,
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
            correlation_id: None,
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
    fn inject_correlation_id_none_leaves_params_untouched() {
        let params = Some(json!({ "query": "fn main" }));
        let out = inject_correlation_id(params.clone(), None);
        assert_eq!(out, params);
    }

    #[test]
    fn inject_correlation_id_creates_meta_when_params_absent() {
        let out = inject_correlation_id(None, Some("corr-1"));
        assert_eq!(
            out,
            Some(json!({ "_meta": { "correlation_id": "corr-1" } }))
        );
    }

    #[test]
    fn inject_correlation_id_adds_to_existing_params_and_meta() {
        let params = Some(json!({
            "query": "fn main",
            "_meta": { "agent_role": "reviewer" }
        }));
        let out = inject_correlation_id(params, Some("corr-2")).expect("params");
        assert_eq!(out["query"], json!("fn main"));
        assert_eq!(out["_meta"]["agent_role"], json!("reviewer"));
        assert_eq!(out["_meta"]["correlation_id"], json!("corr-2"));
    }

    #[test]
    fn resolve_correlation_id_rejects_control_chars() {
        let mut flags = make_flags(None);
        flags.correlation_id = Some("bad\nid".to_owned());
        assert!(flags.resolve_correlation_id().is_err());
    }

    #[test]
    fn resolve_correlation_id_accepts_valid_and_empty() {
        let mut flags = make_flags(None);
        flags.correlation_id = Some("corr-ok".to_owned());
        assert_eq!(
            flags.resolve_correlation_id().expect("valid"),
            Some("corr-ok".to_owned())
        );
        flags.correlation_id = Some(String::new());
        assert_eq!(flags.resolve_correlation_id().expect("empty"), None);
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
