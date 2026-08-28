//! Shim module: lightweight stdio MCP proxy.
//!
//! The shim is the MCP client entry point. It connects to (or spawns) the
//! workspace daemon via IPC, then forwards MCP JSON-RPC from stdin to the
//! daemon and returns the response to stdout before exiting.
//!
//! ## Serve-first startup contract (124-F, stash 870B1AFF)
//!
//! An MCP client spawns the shim and immediately begins writing `initialize`
//! to its stdin. Historically, `run` evaluated three fallible preconditions —
//! workspace admission, daemon readiness, and IPC endpoint derivation — before
//! ever binding the MCP stdio transport; any `Err` terminated the process
//! mid-handshake, which a client observes as a closed pipe (Windows `os error
//! 232`) rather than an attributable failure.
//!
//! `run` now binds the stdio transport unconditionally and immediately, and
//! evaluates the three preconditions concurrently in the background. The MCP
//! session always answers `initialize` and serves the static `tools/list`
//! catalog. Permanent precondition failures keep the session degraded, while
//! a daemon readiness deadline starts a late-readiness monitor so the same
//! stdio session can recover when the named-pipe daemon finishes starting.
//! Until recovery, `tools/call` returns a structured error naming the cause
//! (see [`transport::ShimHandler::call_tool`]).

pub mod ipc_client;
pub mod lifecycle;
pub mod pidfile;
pub mod preinit_compat;
pub mod tools_catalog;
pub mod transport;
pub mod version;

use std::io::Write as _;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use serde_json::json;
use tokio::sync::watch;

use crate::errors::{DaemonError, EngramError, ShimFailureClass, ShimStartupError};

/// IPC request timeout used once a daemon endpoint is `Ready`. Deliberately
/// NOT used to bound awaiting the deferred startup outcome (see
/// `transport::ShimHandler::await_startup_outcome`) — otherwise an
/// `ENGRAM_READY_TIMEOUT_MS` configured above this constant would cause a
/// `tools/call` to report a false `readiness_timeout` while
/// `ensure_daemon_running` was still within its own valid, longer budget.
const IPC_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
/// Initial interval between late daemon-readiness probes.
const RECOVERY_INITIAL_BACKOFF_MS: u64 = 50;
/// Maximum interval between late daemon-readiness probes.
const RECOVERY_MAX_BACKOFF_MS: u64 = 1_000;

/// Outcome of the deferred startup preconditions (workspace admission, daemon
/// readiness, IPC endpoint derivation), computed concurrently with serving
/// the MCP `initialize` handshake.
#[derive(Debug, Clone)]
pub enum StartupOutcome {
    /// All preconditions succeeded; tool calls may be forwarded to `endpoint`.
    Ready { endpoint: String },
    /// The initial daemon-readiness budget expired, but the endpoint remains
    /// eligible for recovery without restarting the stdio session.
    WaitingForReadiness { endpoint: String, message: String },
    /// A permanent precondition failed; the session stays up but every
    /// `tools/call` must fail with the recorded, classified cause.
    Degraded {
        class: ShimFailureClass,
        message: String,
    },
}

impl StartupOutcome {
    fn degraded(class: ShimFailureClass, err: &EngramError) -> Self {
        Self::Degraded {
            class,
            message: err.to_string(),
        }
    }
}

fn readiness_failure_is_recoverable(error: &EngramError) -> bool {
    matches!(error, EngramError::Daemon(DaemonError::NotReady { .. }))
}

/// Resolve the workspace argument in priority order: `workspace_override`
/// argument, then the `ENGRAM_WORKSPACE` environment variable, then the
/// current working directory.
///
/// This is itself the first deferred precondition (classified
/// [`ShimFailureClass::AdmissionFailure`] on failure) — it runs inside the
/// background startup task, *after* the MCP stdio transport has already
/// bound, never before. A missing/deleted current working directory with no
/// override must not recreate the pre-initialize closed-pipe failure this
/// contract exists to prevent.
fn resolve_workspace_arg(workspace_override: Option<&str>) -> Result<String, EngramError> {
    workspace_override
        .map(std::borrow::ToOwned::to_owned)
        .map(Ok)
        .unwrap_or_else(|| {
            std::env::var("ENGRAM_WORKSPACE").or_else(|_| {
                std::env::current_dir()
                    .map(|p| p.display().to_string())
                    .map_err(|e| {
                        EngramError::ShimStartup(ShimStartupError {
                            class: ShimFailureClass::AdmissionFailure,
                            message: format!(
                                "current working directory unavailable and no --workspace or \
                                 ENGRAM_WORKSPACE override was supplied: {e}"
                            ),
                        })
                    })
            })
        })
}

/// Evaluate the deferred preconditions in order, classifying whichever step
/// fails first: workspace-argument resolution, admission (canonicalization),
/// daemon readiness, then IPC endpoint derivation. Each step's outcome maps
/// to a distinct [`ShimFailureClass`] so the cause is attributable without
/// inspecting the underlying [`EngramError`] variant. Called only from the
/// background startup task — never before the MCP stdio transport binds.
///
/// Publishes the resolved outcome on `outcome_tx` immediately at each
/// return point, *before* spawning the best-effort durable-record write
/// ([`spawn_record_startup_failure`]). The record write must never sit on
/// the outcome-publication critical path: `ShimHandler::call_tool` awaits
/// this channel unbounded (see `transport::ShimHandler::await_startup_outcome`),
/// so any slowness in the (already best-effort, fire-and-forget) diagnostic
/// write must not delay every `tools/call` waiting on the real
/// classification.
/// Returns the resolved outcome and, if a durable-record write was spawned,
/// its detached [`tokio::task::JoinHandle`] so the caller may optionally
/// give it a short best-effort grace period before process exit — purely
/// for record durability, never to gate outcome publication or `tools/call`
/// responsiveness (both already happened by the time this returns).
async fn compute_startup_outcome(
    workspace_override: Option<String>,
    outcome_tx: &Arc<watch::Sender<Option<StartupOutcome>>>,
) -> (StartupOutcome, Option<tokio::task::JoinHandle<()>>) {
    let publish = |outcome: StartupOutcome| {
        let _ = outcome_tx.send(Some(outcome.clone()));
        outcome
    };

    let workspace_arg = match resolve_workspace_arg(workspace_override.as_deref()) {
        Ok(arg) => arg,
        Err(EngramError::ShimStartup(ShimStartupError { class, message })) => {
            let outcome = publish(StartupOutcome::Degraded { class, message });
            let handle = spawn_record_startup_failure(None, class);
            return (outcome, handle);
        }
        Err(err) => {
            // Unreachable in practice (resolve_workspace_arg only ever
            // returns ShimStartup), but classify defensively rather than
            // panicking or losing the error.
            let outcome = publish(StartupOutcome::degraded(
                ShimFailureClass::AdmissionFailure,
                &err,
            ));
            let handle = spawn_record_startup_failure(None, ShimFailureClass::AdmissionFailure);
            return (outcome, handle);
        }
    };

    let workspace_path = match crate::db::workspace::canonicalize_workspace(&workspace_arg) {
        Ok(path) => path,
        Err(err) => {
            let err = EngramError::from(err);
            let outcome = publish(StartupOutcome::degraded(
                ShimFailureClass::AdmissionFailure,
                &err,
            ));
            // canonicalize_workspace failed, so workspace_arg is not a
            // validated root; pass it only as a best-effort location hint
            // (record_startup_failure applies its own no-follow guards
            // before writing anything under it).
            let handle = spawn_record_startup_failure(
                Some(workspace_arg),
                ShimFailureClass::AdmissionFailure,
            );
            return (outcome, handle);
        }
    };

    if let Err(err) = lifecycle::ensure_daemon_running(&workspace_path).await {
        // Terminal protocol incompatibility detected at startup.
        if matches!(
            &err,
            EngramError::Ipc(crate::errors::IpcError::VersionMismatch { .. })
        ) {
            let outcome = publish(StartupOutcome::degraded(
                ShimFailureClass::ProtocolIncompatible,
                &err,
            ));
            let handle = spawn_record_startup_failure(
                Some(workspace_path.display().to_string()),
                ShimFailureClass::ProtocolIncompatible,
            );
            return (outcome, handle);
        }

        if !readiness_failure_is_recoverable(&err) {
            let outcome = publish(StartupOutcome::degraded(
                ShimFailureClass::ReadinessTimeout,
                &err,
            ));
            let handle = spawn_record_startup_failure(
                Some(workspace_path.display().to_string()),
                ShimFailureClass::ReadinessTimeout,
            );
            return (outcome, handle);
        }

        let endpoint = match crate::daemon::ipc_server::ipc_endpoint(&workspace_path) {
            Ok(endpoint) => endpoint,
            Err(endpoint_err) => {
                let outcome = publish(StartupOutcome::degraded(
                    ShimFailureClass::EndpointDerivationFailure,
                    &endpoint_err,
                ));
                let handle = spawn_record_startup_failure(
                    Some(workspace_path.display().to_string()),
                    ShimFailureClass::EndpointDerivationFailure,
                );
                return (outcome, handle);
            }
        };
        let outcome = publish(StartupOutcome::WaitingForReadiness {
            endpoint: endpoint.clone(),
            message: err.to_string(),
        });
        spawn_late_readiness_monitor(
            outcome_tx.clone(),
            endpoint,
            workspace_path.display().to_string(),
        );
        let handle = spawn_record_startup_failure(
            Some(workspace_path.display().to_string()),
            ShimFailureClass::ReadinessTimeout,
        );
        return (outcome, handle);
    }

    match crate::daemon::ipc_server::ipc_endpoint(&workspace_path) {
        Ok(endpoint) => (publish(StartupOutcome::Ready { endpoint }), None),
        Err(err) => {
            let outcome = publish(StartupOutcome::degraded(
                ShimFailureClass::EndpointDerivationFailure,
                &err,
            ));
            let handle = spawn_record_startup_failure(
                Some(workspace_path.display().to_string()),
                ShimFailureClass::EndpointDerivationFailure,
            );
            (outcome, handle)
        }
    }
}

/// Type alias for the monitor's health-probe function.
///
/// Defaults to [`lifecycle::probe_health`]. Tests can inject a custom
/// implementation via the `probe` parameter to script outcomes and observe
/// monitor probe cadence deterministically.
type MonitorProbeFn = Arc<
    dyn Fn(
            String,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = lifecycle::HealthOutcome> + Send>>
        + Send
        + Sync,
>;

/// Build the default production monitor probe function.
fn default_monitor_probe() -> MonitorProbeFn {
    Arc::new(|endpoint: String| Box::pin(async move { lifecycle::probe_health(&endpoint).await }))
}

/// Continue probing a daemon after the initial readiness attribution deadline.
///
/// Exactly one monitor is spawned per shim startup. It updates the shared
/// watch channel when the named-pipe daemon becomes ready and stops when the
/// stdio session drops all receivers.
fn spawn_late_readiness_monitor(
    outcome_tx: Arc<watch::Sender<Option<StartupOutcome>>>,
    endpoint: String,
    workspace_hint: String,
) {
    spawn_late_readiness_monitor_with_probe(
        outcome_tx,
        endpoint,
        default_monitor_probe(),
        Arc::new(AtomicUsize::new(0)),
        workspace_hint,
    );
}

/// Inner monitor entry point with injectable probe function and observable
/// counter. Production callers use [`spawn_late_readiness_monitor`] which
/// supplies the defaults.
fn spawn_late_readiness_monitor_with_probe(
    outcome_tx: Arc<watch::Sender<Option<StartupOutcome>>>,
    endpoint: String,
    probe: MonitorProbeFn,
    probe_count: Arc<AtomicUsize>,
    workspace_hint: String,
) {
    tokio::spawn(async move {
        let mut delay_ms = RECOVERY_INITIAL_BACKOFF_MS;
        loop {
            // Another path (the request-triggered probe) may have already
            // latched Degraded. Exit early — but the monitor is the sole
            // late-terminal record writer (see the doc comment on
            // `spawn_record_startup_failure`), so if the externally-latched
            // outcome is a terminal ProtocolIncompatible classification the
            // monitor never itself probed, write the promised diagnostic
            // record before returning rather than silently skipping it.
            let externally_latched_class = outcome_tx.borrow().as_ref().and_then(|o| match o {
                StartupOutcome::Degraded { class, .. } => Some(*class),
                _ => None,
            });
            if let Some(class) = externally_latched_class {
                if class == ShimFailureClass::ProtocolIncompatible {
                    let wh = workspace_hint.clone();
                    let _ = tokio::task::spawn_blocking(move || {
                        write_startup_failure_record(&wh, class);
                    })
                    .await;
                }
                return;
            }

            tokio::select! {
                () = outcome_tx.closed() => return,
                () = tokio::time::sleep(Duration::from_millis(delay_ms)) => {}
            }

            probe_count.fetch_add(1, Ordering::Relaxed);
            match (probe)(endpoint.clone()).await {
                lifecycle::HealthOutcome::Ready => {
                    tracing::info!(
                        endpoint = %endpoint,
                        "daemon became ready after the shim startup deadline"
                    );
                    // Monotonic: refuse to overwrite Degraded.
                    let published = outcome_tx.send_if_modified(|current| {
                        if matches!(current, Some(StartupOutcome::Degraded { .. })) {
                            return false;
                        }
                        *current = Some(StartupOutcome::Ready {
                            endpoint: endpoint.clone(),
                        });
                        true
                    });
                    if !published {
                        // The request path latched Degraded{ProtocolIncompatible}
                        // while this exact probe was in flight (the C5 race,
                        // this time with the monitor's own probe resolving
                        // Ready). The monitor is exiting immediately either
                        // way — this is its only remaining chance to write
                        // the promised late-terminal record; the top-of-loop
                        // check on a future iteration will never run because
                        // this branch always returns.
                        let externally_latched_class =
                            outcome_tx.borrow().as_ref().and_then(|o| match o {
                                StartupOutcome::Degraded { class, .. } => Some(*class),
                                _ => None,
                            });
                        if externally_latched_class == Some(ShimFailureClass::ProtocolIncompatible)
                        {
                            let wh = workspace_hint.clone();
                            let _ = tokio::task::spawn_blocking(move || {
                                write_startup_failure_record(
                                    &wh,
                                    ShimFailureClass::ProtocolIncompatible,
                                );
                            })
                            .await;
                        }
                    }
                    return;
                }
                lifecycle::HealthOutcome::Transient => {
                    delay_ms = (delay_ms * 2).min(RECOVERY_MAX_BACKOFF_MS);
                }
                lifecycle::HealthOutcome::Terminal(kind) => {
                    let message = kind.client_message();
                    tracing::warn!(
                        endpoint = %endpoint,
                        terminal_kind = ?kind,
                        "daemon protocol incompatible — monitor stopping"
                    );
                    // Monotonic: Degraded is absorbing.
                    outcome_tx.send_if_modified(|current| {
                        if matches!(current, Some(StartupOutcome::Degraded { .. })) {
                            return false;
                        }
                        *current = Some(StartupOutcome::Degraded {
                            class: ShimFailureClass::ProtocolIncompatible,
                            message: message.clone(),
                        });
                        true
                    });
                    // Late-terminal durable record (best-effort, sole writer).
                    let wh = workspace_hint.clone();
                    let _ = tokio::task::spawn_blocking(move || {
                        write_startup_failure_record(&wh, ShimFailureClass::ProtocolIncompatible);
                    })
                    .await;
                    return;
                }
            }
        }
    });
}

/// Fire-and-forget the best-effort durable startup-failure record write
/// (`write_startup_failure_record`) on a detached task, without awaiting
/// it. Deliberately NOT on `compute_startup_outcome`'s critical path — the
/// outcome is already published on `outcome_tx` before this is called.
///
/// Contains ONLY a timestamp, the binary build identifier, and the
/// classified failure class. The record deliberately does NOT persist the
/// live error message — some classes' underlying [`EngramError`] embeds the
/// caller-supplied workspace path or other step-specific detail, which is
/// appropriate to surface live (in the `tools/call` response and the stderr
/// line) but not to aggregate into an on-disk record that may later be
/// collected across many workspaces. [`ShimFailureClass::record_message`]
/// supplies a fixed, class-specific, variable-free description instead.
/// Never records credentials, tokens, or environment variable values.
/// Failures to persist the record are swallowed — the record is
/// supplementary diagnostics, not the primary failure signal (the process
/// exit code and stderr line are). Runs on a blocking-pool thread
/// (`tokio::task::spawn_blocking`) so its synchronous file I/O never stalls
/// an async runtime worker thread.
fn spawn_record_startup_failure(
    workspace_hint: Option<String>,
    class: ShimFailureClass,
) -> Option<tokio::task::JoinHandle<()>> {
    let workspace_hint = workspace_hint?;
    Some(tokio::spawn(async move {
        let _ = tokio::task::spawn_blocking(move || {
            write_startup_failure_record(&workspace_hint, class);
        })
        .await;
    }))
}

/// Open (or create) a single path component under `parent` with atomic
/// no-follow semantics: if a real directory already exists, open it
/// directly; otherwise attempt to create it fresh and open the result.
///
/// This is race-free against a concurrent symlink swap: `open_dir_nofollow`
/// (cap-std, backed by `O_NOFOLLOW` on Unix and reparse-point-aware opens on
/// Windows) either succeeds against a real directory or fails — there is no
/// separate check-then-act window between inspecting and opening the entry,
/// unlike a `symlink_metadata` check followed by a plain `open`/`create_dir`.
fn open_or_create_subdir_nofollow(
    parent: &cap_std::fs::Dir,
    name: &str,
) -> Option<cap_std::fs::Dir> {
    use cap_fs_ext::DirExt as _;

    if let Ok(dir) = parent.open_dir_nofollow(Path::new(name)) {
        return Some(dir);
    }
    // Either the entry doesn't exist yet, or it exists but isn't a
    // followable real directory (e.g. a symlink). `create_dir` only
    // succeeds in the former case (`AlreadyExists` otherwise), so the
    // no-follow open retry below is the sole arbiter of success either way.
    let _ = parent.create_dir(name);
    parent.open_dir_nofollow(Path::new(name)).ok()
}

/// Resolve `<root>/.engram/diagnostics` with atomic no-follow semantics:
/// refuses to descend through a pre-existing symlink or reparse point at
/// either `.engram` or `.engram/diagnostics` (workspace containment;
/// Constitution Principle III/IV). Returns an open directory handle rather
/// than a `PathBuf` so the subsequent file write (`write_startup_failure_record`)
/// is also no-follow and relative to this handle, closing the
/// check-then-act race a path-string-based approach would leave open.
fn no_follow_diagnostics_dir(workspace_hint: &str) -> Option<cap_std::fs::Dir> {
    let workspace_root = Path::new(workspace_hint).canonicalize().ok()?;
    if !workspace_root.is_dir() {
        return None;
    }
    let root_dir =
        cap_std::fs::Dir::open_ambient_dir(&workspace_root, cap_std::ambient_authority()).ok()?;
    let engram_dir = open_or_create_subdir_nofollow(&root_dir, ".engram")?;
    open_or_create_subdir_nofollow(&engram_dir, "diagnostics")
}

fn write_startup_failure_record(workspace_hint: &str, class: ShimFailureClass) {
    use cap_fs_ext::{FollowSymlinks, OpenOptionsFollowExt as _};
    use cap_std::fs::OpenOptions;

    let Some(diagnostics_dir) = no_follow_diagnostics_dir(workspace_hint) else {
        return;
    };
    let record = json!({
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "binary_version": version::ENGRAM_BUILD_HASH,
        "failure_class": class.as_str(),
        "message": class.record_message(),
    });
    let Ok(mut line) = serde_json::to_string(&record) else {
        return;
    };
    line.push('\n');
    let mut options = OpenOptions::new();
    options.create(true).append(true).follow(FollowSymlinks::No);
    // `FollowSymlinks::No` prevents symlink traversal but does not require
    // the existing leaf to be a regular file. On Unix, opening a
    // pre-created FIFO for writing blocks indefinitely with no reader
    // present, which would hang this blocking-pool task forever and, in
    // turn, the `tools/call` awaiting the startup outcome (`await_startup_outcome`
    // is deliberately unbounded — see its doc comment). Pass `O_NONBLOCK` so
    // opening a FIFO fails immediately instead of blocking, then verify the
    // opened file is a regular file before writing (Copilot review finding
    // on PR #349).
    #[cfg(unix)]
    {
        use cap_std::fs::OpenOptionsExt as _;
        use rustix::fs::OFlags;

        if let Ok(custom_flags) = i32::try_from(OFlags::NONBLOCK.bits()) {
            options.custom_flags(custom_flags);
        }
    }
    let Ok(mut file) =
        diagnostics_dir.open_with(Path::new("shim-startup-failures.jsonl"), &options)
    else {
        return;
    };
    let Ok(metadata) = file.metadata() else {
        return;
    };
    if !metadata.file_type().is_file() {
        return;
    }
    let _ = file.write_all(line.as_bytes());
}

/// Determine the shim's tracing log format from `ENGRAM_LOG_FORMAT`.
fn shim_log_format() -> crate::config::LogFormat {
    match std::env::var("ENGRAM_LOG_FORMAT") {
        Ok(value) if value.eq_ignore_ascii_case("json") => crate::config::LogFormat::Json,
        _ => crate::config::LogFormat::Pretty,
    }
}

/// Run the shim: bind the MCP stdio transport immediately, then resolve the
/// workspace/daemon preconditions concurrently.
///
/// The stdio transport is bound before ANY precondition — including
/// workspace-argument resolution — is evaluated, so the MCP `initialize`
/// handshake and `tools/list` always succeed regardless of workspace state.
/// `tools/call` fails with a structured, attributable error if a
/// precondition failed. On session end, if the session was ever degraded,
/// the classified failure is returned so the caller can exit with the
/// documented [`ShimFailureClass::exit_code`].
///
/// # Errors
///
/// Returns [`EngramError::ShimStartup`] if a precondition failed (session
/// degraded), or if the MCP stdio transport itself failed to bind or ended
/// abnormally.
pub async fn run(workspace_override: Option<&str>) -> Result<(), EngramError> {
    // Tracing is pinned to stderr (src/lib.rs) so debug logging never
    // contaminates the MCP stdout framing channel (124-F U5, investigation E5).
    crate::init_tracing(shim_log_format());

    let workspace_override = workspace_override.map(str::to_owned);
    let (outcome_tx, outcome_rx) = watch::channel(None);
    let outcome_tx = Arc::new(outcome_tx);
    let outcome_observer = outcome_rx.clone();
    let transport_outcome_tx = Arc::clone(&outcome_tx);
    let mut startup_task =
        tokio::spawn(async move { compute_startup_outcome(workspace_override, &outcome_tx).await });

    let session_result =
        transport::run_shim(transport_outcome_tx, outcome_rx, IPC_REQUEST_TIMEOUT).await;
    let outcome_at_session_end = outcome_observer.borrow().clone();
    drop(outcome_observer);

    // The MCP session has already ended (client disconnected or transport
    // closed). The background precondition task's own result is only needed
    // to classify the final exit code/diagnostics — it must not block
    // process teardown for however long `ensure_daemon_running`'s internal
    // readiness budget (up to 30s+) takes if the client vanished before any
    // `tools/call` ever needed the outcome. Bound the join with a short
    // grace period; if the task is still pending, classify as TransportFailure
    // (exit code 13) since the session ended before preconditions resolved
    // (the background task is dropped/cancelled when the runtime shuts down
    // at process exit).
    let (outcome, record_task) =
        match tokio::time::timeout(Duration::from_secs(2), &mut startup_task).await {
            Ok(Ok((outcome, record_task))) => (outcome, record_task),
            Ok(Err(join_err)) => (
                StartupOutcome::Degraded {
                    class: ShimFailureClass::TransportFailure,
                    message: format!("startup precondition task did not complete: {join_err}"),
                },
                None,
            ),
            Err(_elapsed) => {
                startup_task.abort();
                let _ = startup_task.await;
                (
                    StartupOutcome::Degraded {
                        class: ShimFailureClass::TransportFailure,
                        message: "startup precondition task did not finish before session teardown"
                            .to_owned(),
                    },
                    None,
                )
            }
        };

    // Give the (already-published, non-critical-path) durable-record write
    // a short best-effort grace period to finish before the runtime shuts
    // down and cancels it. This never delays outcome publication or any
    // `tools/call` — the outcome was already sent on `outcome_tx` before
    // this task was even spawned (see `compute_startup_outcome`).
    if let Some(record_task) = record_task {
        let _ = tokio::time::timeout(Duration::from_millis(500), record_task).await;
    }

    // A transport-level failure (e.g. the stdio transport failed to bind, or
    // the MCP session ended with a protocol error) takes precedence over the
    // precondition classification, since it is the more proximate cause.
    session_result?;

    let latest_outcome = outcome_at_session_end.unwrap_or(outcome);
    match latest_outcome {
        StartupOutcome::Ready { .. } => {}
        StartupOutcome::WaitingForReadiness { message, .. } => {
            return Err(EngramError::ShimStartup(ShimStartupError {
                class: ShimFailureClass::ReadinessTimeout,
                message,
            }));
        }
        StartupOutcome::Degraded { class, message } => {
            return Err(EngramError::ShimStartup(ShimStartupError {
                class,
                message,
            }));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A pre-existing `.engram` symlink/reparse point pointing outside the
    /// workspace MUST NOT be followed when resolving the diagnostics
    /// directory (workspace containment; Copilot review finding on PR #349).
    #[test]
    fn no_follow_diagnostics_dir_refuses_engram_symlink() {
        let workspace = tempfile::TempDir::new().expect("workspace tempdir");
        let outside = tempfile::TempDir::new().expect("outside tempdir");

        #[cfg(windows)]
        let symlink_result =
            std::os::windows::fs::symlink_dir(outside.path(), workspace.path().join(".engram"));
        #[cfg(unix)]
        let symlink_result =
            std::os::unix::fs::symlink(outside.path(), workspace.path().join(".engram"));

        if symlink_result.is_err() {
            // Symlink creation can require elevated privileges in some
            // sandboxes; skip rather than fail the build in that case.
            return;
        }

        let workspace_hint = workspace.path().to_string_lossy().into_owned();
        let diagnostics_dir = no_follow_diagnostics_dir(&workspace_hint);
        assert!(
            diagnostics_dir.is_none(),
            "must refuse to resolve a diagnostics directory through a pre-existing \
             .engram symlink: {diagnostics_dir:?}"
        );
        assert!(
            !outside.path().join("diagnostics").exists(),
            "must never create anything under the symlink target outside the workspace"
        );
    }

    /// A healthy workspace with no pre-existing `.engram` entry resolves and
    /// creates the diagnostics directory normally.
    #[test]
    fn no_follow_diagnostics_dir_creates_normally_for_a_clean_workspace() {
        let workspace = tempfile::TempDir::new().expect("workspace tempdir");
        let workspace_hint = workspace.path().to_string_lossy().into_owned();
        let dir =
            no_follow_diagnostics_dir(&workspace_hint).expect("must resolve for a clean workspace");

        // Confirm the returned handle genuinely is
        // `<workspace>/.engram/diagnostics` by writing a marker file through
        // it and observing that file at the expected plain path.
        let mut options = cap_std::fs::OpenOptions::new();
        options.create(true).write(true);
        let mut file = dir
            .open_with(Path::new("marker.txt"), &options)
            .expect("create marker file via the returned handle");
        file.write_all(b"ok").expect("write marker file");

        let expected_marker = workspace
            .path()
            .canonicalize()
            .expect("canonicalize workspace")
            .join(".engram")
            .join("diagnostics")
            .join("marker.txt");
        assert!(
            expected_marker.is_file(),
            "the returned handle must correspond to <workspace>/.engram/diagnostics"
        );
    }

    // ── 138-F Monitor Behavior (T6, R2b, C5) ─────────────────────────────

    /// T6 — monitor stops on terminal (138.012-T).
    ///
    /// With no tools/call, a probe that returns false (simulating a terminal
    /// protocol mismatch) should cause the monitor to publish `Degraded` and
    /// exit. The probe count reaches a fixed value and stays constant for
    /// >= 2s (> `RECOVERY_MAX_BACKOFF_MS`).
    ///
    /// NEW-RED: the monitor currently never publishes Degraded — it retries
    /// indefinitely until `outcome_tx.closed()` or probe returns true.
    #[tokio::test(start_paused = true)]
    async fn t6_monitor_stops_on_terminal() {
        for _ in 0..5 {
            let probe_count = Arc::new(AtomicUsize::new(0));
            let pc = Arc::clone(&probe_count);
            let probe: MonitorProbeFn = Arc::new(move |_| {
                let pc = Arc::clone(&pc);
                Box::pin(async move {
                    pc.fetch_add(1, Ordering::SeqCst);
                    lifecycle::HealthOutcome::Terminal(lifecycle::TerminalKind::MethodNotFound)
                })
            });

            let (tx, mut rx) = tokio::sync::watch::channel(None::<StartupOutcome>);
            let tx = Arc::new(tx);
            let count = Arc::clone(&probe_count);

            spawn_late_readiness_monitor_with_probe(
                Arc::clone(&tx),
                "test-endpoint".to_owned(),
                probe,
                count,
                "test-workspace".to_owned(),
            );

            // Advance time enough for the monitor to probe and (should) stop.
            for _ in 0..300 {
                tokio::time::advance(Duration::from_millis(10)).await;
                tokio::task::yield_now().await;
            }

            // Assert the monitor published Degraded.
            let current = rx.borrow_and_update().clone();
            // RED: currently the monitor never publishes Degraded
            assert!(
                matches!(current, Some(StartupOutcome::Degraded { .. })),
                "T6: monitor must publish Degraded on terminal; got {current:?}"
            );

            // Assert probe count is stable (monitor stopped).
            let count_before = probe_count.load(Ordering::SeqCst);
            for _ in 0..200 {
                tokio::time::advance(Duration::from_millis(10)).await;
                tokio::task::yield_now().await;
            }
            let count_after = probe_count.load(Ordering::SeqCst);
            assert_eq!(
                count_before, count_after,
                "T6: probe count must be stable after terminal (monitor stopped)"
            );
        }
    }

    /// R2b — monitor keeps probing on transient (138.012-T).
    ///
    /// A bound counting probe returns false (version-compatible non-ready).
    /// The monitor probe counter strictly increases across a 1s window and
    /// NO `Degraded` is ever published (the session stays recoverable).
    ///
    /// GREEN PIN: this is the current monitor behavior (retries on false).
    /// Guards against over-terminalization in the monitor path.
    #[tokio::test(start_paused = true)]
    async fn r2b_monitor_keeps_probing_on_transient() {
        for _ in 0..5 {
            let probe_count = Arc::new(AtomicUsize::new(0));
            let pc = Arc::clone(&probe_count);
            let probe: MonitorProbeFn = Arc::new(move |_| {
                let pc = Arc::clone(&pc);
                Box::pin(async move {
                    pc.fetch_add(1, Ordering::SeqCst);
                    lifecycle::HealthOutcome::Transient
                })
            });

            let (tx, mut rx) = tokio::sync::watch::channel(None::<StartupOutcome>);
            let tx = Arc::new(tx);
            let count = Arc::clone(&probe_count);

            spawn_late_readiness_monitor_with_probe(
                Arc::clone(&tx),
                "test-endpoint".to_owned(),
                probe,
                count,
                "test-workspace".to_owned(),
            );

            // Advance 500ms in small steps.
            for _ in 0..50 {
                tokio::time::advance(Duration::from_millis(10)).await;
                tokio::task::yield_now().await;
            }
            let mid_count = probe_count.load(Ordering::SeqCst);
            assert!(
                mid_count > 0,
                "R2b: monitor must probe at least once in 500ms"
            );

            // Advance another 500ms.
            for _ in 0..50 {
                tokio::time::advance(Duration::from_millis(10)).await;
                tokio::task::yield_now().await;
            }
            let final_count = probe_count.load(Ordering::SeqCst);
            assert!(
                final_count > mid_count,
                "R2b: probe count must strictly increase over 1s; mid={mid_count}, final={final_count}"
            );

            // No Degraded published.
            let current = rx.borrow_and_update().clone();
            assert!(
                !matches!(current, Some(StartupOutcome::Degraded { .. })),
                "R2b: monitor must NOT publish Degraded on transient failures"
            );

            // Clean up: drop tx to allow monitor to exit
            drop(tx);
            tokio::task::yield_now().await;
        }
    }

    /// C5 — request-terminal vs monitor race guard (138.012-T).
    ///
    /// The request path latches `Degraded` WHILE the monitor's second probe
    /// is already in flight (genuinely concurrent with the probe call, not
    /// merely before it starts), and that in-flight probe then resolves
    /// `Ready`. The monitor's `Ready` branch MUST NOT publish `Ready` over
    /// an externally-latched `Degraded` — the final published state must
    /// remain `Degraded`.
    ///
    /// This is the BLOCKING guard for the revision-1 fail-open race: without
    /// a monotonic latch (`send_if_modified`) in the `Ready` branch itself,
    /// the monitor would unconditionally overwrite the watch channel value
    /// including a `Degraded` published by the request path. The race is
    /// deliberately forced to land *inside* the second probe call (via
    /// `Notify` signals, not sleeps) so a regression that removed only the
    /// top-of-loop early-return optimisation (leaving the unconditional
    /// `send` in the `Ready` branch) would still be caught — the earlier
    /// version of this test exited via the early-return before the second
    /// probe ever ran, and so could not have caught that regression.
    #[tokio::test(start_paused = true)]
    async fn c5_request_terminal_vs_monitor_race() {
        for _ in 0..5 {
            let workspace = tempfile::TempDir::new().expect("workspace tempdir");
            let workspace_hint = workspace.path().display().to_string();
            let probe_count = Arc::new(AtomicUsize::new(0));
            let call_count = Arc::new(AtomicUsize::new(0));
            let cc = Arc::clone(&call_count);
            let probe_entered = Arc::new(tokio::sync::Notify::new());
            let release_probe = Arc::new(tokio::sync::Notify::new());
            let pe = Arc::clone(&probe_entered);
            let rp = Arc::clone(&release_probe);

            // Probe: Transient on the first call. On the second call, signal
            // that it has genuinely entered, block on a test-controlled
            // release, then resolve Ready — landing the race window inside
            // the in-flight probe rather than before it starts.
            let probe: MonitorProbeFn = Arc::new(move |_| {
                let cc = Arc::clone(&cc);
                let pe = Arc::clone(&pe);
                let rp = Arc::clone(&rp);
                Box::pin(async move {
                    let n = cc.fetch_add(1, Ordering::SeqCst);
                    if n >= 1 {
                        pe.notify_one();
                        rp.notified().await;
                        lifecycle::HealthOutcome::Ready
                    } else {
                        lifecycle::HealthOutcome::Transient
                    }
                })
            });

            let (tx, mut rx) = tokio::sync::watch::channel(None::<StartupOutcome>);
            let tx = Arc::new(tx);
            let count = Arc::clone(&probe_count);

            spawn_late_readiness_monitor_with_probe(
                Arc::clone(&tx),
                "test-endpoint".to_owned(),
                probe,
                count,
                workspace_hint.clone(),
            );

            // Advance the paused clock past the first (Transient) probe and
            // into the second backoff tick, entering the second probe call.
            for _ in 0..30 {
                tokio::time::advance(Duration::from_millis(10)).await;
                tokio::task::yield_now().await;
                if probe_count.load(Ordering::SeqCst) >= 2 {
                    break;
                }
            }
            probe_entered.notified().await;
            assert_eq!(
                probe_count.load(Ordering::SeqCst),
                2,
                "C5 setup: the second probe must have genuinely entered before the race is injected"
            );

            // Race: latch Degraded (simulating the request path) WHILE the
            // second probe is still in flight, awaiting release.
            let _ = tx.send(Some(StartupOutcome::Degraded {
                class: ShimFailureClass::ProtocolIncompatible,
                message: "terminal latch by request path".to_owned(),
            }));
            tokio::task::yield_now().await;

            // Release the in-flight probe; it now resolves Ready.
            release_probe.notify_one();

            // Let the monitor process the Ready result and (incorrectly, if
            // regressed) publish it.
            for _ in 0..20 {
                tokio::time::advance(Duration::from_millis(10)).await;
                tokio::task::yield_now().await;
            }

            // Final state: must still be Degraded, not Ready — the race must
            // not un-latch a proven-terminal session.
            let current = rx.borrow_and_update().clone();
            assert!(
                matches!(current, Some(StartupOutcome::Degraded { .. })),
                "C5: final state must be Degraded (not overwritten by a concurrent monitor Ready); got {current:?}"
            );
            assert_eq!(
                probe_count.load(Ordering::SeqCst),
                2,
                "C5: exactly 2 probes (Transient, then the raced Ready) — no further probing after the race"
            );

            // The monitor's Ready branch lost the race (its publish was
            // rejected because Degraded was already latched); it must still
            // have written the promised late-terminal record before
            // exiting, since this is its only remaining chance to do so —
            // the top-of-loop early-return check never runs again after
            // this branch returns (Copilot review finding on PR #366).
            //
            // The write happens on tokio's real (non-virtual) blocking
            // thread pool, which `tokio::time::advance` does not drive —
            // give it a short real-wall-clock allowance to complete and
            // retry a few times rather than asserting immediately.
            let mut content = String::new();
            let record_path = workspace
                .path()
                .join(".engram")
                .join("diagnostics")
                .join("shim-startup-failures.jsonl");
            for attempt in 0..20 {
                content = std::fs::read_to_string(&record_path).unwrap_or_default();
                if content.contains("protocol_incompatible") {
                    break;
                }
                let _ = attempt;
                std::thread::sleep(std::time::Duration::from_millis(25));
            }
            let protocol_records = content
                .lines()
                .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
                .filter(|r| r["failure_class"] == "protocol_incompatible")
                .count();
            assert_eq!(
                protocol_records, 1,
                "C5: the monitor must write exactly one protocol_incompatible record even when \
                 its own Ready publish loses the race to an externally-latched Degraded; \
                 record file contents: {content}"
            );
        }
    }
}
