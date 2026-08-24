//! rmcp `StdioTransport` setup and `ServerHandler` implementation for the shim.
//!
//! The shim's `ServerHandler` does not execute tools locally; it forwards every
//! `call_tool` request to the workspace daemon via the IPC client. The daemon's
//! JSON-RPC response is parsed: if the result contains a `content` array,
//! text items are extracted and returned as MCP content; otherwise the full
//! result is serialised as a single text block.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use rmcp::model::{
    CallToolRequestParams, CallToolResult, Content, Implementation, ListToolsResult,
    PaginatedRequestParams, ServerCapabilities, ServerInfo,
};
use rmcp::service::RequestContext;
use rmcp::{ErrorData, RoleServer, ServerHandler};
use serde_json::Value;
use tokio::sync::watch;
use tracing::instrument;

use crate::daemon::protocol::IpcRequest;
use crate::errors::{EngramError, ShimFailureClass, ShimStartupError};
use crate::shim::StartupOutcome;

// ── Handler ───────────────────────────────────────────────────────────────────

/// MCP `ServerHandler` for the shim.
///
/// Forwards every [`call_tool`](ServerHandler::call_tool) request to the
/// workspace daemon via IPC once the deferred startup preconditions resolve
/// successfully. Responses with a `content` array have text items extracted;
/// other results are serialised as a single text block. If the preconditions
/// resolved to a degraded outcome, every `call_tool` fails with a structured
/// error naming the cause instead of touching IPC (124-F invariant: no
/// `tools/call` may succeed in a degraded session). All other MCP methods
/// use the default no-op implementations from [`ServerHandler`], except
/// [`list_tools`](ServerHandler::list_tools) which always serves the static
/// catalog regardless of startup outcome.
#[derive(Clone)]
pub struct ShimHandler {
    /// Deferred startup outcome, published once workspace admission, daemon
    /// readiness, and IPC endpoint derivation resolve. `None` until resolved.
    startup: watch::Receiver<Option<StartupOutcome>>,
    /// Request timeout for IPC calls and for awaiting startup resolution.
    timeout: Duration,
    /// Monotonically incrementing request-id counter for JSON-RPC requests.
    next_id: Arc<AtomicU64>,
}

impl ShimHandler {
    /// Create a new `ShimHandler` that awaits the deferred startup outcome
    /// on `startup` before forwarding requests.
    pub fn new(startup: watch::Receiver<Option<StartupOutcome>>, timeout: Duration) -> Self {
        Self {
            startup,
            timeout,
            next_id: Arc::new(AtomicU64::new(1)),
        }
    }

    fn next_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Wait for the deferred startup outcome to resolve.
    ///
    /// Deliberately NOT bounded by `self.timeout` (the IPC request timeout):
    /// the background precondition task is already bounded by
    /// `ensure_daemon_running`'s own configurable readiness budget
    /// (`ENGRAM_READY_TIMEOUT_MS`), which may legitimately exceed
    /// `self.timeout`. Imposing a second, shorter, hard-coded bound here
    /// would report a false `readiness_timeout` for a `tools/call` made
    /// while the real precondition work is still validly in progress
    /// (Copilot review finding on PR #349). The channel is guaranteed to
    /// resolve: every path through `compute_startup_outcome` sends exactly
    /// one value, and if the sender is dropped without sending (e.g. task
    /// panic), `changed()` returns `Err` immediately.
    async fn await_startup_outcome(&self) -> StartupOutcome {
        let mut rx = self.startup.clone();
        loop {
            if let Some(outcome) = rx.borrow().clone() {
                return outcome;
            }
            if rx.changed().await.is_err() {
                return StartupOutcome::Degraded {
                    class: ShimFailureClass::TransportFailure,
                    message: "startup outcome sender dropped before publishing a result".to_owned(),
                };
            }
        }
    }
}

impl ServerHandler for ShimHandler {
    /// Return this shim's identity information.
    ///
    /// Advertises the `tools` capability so spec-compliant MCP clients call
    /// `tools/list` and `tools/call`; without it engram's tools are silently
    /// omitted from the agent even though [`list_tools`](ServerHandler::list_tools)
    /// and [`call_tool`](ServerHandler::call_tool) are implemented.
    fn get_info(&self) -> ServerInfo {
        let mut info = ServerInfo::default().with_server_info(Implementation::new(
            "engram-shim",
            env!("CARGO_PKG_VERSION"),
        ));
        info.capabilities = ServerCapabilities::builder().enable_tools().build();
        info
    }

    /// Forward a tool call to the daemon via IPC and translate the response.
    ///
    /// If the deferred startup preconditions resolved to a degraded outcome
    /// (workspace admission, daemon readiness, or IPC endpoint derivation
    /// failed), this returns a structured [`ErrorData`] naming the recorded
    /// cause without attempting IPC. No `tools/call` succeeds in a degraded
    /// session (124-F invariant 3).
    #[instrument(skip(self, _cx), fields(tool = %request.name))]
    fn call_tool(
        &self,
        request: CallToolRequestParams,
        _cx: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<CallToolResult, ErrorData>> + Send + '_ {
        async move {
            let endpoint = match self.await_startup_outcome().await {
                StartupOutcome::Ready { endpoint } => endpoint,
                StartupOutcome::Degraded { class, message } => {
                    return Ok(degraded_call_tool_result(class, &message));
                }
            };

            let id = self.next_id();

            let params: Option<Value> = request
                .arguments
                .as_ref()
                .map(|obj| Value::Object(obj.clone()));

            let ipc_req = IpcRequest {
                jsonrpc: "2.0".to_owned(),
                id: Some(Value::Number(serde_json::Number::from(id))),
                method: request.name.to_string(),
                params,
            };

            let response = crate::shim::ipc_client::send_request(&endpoint, &ipc_req, self.timeout)
                .await
                .map_err(domain_to_mcp)?;

            if let Some(wire_err) = response.error {
                return Err(ErrorData::new(
                    rmcp::model::ErrorCode(wire_err.code),
                    wire_err.message,
                    wire_err.data,
                ));
            }

            let result_value = response.result.unwrap_or(Value::Null);

            // If the daemon result has a `content` array, extract text items.
            // Otherwise serialise the whole result as a single text block.
            let content = if let Some(arr) = result_value.get("content").and_then(Value::as_array) {
                arr.iter()
                    .filter_map(|item| item.get("text").and_then(Value::as_str).map(Content::text))
                    .collect()
            } else {
                vec![Content::text(result_value.to_string())]
            };

            Ok(CallToolResult::success(content))
        }
    }

    /// Return the full static tool catalog.
    ///
    /// The catalog is built at call time from [`crate::shim::tools_catalog::all_tools`]
    /// so that MCP clients receive accurate schema information without requiring
    /// a round-trip to the daemon.
    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _cx: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult {
            tools: crate::shim::tools_catalog::all_tools(),
            next_cursor: None,
            meta: None,
        })
    }
}

// ── Error conversion ──────────────────────────────────────────────────────────

fn domain_to_mcp(err: EngramError) -> ErrorData {
    ErrorData::internal_error(err.to_string(), None)
}

/// Translate a degraded startup outcome into a tool-level `CallToolResult`
/// naming the classified cause (124-F U4).
///
/// MCP distinguishes tool-level failures (`Ok(CallToolResult{ is_error: true,
/// .. })`, caller-visible) from protocol-level failures (`Err(ErrorData)`,
/// which MCP clients typically render opaquely without surfacing the
/// message). A degraded startup precondition is squarely a tool-level
/// failure — the request was valid and routed, but the shim currently cannot
/// execute it for a known, communicable reason — so it MUST be reported as a
/// `CallToolResult` for the calling agent to actually see the cause,
/// consistent with `rmcp::model::CallToolResult::error`'s documented
/// guidance. Uses `structured_error` so the `engram_code`/`failure_class`
/// (matching the `-32603` + `data.engram_code` convention used by
/// `daemon::ipc_server` and `cli::runner::friendly_error_message`) are both
/// machine-parseable via `structured_content` and human-visible in the
/// rendered `content` text.
fn degraded_call_tool_result(class: ShimFailureClass, message: &str) -> CallToolResult {
    let err = EngramError::ShimStartup(ShimStartupError {
        class,
        message: message.to_owned(),
    });
    CallToolResult::structured_error(serde_json::json!({
        "engram_code": class.wire_code(),
        "failure_class": class.as_str(),
        "message": err.to_string(),
    }))
}

// ── Server entry point ────────────────────────────────────────────────────────

/// Type-erased stdio pair rmcp binds as its transport.
type StdioTransportPair = (
    Box<dyn tokio::io::AsyncRead + Send + Unpin>,
    Box<dyn tokio::io::AsyncWrite + Send + Unpin>,
);

/// Build the stdio transport rmcp consumes, interposing the pre-`initialize`
/// compatibility window when it is enabled.
///
/// When enabled (the default), a narrow filter answers Copilot's pre-handshake
/// `server/discover` probe with JSON-RPC `-32601` and keeps the session alive
/// for the real `initialize`; every other frame reaches rmcp unchanged. Both
/// halves become in-memory pipes so a single background task owns the real
/// stdout and is its only writer. When `ENGRAM_MCP_PREINIT_COMPAT=0`, this
/// returns bare stdin/stdout so the transport matches
/// `rmcp::transport::io::stdio()` and strict rmcp handshake ordering is
/// restored. See [`crate::shim::preinit_compat`].
fn stdio_transport() -> StdioTransportPair {
    if crate::shim::preinit_compat::compat_enabled() {
        let interposed = crate::shim::preinit_compat::interpose_pre_initialize_filter(
            tokio::io::stdin(),
            tokio::io::stdout(),
        );
        (Box::new(interposed.reader), Box::new(interposed.writer))
    } else {
        (Box::new(tokio::io::stdin()), Box::new(tokio::io::stdout()))
    }
}

/// Start the shim MCP server over stdio.
///
/// Binds the transport and answers `initialize`/`tools/list` immediately.
/// `startup` publishes the deferred precondition outcome once resolved;
/// until then (and if it resolves to [`StartupOutcome::Degraded`]),
/// `tools/call` fails with a structured error instead of forwarding to a
/// daemon endpoint. Blocks until the MCP transport is closed (i.e., the
/// parent process closes stdin) or an unrecoverable error occurs.
///
/// # Errors
///
/// Returns [`EngramError::ShimStartup`] with [`ShimFailureClass::TransportFailure`]
/// if the rmcp server fails to bind or the MCP session ends with a protocol
/// error.
pub async fn run_shim(
    startup: watch::Receiver<Option<StartupOutcome>>,
    timeout: Duration,
) -> Result<(), EngramError> {
    let handler = ShimHandler::new(startup, timeout);
    let transport = stdio_transport();

    let running = rmcp::serve_server(handler, transport).await.map_err(|e| {
        EngramError::ShimStartup(ShimStartupError {
            class: ShimFailureClass::TransportFailure,
            message: format!("failed to bind MCP stdio transport: {e}"),
        })
    })?;

    // Wait for the MCP session to end (client disconnects or EOF on stdin).
    // Propagate errors so the caller can distinguish clean shutdown from failures.
    running.waiting().await.map_err(|e| {
        EngramError::ShimStartup(ShimStartupError {
            class: ShimFailureClass::TransportFailure,
            message: format!("MCP session ended with error: {e}"),
        })
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// The shim's `initialize` handshake MUST advertise the `tools` capability.
    ///
    /// Spec-compliant MCP clients (e.g. Copilot CLI) only call `tools/list` and
    /// `tools/call` when the server advertises the `tools` capability in its
    /// `InitializeResult`. Regression guard: `ServerInfo::default()` sets
    /// `capabilities.tools = None`, which caused engram's tools to be silently
    /// omitted from the agent even though `list_tools`/`call_tool` are implemented.
    #[test]
    fn get_info_advertises_tools_capability() {
        let (_tx, rx) = watch::channel(None);
        let handler = ShimHandler::new(rx, Duration::from_secs(1));
        let info = handler.get_info();
        assert!(
            info.capabilities.tools.is_some(),
            "shim get_info() must advertise the MCP tools capability so clients discover engram tools"
        );
    }
}
