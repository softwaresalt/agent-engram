//! Pre-`initialize` MCP compatibility window for the stdio shim (130-F).
//!
//! # Why this exists
//!
//! GitHub Copilot CLI `1.0.81-8` (prerelease) sends a JSON-RPC request with
//! id `0` and method `server/discover` **before** the MCP `initialize`
//! request. rmcp's server handshake reads the first frame and, if it is not
//! an `initialize` request, fails with `expect initialized request` and
//! terminates the process. The client observes a broken pipe and the shim
//! exits with [`crate::errors::ShimFailureClass::TransportFailure`]
//! (exit code 13).
//!
//! `server/discover` is undocumented in the `1.0.81-8` prerelease notes and
//! may be prerelease-only, so this module deliberately does **not** try to
//! implement it. It answers JSON-RPC `-32601` (`Method not found`) — exactly
//! what the GitHub MCP server does in the same Copilot run, which Copilot
//! demonstrably tolerates — and keeps waiting for a standards-compliant
//! `initialize`.
//!
//! # Blast radius
//!
//! The interception allowlist is **exactly one method**, and even then only
//! for a well-formed JSON-RPC 2.0 request carrying an id rmcp itself would
//! accept. Every other frame — including malformed envelopes and unsupported
//! id shapes — is forwarded so rmcp's own `Invalid Request` and ordering
//! semantics apply unchanged. The filter disarms permanently on the first
//! `initialize`, so there is no steady-state cost and no post-handshake
//! behavior change.
//!
//! # Single-writer invariant
//!
//! Both this module's synthesized responses and rmcp's own responses reach
//! stdout through **one** task holding **one** stdout handle
//! ([`run_output_pump`]), which writes a whole frame per call. Tokio makes no
//! atomicity guarantee for concurrent writes through separate `Stdout`
//! handles, so the compatibility window never takes a second handle.
//!
//! Set `ENGRAM_MCP_PREINIT_COMPAT=0` to disable the window entirely and
//! restore strict rmcp ordering.

use serde_json::Value;
use tokio::io::{
    AsyncBufReadExt as _, AsyncRead, AsyncWrite, AsyncWriteExt as _, BufReader, DuplexStream,
};
use tokio::sync::mpsc;

/// Environment variable gating the pre-`initialize` compatibility window.
///
/// Enabled by default. Set to `0` to restore strict rmcp handshake ordering.
pub const PREINIT_COMPAT_ENV: &str = "ENGRAM_MCP_PREINIT_COMPAT";

/// The single method the compatibility window intercepts.
pub const COMPAT_METHOD: &str = "server/discover";

/// JSON-RPC 2.0 `Method not found` error code.
const METHOD_NOT_FOUND: i32 = -32601;

/// In-memory pipe capacity between the filter and rmcp.
///
/// This is a backpressure buffer, not a frame-size limit: writes simply yield
/// until the peer drains the pipe, so larger frames still pass through intact.
const PIPE_CAPACITY: usize = 64 * 1024;

/// What the compatibility window decides to do with one decoded frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreInitDecision {
    /// Pass the frame to rmcp byte-for-byte. `disarm` is true only for
    /// `initialize`, which closes the compatibility window permanently.
    Forward {
        /// Whether this frame permanently disarms the compatibility window.
        disarm: bool,
    },
    /// Answer directly with this JSON-RPC frame and stay armed.
    Respond(String),
    /// Drop silently and stay armed. Used for an allowlisted frame with no
    /// `id` at all: JSON-RPC forbids responding to a notification.
    Drop,
}

/// Whether `id` is a request id rmcp itself accepts: a JSON string or an
/// integer.
///
/// Booleans, objects, arrays, fractional numbers, and integers outside the
/// signed/unsigned 64-bit range are **not** valid MCP request ids. Answering
/// them here would replace rmcp's `Invalid Request` handling with a
/// misleading `-32601`, so they are forwarded instead.
fn is_supported_request_id(id: &Value) -> bool {
    match id {
        Value::String(_) => true,
        Value::Number(number) => number.is_i64() || number.is_u64(),
        _ => false,
    }
}

/// Whether `params`, if present, is a structured JSON-RPC parameter value.
///
/// JSON-RPC 2.0 requires `params` to be an object or an array. A frame
/// carrying a scalar such as `"params": 1` is malformed, so it belongs to
/// rmcp's `Invalid Request` handling rather than to this compatibility
/// window.
fn has_valid_params(object: &serde_json::Map<String, Value>) -> bool {
    match object.get("params") {
        None => true,
        Some(params) => params.is_object() || params.is_array(),
    }
}

/// Decide whether the compatibility window should intercept `line`.
///
/// Interception requires **all** of: valid JSON, a JSON object, an exact
/// `"jsonrpc": "2.0"` envelope, structured or absent `params`, method
/// [`COMPAT_METHOD`], and a supported request id. Anything else forwards to
/// rmcp unchanged (plan review finding F2), so this can only ever absorb the
/// precise frame Copilot sends.
#[must_use]
pub fn classify_pre_initialize_frame(line: &str) -> PreInitDecision {
    let forward = PreInitDecision::Forward { disarm: false };
    let Ok(frame) = serde_json::from_str::<Value>(line) else {
        return forward;
    };
    let Some(object) = frame.as_object() else {
        return forward;
    };
    match object.get("method").and_then(Value::as_str) {
        Some("initialize") => PreInitDecision::Forward { disarm: true },
        Some(COMPAT_METHOD) => {
            // A malformed envelope is rmcp's to reject, not ours to answer.
            if object.get("jsonrpc").and_then(Value::as_str) != Some("2.0")
                || !has_valid_params(object)
            {
                return forward;
            }
            match object.get("id") {
                // No `id` member at all: a notification. Never answer it.
                None => PreInitDecision::Drop,
                Some(id) if is_supported_request_id(id) => {
                    PreInitDecision::Respond(method_not_found_frame(id))
                }
                // Explicit null, boolean, object, array, or a number rmcp
                // would reject: let rmcp classify it.
                Some(_) => forward,
            }
        }
        _ => forward,
    }
}

/// Build the `-32601` response frame for `id`.
///
/// `id` is echoed **verbatim and type-preserving**. This matters: Copilot uses
/// request id `0`, and coercing it to `null`, absent, or the string `"0"`
/// would leave the client unable to correlate the response (plan review
/// finding F4). Callers guarantee `id` passed [`is_supported_request_id`], so
/// the echoed value is always a valid JSON-RPC response id.
fn method_not_found_frame(id: &Value) -> String {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id.clone(),
        "error": {
            "code": METHOD_NOT_FOUND,
            "message": "Method not found",
        },
    })
    .to_string()
}

/// Whether the compatibility window is enabled for `value`, the raw value of
/// [`PREINIT_COMPAT_ENV`].
///
/// Enabled by default (including when unset or empty); `0` disables it.
#[must_use]
pub fn compat_enabled_for(value: Option<&str>) -> bool {
    !matches!(value.map(str::trim), Some("0"))
}

/// Whether the compatibility window is enabled in the current process
/// environment.
#[must_use]
pub fn compat_enabled() -> bool {
    compat_enabled_for(std::env::var(PREINIT_COMPAT_ENV).ok().as_deref())
}

/// The reader and writer rmcp binds when the compatibility window is active.
///
/// Both are in-memory pipes. Real stdin and stdout are owned exclusively by
/// the two background tasks that [`interpose_pre_initialize_filter`] spawns.
pub struct PreInitTransport {
    /// Stream rmcp reads client frames from, after filtering.
    pub reader: DuplexStream,
    /// Stream rmcp writes its responses to, drained by the output pump.
    pub writer: DuplexStream,
}

/// Interpose the compatibility window between rmcp and the real stdio streams.
///
/// Spawns two tasks:
///
/// * an **input filter** that reads frames from `input`, applies
///   [`classify_pre_initialize_frame`] while armed, and forwards accepted
///   frames byte-for-byte to [`PreInitTransport::reader`];
/// * an **output pump** that owns `output` and is the sole writer to it,
///   interleaving rmcp's responses and the filter's synthesized frames at
///   whole-frame granularity.
///
/// Framing is never re-encoded; rmcp still owns decoding and response
/// generation.
pub fn interpose_pre_initialize_filter<R, W>(input: R, output: W) -> PreInitTransport
where
    R: AsyncRead + Send + Unpin + 'static,
    W: AsyncWrite + Send + Unpin + 'static,
{
    let (rmcp_reader, filter_sink) = tokio::io::duplex(PIPE_CAPACITY);
    let (rmcp_writer, rmcp_responses) = tokio::io::duplex(PIPE_CAPACITY);
    let (frame_tx, frame_rx) = mpsc::unbounded_channel::<String>();

    tokio::spawn(async move {
        run_output_pump(output, rmcp_responses, frame_rx).await;
    });
    tokio::spawn(async move {
        run_pre_initialize_filter(input, frame_tx, filter_sink).await;
    });

    PreInitTransport {
        reader: rmcp_reader,
        writer: rmcp_writer,
    }
}

/// Drain rmcp's responses and the filter's synthesized frames onto `output`.
///
/// This task is the **only** writer to `output`. Each branch writes exactly
/// one complete newline-terminated frame per iteration, so frames are never
/// spliced together regardless of arrival timing.
async fn run_output_pump<W, S>(
    mut output: W,
    rmcp_responses: S,
    mut frames: mpsc::UnboundedReceiver<String>,
) where
    W: AsyncWrite + Unpin,
    S: AsyncRead + Unpin,
{
    let mut rmcp = BufReader::new(rmcp_responses);
    // Persisted across iterations on purpose: `read_until` is cancel-safe
    // only if bytes it already appended are retained, and `select!` may
    // cancel it. Cleared solely after a completed frame is written.
    let mut line = Vec::new();
    let mut frames_open = true;

    loop {
        tokio::select! {
            result = rmcp.read_until(b'\n', &mut line) => {
                match result {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {
                        if output.write_all(&line).await.is_err() {
                            break;
                        }
                        if output.flush().await.is_err() {
                            break;
                        }
                        line.clear();
                    }
                }
            }
            frame = frames.recv(), if frames_open => {
                match frame {
                    Some(frame) => {
                        let mut buffer = frame.into_bytes();
                        buffer.push(b'\n');
                        if output.write_all(&buffer).await.is_err() {
                            break;
                        }
                        if output.flush().await.is_err() {
                            break;
                        }
                    }
                    // The filter finished; keep draining rmcp until EOF.
                    None => frames_open = false,
                }
            }
        }
    }

    let _ = output.flush().await;
}

/// Pump frames from `input` to `sink`, routing allowlisted pre-`initialize`
/// probe answers to the output pump via `responses`.
///
/// Returns when `input` reaches EOF or either side errors; `sink` is always
/// shut down so rmcp observes a clean EOF and the session ends normally
/// (preserving the exit-code taxonomy).
async fn run_pre_initialize_filter<R, S>(
    input: R,
    responses: mpsc::UnboundedSender<String>,
    mut sink: S,
) where
    R: AsyncRead + Unpin,
    S: AsyncWrite + Unpin,
{
    let mut reader = BufReader::new(input);
    let mut armed = true;
    let mut line = Vec::new();

    loop {
        line.clear();
        match reader.read_until(b'\n', &mut line).await {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }

        if armed {
            // `from_utf8_lossy` never fails; a non-UTF-8 frame simply fails to
            // parse as JSON and is forwarded, letting rmcp report it.
            let text = String::from_utf8_lossy(&line);
            match classify_pre_initialize_frame(text.trim()) {
                PreInitDecision::Respond(frame) => {
                    if responses.send(frame).is_err() {
                        break;
                    }
                    continue;
                }
                PreInitDecision::Drop => continue,
                PreInitDecision::Forward { disarm } => armed = !disarm,
            }
        }

        if sink.write_all(&line).await.is_err() || sink.flush().await.is_err() {
            break;
        }

        if !armed {
            // The window is closed for the rest of the session: stop decoding
            // and become a pure byte copy so rmcp sees the untouched stream.
            let _ = tokio::io::copy_buf(&mut reader, &mut sink).await;
            break;
        }
    }

    let _ = sink.shutdown().await;
}

#[cfg(test)]
mod tests {
    use super::*;

    const FORWARD: PreInitDecision = PreInitDecision::Forward { disarm: false };

    fn respond_body(decision: &PreInitDecision) -> Value {
        match decision {
            PreInitDecision::Respond(frame) => {
                serde_json::from_str(frame).expect("synthesized frame must be valid JSON")
            }
            other => panic!("expected a Respond decision, got {other:?}"),
        }
    }

    /// The reproduced Copilot frame is intercepted and answered with -32601.
    #[test]
    fn copilot_probe_is_answered_with_method_not_found() {
        let decision = classify_pre_initialize_frame(
            r#"{"jsonrpc":"2.0","id":0,"method":"server/discover","params":{}}"#,
        );
        let body = respond_body(&decision);
        assert_eq!(body["jsonrpc"], "2.0");
        assert_eq!(body["error"]["code"], METHOD_NOT_FOUND);
    }

    /// Review finding F4: id `0` must round-trip as the JSON number `0`.
    #[test]
    fn zero_id_round_trips_as_a_json_number() {
        let decision =
            classify_pre_initialize_frame(r#"{"jsonrpc":"2.0","id":0,"method":"server/discover"}"#);
        let body = respond_body(&decision);
        assert!(
            body["id"].is_number(),
            "id 0 must stay a JSON number, not be coerced: {body}"
        );
        assert_eq!(body["id"].as_i64(), Some(0));
    }

    /// String ids are echoed with their original type preserved.
    #[test]
    fn string_id_is_echoed_type_preserving() {
        let decision = classify_pre_initialize_frame(
            r#"{"jsonrpc":"2.0","id":"probe-1","method":"server/discover"}"#,
        );
        let body = respond_body(&decision);
        assert_eq!(body["id"], Value::from("probe-1"));
    }

    /// A frame with no `id` member is a notification and must draw no reply.
    #[test]
    fn id_less_probe_is_dropped_silently() {
        assert_eq!(
            classify_pre_initialize_frame(r#"{"jsonrpc":"2.0","method":"server/discover"}"#),
            PreInitDecision::Drop
        );
    }

    /// Ids rmcp itself would reject must reach rmcp so its `Invalid Request`
    /// handling applies, rather than being answered with a misleading -32601.
    #[test]
    fn unsupported_id_shapes_forward_to_rmcp() {
        for frame in [
            r#"{"jsonrpc":"2.0","id":null,"method":"server/discover"}"#,
            r#"{"jsonrpc":"2.0","id":true,"method":"server/discover"}"#,
            r#"{"jsonrpc":"2.0","id":{},"method":"server/discover"}"#,
            r#"{"jsonrpc":"2.0","id":[1],"method":"server/discover"}"#,
            r#"{"jsonrpc":"2.0","id":1.5,"method":"server/discover"}"#,
        ] {
            assert_eq!(
                classify_pre_initialize_frame(frame),
                FORWARD,
                "unsupported id shape must forward to rmcp: {frame}"
            );
        }
    }

    /// A non-2.0 envelope or unstructured `params` is malformed and belongs
    /// to rmcp's `Invalid Request` handling.
    #[test]
    fn malformed_envelope_forwards_to_rmcp() {
        for frame in [
            r#"{"id":0,"method":"server/discover"}"#,
            r#"{"jsonrpc":"1.0","id":0,"method":"server/discover"}"#,
            r#"{"jsonrpc":2.0,"id":0,"method":"server/discover"}"#,
            r#"{"jsonrpc":"2.0","id":0,"method":"server/discover","params":1}"#,
            r#"{"jsonrpc":"2.0","id":0,"method":"server/discover","params":"x"}"#,
            r#"{"jsonrpc":"2.0","id":0,"method":"server/discover","params":null}"#,
            r#"{"jsonrpc":"2.0","id":0,"method":"server/discover","params":true}"#,
        ] {
            assert_eq!(
                classify_pre_initialize_frame(frame),
                FORWARD,
                "malformed envelope must forward to rmcp: {frame}"
            );
        }
    }

    /// Structured `params` — object, array, or absent — is accepted.
    #[test]
    fn structured_params_are_accepted() {
        for frame in [
            r#"{"jsonrpc":"2.0","id":0,"method":"server/discover"}"#,
            r#"{"jsonrpc":"2.0","id":0,"method":"server/discover","params":{}}"#,
            r#"{"jsonrpc":"2.0","id":0,"method":"server/discover","params":[]}"#,
        ] {
            let body = respond_body(&classify_pre_initialize_frame(frame));
            assert_eq!(
                body["error"]["code"], METHOD_NOT_FOUND,
                "structured params must still be answered: {frame}"
            );
        }
    }

    /// `initialize` forwards AND disarms the window permanently.
    #[test]
    fn initialize_forwards_and_disarms() {
        assert_eq!(
            classify_pre_initialize_frame(r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#),
            PreInitDecision::Forward { disarm: true }
        );
    }

    /// Review finding F2: the allowlist is exactly one method. Everything
    /// else — including unknown id-bearing methods rmcp would reject — must
    /// reach rmcp unchanged rather than being silently absorbed here.
    #[test]
    fn every_other_method_forwards_unchanged() {
        for frame in [
            r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
            r#"{"jsonrpc":"2.0","id":3,"method":"server/unknown"}"#,
            r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
            "not json at all",
            "[]",
            "",
        ] {
            assert_eq!(
                classify_pre_initialize_frame(frame),
                FORWARD,
                "frame must forward to rmcp unchanged: {frame}"
            );
        }
    }

    /// A synthesized frame must occupy exactly one stdout line.
    #[test]
    fn synthesized_frame_is_a_single_line() {
        match classify_pre_initialize_frame(
            r#"{"jsonrpc":"2.0","id":0,"method":"server/discover"}"#,
        ) {
            PreInitDecision::Respond(frame) => assert!(
                !frame.contains('\n'),
                "the synthesized frame must not embed a newline: {frame}"
            ),
            other => panic!("expected a Respond decision, got {other:?}"),
        }
    }

    /// The kill-switch defaults to enabled and only `0` disables it.
    #[test]
    fn kill_switch_defaults_to_enabled() {
        assert!(compat_enabled_for(None));
        assert!(compat_enabled_for(Some("")));
        assert!(compat_enabled_for(Some("1")));
        assert!(compat_enabled_for(Some("true")));
        assert!(!compat_enabled_for(Some("0")));
        assert!(!compat_enabled_for(Some(" 0 ")));
    }
}
