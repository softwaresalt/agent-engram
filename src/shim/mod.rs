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
//! catalog. If the preconditions fail, the session is degraded: `tools/call`
//! returns a structured error naming the cause (see
//! [`transport::ShimHandler::call_tool`]), and the shim's own process exit
//! code and a durable startup-failure record carry the classified cause for
//! offline diagnosis (see [`record_startup_failure`]).

pub mod ipc_client;
pub mod lifecycle;
pub mod pidfile;
pub mod tools_catalog;
pub mod transport;
pub mod version;

use std::io::Write as _;
use std::path::Path;
use std::time::Duration;

use serde_json::json;
use tokio::sync::watch;

use crate::errors::{EngramError, ShimFailureClass, ShimStartupError};

/// IPC request timeout used once a daemon endpoint is `Ready`. Deliberately
/// NOT used to bound awaiting the deferred startup outcome (see
/// `transport::ShimHandler::await_startup_outcome`) — otherwise an
/// `ENGRAM_READY_TIMEOUT_MS` configured above this constant would cause a
/// `tools/call` to report a false `readiness_timeout` while
/// `ensure_daemon_running` was still within its own valid, longer budget.
const IPC_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

/// Outcome of the deferred startup preconditions (workspace admission, daemon
/// readiness, IPC endpoint derivation), computed concurrently with serving
/// the MCP `initialize` handshake.
#[derive(Debug, Clone)]
pub enum StartupOutcome {
    /// All preconditions succeeded; tool calls may be forwarded to `endpoint`.
    Ready { endpoint: String },
    /// A precondition failed; the session stays up but every `tools/call`
    /// must fail with the recorded, classified cause.
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
async fn compute_startup_outcome(workspace_override: Option<String>) -> StartupOutcome {
    let workspace_arg = match resolve_workspace_arg(workspace_override.as_deref()) {
        Ok(arg) => arg,
        Err(EngramError::ShimStartup(ShimStartupError { class, message })) => {
            let outcome = StartupOutcome::Degraded { class, message };
            record_startup_failure(None, class).await;
            return outcome;
        }
        Err(err) => {
            // Unreachable in practice (resolve_workspace_arg only ever
            // returns ShimStartup), but classify defensively rather than
            // panicking or losing the error.
            let outcome = StartupOutcome::degraded(ShimFailureClass::AdmissionFailure, &err);
            record_startup_failure(None, ShimFailureClass::AdmissionFailure).await;
            return outcome;
        }
    };

    let workspace_path = match crate::db::workspace::canonicalize_workspace(&workspace_arg) {
        Ok(path) => path,
        Err(err) => {
            let err = EngramError::from(err);
            let outcome = StartupOutcome::degraded(ShimFailureClass::AdmissionFailure, &err);
            // canonicalize_workspace failed, so workspace_arg is not a
            // validated root; pass it only as a best-effort location hint
            // (record_startup_failure applies its own no-follow guards
            // before writing anything under it).
            record_startup_failure(Some(&workspace_arg), ShimFailureClass::AdmissionFailure).await;
            return outcome;
        }
    };

    if let Err(err) = lifecycle::ensure_daemon_running(&workspace_path).await {
        let outcome = StartupOutcome::degraded(ShimFailureClass::ReadinessTimeout, &err);
        record_startup_failure(
            Some(&workspace_path.display().to_string()),
            ShimFailureClass::ReadinessTimeout,
        )
        .await;
        return outcome;
    }

    match crate::daemon::ipc_server::ipc_endpoint(&workspace_path) {
        Ok(endpoint) => StartupOutcome::Ready { endpoint },
        Err(err) => {
            let outcome =
                StartupOutcome::degraded(ShimFailureClass::EndpointDerivationFailure, &err);
            record_startup_failure(
                Some(&workspace_path.display().to_string()),
                ShimFailureClass::EndpointDerivationFailure,
            )
            .await;
            outcome
        }
    }
}

/// Best-effort durable startup-failure record under `<workspace>/.engram/diagnostics/`.
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
async fn record_startup_failure(workspace_hint: Option<&str>, class: ShimFailureClass) {
    let Some(workspace_hint) = workspace_hint else {
        return;
    };
    let workspace_hint = workspace_hint.to_owned();
    let _ = tokio::task::spawn_blocking(move || {
        write_startup_failure_record(&workspace_hint, class);
    })
    .await;
}

/// Resolve `<root>/.engram/diagnostics` with no-follow semantics: refuses to
/// write through a pre-existing symlink or reparse point at either `.engram`
/// or `.engram/diagnostics`, and verifies the final canonicalized directory
/// still resolves inside the canonicalized workspace root (workspace
/// containment; Constitution Principle III/IV).
fn no_follow_diagnostics_dir(workspace_hint: &str) -> Option<std::path::PathBuf> {
    let workspace_root = Path::new(workspace_hint).canonicalize().ok()?;
    if !workspace_root.is_dir() {
        return None;
    }
    let engram_dir = workspace_root.join(".engram");
    if let Ok(meta) = std::fs::symlink_metadata(&engram_dir) {
        if meta.file_type().is_symlink() {
            return None;
        }
    }
    let diagnostics_dir = engram_dir.join("diagnostics");
    if let Ok(meta) = std::fs::symlink_metadata(&diagnostics_dir) {
        if meta.file_type().is_symlink() {
            return None;
        }
    }
    std::fs::create_dir_all(&diagnostics_dir).ok()?;
    let canonical_diagnostics_dir = diagnostics_dir.canonicalize().ok()?;
    if !canonical_diagnostics_dir.starts_with(&workspace_root) {
        return None;
    }
    Some(canonical_diagnostics_dir)
}

fn write_startup_failure_record(workspace_hint: &str, class: ShimFailureClass) {
    let Some(diagnostics_dir) = no_follow_diagnostics_dir(workspace_hint) else {
        return;
    };
    let record_path = diagnostics_dir.join("shim-startup-failures.jsonl");
    if let Ok(meta) = std::fs::symlink_metadata(&record_path) {
        if meta.file_type().is_symlink() {
            return;
        }
    }
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
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&record_path)
    {
        let _ = file.write_all(line.as_bytes());
    }
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
    let startup_task = tokio::spawn(async move {
        let outcome = compute_startup_outcome(workspace_override).await;
        let _ = outcome_tx.send(Some(outcome.clone()));
        outcome
    });

    let session_result = transport::run_shim(outcome_rx, IPC_REQUEST_TIMEOUT).await;

    // The MCP session has already ended (client disconnected or transport
    // closed). The background precondition task's own result is only needed
    // to classify the final exit code/diagnostics — it must not block
    // process teardown for however long `ensure_daemon_running`'s internal
    // readiness budget (up to 30s+) takes if the client vanished before any
    // `tools/call` ever needed the outcome. Bound the join with a short
    // grace period; if the task is still pending, exit cleanly rather than
    // linger (the background task is dropped/cancelled when the runtime
    // shuts down at process exit).
    let outcome = match tokio::time::timeout(Duration::from_secs(2), startup_task).await {
        Ok(join_result) => join_result.unwrap_or_else(|join_err| StartupOutcome::Degraded {
            class: ShimFailureClass::TransportFailure,
            message: format!("startup precondition task did not complete: {join_err}"),
        }),
        Err(_elapsed) => {
            // No definitive classification available in time; treat as a
            // benign, unclassified session end rather than guessing a cause.
            StartupOutcome::Ready {
                endpoint: String::new(),
            }
        }
    };

    // A transport-level failure (e.g. the stdio transport failed to bind, or
    // the MCP session ended with a protocol error) takes precedence over the
    // precondition classification, since it is the more proximate cause.
    session_result?;

    if let StartupOutcome::Degraded { class, message } = outcome {
        return Err(EngramError::ShimStartup(ShimStartupError {
            class,
            message,
        }));
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
        let diagnostics_dir =
            no_follow_diagnostics_dir(&workspace_hint).expect("must resolve for a clean workspace");
        assert!(
            diagnostics_dir.starts_with(
                workspace
                    .path()
                    .canonicalize()
                    .expect("canonicalize workspace")
            )
        );
        assert!(diagnostics_dir.is_dir());
    }
}
