//! Fake `_health` IPC responder for shim contract tests (138-F H1).
//!
//! Binds the platform IPC endpoint (Windows named pipe / Unix domain socket)
//! and replies to incoming `_health` JSON-RPC requests with caller-scripted
//! responses. Counts received requests behind an [`AtomicUsize`] exposed to
//! the test so probe-amplification and single-flight assertions are
//! observable end-to-end.
//!
//! This module adds NO production dependency — it lives under `tests/` and
//! uses only the `interprocess` crate already in the workspace dependency
//! graph.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use interprocess::local_socket::{
    ListenerOptions,
    tokio::{Listener, Stream, prelude::*},
};
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

use engram::shim::version::ENGRAM_PROTOCOL_VERSION;

/// How long the respawn-specific fake leaves its endpoint unbound after the
/// first `_shutdown`. This is long enough for the lifecycle's reachability
/// check to observe daemon exit, while remaining well inside its readiness
/// and concurrent-winner budgets.
const REBIND_RELEASE_WINDOW: std::time::Duration = std::time::Duration::from_millis(200);

/// What the fake responder should reply to each `_health` request.
#[derive(Clone, Debug)]
pub enum HealthScript {
    /// Version-compatible, `status == "ready"`.
    Ready,
    /// Version-compatible, caller-chosen non-ready status string.
    NotReady { status: String },
    /// Wrong `protocol_version`.
    VersionMismatch { version: u32 },
    /// JSON-RPC error with a caller-chosen code and message.
    JsonRpcError { code: i64, message: String },
    /// Valid JSON-RPC response with no `result` key.
    MissingResult,
    /// Valid JSON-RPC response with an undecodable `result`.
    UndecodableResult,
    /// Valid JSON-RPC response with an undecodable `result` whose
    /// wrong-typed `protocol_version` field is the caller-supplied string
    /// (used to prove serde's "invalid type" error text — which echoes the
    /// received value verbatim — never reaches an agent-visible surface).
    UndecodableResultWithPoisonedText(String),
    /// Write a truncated (non-JSON) line then close the connection.
    TruncatedThenClose,
    /// Write a syntactically valid JSON response without its newline frame
    /// delimiter, then close the connection.
    ValidJsonWithoutNewlineThenClose,
    /// Write more than the shim's response cap without a newline, then keep
    /// the connection open until the client disconnects.
    OversizedThenRemainOpen,
    /// Use `initial` for the first `switch_after` probes, then switch to `then`.
    Sequence {
        initial: Box<HealthScript>,
        switch_after: usize,
        then: Box<HealthScript>,
    },
}

/// A running fake `_health` responder with an observable probe counter.
pub struct FakeHealthResponder {
    /// Number of `_health` requests received so far.
    pub probe_count: Arc<AtomicUsize>,
    /// Handle to the background accept loop. Aborted (not merely detached)
    /// by this struct's `Drop` impl — a plain `JoinHandle` drop only
    /// detaches the task, it does not cancel it, which would otherwise
    /// leave the accept loop and bound platform endpoint alive across test
    /// boundaries and risk cross-test interference (Copilot review finding
    /// on PR #366).
    task: tokio::task::JoinHandle<()>,
    /// Signal to shut down the accept loop (set on `_shutdown` receipt).
    shutdown: Arc<tokio::sync::Notify>,
}

impl Drop for FakeHealthResponder {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl FakeHealthResponder {
    /// Bind the platform endpoint and start serving scripted `_health`
    /// responses. Each accepted connection reads one request line, increments
    /// `probe_count`, and writes the response determined by `script`.
    ///
    /// The `script` is shared across all connections — to change behavior
    /// mid-test, use `ScriptedResponder` (below) instead.
    ///
    /// The responder also handles `_shutdown` requests by stopping the accept
    /// loop, causing subsequent `probe()` calls to fail with connect-refused.
    pub fn spawn(endpoint: &str, script: HealthScript) -> Self {
        let probe_count = Arc::new(AtomicUsize::new(0));
        let count = Arc::clone(&probe_count);
        let shutdown = Arc::new(tokio::sync::Notify::new());
        let shutdown_clone = Arc::clone(&shutdown);
        let listener = bind_listener(endpoint);
        let task = tokio::spawn(async move {
            accept_loop(listener, Arc::new(script), count, shutdown_clone).await;
        });
        Self {
            probe_count,
            task,
            shutdown,
        }
    }

    /// Bind the endpoint, release it after the first `_shutdown`, then rebind
    /// once with the same script.
    ///
    /// This models an incompatible daemon being replaced by another
    /// incompatible daemon, allowing contract tests to exercise the shim's
    /// one permitted respawn without introducing a production-only seam.
    pub fn spawn_rebinding_after_shutdown(endpoint: &str, script: HealthScript) -> Self {
        let probe_count = Arc::new(AtomicUsize::new(0));
        let count = Arc::clone(&probe_count);
        let shutdown = Arc::new(tokio::sync::Notify::new());
        let shutdown_clone = Arc::clone(&shutdown);
        let listener = bind_listener(endpoint);
        let rebound_endpoint = endpoint.to_owned();
        let task = tokio::spawn(async move {
            let script = Arc::new(script);
            accept_loop(
                listener,
                Arc::clone(&script),
                Arc::clone(&count),
                Arc::clone(&shutdown_clone),
            )
            .await;

            tokio::time::sleep(REBIND_RELEASE_WINDOW).await;
            let listener = bind_listener(&rebound_endpoint);
            accept_loop(listener, script, count, shutdown_clone).await;
        });
        Self {
            probe_count,
            task,
            shutdown,
        }
    }

    /// Spawn with a dynamically switchable script.
    pub fn spawn_dynamic(endpoint: &str, script: Arc<tokio::sync::Mutex<HealthScript>>) -> Self {
        let probe_count = Arc::new(AtomicUsize::new(0));
        let count = Arc::clone(&probe_count);
        let shutdown = Arc::new(tokio::sync::Notify::new());
        let shutdown_clone = Arc::clone(&shutdown);
        let listener = bind_listener(endpoint);
        let task = tokio::spawn(async move {
            accept_loop_dynamic(listener, script, count, shutdown_clone).await;
        });
        Self {
            probe_count,
            task,
            shutdown,
        }
    }

    /// Current probe count.
    pub fn count(&self) -> usize {
        self.probe_count.load(Ordering::Relaxed)
    }
}

// ── Accept loops ──────────────────────────────────────────────────────────────

async fn accept_loop(
    listener: Listener,
    script: Arc<HealthScript>,
    probe_count: Arc<AtomicUsize>,
    shutdown: Arc<tokio::sync::Notify>,
) {
    loop {
        let stream = tokio::select! {
            result = listener.accept() => match result {
                Ok(s) => s,
                Err(_) => continue,
            },
            () = shutdown.notified() => return,
        };
        let s = Arc::clone(&script);
        let c = Arc::clone(&probe_count);
        let sd = Arc::clone(&shutdown);
        tokio::spawn(async move {
            handle_connection(stream, &s, &c, &sd).await;
        });
    }
}

async fn accept_loop_dynamic(
    listener: Listener,
    script: Arc<tokio::sync::Mutex<HealthScript>>,
    probe_count: Arc<AtomicUsize>,
    shutdown: Arc<tokio::sync::Notify>,
) {
    loop {
        let stream = tokio::select! {
            result = listener.accept() => match result {
                Ok(s) => s,
                Err(_) => continue,
            },
            () = shutdown.notified() => return,
        };
        let s = Arc::clone(&script);
        let c = Arc::clone(&probe_count);
        let sd = Arc::clone(&shutdown);
        tokio::spawn(async move {
            let current = s.lock().await.clone();
            handle_connection(stream, &current, &c, &sd).await;
        });
    }
}

async fn handle_connection(
    stream: Stream,
    script: &HealthScript,
    probe_count: &AtomicUsize,
    shutdown: &tokio::sync::Notify,
) {
    let (recv, mut send) = stream.split();
    let mut reader = BufReader::new(recv);
    let mut line = String::new();

    // Read the incoming request line.
    if reader.read_line(&mut line).await.is_err() {
        return;
    }

    // Parse the request to determine the method.
    if let Ok(req) = serde_json::from_str::<Value>(line.trim()) {
        let method = req.get("method").and_then(Value::as_str).unwrap_or("");
        if method == "_shutdown" {
            // Acknowledge shutdown and signal the accept loop to stop.
            let id = req.get("id").cloned().unwrap_or(Value::Null);
            let resp = json!({"jsonrpc": "2.0", "id": id, "result": {"status": "shutting_down"}});
            let mut resp_line = serde_json::to_string(&resp).unwrap_or_default();
            resp_line.push('\n');
            let _ = send.write_all(resp_line.as_bytes()).await;
            let _ = send.flush().await;
            // `notify_one`, not `notify_waiters`: the latter does not retain
            // a permit for a waiter that has not yet re-registered by the
            // time this fires (accept_loop's `tokio::select!` briefly drops
            // out of `shutdown.notified()` between loop iterations), which
            // could silently lose the shutdown signal and leave the
            // listener bound. `notify_one` stores exactly one permit when no
            // task is currently waiting, so the accept loop's next
            // `shutdown.notified().await` reliably observes it regardless
            // of timing (Copilot review finding on PR #366). Exactly one
            // task (accept_loop) ever awaits this Notify, so single-permit
            // semantics are correct here.
            shutdown.notify_one();
            return;
        }
        if method == "_health" {
            let count = probe_count.fetch_add(1, Ordering::Relaxed);
            let effective = resolve_script(script, count);
            let id = req.get("id").cloned().unwrap_or(Value::Null);

            if let HealthScript::TruncatedThenClose = effective {
                // Write a partial, non-JSON line then drop the connection.
                let _ = send.write_all(b"{\"jsonrpc\":\"2.0\",\"id\"").await;
                let _ = send.flush().await;
                return;
            }
            if let HealthScript::OversizedThenRemainOpen = effective {
                let oversized = vec![b' '; 1024 * 1024 + 1];
                let _ = send.write_all(&oversized).await;
                let _ = send.flush().await;
                let _ = reader.read_u8().await;
                return;
            }

            let response = build_response(&id, effective);
            let mut resp_line =
                serde_json::to_string(&response).unwrap_or_else(|_| String::from("{}"));
            if !matches!(effective, HealthScript::ValidJsonWithoutNewlineThenClose) {
                resp_line.push('\n');
            }
            let _ = send.write_all(resp_line.as_bytes()).await;
            let _ = send.flush().await;
        }
    }
}

/// Resolve a potentially sequenced script to the effective leaf script.
fn resolve_script(script: &HealthScript, probe_index: usize) -> &HealthScript {
    match script {
        HealthScript::Sequence {
            initial,
            switch_after,
            then,
        } => {
            if probe_index < *switch_after {
                resolve_script(initial, probe_index)
            } else {
                resolve_script(then, probe_index)
            }
        }
        other => other,
    }
}

fn build_response(id: &Value, script: &HealthScript) -> Value {
    match script {
        HealthScript::Ready => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "status": "ready",
                "uptime_seconds": 42,
                "workspace": null,
                "active_connections": 1,
                "protocol_version": ENGRAM_PROTOCOL_VERSION,
                "build_hash": "test-fake"
            }
        }),
        HealthScript::NotReady { status } => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "status": status,
                "uptime_seconds": 1,
                "workspace": null,
                "active_connections": 0,
                "protocol_version": ENGRAM_PROTOCOL_VERSION,
                "build_hash": "test-fake"
            }
        }),
        HealthScript::VersionMismatch { version } => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "status": "ready",
                "uptime_seconds": 42,
                "workspace": null,
                "active_connections": 1,
                "protocol_version": version,
                "build_hash": "test-fake-mismatched"
            }
        }),
        HealthScript::JsonRpcError { code, message } => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": code,
                "message": message
            }
        }),
        HealthScript::MissingResult => json!({
            "jsonrpc": "2.0",
            "id": id
        }),
        HealthScript::UndecodableResult => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "status": 42
            }
        }),
        HealthScript::UndecodableResultWithPoisonedText(poisoned) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "status": "ready",
                "protocol_version": poisoned
            }
        }),
        HealthScript::TruncatedThenClose => {
            // Actual truncation is handled in handle_connection
            Value::Null
        }
        HealthScript::ValidJsonWithoutNewlineThenClose => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {}
        }),
        HealthScript::OversizedThenRemainOpen => {
            // Actual oversized response is handled in handle_connection.
            Value::Null
        }
        HealthScript::Sequence { .. } => {
            // Sequence is resolved before reaching build_response.
            unreachable!("Sequence should be resolved by resolve_script")
        }
    }
}

// ── Platform-specific listener binding ────────────────────────────────────────

#[cfg(unix)]
fn bind_listener(endpoint: &str) -> Listener {
    use interprocess::local_socket::{GenericFilePath, ToFsName};

    // Remove stale socket file.
    let _ = std::fs::remove_file(endpoint);
    // Ensure parent directories exist.
    if let Some(parent) = std::path::Path::new(endpoint).parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let name = endpoint
        .to_fs_name::<GenericFilePath>()
        .expect("valid endpoint name");

    ListenerOptions::new()
        .name(name)
        .create_tokio()
        .expect("bind fake health responder listener")
}

#[cfg(windows)]
fn bind_listener(endpoint: &str) -> Listener {
    use interprocess::local_socket::{GenericNamespaced, ToNsName};

    let pipe_name = endpoint.strip_prefix(r"\\.\pipe\").unwrap_or(endpoint);

    let name = pipe_name
        .to_ns_name::<GenericNamespaced>()
        .expect("valid pipe name");

    ListenerOptions::new()
        .name(name)
        .create_tokio()
        .expect("bind fake health responder listener")
}
