//! Root preflight relay (plan unit NEW-4, task 142.072-T).
//!
//! The relay resolves the sibling `engram-indexer` supervisor, runs it under a
//! bounded child I/O discipline, and normalizes its output to exactly one
//! verdict line on stdout plus one structured relay line on stderr
//! (`engram-preflight-relay stage=<Stage|none> cause=<cause> elapsed_ms=<n>`).
//!
//! Child I/O is bounded and keyed on the supervisor's *exit*: a detached
//! reader thread feeds a channel, the relay polls `try_wait`, drains for a
//! short window after exit, and never joins the reader, so a grandchild that
//! inherited the pipe cannot block the relay.

use std::env::consts::EXE_SUFFIX;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError, TryRecvError};
use std::time::{Duration, Instant};

/// Grace period added to the caller's timeout before the supervisor is killed.
pub const RELAY_GRACE_MS: u64 = 2000;

/// Maximum supervisor stdout bytes the relay accepts or echoes (4 KiB).
const OUTPUT_CAP: usize = 4096;
/// Post-exit window for draining buffered stdout.
const DRAIN_MS: u64 = 250;
/// Poll interval of the exit-keyed wait loop.
const POLL_MS: u64 = 10;
/// Largest deadline, in milliseconds, the relay will schedule (`u32::MAX` seconds).
const MAX_DEADLINE_MS: u64 = (u32::MAX as u64) * 1000;
/// The single success line.
const SUCCEEDED: &str = r#"{"state":"Succeeded"}"#;
/// Prefix shared by every `Failed` line.
const FAILED_PREFIX: &str = r#"{"state":"Failed","stage":""#;

/// The relay's closed set of `Failed` verdict lines; `Build` is element 0.
pub const RELAY_STAGES: [&str; 7] = [
    r#"{"state":"Failed","stage":"Build"}"#,
    r#"{"state":"Failed","stage":"Seal"}"#,
    r#"{"state":"Failed","stage":"Publish"}"#,
    r#"{"state":"Failed","stage":"DaemonVerified"}"#,
    r#"{"state":"Failed","stage":"HealthVerified"}"#,
    r#"{"state":"Failed","stage":"CliProbeVerified"}"#,
    r#"{"state":"Failed","stage":"McpProbeVerified"}"#,
];

/// Closed set of causes reported on the relay's structured stderr line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelayCause {
    /// The supervisor produced a conforming verdict.
    Ok,
    /// The sibling supervisor binary is missing.
    MissingSibling,
    /// The supervisor could not be spawned.
    RelaySpawn,
    /// The supervisor exceeded the relay deadline, or the deadline overflowed.
    RelayDeadline,
    /// The supervisor wrote more than the 4 KiB output cap.
    RelayOversize,
    /// Reading the supervisor's stdout failed.
    RelayRead,
    /// The supervisor output was non-conforming or contradictory.
    RelayVerdict,
}

impl RelayCause {
    /// Returns the stable stderr token for this cause.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::MissingSibling => "missing_sibling",
            Self::RelaySpawn => "relay_spawn",
            Self::RelayDeadline => "relay_deadline",
            Self::RelayOversize => "relay_oversize",
            Self::RelayRead => "relay_read",
            Self::RelayVerdict => "relay_verdict",
        }
    }
}

/// The normalized result of one relay run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelayOutcome {
    /// The single verdict line for stdout, without a trailing newline.
    pub line: &'static str,
    /// The exit code the launcher returns: 0 for `Succeeded`, otherwise 1.
    pub exit: i32,
    /// Why the relay produced this outcome.
    pub cause: RelayCause,
    /// Elapsed wall time in milliseconds.
    pub elapsed_ms: u64,
}

impl RelayOutcome {
    /// A `Failed`/`Build` outcome with the given cause.
    const fn build_failure(cause: RelayCause, elapsed_ms: u64) -> Self {
        Self {
            line: RELAY_STAGES[0],
            exit: 1,
            cause,
            elapsed_ms,
        }
    }

    /// Formats the structured relay stderr line
    /// `engram-preflight-relay stage=<Stage> cause=<cause> elapsed_ms=<n>`;
    /// `missing_sibling` appends the constant `sibling=engram-indexer<EXE_SUFFIX>`.
    #[must_use]
    pub fn stderr_line(&self) -> String {
        let stage = self
            .line
            .strip_prefix(FAILED_PREFIX)
            .and_then(|rest| rest.strip_suffix("\"}"))
            .unwrap_or("none");
        let mut line = format!(
            "engram-preflight-relay stage={stage} cause={} elapsed_ms={}",
            self.cause.as_str(),
            self.elapsed_ms
        );
        if self.cause == RelayCause::MissingSibling {
            line.push_str(&format!(" sibling=engram-indexer{EXE_SUFFIX}"));
        }
        line
    }
}

/// Resolves the sibling `engram-indexer` supervisor next to `engram_exe`.
///
/// The lookup is the canonical parent directory of `engram_exe` only: no
/// `PATH`, environment, or fixed-install-directory search.
///
/// # Errors
///
/// Returns a `Failed`/`Build` outcome with cause `MissingSibling` when the
/// sibling is not a regular file.
pub fn resolve_supervisor(engram_exe: &Path) -> Result<PathBuf, RelayOutcome> {
    let missing = || RelayOutcome::build_failure(RelayCause::MissingSibling, 0);
    let exe = engram_exe.canonicalize().map_err(|_| missing())?;
    let sibling = exe
        .parent()
        .ok_or_else(missing)?
        .join(format!("engram-indexer{EXE_SUFFIX}"));
    if sibling.is_file() {
        Ok(sibling)
    } else {
        Err(missing())
    }
}

/// Classifies supervisor output; the flag is true when the supervisor's own
/// verdict is relayed faithfully.
fn classify(stdout: &[u8], exit_code: Option<i32>) -> (&'static str, i32, bool) {
    let body = stdout
        .strip_suffix(b"\r\n")
        .or_else(|| stdout.strip_suffix(b"\n"))
        .unwrap_or(stdout);
    match exit_code {
        Some(0) if body == SUCCEEDED.as_bytes() => (SUCCEEDED, 0, true),
        Some(n) if n != 0 => RELAY_STAGES
            .iter()
            .find(|s| s.as_bytes() == body)
            .map_or((RELAY_STAGES[0], 1, false), |s| (*s, 1, true)),
        _ => (RELAY_STAGES[0], 1, false),
    }
}

/// Echoes non-conforming supervisor bytes (at most 4 KiB) to stderr.
fn echo_nonconforming(stdout: &[u8]) {
    if stdout.len() <= OUTPUT_CAP {
        let _ = writeln!(std::io::stderr(), "{}", String::from_utf8_lossy(stdout));
    }
}

/// Normalizes supervisor stdout and exit code to a verdict line and exit code.
///
/// Exactly one trailing `\n` or `\r\n` is stripped. `Succeeded`/0 requires the
/// `Succeeded` line and exit code 0; a well-formed `Failed` line passes through
/// as exit 1 only for a non-zero exit code. Everything else is `Failed`/`Build`/1
/// with the non-conforming bytes (at most 4 KiB) echoed to stderr. The raw child
/// exit code is never relayed.
#[must_use]
pub fn normalize_verdict(stdout: &[u8], exit_code: Option<i32>) -> (&'static str, i32) {
    let (line, exit, conforming) = classify(stdout, exit_code);
    if !conforming {
        echo_nonconforming(stdout);
    }
    (line, exit)
}

/// Kills and reaps the child, ignoring errors from an already-exited process.
fn kill_and_reap(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

/// Spawns the detached stdout reader; it stops once more than the cap is read.
fn spawn_reader(mut pipe: impl Read + Send + 'static) -> mpsc::Receiver<std::io::Result<Vec<u8>>> {
    let (tx, rx) = mpsc::channel();
    // The handle is dropped on purpose: joining could block on a grandchild
    // that inherited the pipe.
    drop(std::thread::spawn(move || {
        let mut total = 0usize;
        let mut chunk = [0u8; 1024];
        while total <= OUTPUT_CAP {
            match pipe.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => {
                    total += n;
                    if tx.send(Ok(chunk[..n].to_vec())).is_err() {
                        break;
                    }
                }
                Err(e) => {
                    let _ = tx.send(Err(e));
                    break;
                }
            }
        }
    }));
    rx
}

/// Runs the supervisor under bounded child I/O and normalizes its verdict.
///
/// The bound is `timeout_ms + RELAY_GRACE_MS`; an overflowing bound is a
/// `relay_deadline` failure and nothing is spawned.
#[must_use]
pub fn run_relay(
    supervisor: &Path,
    workspace: &Path,
    timeout_ms: u64,
    engram_exe: &Path,
) -> RelayOutcome {
    let started = Instant::now();
    let elapsed = || u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    let fail = |cause| RelayOutcome::build_failure(cause, elapsed());

    let Some(deadline) = timeout_ms
        .checked_add(RELAY_GRACE_MS)
        .filter(|ms| *ms <= MAX_DEADLINE_MS)
        .and_then(|ms| started.checked_add(Duration::from_millis(ms)))
    else {
        return fail(RelayCause::RelayDeadline);
    };

    let absolute = |p: &Path| std::path::absolute(p).unwrap_or_else(|_| p.to_path_buf());
    let spawned = Command::new(supervisor)
        .arg("preflight")
        .arg("--workspace")
        .arg(absolute(workspace))
        .arg("--timeout-ms")
        .arg(timeout_ms.to_string())
        .arg("--engram")
        .arg(absolute(engram_exe))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn();
    let Ok(mut child) = spawned else {
        return fail(RelayCause::RelaySpawn);
    };
    let Some(pipe) = child.stdout.take() else {
        kill_and_reap(&mut child);
        return fail(RelayCause::RelaySpawn);
    };
    let rx = spawn_reader(pipe);

    let mut buf: Vec<u8> = Vec::new();
    // Collects pending chunks; reports a read error or an oversize condition.
    let collect = |buf: &mut Vec<u8>| -> Option<RelayCause> {
        loop {
            match rx.try_recv() {
                Ok(Ok(bytes)) => buf.extend_from_slice(&bytes),
                Ok(Err(_)) => return Some(RelayCause::RelayRead),
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
            if buf.len() > OUTPUT_CAP {
                return Some(RelayCause::RelayOversize);
            }
        }
        (buf.len() > OUTPUT_CAP).then_some(RelayCause::RelayOversize)
    };

    let status = loop {
        if let Some(cause) = collect(&mut buf) {
            kill_and_reap(&mut child);
            return fail(cause);
        }
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() >= deadline => {
                kill_and_reap(&mut child);
                return fail(RelayCause::RelayDeadline);
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(POLL_MS)),
            Err(_) => {
                kill_and_reap(&mut child);
                return fail(RelayCause::RelayRead);
            }
        }
    };

    // Bounded post-exit drain: ends at EOF or after the drain window, whichever
    // is first, so an inherited pipe held by a grandchild cannot block us.
    let drain_end = Instant::now() + Duration::from_millis(DRAIN_MS);
    loop {
        let wait = drain_end.saturating_duration_since(Instant::now());
        match rx.recv_timeout(wait) {
            Ok(Ok(bytes)) => buf.extend_from_slice(&bytes),
            Ok(Err(_)) => return fail(RelayCause::RelayRead),
            Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => break,
        }
        if buf.len() > OUTPUT_CAP {
            return fail(RelayCause::RelayOversize);
        }
    }

    let (line, exit, conforming) = classify(&buf, status.code());
    if !conforming {
        echo_nonconforming(&buf);
    }
    let cause = if conforming {
        RelayCause::Ok
    } else {
        RelayCause::RelayVerdict
    };
    RelayOutcome {
        line,
        exit,
        cause,
        elapsed_ms: elapsed(),
    }
}

/// Launcher entry point: resolves the supervisor, relays, prints one verdict
/// line to stdout and one structured line to stderr, and returns the exit code.
#[must_use]
pub fn run_preflight(workspace: &Path, timeout_ms: u64) -> i32 {
    let outcome = match std::env::current_exe() {
        Ok(exe) => resolve_supervisor(&exe)
            .map_or_else(|o| o, |sup| run_relay(&sup, workspace, timeout_ms, &exe)),
        Err(_) => RelayOutcome::build_failure(RelayCause::RelaySpawn, 0),
    };
    let _ = writeln!(std::io::stdout(), "{}", outcome.line);
    let _ = writeln!(std::io::stderr(), "{}", outcome.stderr_line());
    outcome.exit
}
