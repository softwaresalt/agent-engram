//! rmcp `StdioTransport` setup and `ServerHandler` implementation for the shim.
//!
//! The shim's `ServerHandler` does not execute tools locally; it forwards every
//! `call_tool` request to the workspace daemon via the IPC client and returns
//! the daemon's response verbatim.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use rmcp::model::{
    CallToolRequestParams, CallToolResult, Content, Implementation, ListToolsResult,
    PaginatedRequestParams, ServerInfo,
};
use rmcp::service::RequestContext;
use rmcp::{ErrorData, RoleServer, ServerHandler};
use serde_json::Value;
use tracing::instrument;

use crate::daemon::protocol::IpcRequest;
use crate::errors::{EngramError, IpcError};

// ── Handler ───────────────────────────────────────────────────────────────────

/// MCP `ServerHandler` for the shim.
///
/// Forwards every [`call_tool`](ServerHandler::call_tool) request to the
/// workspace daemon via IPC and returns the daemon's response verbatim.
/// All other MCP methods use the default no-op implementations from
/// [`ServerHandler`].
#[derive(Clone)]
pub struct ShimHandler {
    /// IPC endpoint address for the daemon serving this workspace.
    endpoint: String,
    /// Request timeout for IPC calls.
    timeout: Duration,
    /// Monotonically incrementing request-id counter for JSON-RPC requests.
    next_id: Arc<AtomicU64>,
}

impl ShimHandler {
    /// Create a new `ShimHandler` that proxies requests to `endpoint`.
    pub fn new(endpoint: String, timeout: Duration) -> Self {
        Self {
            endpoint,
            timeout,
            next_id: Arc::new(AtomicU64::new(1)),
        }
    }

    fn next_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }
}

impl ServerHandler for ShimHandler {
    /// Return this shim's identity information.
    fn get_info(&self) -> ServerInfo {
        ServerInfo::default().with_server_info(Implementation::new(
            "engram-shim",
            env!("CARGO_PKG_VERSION"),
        ))
    }

    /// Forward a tool call to the daemon via IPC and translate the response.
    #[instrument(skip(self, _cx), fields(tool = %request.name, endpoint = %self.endpoint))]
    fn call_tool(
        &self,
        request: CallToolRequestParams,
        _cx: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<CallToolResult, ErrorData>> + Send + '_ {
        async move {
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

            let response =
                crate::shim::ipc_client::send_request(&self.endpoint, &ipc_req, self.timeout)
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

// ── Server entry point ────────────────────────────────────────────────────────

/// Start the shim MCP server over stdio, forwarding requests to `endpoint`.
///
/// Blocks until the MCP transport is closed (i.e., the parent process closes
/// stdin) or an unrecoverable error occurs.
///
/// # Errors
///
/// Returns [`EngramError::Ipc`] if the rmcp server fails to initialise.
pub async fn run_shim(endpoint: String, timeout: Duration) -> Result<(), EngramError> {
    let handler = ShimHandler::new(endpoint, timeout);
    let transport = rmcp::transport::io::stdio();

    let running = rmcp::serve_server(handler, transport).await.map_err(|e| {
        EngramError::Ipc(IpcError::ConnectionFailed {
            address: "stdio".to_owned(),
            reason: e.to_string(),
        })
    })?;

    // Wait for the MCP session to end (client disconnects or EOF on stdin).
    // Propagate errors so the caller can distinguish clean shutdown from failures.
    running.waiting().await.map_err(|e| {
        EngramError::Ipc(IpcError::ConnectionFailed {
            address: "stdio".to_owned(),
            reason: format!("MCP session ended with error: {e}"),
        })
    })?;
    Ok(())
}
