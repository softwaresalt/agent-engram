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

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use interprocess::local_socket::{
    tokio::{prelude::*, Listener, Stream},
    ListenerOptions,
};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use engram::shim::version::ENGRAM_PROTOCOL_VERSION;

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
    /// Write a truncated (non-JSON) line then close the connection.
    TruncatedThenClose,
}

/// A running fake `_health` responder with an observable probe counter.
pub struct FakeHealthResponder {
    /// Number of `_health` requests received so far.
    pub probe_count: Arc<AtomicUsize>,
    /// Handle to the background accept loop; dropping cancels it.
    _task: tokio::task::JoinHandle<()>,
}

impl FakeHealthResponder {
    /// Bind the platform endpoint and start serving scripted `_health`
    /// responses. Each accepted connection reads one request line, increments
    /// `probe_count`, and writes the response determined by `script`.
    ///
    /// The `script` is shared across all connections — to change behavior
    /// mid-test, use `ScriptedResponder` (below) instead.
    pub fn spawn(endpoint: &str, script: HealthScript) -> Self {
        let probe_count = Arc::new(AtomicUsize::new(0));
        let count = Arc::clone(&probe_count);
        let listener = bind_listener(endpoint);
        let task = tokio::spawn(async move {
            accept_loop(listener, Arc::new(script), count).await;
        });
        Self {
            probe_count,
            _task: task,
        }
    }

    /// Spawn with a dynamically switchable script.
    pub fn spawn_dynamic(
        endpoint: &str,
        script: Arc<tokio::sync::Mutex<HealthScript>>,
    ) -> Self {
        let probe_count = Arc::new(AtomicUsize::new(0));
        let count = Arc::clone(&probe_count);
        let listener = bind_listener(endpoint);
        let task = tokio::spawn(async move {
            accept_loop_dynamic(listener, script, count).await;
        });
        Self {
            probe_count,
            _task: task,
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
) {
    loop {
        let Ok(stream) = listener.accept().await else {
            continue;
        };
        let s = Arc::clone(&script);
        let c = Arc::clone(&probe_count);
        tokio::spawn(async move {
            handle_connection(stream, &s, &c).await;
        });
    }
}

async fn accept_loop_dynamic(
    listener: Listener,
    script: Arc<tokio::sync::Mutex<HealthScript>>,
    probe_count: Arc<AtomicUsize>,
) {
    loop {
        let Ok(stream) = listener.accept().await else {
            continue;
        };
        let s = Arc::clone(&script);
        let c = Arc::clone(&probe_count);
        tokio::spawn(async move {
            let current = s.lock().await.clone();
            handle_connection(stream, &current, &c).await;
        });
    }
}

async fn handle_connection(
    stream: Stream,
    script: &HealthScript,
    probe_count: &AtomicUsize,
) {
    let (recv, mut send) = stream.split();
    let mut reader = BufReader::new(recv);
    let mut line = String::new();

    // Read the incoming request line.
    if reader.read_line(&mut line).await.is_err() {
        return;
    }

    // Only count and respond to _health requests.
    if let Ok(req) = serde_json::from_str::<Value>(line.trim()) {
        if req.get("method").and_then(Value::as_str) == Some("_health") {
            probe_count.fetch_add(1, Ordering::Relaxed);
            let id = req.get("id").cloned().unwrap_or(Value::Null);
            let response = build_response(&id, script);

            if let HealthScript::TruncatedThenClose = script {
                // Write a partial, non-JSON line then drop the connection.
                let _ = send.write_all(b"{\"jsonrpc\":\"2.0\",\"id\"").await;
                let _ = send.flush().await;
                return;
            }

            let mut resp_line = serde_json::to_string(&response)
                .unwrap_or_else(|_| String::from("{}"));
            resp_line.push('\n');
            let _ = send.write_all(resp_line.as_bytes()).await;
            let _ = send.flush().await;
        }
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
        HealthScript::TruncatedThenClose => {
            // Actual truncation is handled in handle_connection
            Value::Null
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
