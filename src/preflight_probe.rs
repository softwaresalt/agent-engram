//! CLI provenance probe facade (plan unit PRE-5, task 142.067-T).
//!
//! Synchronous, bounded child I/O: a capped reader thread feeds the owner
//! loop, which keys on the CLI child's exit and never joins the reader.

use std::io::{self, Read};
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

/// Run one bounded MCP `tools/call` through `engram shim` (plan unit PRE-6).
///
/// # Errors
///
/// Returns a [`ProbeError`] when the session fails.
pub fn probe_mcp_read(
    engram: &Path,
    workspace: &Path,
    read: &ProbeRead,
    deadline: Instant,
) -> Result<ProbeReport, ProbeError> {
    let _ = (engram, workspace, read, deadline);
    unimplemented!("Worker: 142.068-T")
}
