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
/// This is the one remaining synchronous step that runs before the MCP
/// stdio transport binds. It fails only if the current working directory is
/// unavailable (e.g. deleted mid-session) when no override or environment
/// value is present — a genuinely unrecoverable admission failure.
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

/// Evaluate the three deferred preconditions in order, classifying whichever
/// step fails first. Each step's outcome maps to a distinct
/// [`ShimFailureClass`] so the cause is attributable without inspecting the
/// underlying [`EngramError`] variant.
async fn compute_startup_outcome(workspace_arg: &str) -> StartupOutcome {
    let workspace_path = match crate::db::workspace::canonicalize_workspace(workspace_arg) {
        Ok(path) => path,
        Err(err) => {
            let err = EngramError::from(err);
            let outcome = StartupOutcome::degraded(ShimFailureClass::AdmissionFailure, &err);
            record_startup_failure(
                workspace_arg,
                ShimFailureClass::AdmissionFailure,
                &err.to_string(),
            );
            return outcome;
        }
    };

    if let Err(err) = lifecycle::ensure_daemon_running(&workspace_path).await {
        let outcome = StartupOutcome::degraded(ShimFailureClass::ReadinessTimeout, &err);
        record_startup_failure(
            &workspace_path.display().to_string(),
            ShimFailureClass::ReadinessTimeout,
            &err.to_string(),
        );
        return outcome;
    }

    match crate::daemon::ipc_server::ipc_endpoint(&workspace_path) {
        Ok(endpoint) => StartupOutcome::Ready { endpoint },
        Err(err) => {
            let outcome =
                StartupOutcome::degraded(ShimFailureClass::EndpointDerivationFailure, &err);
            record_startup_failure(
                &workspace_path.display().to_string(),
                ShimFailureClass::EndpointDerivationFailure,
                &err.to_string(),
            );
            outcome
        }
    }
}

/// Best-effort durable startup-failure record under `<workspace>/.engram/diagnostics/`.
///
/// Contains ONLY a timestamp, the binary build identifier, the classified
/// failure class, and the sanitized error message. Never records
/// credentials, tokens, environment variable values, or paths outside the
/// workspace. Failures to persist the record are swallowed — the record is
/// supplementary diagnostics, not the primary failure signal (the process
/// exit code and stderr line are).
fn record_startup_failure(workspace_hint: &str, class: ShimFailureClass, message: &str) {
    let workspace_root = Path::new(workspace_hint);
    if !workspace_root.is_dir() {
        return;
    }
    let diagnostics_dir = workspace_root.join(".engram").join("diagnostics");
    if std::fs::create_dir_all(&diagnostics_dir).is_err() {
        return;
    }
    let record = json!({
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "binary_version": version::ENGRAM_BUILD_HASH,
        "failure_class": class.as_str(),
        "message": message,
    });
    let Ok(mut line) = serde_json::to_string(&record) else {
        return;
    };
    line.push('\n');
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(diagnostics_dir.join("shim-startup-failures.jsonl"))
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
/// The stdio transport is bound before any daemon-dependent precondition is
/// evaluated, so the MCP `initialize` handshake and `tools/list` always
/// succeed. `tools/call` fails with a structured, attributable error if a
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
    let workspace_arg = resolve_workspace_arg(workspace_override)?;

    // Tracing is pinned to stderr (src/lib.rs) so debug logging never
    // contaminates the MCP stdout framing channel (124-F U5, investigation E5).
    crate::init_tracing(shim_log_format());

    let (outcome_tx, outcome_rx) = watch::channel(None);
    let startup_task = tokio::spawn(async move {
        let outcome = compute_startup_outcome(&workspace_arg).await;
        let _ = outcome_tx.send(Some(outcome.clone()));
        outcome
    });

    let session_result = transport::run_shim(outcome_rx, Duration::from_secs(60)).await;

    let outcome = startup_task
        .await
        .unwrap_or_else(|join_err| StartupOutcome::Degraded {
            class: ShimFailureClass::TransportFailure,
            message: format!("startup precondition task did not complete: {join_err}"),
        });

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
