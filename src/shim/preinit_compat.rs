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
//! The interception allowlist is **exactly one method**. Every other frame,
//! including unknown id-bearing methods, is forwarded to rmcp unchanged so
//! rmcp's existing ordering and error semantics are preserved verbatim. The
//! filter disarms permanently on the first `initialize`, so there is no
//! steady-state cost and no post-handshake behavior change.
//!
//! Set `ENGRAM_MCP_PREINIT_COMPAT=0` to disable the window entirely and
//! restore strict rmcp ordering with no redeploy.

use serde_json::Value;
use tokio::io::{
    AsyncBufReadExt as _, AsyncRead, AsyncWrite, AsyncWriteExt as _, BufReader, DuplexStream,
};

/// Environment variable gating the pre-`initialize` compatibility window.
///
/// Enabled by default. Set to `0` to restore strict rmcp handshake ordering.
pub const PREINIT_COMPAT_ENV: &str = "ENGRAM_MCP_PREINIT_COMPAT";

/// The single method the compatibility window intercepts.
pub const COMPAT_METHOD: &str = "server/discover";

/// JSON-RPC 2.0 `Method not found` error code.
const METHOD_NOT_FOUND: i32 = -32601;

/// In-memory pipe capacity between the filter and rmcp's reader.
///
/// This is a backpressure buffer, not a frame-size limit: `write_all` simply
/// yields until rmcp drains the pipe, so frames larger than this still pass
/// through intact.
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
    /// Answer directly on stdout with this JSON-RPC frame and stay armed.
    Respond(String),
    /// Drop silently and stay armed. Used for allowlisted frames that carry
    /// no id: JSON-RPC forbids responding to a notification.
    Drop,
}

/// Decide whether the compatibility window should intercept `line`.
///
/// Only [`COMPAT_METHOD`] is intercepted (plan review finding F2). Anything
/// that is not valid JSON, not a JSON object, or carries any other method is
/// forwarded so rmcp's own semantics apply unchanged.
#[must_use]
pub fn classify_pre_initialize_frame(line: &str) -> PreInitDecision {
    let Ok(frame) = serde_json::from_str::<Value>(line) else {
        return PreInitDecision::Forward { disarm: false };
    };
    let Some(object) = frame.as_object() else {
        return PreInitDecision::Forward { disarm: false };
    };
    match object.get("method").and_then(Value::as_str) {
        Some("initialize") => PreInitDecision::Forward { disarm: true },
        Some(COMPAT_METHOD) => match object.get("id") {
            // A JSON-RPC request carries an id and MUST receive a response.
            // An absent or null id makes the frame a notification, which MUST
            // NOT be answered.
            Some(id) if !id.is_null() => PreInitDecision::Respond(method_not_found_frame(id)),
            _ => PreInitDecision::Drop,
        },
        _ => PreInitDecision::Forward { disarm: false },
    }
}

/// Build the `-32601` response frame for `id`.
///
/// `id` is echoed **verbatim and type-preserving** by cloning the parsed
/// [`Value`]. This matters: Copilot uses request id `0`, and coercing it to
/// `null`, absent, or the string `"0"` would leave the client unable to
/// correlate the response (plan review finding F4).
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

/// Interpose the compatibility window between `input` and rmcp's reader.
///
/// Returns a reader that rmcp consumes exactly as it would consume stdin.
/// A background task reads newline-delimited frames from `input`, applies
/// [`classify_pre_initialize_frame`] while armed, writes any synthesized
/// response to `output`, and forwards accepted frames **byte-for-byte** to
/// the returned reader. Framing is never re-encoded, so rmcp still owns
/// decoding and rmcp's own writer still owns every response it generates.
///
/// # Write-ordering invariant
///
/// The filter and rmcp hold independent handles to stdout, yet their writes
/// can never interleave. While armed, the filter only writes for frames it
/// does **not** forward, so rmcp has received nothing to respond to. The
/// first frame the filter forwards either disarms the window (`initialize`,
/// after which the filter never writes again) or is a non-`initialize` frame
/// that ends rmcp's handshake. In both cases the filter's last write happens
/// strictly before rmcp's first.
pub fn interpose_pre_initialize_filter<R, W>(input: R, output: W) -> DuplexStream
where
    R: AsyncRead + Send + Unpin + 'static,
    W: AsyncWrite + Send + Unpin + 'static,
{
    let (rmcp_side, filter_side) = tokio::io::duplex(PIPE_CAPACITY);
    tokio::spawn(async move {
        run_pre_initialize_filter(input, output, filter_side).await;
    });
    rmcp_side
}

/// Pump frames from `input` to `sink`, answering allowlisted pre-`initialize`
/// probes on `output`.
///
/// Returns when `input` reaches EOF or either side errors; `sink` is always
/// shut down so rmcp observes a clean EOF and the session ends normally
/// (preserving the exit-code taxonomy).
async fn run_pre_initialize_filter<R, W, S>(input: R, mut output: W, mut sink: S)
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
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
                    if write_frame(&mut output, &frame).await.is_err() {
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

/// Write one newline-delimited JSON-RPC frame and flush it.
async fn write_frame<W>(output: &mut W, frame: &str) -> std::io::Result<()>
where
    W: AsyncWrite + Unpin,
{
    output.write_all(frame.as_bytes()).await?;
    output.write_all(b"\n").await?;
    output.flush().await
}

#[cfg(test)]
mod tests {
    use super::*;

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

    /// Non-numeric ids are echoed with their original type preserved.
    #[test]
    fn string_id_is_echoed_type_preserving() {
        let decision = classify_pre_initialize_frame(
            r#"{"jsonrpc":"2.0","id":"probe-1","method":"server/discover"}"#,
        );
        let body = respond_body(&decision);
        assert_eq!(body["id"], Value::from("probe-1"));
    }

    /// JSON-RPC forbids responding to a notification.
    #[test]
    fn id_less_probe_is_dropped_silently() {
        assert_eq!(
            classify_pre_initialize_frame(r#"{"jsonrpc":"2.0","method":"server/discover"}"#),
            PreInitDecision::Drop
        );
        assert_eq!(
            classify_pre_initialize_frame(
                r#"{"jsonrpc":"2.0","id":null,"method":"server/discover"}"#
            ),
            PreInitDecision::Drop
        );
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
                PreInitDecision::Forward { disarm: false },
                "frame must forward to rmcp unchanged: {frame}"
            );
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
