//! IPC client: connects to the daemon via `interprocess` `LocalSocketStream`.
//!
//! Sends a newline-delimited JSON-RPC request and reads the response with
//! a configurable timeout. Used exclusively by the shim transport.

use std::time::Duration;

use interprocess::local_socket::tokio::prelude::*;
use interprocess::local_socket::tokio::{RecvHalf, SendHalf};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tracing::instrument;

use crate::daemon::protocol::{IpcRequest, IpcResponse};
use crate::errors::{EngramError, IpcError};

/// Send a single IPC request to the daemon at `endpoint` and return the response.
///
/// A fresh connection is opened for each call (stateless per-connection per
/// the IPC protocol contract). The entire connect + send + receive cycle is
/// wrapped in `timeout`.
///
/// # Errors
///
/// - [`EngramError::Ipc`] with [`IpcError::Timeout`] if the deadline elapses.
/// - [`EngramError::Ipc`] with [`IpcError::ConnectionFailed`] if the
///   socket/pipe cannot be reached.
/// - [`EngramError::Ipc`] with [`IpcError::SendFailed`] or
///   [`IpcError::ReceiveFailed`] for framing or serialization errors.
#[instrument(skip(request), fields(method = %request.method, endpoint = %endpoint))]
pub async fn send_request(
    endpoint: &str,
    request: &IpcRequest,
    timeout: Duration,
) -> Result<IpcResponse, EngramError> {
    let timeout_ms = u64::try_from(timeout.as_millis()).unwrap_or(u64::MAX);
    tokio::time::timeout(timeout, do_send(endpoint, request))
        .await
        .map_err(|_| EngramError::Ipc(IpcError::Timeout { timeout_ms }))?
}

/// Maximum byte count for a single daemon IPC response (1 MiB).
///
/// Enforced via [`tokio::io::AsyncReadExt::take`] to prevent a misbehaving
/// daemon from causing unbounded allocation in the shim process.
const MAX_RESPONSE_BYTES: u64 = 1024 * 1024;

async fn do_send(endpoint: &str, request: &IpcRequest) -> Result<IpcResponse, EngramError> {
    let (recv_half, send_half) = connect(endpoint).await?;

    // Serialize the request to a newline-terminated JSON line.
    let mut line = serde_json::to_string(request).map_err(|e| {
        EngramError::Ipc(IpcError::SendFailed {
            reason: e.to_string(),
        })
    })?;
    line.push('\n');

    let mut send_half = send_half;
    send_half.write_all(line.as_bytes()).await.map_err(|e| {
        EngramError::Ipc(IpcError::SendFailed {
            reason: e.to_string(),
        })
    })?;
    send_half.flush().await.map_err(|e| {
        EngramError::Ipc(IpcError::SendFailed {
            reason: e.to_string(),
        })
    })?;

    // Read exactly one response line with a 1 MiB size cap to prevent a
    // misbehaving daemon from causing unbounded allocation.
    let mut reader = BufReader::new(recv_half.take(MAX_RESPONSE_BYTES));
    let mut response_line = String::new();
    let n = reader.read_line(&mut response_line).await.map_err(|e| {
        EngramError::Ipc(IpcError::ReceiveFailed {
            reason: e.to_string(),
        })
    })?;

    // `read_line` returns `Ok(0)` on EOF — the daemon closed the connection
    // without writing a response (possible crash or early exit).
    if n == 0 {
        return Err(EngramError::Ipc(IpcError::ReceiveFailed {
            reason: "daemon closed connection without sending a response (possible crash)"
                .to_owned(),
        }));
    }

    serde_json::from_str(response_line.trim()).map_err(|e| {
        EngramError::Ipc(IpcError::ReceiveFailed {
            reason: format!("invalid JSON in daemon response: {e}"),
        })
    })
}

// ── Platform-specific connection helpers ─────────────────────────────────────

#[cfg(unix)]
async fn connect(endpoint: &str) -> Result<(RecvHalf, SendHalf), EngramError> {
    use interprocess::local_socket::{GenericFilePath, ToFsName};

    let name = endpoint.to_fs_name::<GenericFilePath>().map_err(|e| {
        EngramError::Ipc(IpcError::ConnectionFailed {
            address: endpoint.to_owned(),
            reason: e.to_string(),
        })
    })?;

    let stream = interprocess::local_socket::tokio::Stream::connect(name)
        .await
        .map_err(|e| {
            EngramError::Ipc(IpcError::ConnectionFailed {
                address: endpoint.to_owned(),
                reason: e.to_string(),
            })
        })?;

    Ok(stream.split())
}

#[cfg(windows)]
async fn connect(endpoint: &str) -> Result<(RecvHalf, SendHalf), EngramError> {
    use interprocess::local_socket::{GenericNamespaced, ToNsName};

    // `GenericNamespaced` expects the pipe name WITHOUT the `\\.\pipe\` prefix.
    let pipe_name = endpoint.strip_prefix(r"\\.\pipe\").unwrap_or(endpoint);

    let name = pipe_name.to_ns_name::<GenericNamespaced>().map_err(|e| {
        EngramError::Ipc(IpcError::ConnectionFailed {
            address: endpoint.to_owned(),
            reason: e.to_string(),
        })
    })?;

    let stream = interprocess::local_socket::tokio::Stream::connect(name)
        .await
        .map_err(|e| {
            EngramError::Ipc(IpcError::ConnectionFailed {
                address: endpoint.to_owned(),
                reason: e.to_string(),
            })
        })?;

    Ok(stream.split())
}

#[cfg(not(any(unix, windows)))]
async fn connect(_endpoint: &str) -> Result<(RecvHalf, SendHalf), EngramError> {
    Err(EngramError::Ipc(IpcError::ConnectionFailed {
        address: _endpoint.to_owned(),
        reason: "unsupported platform for IPC".to_owned(),
    }))
}
