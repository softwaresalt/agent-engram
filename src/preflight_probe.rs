//! Provenance probe facades: the CLI probe (plan unit PRE-5, task 142.067-T)
//! and the stdio MCP probe (plan unit PRE-6, task 142.068-T).
//!
//! Synchronous, bounded child I/O: a capped reader thread feeds the owner
//! loop, which keys on the CLI child's exit and never joins the reader.

use std::io::{self, BufRead, Read, Write};
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

use thiserror::Error;

/// One read the probe issues through the engram CLI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProbeRead {
    /// CLI arguments appended after `--workspace <W> --json`.
    pub cli_args: &'static [&'static str],
    /// MCP tool the CLI read maps to.
    pub mcp_tool: &'static str,
    /// MCP arguments, as a JSON text.
    pub mcp_arguments_json: &'static str,
}

/// Generation-pinned statistics read.
pub const PREFLIGHT_READ: ProbeRead = ProbeRead {
    cli_args: &["stats"],
    mcp_tool: "get_workspace_statistics",
    mcp_arguments_json: "{}",
};

/// Health read.
pub const HEALTH_READ: ProbeRead = ProbeRead {
    cli_args: &["health"],
    mcp_tool: "get_health_report",
    mcp_arguments_json: "{}",
};

/// Parsed probe result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeReport {
    /// `false` when the daemon answered with a typed JSON refusal.
    pub accepted: bool,
    /// Published generation ID, when the read carries provenance.
    pub generation_id: Option<String>,
    /// `code_files=<n>;functions=<n>;classes=<n>;interfaces=<n>;edges=<n>`.
    pub data_fingerprint: Option<String>,
}

/// Typed probe failure; causes are static text only.
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum ProbeError {
    /// The child could not be spawned.
    #[error("probe spawn failed: {0}")]
    Spawn(&'static str),
    /// The deadline elapsed.
    #[error("probe deadline elapsed")]
    Deadline,
    /// The child exited unsuccessfully without a typed refusal.
    #[error("probe child exit: {0}")]
    Exit(&'static str),
    /// Output exceeded the 1 MiB cap.
    #[error("probe output exceeded the cap")]
    Oversize,
    /// Output was unreadable or not a report.
    #[error("probe output malformed: {0}")]
    Malformed(&'static str),
    /// The observed generation differs from the expected one.
    #[error("probe generation mismatch")]
    Mismatch,
}

const OUTPUT_CAP: usize = 1024 * 1024;
const POLL_INTERVAL: Duration = Duration::from_millis(20);
const POST_EXIT_DRAIN: Duration = Duration::from_millis(250);
const BACKOFF_START: Duration = Duration::from_millis(50);
const BACKOFF_MAX: Duration = Duration::from_millis(500);

enum ReaderMsg {
    Chunk(Vec<u8>),
    ReadError,
    Eof,
}

struct BoundedOutput {
    stdout: Vec<u8>,
    status: ExitStatus,
}

fn spawn_reader(stdout: impl Read + Send + 'static) -> Receiver<ReaderMsg> {
    let (tx, rx) = mpsc::channel();
    // The handle is dropped on purpose: a grandchild holding the pipe open
    // must never block the owner. Tradeoff: such a detached reader thread can
    // stay blocked (leaked) until the grandchild closes the pipe or exits.
    drop(thread::spawn(move || {
        let mut capped = stdout.take(OUTPUT_CAP as u64 + 1);
        let mut buf = [0_u8; 8192];
        loop {
            let msg = match capped.read(&mut buf) {
                Ok(0) => ReaderMsg::Eof,
                Ok(n) => ReaderMsg::Chunk(buf[..n].to_vec()),
                Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => ReaderMsg::ReadError,
            };
            let last = !matches!(msg, ReaderMsg::Chunk(_));
            if tx.send(msg).is_err() || last {
                break;
            }
        }
    }));
    rx
}

fn kill_and_reap(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

/// Drain pending reader messages into the owner state.
fn drain(rx: &Receiver<ReaderMsg>, stdout: &mut Vec<u8>, eof: &mut bool, read_error: &mut bool) {
    loop {
        match rx.try_recv() {
            Ok(ReaderMsg::Chunk(bytes)) => stdout.extend_from_slice(&bytes),
            Ok(ReaderMsg::ReadError) => *read_error = true,
            Ok(ReaderMsg::Eof) => *eof = true,
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
        }
    }
}

fn run_bounded(
    engram: &Path,
    workspace: &Path,
    read: &ProbeRead,
    deadline: Instant,
) -> Result<BoundedOutput, ProbeError> {
    let mut child = Command::new(engram)
        .arg("--workspace")
        .arg(workspace)
        .arg("--json")
        .args(read.cli_args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| ProbeError::Spawn("spawn_failed"))?;
    let Some(pipe) = child.stdout.take() else {
        kill_and_reap(&mut child);
        return Err(ProbeError::Spawn("stdout_unavailable"));
    };
    let rx = spawn_reader(pipe);

    let mut stdout = Vec::new();
    let (mut eof, mut read_error) = (false, false);
    let mut exited: Option<(ExitStatus, Instant)> = None;
    loop {
        drain(&rx, &mut stdout, &mut eof, &mut read_error);
        if stdout.len() > OUTPUT_CAP {
            kill_and_reap(&mut child);
            return Err(ProbeError::Oversize);
        }
        if let Some((status, exit_at)) = exited {
            if eof || read_error || exit_at.elapsed() >= POST_EXIT_DRAIN {
                if read_error {
                    return Err(ProbeError::Malformed("stdout_read"));
                }
                return Ok(BoundedOutput { stdout, status });
            }
        } else {
            if Instant::now() >= deadline {
                kill_and_reap(&mut child);
                return Err(ProbeError::Deadline);
            }
            match child.try_wait() {
                Ok(Some(status)) => exited = Some((status, Instant::now())),
                Ok(None) => {}
                Err(_) => {
                    kill_and_reap(&mut child);
                    return Err(ProbeError::Exit("wait_failed"));
                }
            }
        }
        thread::sleep(POLL_INTERVAL);
    }
}

/// Parses the first non-empty stdout line as a JSON-RPC response.
///
/// Returns `None` (malformed) unless the line is a JSON object carrying either
/// an `error` member or a `result` member that is itself a JSON object.
fn parse_report(stdout: &[u8]) -> Option<ProbeReport> {
    let line = stdout
        .split(|byte| *byte == b'\n')
        .map(<[u8]>::trim_ascii)
        .find(|line| !line.is_empty())?;
    let value: serde_json::Value = serde_json::from_slice(line).ok()?;
    let object = value.as_object()?;
    if object.contains_key("error") {
        return Some(ProbeReport {
            accepted: false,
            generation_id: None,
            data_fingerprint: None,
        });
    }
    let result = object.get("result").filter(|v| v.is_object())?;
    let generation_id = result
        .pointer("/provenance/generation_id")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned);
    let count = |key: &str| result.get(key).and_then(serde_json::Value::as_u64);
    let data_fingerprint = match (
        count("code_files"),
        count("functions"),
        count("classes"),
        count("interfaces"),
        count("edges"),
    ) {
        (Some(files), Some(functions), Some(classes), Some(interfaces), Some(edges)) => {
            Some(format!(
                "code_files={files};functions={functions};classes={classes};interfaces={interfaces};edges={edges}"
            ))
        }
        _ => None,
    };
    Some(ProbeReport {
        accepted: true,
        generation_id,
        data_fingerprint,
    })
}

/// Run one bounded CLI read and parse its first JSON line.
///
/// `deadline` is an absolute instant; the child is killed once it passes.
///
/// Typed-result precedence: `Spawn` > `Oversize` > `Deadline` > reader
/// `ReadError` (reported as `Malformed("stdout_read")`) > parse. A parseable
/// result report is `accepted: true` independent of the exit status by
/// design; an `error` JSON object is `accepted: false` and never `Exit`.
/// Only unparseable output maps to `Malformed`/`Exit`.
///
/// The child inherits the caller environment, so callers must not set
/// `ENGRAM_DIRECT`, `ENGRAM_DATA_DIR`, or `ENGRAM_WORKSPACE`.
///
/// The stdout reader runs on a detached thread; a grandchild that keeps the
/// pipe open can leave that thread blocked after this function returns.
///
/// # Errors
///
/// Returns a [`ProbeError`] per the typed-result precedence.
pub fn probe_cli_read(
    engram: &Path,
    workspace: &Path,
    read: &ProbeRead,
    deadline: Instant,
) -> Result<ProbeReport, ProbeError> {
    let output = run_bounded(engram, workspace, read, deadline)?;
    if let Some(report) = parse_report(&output.stdout) {
        return Ok(report);
    }
    if output.status.success() {
        Err(ProbeError::Malformed("not_a_report"))
    } else {
        Err(ProbeError::Exit("nonzero_exit"))
    }
}

/// Re-probe until the report's generation equals `expected` or the absolute
/// `deadline` instant passes; returns the matching report.
///
/// Intended for reads that carry provenance (`PREFLIGHT_READ`); a read
/// without a `generation_id` can never match.
///
/// # Errors
///
/// Returns [`ProbeError::Mismatch`] or [`ProbeError::Deadline`] when the
/// expected generation is not observed in time.
pub fn wait_for_generation(
    engram: &Path,
    workspace: &Path,
    read: &ProbeRead,
    expected: &str,
    deadline: Instant,
) -> Result<ProbeReport, ProbeError> {
    let mut backoff = BACKOFF_START;
    let mut saw_other = false;
    loop {
        if Instant::now() >= deadline {
            break;
        }
        match probe_cli_read(engram, workspace, read, deadline) {
            Ok(report) if report.accepted => {
                if report.generation_id.as_deref() == Some(expected) {
                    return Ok(report);
                }
                saw_other = true;
            }
            Ok(_) | Err(ProbeError::Exit(_) | ProbeError::Malformed(_)) => {}
            Err(ProbeError::Deadline) => break,
            Err(other) => return Err(other),
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        thread::sleep(backoff.min(remaining));
        backoff = (backoff * 2).min(BACKOFF_MAX);
    }
    if saw_other {
        Err(ProbeError::Mismatch)
    } else {
        Err(ProbeError::Deadline)
    }
}

const MCP_INIT_ID: u64 = 1;
const MCP_CALL_ID: u64 = 2;

enum LineMsg {
    Line(Vec<u8>),
    Oversize,
    ReadError,
    Eof,
}

/// Reader thread for the MCP session: sends whole stdout lines. Total bytes
/// are capped through `take(cap + 1)`; exceeding the cap reports `Oversize`.
fn spawn_line_reader(stdout: impl Read + Send + 'static) -> Result<Receiver<LineMsg>, ProbeError> {
    let (tx, rx) = mpsc::channel();
    // Detached on purpose; see `spawn_reader`.
    let spawned = thread::Builder::new().spawn(move || {
        let mut reader = io::BufReader::new(stdout.take(OUTPUT_CAP as u64 + 1));
        let mut total = 0_usize;
        loop {
            let mut line = Vec::new();
            let msg = match reader.read_until(b'\n', &mut line) {
                Ok(0) => LineMsg::Eof,
                Ok(n) => {
                    total += n;
                    if total > OUTPUT_CAP {
                        LineMsg::Oversize
                    } else {
                        LineMsg::Line(line)
                    }
                }
                Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => LineMsg::ReadError,
            };
            let last = !matches!(msg, LineMsg::Line(_));
            if tx.send(msg).is_err() || last {
                break;
            }
        }
    });
    spawned
        .map(|_| rx)
        .map_err(|_| ProbeError::Spawn("reader_thread"))
}

/// Turn one received line into the JSON-RPC response for `id`.
///
/// `Ok(None)` means a valid JSON line that is not the awaited response.
fn match_response(line: &[u8], id: u64) -> Result<Option<serde_json::Value>, ProbeError> {
    let trimmed = line.trim_ascii();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let value: serde_json::Value =
        serde_json::from_slice(trimmed).map_err(|_| ProbeError::Malformed("not_json"))?;
    if !value.is_object() {
        return Err(ProbeError::Malformed("not_a_response"));
    }
    // A JSON-RPC error with a null id answers an unparseable request: fail fast.
    if value.get("error").is_some() && value.get("id").is_some_and(serde_json::Value::is_null) {
        return Err(ProbeError::Malformed("rpc_error_no_id"));
    }
    // A response carries `result` or `error` and no `method`; server-initiated
    // requests that reuse the id are skipped.
    let is_response = value.get("method").is_none()
        && (value.get("result").is_some() || value.get("error").is_some());
    if is_response && value.get("id").and_then(serde_json::Value::as_u64) == Some(id) {
        Ok(Some(value))
    } else {
        Ok(None)
    }
}

/// Await the response line carrying `id`.
///
/// Every wait is a `recv_timeout` sliced to `POLL_INTERVAL`, interleaved with
/// `try_wait()`. On an early shim exit the post-exit drain runs once; a line
/// still missing afterwards is `Exit`. Never waits for stdout EOF.
fn await_response(
    rx: &Receiver<LineMsg>,
    child: &mut Child,
    id: u64,
    deadline: Instant,
) -> Result<serde_json::Value, ProbeError> {
    let mut exited_at: Option<Instant> = None;
    loop {
        // Polled on every iteration, independent of the receive outcome, so a
        // chatty grandchild cannot starve exit detection.
        if exited_at.is_none() {
            match child.try_wait() {
                Ok(Some(_)) => exited_at = Some(Instant::now()),
                Ok(None) => {}
                Err(_) => return Err(ProbeError::Exit("wait_failed")),
            }
        }
        let now = Instant::now();
        let limit = match exited_at {
            Some(at) => at + POST_EXIT_DRAIN,
            None => deadline,
        };
        if now >= limit {
            // A line queued just before the limit still counts.
            while let Ok(LineMsg::Line(line)) = rx.try_recv() {
                if let Some(value) = match_response(&line, id)? {
                    return Ok(value);
                }
            }
            return Err(if exited_at.is_some() {
                ProbeError::Exit("shim_exited")
            } else {
                ProbeError::Deadline
            });
        }
        let slice = POLL_INTERVAL.min(limit - now);
        match rx.recv_timeout(slice) {
            Ok(LineMsg::Line(line)) => {
                if let Some(value) = match_response(&line, id)? {
                    return Ok(value);
                }
            }
            Ok(LineMsg::Oversize) => return Err(ProbeError::Oversize),
            Ok(LineMsg::ReadError) => return Err(ProbeError::Malformed("stdout_read")),
            Ok(LineMsg::Eof) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(ProbeError::Exit("shim_exited"));
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
    }
}

fn send_line(stdin: &mut impl Write, value: &serde_json::Value) -> Result<(), ProbeError> {
    let mut text = value.to_string();
    text.push('\n');
    stdin
        .write_all(text.as_bytes())
        .and_then(|()| stdin.flush())
        .map_err(|_| ProbeError::Exit("stdin_write"))
}

/// Build a report from a `tools/call` response: the `generation_id` and
/// `data_fingerprint` come from the tool result's `structuredContent`.
fn report_from_call(response: &serde_json::Value) -> Result<ProbeReport, ProbeError> {
    let refused = ProbeReport {
        accepted: false,
        generation_id: None,
        data_fingerprint: None,
    };
    if response.get("error").is_some() {
        return Ok(refused);
    }
    let result = response
        .get("result")
        .filter(|v| v.is_object())
        .ok_or(ProbeError::Malformed("not_a_report"))?;
    if result.get("isError").and_then(serde_json::Value::as_bool) == Some(true) {
        return Ok(refused);
    }
    let structured = result
        .get("structuredContent")
        .filter(|v| v.is_object())
        .ok_or(ProbeError::Malformed("no_structured_content"))?;
    let text = |pointer: &str| {
        structured
            .pointer(pointer)
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned)
    };
    let generation_id = text("/provenance/generation_id").or_else(|| text("/generation_id"));
    let count = |key: &str| structured.get(key).and_then(serde_json::Value::as_u64);
    let data_fingerprint = text("/data_fingerprint").or_else(|| {
        Some(format!(
            "code_files={};functions={};classes={};interfaces={};edges={}",
            count("code_files")?,
            count("functions")?,
            count("classes")?,
            count("interfaces")?,
            count("edges")?
        ))
    });
    Ok(ProbeReport {
        accepted: true,
        generation_id,
        data_fingerprint,
    })
}

/// Drive one MCP session: initialize, one `tools/call`, read its response
/// BEFORE closing stdin, then wait for the shim's exit within the deadline.
fn run_mcp_session(
    child: &mut Child,
    arguments: serde_json::Value,
    tool: &str,
    deadline: Instant,
) -> Result<ProbeReport, ProbeError> {
    let (Some(mut stdin), Some(pipe)) = (child.stdin.take(), child.stdout.take()) else {
        return Err(ProbeError::Spawn("pipe_unavailable"));
    };
    let rx = spawn_line_reader(pipe)?;
    send_line(
        &mut stdin,
        &serde_json::json!({
            "jsonrpc": "2.0",
            "id": MCP_INIT_ID,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "engram-preflight-probe", "version": "0"}
            }
        }),
    )?;
    let init = await_response(&rx, child, MCP_INIT_ID, deadline)?;
    if init.get("error").is_some() {
        return Ok(ProbeReport {
            accepted: false,
            generation_id: None,
            data_fingerprint: None,
        });
    }
    send_line(
        &mut stdin,
        &serde_json::json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
    )?;
    send_line(
        &mut stdin,
        &serde_json::json!({
            "jsonrpc": "2.0",
            "id": MCP_CALL_ID,
            "method": "tools/call",
            "params": {"name": tool, "arguments": arguments}
        }),
    )?;
    let response = await_response(&rx, child, MCP_CALL_ID, deadline)?;
    let report = report_from_call(&response)?;
    // The response is in hand; closing stdin asks the shim to exit.
    drop(stdin);
    loop {
        match child.try_wait() {
            Ok(Some(_)) | Err(_) => return Ok(report),
            // A shim that ignores the stdin close is a failure; the caller
            // kills and reaps it.
            Ok(None) if Instant::now() >= deadline => return Err(ProbeError::Deadline),
            Ok(None) => thread::sleep(POLL_INTERVAL),
        }
    }
}

/// Run one bounded MCP `tools/call` through `engram shim --workspace <W>`.
///
/// `read.mcp_arguments_json` is parsed before anything is spawned; invalid
/// JSON or a non-object is `Malformed("mcp_arguments")`. The `generation_id`
/// and `data_fingerprint` are taken from the tool result's
/// `structuredContent`. The response is read before stdin is closed; the
/// shim is then awaited by `try_wait` until the absolute `deadline` (a shim still running then is `Deadline`), never
/// by stdout EOF, and is killed and reaped on every error path or expiry.
///
/// The stdout reader runs on a detached thread that a grandchild holding the
/// pipe can leave blocked after this function returns.
///
/// # Errors
///
/// Returns a [`ProbeError`]: `Malformed` for bad arguments or output,
/// `Spawn`, `Deadline`, `Exit` (shim gone without a response) or `Oversize`.
pub fn probe_mcp_read(
    engram: &Path,
    workspace: &Path,
    read: &ProbeRead,
    deadline: Instant,
) -> Result<ProbeReport, ProbeError> {
    let arguments: serde_json::Value = serde_json::from_str(read.mcp_arguments_json)
        .ok()
        .filter(serde_json::Value::is_object)
        .ok_or(ProbeError::Malformed("mcp_arguments"))?;
    let mut child = Command::new(engram)
        .arg("shim")
        .arg("--workspace")
        .arg(workspace)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| ProbeError::Spawn("spawn_failed"))?;
    let result = run_mcp_session(&mut child, arguments, read.mcp_tool, deadline);
    // Reaps in every case; a shim that already exited makes `kill` a no-op.
    kill_and_reap(&mut child);
    result
}
