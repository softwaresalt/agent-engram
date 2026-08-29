//! rmcp `StdioTransport` setup and `ServerHandler` implementation for the shim.
//!
//! The shim's `ServerHandler` does not execute tools locally; it forwards every
//! `call_tool` request to the workspace daemon via the IPC client. The daemon's
//! JSON-RPC response is parsed: if the result contains a `content` array,
//! text items are extracted and returned as MCP content; otherwise the full
//! result is serialised as a single text block.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Weak};
use std::time::Duration;

use rmcp::model::{
    CallToolRequestParams, CallToolResult, Content, Implementation, ListToolsResult,
    PaginatedRequestParams, ServerCapabilities, ServerInfo,
};
use rmcp::service::RequestContext;
use rmcp::{ErrorData, RoleServer, ServerHandler};
use serde_json::Value;
use tokio::sync::{Mutex, watch};
use tokio::time::Instant;
use tracing::instrument;

use crate::daemon::protocol::IpcRequest;
use crate::errors::{EngramError, ShimFailureClass, ShimStartupError};
use crate::shim::StartupOutcome;
use crate::shim::lifecycle::HealthOutcome;

const RECOVERY_PROBE_COOLDOWN: Duration = Duration::from_millis(250);

/// Type alias for the probe function used by `ShimHandler`.
///
/// Defaults to [`crate::shim::lifecycle::probe_health`]. Tests can inject a
/// custom implementation via [`ShimHandler::with_probe`] to script outcomes
/// and observe probe counts without touching the real IPC path.
type ProbeFn = Arc<
    dyn Fn(String) -> std::pin::Pin<Box<dyn std::future::Future<Output = HealthOutcome> + Send>>
        + Send
        + Sync,
>;

/// Build the default production probe function.
fn default_probe() -> ProbeFn {
    Arc::new(|endpoint: String| {
        Box::pin(async move { crate::shim::lifecycle::probe_health(&endpoint).await })
    })
}

#[derive(Default)]
struct RecoveryProbeState {
    last_failure: Option<Instant>,
}

enum EndpointResolutionError {
    Permanent {
        class: ShimFailureClass,
        message: String,
    },
    Recoverable {
        message: String,
    },
}

#[derive(Clone)]
enum StartupPublisher {
    Legacy(Weak<watch::Sender<Option<StartupOutcome>>>),
    Strong(Arc<watch::Sender<Option<StartupOutcome>>>),
}

impl StartupPublisher {
    fn upgrade(&self) -> Option<Arc<watch::Sender<Option<StartupOutcome>>>> {
        match self {
            Self::Legacy(publisher) => publisher.upgrade(),
            Self::Strong(publisher) => Some(Arc::clone(publisher)),
        }
    }
}

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
    /// Startup publisher used by request-triggered recovery probes.
    ///
    /// Production construction retains a strong publisher for the handler's
    /// lifetime, closing the terminal-latch race described in
    /// [`ShimHandler::with_strong_publisher`]. The legacy public constructor
    /// retains its source-compatible weak parameter.
    startup_tx: StartupPublisher,
    /// Deferred startup outcome, published once workspace admission, daemon
    /// readiness, and IPC endpoint derivation resolve. `None` until resolved.
    startup: watch::Receiver<Option<StartupOutcome>>,
    /// Single-flight guard for request-triggered late-readiness probes.
    recovery_lock: Arc<Mutex<RecoveryProbeState>>,
    /// Request timeout for IPC calls and for awaiting startup resolution.
    timeout: Duration,
    /// Monotonically incrementing request-id counter for JSON-RPC requests.
    next_id: Arc<AtomicU64>,
    /// Health-probe function invoked during late-readiness recovery.
    /// Default: [`crate::shim::lifecycle::probe_health`].
    probe: ProbeFn,
}

impl ShimHandler {
    /// Create a new `ShimHandler` that awaits the deferred startup outcome
    /// on `startup` before forwarding requests.
    ///
    /// This source-compatible entry point accepts the weak publisher exposed
    /// by the target branch. Production code uses the internal strong
    /// publisher constructor so terminal health proofs cannot be lost.
    pub fn new(
        startup_tx: Weak<watch::Sender<Option<StartupOutcome>>>,
        startup: watch::Receiver<Option<StartupOutcome>>,
        timeout: Duration,
    ) -> Self {
        Self::from_publisher(StartupPublisher::Legacy(startup_tx), startup, timeout)
    }

    /// Construct a handler that retains the startup publisher strongly.
    fn with_strong_publisher(
        startup_tx: Arc<watch::Sender<Option<StartupOutcome>>>,
        startup: watch::Receiver<Option<StartupOutcome>>,
        timeout: Duration,
    ) -> Self {
        Self::from_publisher(StartupPublisher::Strong(startup_tx), startup, timeout)
    }

    fn from_publisher(
        startup_tx: StartupPublisher,
        startup: watch::Receiver<Option<StartupOutcome>>,
        timeout: Duration,
    ) -> Self {
        Self {
            startup_tx,
            startup,
            recovery_lock: Arc::new(Mutex::new(RecoveryProbeState::default())),
            timeout,
            next_id: Arc::new(AtomicU64::new(1)),
            probe: default_probe(),
        }
    }

    /// Test-only constructor that overrides the health-probe function.
    #[cfg(test)]
    pub(crate) fn with_probe(
        startup_tx: Arc<watch::Sender<Option<StartupOutcome>>>,
        startup: watch::Receiver<Option<StartupOutcome>>,
        timeout: Duration,
        probe: ProbeFn,
    ) -> Self {
        Self {
            startup_tx: StartupPublisher::Strong(startup_tx),
            startup,
            recovery_lock: Arc::new(Mutex::new(RecoveryProbeState::default())),
            timeout,
            next_id: Arc::new(AtomicU64::new(1)),
            probe,
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

    async fn forwarding_endpoint(&self) -> Result<String, EndpointResolutionError> {
        match self.await_startup_outcome().await {
            StartupOutcome::Ready { endpoint } => Ok(endpoint),
            StartupOutcome::Degraded { class, message } => {
                Err(EndpointResolutionError::Permanent { class, message })
            }
            StartupOutcome::WaitingForReadiness { .. } => {
                let mut recovery = self.recovery_lock.lock().await;
                match self.await_startup_outcome().await {
                    StartupOutcome::Ready { endpoint } => Ok(endpoint),
                    StartupOutcome::Degraded { class, message } => {
                        Err(EndpointResolutionError::Permanent { class, message })
                    }
                    StartupOutcome::WaitingForReadiness { endpoint, message } => {
                        if recovery
                            .last_failure
                            .is_some_and(|failed_at| failed_at.elapsed() < RECOVERY_PROBE_COOLDOWN)
                        {
                            return Err(EndpointResolutionError::Recoverable { message });
                        }

                        let Some(startup_tx) = self.startup_tx.upgrade() else {
                            return Err(EndpointResolutionError::Permanent {
                                class: ShimFailureClass::TransportFailure,
                                message: "startup outcome publisher dropped during late-readiness \
                                          recovery"
                                    .to_owned(),
                            });
                        };

                        match (self.probe)(endpoint.clone()).await {
                            HealthOutcome::Ready => {
                                recovery.last_failure = None;
                                let published = startup_tx.send_if_modified(|current| {
                                    if matches!(current, Some(StartupOutcome::Degraded { .. })) {
                                        return false; // Degraded is absorbing
                                    }
                                    *current = Some(StartupOutcome::Ready {
                                        endpoint: endpoint.clone(),
                                    });
                                    true
                                });
                                if published {
                                    Ok(endpoint)
                                } else {
                                    // Reconcile with the channel: a concurrent
                                    // publisher (the independent monitor)
                                    // latched Degraded while THIS probe was
                                    // in flight and resolved Ready. Forwarding
                                    // this call to the daemon anyway would
                                    // contradict the now-authoritative
                                    // terminal state (Copilot review finding
                                    // on PR #366) — report the terminal
                                    // classification that actually won
                                    // instead of the stale Ready this probe
                                    // observed.
                                    match startup_tx.borrow().clone() {
                                        Some(StartupOutcome::Degraded { class, message }) => {
                                            Err(EndpointResolutionError::Permanent {
                                                class,
                                                message,
                                            })
                                        }
                                        _ => Ok(endpoint),
                                    }
                                }
                            }
                            HealthOutcome::Transient => {
                                recovery.last_failure = Some(Instant::now());
                                // Reconcile with the channel: a concurrent
                                // publisher may have latched Degraded while
                                // this probe was in flight and resolved
                                // Transient — report the terminal
                                // classification that actually won instead of
                                // a stale Recoverable.
                                match startup_tx.borrow().clone() {
                                    Some(StartupOutcome::Degraded { class, message }) => {
                                        Err(EndpointResolutionError::Permanent { class, message })
                                    }
                                    _ => Err(EndpointResolutionError::Recoverable { message }),
                                }
                            }
                            HealthOutcome::Terminal(kind) => {
                                let terminal_message = kind.client_message();
                                startup_tx.send_if_modified(|current| {
                                    if matches!(current, Some(StartupOutcome::Degraded { .. })) {
                                        return false; // Already terminal
                                    }
                                    *current = Some(StartupOutcome::Degraded {
                                        class: ShimFailureClass::ProtocolIncompatible,
                                        message: terminal_message.clone(),
                                    });
                                    true
                                });
                                Err(EndpointResolutionError::Permanent {
                                    class: ShimFailureClass::ProtocolIncompatible,
                                    message: terminal_message,
                                })
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Fixed, client-safe message for a terminal `HealthOutcome` kind.
///
/// Contains NO daemon-supplied free-form text, no paths, no env values.
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
            let endpoint = match self.forwarding_endpoint().await {
                Ok(endpoint) => endpoint,
                Err(EndpointResolutionError::Permanent { class, message }) => {
                    return Ok(degraded_call_tool_result(class, &message, false));
                }
                Err(EndpointResolutionError::Recoverable { message }) => {
                    return Ok(degraded_call_tool_result(
                        ShimFailureClass::ReadinessTimeout,
                        &message,
                        true,
                    ));
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
fn degraded_call_tool_result(
    class: ShimFailureClass,
    message: &str,
    recoverable: bool,
) -> CallToolResult {
    let err = EngramError::ShimStartup(ShimStartupError {
        class,
        message: message.to_owned(),
    });
    let mut structured = serde_json::json!({
        "engram_code": class.wire_code(),
        "failure_class": class.as_str(),
        "message": err.to_string(),
        "recoverable": recoverable,
    });
    if recoverable {
        structured["retry_after_ms"] =
            Value::from(u64::try_from(RECOVERY_PROBE_COOLDOWN.as_millis()).unwrap_or(u64::MAX));
    }
    CallToolResult::structured_error(structured)
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
/// if the weak startup publisher can no longer be upgraded, the rmcp server
/// fails to bind, or the MCP session ends with a protocol error.
pub async fn run_shim(
    startup_tx: Weak<watch::Sender<Option<StartupOutcome>>>,
    startup: watch::Receiver<Option<StartupOutcome>>,
    timeout: Duration,
) -> Result<(), EngramError> {
    let startup_tx = startup_tx.upgrade().ok_or_else(|| {
        EngramError::ShimStartup(ShimStartupError {
            class: ShimFailureClass::TransportFailure,
            message: "startup outcome publisher dropped before shim transport initialization"
                .to_owned(),
        })
    })?;
    run_shim_with_strong_publisher(startup_tx, startup, timeout).await
}

/// Start the shim MCP server while retaining the startup publisher strongly.
///
/// This is the production path: keeping the publisher alive for the handler's
/// full lifetime prevents a request-triggered terminal health proof from being
/// lost when the independent readiness monitor exits.
pub(crate) async fn run_shim_with_strong_publisher(
    startup_tx: Arc<watch::Sender<Option<StartupOutcome>>>,
    startup: watch::Receiver<Option<StartupOutcome>>,
    timeout: Duration,
) -> Result<(), EngramError> {
    let handler = ShimHandler::with_strong_publisher(startup_tx, startup, timeout);
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
    use crate::shim::lifecycle::{HealthOutcome, TerminalKind};
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
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
        let (tx, rx) = watch::channel(None);
        let tx = Arc::new(tx);
        let handler = ShimHandler::new(Arc::downgrade(&tx), rx, Duration::from_secs(1));
        let info = handler.get_info();
        assert!(
            info.capabilities.tools.is_some(),
            "shim get_info() must advertise the MCP tools capability so clients discover engram tools"
        );
    }

    /// Source-compatibility contract: the public constructor continues to
    /// accept the weak startup publisher exposed by the target branch.
    #[test]
    fn public_constructor_accepts_weak_startup_publisher() {
        let (tx, rx) = watch::channel(None);
        let tx = Arc::new(tx);

        let _handler = ShimHandler::new(Arc::downgrade(&tx), rx, Duration::from_secs(1));
    }

    /// Source-compatibility and failure contract for the public transport
    /// entry point: an expired legacy weak publisher is rejected explicitly
    /// before stdio is bound.
    #[tokio::test]
    async fn public_run_shim_accepts_weak_and_rejects_expired_publisher() {
        let (tx, rx) = watch::channel(None);
        let tx = Arc::new(tx);
        let weak_tx = Arc::downgrade(&tx);
        drop(tx);

        let result = run_shim(weak_tx, rx, Duration::from_secs(1)).await;
        let Err(EngramError::ShimStartup(error)) = result else {
            panic!("expired startup publisher must return a classified shim error");
        };
        assert_eq!(error.class, ShimFailureClass::TransportFailure);
        assert!(
            error.message.contains("publisher"),
            "upgrade failure should identify the dropped publisher: {}",
            error.message
        );
    }

    // ── 138-F concurrency / amplification harness ─────────────────────────

    /// Build a `ShimHandler` in `WaitingForReadiness` state with a custom probe.
    fn handler_in_waiting(
        probe: ProbeFn,
    ) -> (Arc<watch::Sender<Option<StartupOutcome>>>, ShimHandler) {
        let (tx, rx) = watch::channel(None);
        let tx = Arc::new(tx);
        let _ = tx.send(Some(StartupOutcome::WaitingForReadiness {
            endpoint: "test-endpoint".to_owned(),
            message: "waiting for readiness (test)".to_owned(),
        }));
        let handler = ShimHandler::with_probe(Arc::clone(&tx), rx, Duration::from_secs(30), probe);
        (tx, handler)
    }

    /// C1 — single-flight suppresses concurrent probes (138.006-T).
    ///
    /// 8 callers synchronized by a start barrier outside the handler. The seam
    /// probe signals `probe_entered` on entry then awaits `release`. While held
    /// assert `probe_count == 1`. After release and join: `probe_count == 1`
    /// and all 8 returned recoverable payloads (the 7 non-winners parked on the
    /// mutex, saw fresh cooldown, and returned without probing). No sleeps; the
    /// probe never waits on sibling callers (D8 corrected topology).
    ///
    /// NEW-RED: asserts all callers get recoverable; currently green because
    /// the single-flight mutex + cooldown already produce this result. This test
    /// pins the current behavior so that Phase 3 changes preserve it.
    #[tokio::test]
    #[allow(clippy::similar_names)]
    async fn c1_single_flight_suppresses_concurrent_probes() {
        for _ in 0..5 {
            let probe_count = Arc::new(AtomicUsize::new(0));
            let probe_entered = Arc::new(tokio::sync::Notify::new());
            let release = Arc::new(tokio::sync::Notify::new());

            let pc = Arc::clone(&probe_count);
            let pe = Arc::clone(&probe_entered);
            let rel = Arc::clone(&release);
            let probe: ProbeFn = Arc::new(move |_endpoint| {
                let pc = Arc::clone(&pc);
                let pe = Arc::clone(&pe);
                let rel = Arc::clone(&rel);
                Box::pin(async move {
                    pc.fetch_add(1, AtomicOrdering::SeqCst);
                    pe.notify_one();
                    rel.notified().await;
                    HealthOutcome::Transient
                })
            });

            let (_tx, handler) = handler_in_waiting(probe);
            let start = Arc::new(tokio::sync::Barrier::new(9)); // 8 callers + test driver

            let mut handles = Vec::new();
            for _ in 0..8 {
                let h = handler.clone();
                let b = Arc::clone(&start);
                handles.push(tokio::spawn(async move {
                    b.wait().await;
                    h.forwarding_endpoint().await
                }));
            }
            start.wait().await;

            // Wait for the single winner to enter the probe.
            probe_entered.notified().await;
            assert_eq!(
                probe_count.load(AtomicOrdering::SeqCst),
                1,
                "C1: only one probe should be in-flight while held"
            );

            // Release the probe.
            release.notify_one();

            let mut recoverable_count = 0;
            for h in handles {
                let result = h.await.expect("task should not panic");
                if let Err(EndpointResolutionError::Recoverable { .. }) = result {
                    recoverable_count += 1;
                }
            }
            assert_eq!(
                probe_count.load(AtomicOrdering::SeqCst),
                1,
                "C1: total probes must be 1"
            );
            assert_eq!(
                recoverable_count, 8,
                "C1: all 8 callers should get recoverable"
            );
        }
    }

    /// C2 — cooldown suppresses a follow-up probe (138.006-T).
    ///
    /// Uses `start_paused = true` for deterministic time control. After a
    /// transient probe at `t0`: a call at `t0 + 50 ms` performs 0 probes and
    /// returns recoverable; a call after `t0 + 250 ms` performs exactly 1 probe.
    /// Requires the 138.013-T `tokio::time::Instant` clock seam.
    ///
    /// NEW-RED: asserts cooldown behavior; currently green because the 250 ms
    /// cooldown already works. Pins the current behavior for Phase 3.
    #[tokio::test(start_paused = true)]
    async fn c2_cooldown_suppresses_followup_probe() {
        for _ in 0..5 {
            let probe_count = Arc::new(AtomicUsize::new(0));
            let pc = Arc::clone(&probe_count);
            let probe: ProbeFn = Arc::new(move |_| {
                let pc = Arc::clone(&pc);
                Box::pin(async move {
                    pc.fetch_add(1, AtomicOrdering::SeqCst);
                    HealthOutcome::Transient
                })
            });

            let (_tx, handler) = handler_in_waiting(probe);

            // First probe at t0.
            let r = handler.forwarding_endpoint().await;
            assert!(matches!(
                r,
                Err(EndpointResolutionError::Recoverable { .. })
            ));
            assert_eq!(
                probe_count.load(AtomicOrdering::SeqCst),
                1,
                "C2: first call probes"
            );

            // At t0 + 50 ms: within cooldown, no probe.
            tokio::time::advance(Duration::from_millis(50)).await;
            let r = handler.forwarding_endpoint().await;
            assert!(matches!(
                r,
                Err(EndpointResolutionError::Recoverable { .. })
            ));
            assert_eq!(
                probe_count.load(AtomicOrdering::SeqCst),
                1,
                "C2: within cooldown should not probe"
            );

            // At t0 + 250 ms: cooldown expired, should probe again.
            tokio::time::advance(Duration::from_millis(200)).await;
            let r = handler.forwarding_endpoint().await;
            assert!(matches!(
                r,
                Err(EndpointResolutionError::Recoverable { .. })
            ));
            assert_eq!(
                probe_count.load(AtomicOrdering::SeqCst),
                2,
                "C2: after cooldown should probe"
            );
        }
    }

    /// C3 — terminal latch under concurrency (138.006-T).
    ///
    /// 8 concurrent calls where the single in-flight probe resolves with a
    /// terminal outcome. All 8 must return the terminal (Permanent) payload,
    /// total probe count is 1, and a 9th call afterwards performs 0 probes.
    ///
    /// NEW-RED: currently `probe() -> false` yields `Recoverable`, not
    /// `Permanent`. The terminal latch does not exist until 138.004-T lands.
    #[tokio::test]
    #[allow(clippy::similar_names)]
    async fn c3_terminal_latch_under_concurrency() {
        for _ in 0..5 {
            let probe_count = Arc::new(AtomicUsize::new(0));
            let probe_entered = Arc::new(tokio::sync::Notify::new());
            let release = Arc::new(tokio::sync::Notify::new());

            let pc = Arc::clone(&probe_count);
            let pe = Arc::clone(&probe_entered);
            let rel = Arc::clone(&release);
            // Probe returns Terminal: simulates a terminal outcome.
            let probe: ProbeFn = Arc::new(move |_| {
                let pc = Arc::clone(&pc);
                let pe = Arc::clone(&pe);
                let rel = Arc::clone(&rel);
                Box::pin(async move {
                    pc.fetch_add(1, AtomicOrdering::SeqCst);
                    pe.notify_one();
                    rel.notified().await;
                    HealthOutcome::Terminal(TerminalKind::MethodNotFound)
                })
            });

            let (_tx, handler) = handler_in_waiting(probe);
            let start = Arc::new(tokio::sync::Barrier::new(9));

            let mut handles = Vec::new();
            for _ in 0..8 {
                let h = handler.clone();
                let b = Arc::clone(&start);
                handles.push(tokio::spawn(async move {
                    b.wait().await;
                    h.forwarding_endpoint().await
                }));
            }
            start.wait().await;
            probe_entered.notified().await;
            release.notify_one();

            let mut permanent_count = 0;
            for h in handles {
                let result = h.await.expect("task should not panic");
                if matches!(result, Err(EndpointResolutionError::Permanent { .. })) {
                    permanent_count += 1;
                }
            }
            // RED: currently false → Recoverable, not Permanent (no terminal latch)
            assert_eq!(
                permanent_count, 8,
                "C3: all 8 should get terminal (Permanent) payload"
            );
            assert_eq!(
                probe_count.load(AtomicOrdering::SeqCst),
                1,
                "C3: total probes must be 1"
            );

            // 9th call: terminal is latched, must perform 0 additional probes.
            let before = probe_count.load(AtomicOrdering::SeqCst);
            let ninth = handler.forwarding_endpoint().await;
            assert_eq!(
                probe_count.load(AtomicOrdering::SeqCst),
                before,
                "C3: 9th call must not probe (terminal latched)"
            );
            assert!(
                matches!(ninth, Err(EndpointResolutionError::Permanent { .. })),
                "C3: 9th call must still return terminal payload"
            );
        }
    }

    /// C6 — inverse race: an independent publisher (standing in for the
    /// late-readiness monitor) publishes `Ready` WHILE the request-triggered
    /// probe is already in flight and later resolves `Terminal`. The
    /// request path's terminal latch must still win — `Degraded` is the
    /// final state — because `Degraded` proof arriving after `Ready` is
    /// definitive and must not be dropped merely because it lands second.
    ///
    /// This is the "inverse" direction of C5 (monitor Ready racing a
    /// request-path Terminal, rather than a request-path Degraded racing a
    /// monitor Ready) — the pairing Copilot review requested on PR #366 to
    /// accompany the fix that made `ShimHandler.startup_tx` a strong `Arc`
    /// (previously `Weak`, which could fail to publish at all if the
    /// monitor had already exited and dropped the last other strong
    /// sender).
    #[tokio::test]
    async fn c6_inverse_race_monitor_ready_then_request_terminal_still_latches() {
        let probe_entered = Arc::new(tokio::sync::Notify::new());
        let release = Arc::new(tokio::sync::Notify::new());
        let pe = Arc::clone(&probe_entered);
        let rel = Arc::clone(&release);

        // Probe blocks until released, then resolves Terminal — simulating
        // a request-triggered probe that is already in flight when an
        // independent publisher (the monitor) races ahead and publishes
        // Ready on the same shared channel.
        let probe: ProbeFn = Arc::new(move |_| {
            let pe = Arc::clone(&pe);
            let rel = Arc::clone(&rel);
            Box::pin(async move {
                pe.notify_one();
                rel.notified().await;
                HealthOutcome::Terminal(TerminalKind::MethodNotFound)
            })
        });

        let (tx, handler) = handler_in_waiting(probe);
        let mut rx = tx.subscribe();

        let call = tokio::spawn({
            let handler = handler.clone();
            async move { handler.forwarding_endpoint().await }
        });

        // Wait for the request-path probe to genuinely enter (in flight).
        probe_entered.notified().await;

        // Simulate the monitor independently publishing Ready while the
        // request-path probe is still in flight.
        tx.send_if_modified(|current| {
            if matches!(current, Some(StartupOutcome::Degraded { .. })) {
                return false;
            }
            *current = Some(StartupOutcome::Ready {
                endpoint: "test-endpoint".to_owned(),
            });
            true
        });
        assert!(
            matches!(
                rx.borrow_and_update().clone(),
                Some(StartupOutcome::Ready { .. })
            ),
            "C6 setup: the simulated monitor publish must have landed as Ready first"
        );

        // Simulate the monitor task exiting immediately after publishing
        // Ready, dropping its own strong sender — the exact sequence
        // Copilot's review flagged. Only ShimHandler's own strong Arc
        // (this test's whole point) keeps the channel writable from here.
        drop(tx);

        // Release the in-flight probe; it now resolves Terminal and must
        // still be able to publish — ShimHandler.startup_tx is a strong Arc,
        // so this publish can never silently no-op even after the
        // monitor-stand-in's own sender has been dropped.
        release.notify_one();
        let result = call.await.expect("task should not panic");
        assert!(
            matches!(result, Err(EndpointResolutionError::Permanent { .. })),
            "C6: the caller whose probe resolved Terminal must get the terminal payload"
        );

        let final_state = rx.borrow_and_update().clone();
        assert!(
            matches!(final_state, Some(StartupOutcome::Degraded { .. })),
            "C6: final state must be Degraded — a later-arriving Terminal proof must win over an \
             earlier Ready, and must never be silently dropped; got {final_state:?}"
        );
    }

    /// C7 — the other inverse race: an independent publisher (standing in
    /// for the monitor) latches `Degraded` WHILE the request-triggered probe
    /// is already in flight and later resolves `Ready`. The request path
    /// must NOT forward the call on the strength of its own stale `Ready`
    /// result — it must reconcile with the channel and report the
    /// already-latched terminal classification instead.
    ///
    /// This is the counterpart to C6 covering the two remaining
    /// probe-outcome branches (`Ready` and `Transient`) that previously
    /// returned their own probe's verdict unconditionally, even when a
    /// concurrent publisher had already latched `Degraded` while that exact
    /// probe was in flight (Copilot review finding on PR #366: "the current
    /// tools/call is forwarded even though the session is already
    /// degraded").
    #[tokio::test]
    async fn c7_request_ready_reconciles_with_concurrently_latched_terminal() {
        let probe_entered = Arc::new(tokio::sync::Notify::new());
        let release = Arc::new(tokio::sync::Notify::new());
        let pe = Arc::clone(&probe_entered);
        let rel = Arc::clone(&release);

        // Probe blocks until released, then resolves Ready — simulating a
        // request-triggered probe already in flight when an independent
        // publisher (the monitor) races ahead and latches Degraded.
        let probe: ProbeFn = Arc::new(move |_| {
            let pe = Arc::clone(&pe);
            let rel = Arc::clone(&rel);
            Box::pin(async move {
                pe.notify_one();
                rel.notified().await;
                HealthOutcome::Ready
            })
        });

        let (tx, handler) = handler_in_waiting(probe);

        let call = tokio::spawn({
            let handler = handler.clone();
            async move { handler.forwarding_endpoint().await }
        });

        probe_entered.notified().await;

        // Simulate the monitor independently latching Degraded while the
        // request-path probe is still in flight.
        tx.send_if_modified(|current| {
            if matches!(current, Some(StartupOutcome::Degraded { .. })) {
                return false;
            }
            *current = Some(StartupOutcome::Degraded {
                class: ShimFailureClass::ProtocolIncompatible,
                message: "monitor latched terminal (test)".to_owned(),
            });
            true
        });

        release.notify_one();
        let result = call.await.expect("task should not panic");
        assert!(
            matches!(result, Err(EndpointResolutionError::Permanent { .. })),
            "C7: a stale Ready must not forward the call once a concurrent publisher has already \
             latched Degraded — the caller must get the terminal classification instead"
        );

        let final_state = tx.borrow().clone();
        assert!(
            matches!(final_state, Some(StartupOutcome::Degraded { .. })),
            "C7: final state must remain Degraded; got {final_state:?}"
        );
    }

    /// N2 — no extra round trip (138.006-T).
    ///
    /// A terminal outcome must consume exactly 1 `_health` request, not 2.
    ///
    /// NEW-RED: currently `probe() -> false` yields `Recoverable`, not
    /// `Permanent`. Terminal classification does not exist until 138.001-T.
    #[tokio::test]
    async fn n2_terminal_outcome_consumes_exactly_one_probe() {
        for _ in 0..5 {
            let probe_count = Arc::new(AtomicUsize::new(0));
            let pc = Arc::clone(&probe_count);
            let probe: ProbeFn = Arc::new(move |_| {
                let pc = Arc::clone(&pc);
                Box::pin(async move {
                    pc.fetch_add(1, AtomicOrdering::SeqCst);
                    HealthOutcome::Terminal(TerminalKind::MethodNotFound)
                })
            });

            let (_tx, handler) = handler_in_waiting(probe);

            // First call: should produce exactly 1 probe.
            let result = handler.forwarding_endpoint().await;
            assert_eq!(
                probe_count.load(AtomicOrdering::SeqCst),
                1,
                "N2: terminal must consume exactly 1 probe"
            );
            // RED: currently returns Recoverable, not Permanent
            assert!(
                matches!(result, Err(EndpointResolutionError::Permanent { .. })),
                "N2: terminal outcome should be permanent"
            );
        }
    }

    /// N1 — happy-path probe-count neutrality pin (138.011-T).
    ///
    /// When the session is already `Ready`, `forwarding_endpoint` returns the
    /// endpoint immediately without invoking the probe function at all. This
    /// pins the exact request-path probe count: **0** probes per `tools/call`
    /// in the happy path (the same behavior as `main` at `2e1e01cf`).
    ///
    /// NEUTRALITY PIN: must be GREEN at authoring. If this fails, the phase-1
    /// seam (138.013-T) introduced a behavior change.
    #[tokio::test]
    async fn n1_happy_path_zero_request_path_probes() {
        let probe_count = Arc::new(AtomicUsize::new(0));
        let pc = Arc::clone(&probe_count);
        let probe: ProbeFn = Arc::new(move |_| {
            let pc = Arc::clone(&pc);
            Box::pin(async move {
                pc.fetch_add(1, AtomicOrdering::SeqCst);
                HealthOutcome::Ready
            })
        });

        let (tx, rx) = watch::channel(None);
        let tx = Arc::new(tx);
        // Publish Ready — the happy path.
        let _ = tx.send(Some(StartupOutcome::Ready {
            endpoint: "test-happy-path-endpoint".to_owned(),
        }));
        let handler = ShimHandler::with_probe(Arc::clone(&tx), rx, Duration::from_secs(30), probe);

        // Multiple forwarding_endpoint calls in the Ready state.
        for _ in 0..10 {
            let result = handler.forwarding_endpoint().await;
            assert!(matches!(result, Ok(ref e) if e == "test-happy-path-endpoint"));
        }

        // N1 PIN: exactly 0 probes in the happy path.
        assert_eq!(
            probe_count.load(AtomicOrdering::SeqCst),
            0,
            "N1 NEUTRALITY PIN: ready-state forwarding_endpoint must perform exactly 0 probes"
        );
    }
}
