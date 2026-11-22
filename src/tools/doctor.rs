//! Doctor diagnostic tool: health report and smoke-test functionality.
//!
//! Provides [`get_health_report_for_daemon`] (returns a structured
//! [`HealthReport`] covering all eight 029-F failure modes) and
//! [`run_smoke_test`] (runs a full shim→daemon handshake round-trip
//! for the `doctor --smoke` CLI subcommand).

use std::path::Path;
use std::sync::{Mutex, OnceLock};

use tokio::sync::oneshot;

use crate::errors::EngramError;
use crate::models::config::DaemonMode;
use crate::models::health::{HealthCheck, HealthReport, HealthStatus, SmokeResult};
use crate::server::state::{AppState, DispatchSnapshot};
use crate::shim::version::ENGRAM_PROTOCOL_VERSION;

struct GenerationPinTestHook {
    method: String,
    reached: Option<oneshot::Sender<()>>,
    resume: Option<oneshot::Receiver<()>>,
}

fn generation_pin_test_hook() -> &'static Mutex<Option<GenerationPinTestHook>> {
    static HOOK: OnceLock<Mutex<Option<GenerationPinTestHook>>> = OnceLock::new();
    HOOK.get_or_init(|| Mutex::new(None))
}

/// Install a one-shot barrier reached immediately after a doctor handler pins its dispatch context.
#[doc(hidden)]
pub fn install_generation_pin_test_hook(
    method: &str,
    reached: oneshot::Sender<()>,
    resume: oneshot::Receiver<()>,
) {
    let hook = GenerationPinTestHook {
        method: method.to_owned(),
        reached: Some(reached),
        resume: Some(resume),
    };
    *generation_pin_test_hook()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(hook);
}

async fn maybe_pause_generation_pin_test_hook(method: &str) {
    let pending_resume = {
        let mut slot = generation_pin_test_hook()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(mut hook) = slot.take() else {
            return;
        };
        if hook.method != method {
            *slot = Some(hook);
            return;
        }
        if let Some(reached) = hook.reached.take() {
            let _ = reached.send(());
        }
        hook.resume.take()
    };

    if let Some(resume) = pending_resume {
        let _ = resume.await;
    }
}

struct HealthInputs {
    dispatch: Option<DispatchSnapshot>,
    last_indexed_at: Option<chrono::DateTime<chrono::Utc>>,
    tool_call_count: u64,
    watcher_events: u64,
}

async fn snapshot_health_inputs(state: &AppState, method: &str) -> HealthInputs {
    let dispatch = state.snapshot_dispatch_context().await;
    if dispatch.is_some() {
        maybe_pause_generation_pin_test_hook(method).await;
    }
    let last_indexed_at = state.last_indexed_at().await;
    let tool_call_count = state.tool_call_count();
    let (watcher_events, _) = state.watcher_stats().await;
    HealthInputs {
        dispatch,
        last_indexed_at,
        tool_call_count,
        watcher_events,
    }
}

// ── Individual health checks ─────────────────────────────────────────────────

fn check_binary_version() -> HealthCheck {
    HealthCheck {
        name: "binary_version".to_owned(),
        status: HealthStatus::Green,
        message: Some(format!(
            "v{} protocol {}",
            env!("CARGO_PKG_VERSION"),
            ENGRAM_PROTOCOL_VERSION
        )),
        remediation: None,
    }
}

fn check_pid_liveness() -> HealthCheck {
    let pid = std::process::id();
    HealthCheck {
        name: "pid_liveness".to_owned(),
        status: HealthStatus::Green,
        message: Some(format!("PID {pid} is live")),
        remediation: None,
    }
}

fn check_workspace_identity(inputs: &HealthInputs) -> HealthCheck {
    match inputs.dispatch.as_ref() {
        Some(dispatch) => HealthCheck {
            name: "workspace_identity".to_owned(),
            status: HealthStatus::Green,
            message: Some(format!(
                "workspace {} bound at {}",
                dispatch.workspace.workspace_id, dispatch.workspace.path
            )),
            remediation: None,
        },
        None => HealthCheck {
            name: "workspace_identity".to_owned(),
            status: HealthStatus::Yellow,
            message: Some("no workspace bound".to_owned()),
            remediation: Some("call set_workspace to bind a workspace".to_owned()),
        },
    }
}

fn check_pipe_reachability() -> HealthCheck {
    // In-process check: if this function is executing, the IPC pipe is
    // reachable — we are responding to a live request through it.
    HealthCheck {
        name: "pipe_reachability".to_owned(),
        status: HealthStatus::Green,
        message: Some("IPC endpoint is serving this request".to_owned()),
        remediation: None,
    }
}

fn check_registry_validity(inputs: &HealthInputs) -> HealthCheck {
    match inputs.dispatch.as_ref() {
        None => HealthCheck {
            name: "registry_validity".to_owned(),
            status: HealthStatus::Yellow,
            message: Some("no workspace config loaded (no workspace bound)".to_owned()),
            remediation: Some("call set_workspace to load the registry configuration".to_owned()),
        },
        Some(_) => HealthCheck {
            name: "registry_validity".to_owned(),
            status: HealthStatus::Green,
            message: Some("workspace configuration loaded".to_owned()),
            remediation: None,
        },
    }
}

fn check_offline_scan(inputs: &HealthInputs) -> HealthCheck {
    match inputs.dispatch.as_ref() {
        None => HealthCheck {
            name: "offline_scan".to_owned(),
            status: HealthStatus::Yellow,
            message: Some("offline scan skipped — no workspace bound".to_owned()),
            remediation: Some("call set_workspace before requesting an offline scan".to_owned()),
        },
        Some(dispatch) if dispatch.workspace.stale_files => HealthCheck {
            name: "offline_scan".to_owned(),
            status: HealthStatus::Yellow,
            message: Some("workspace has stale files since last flush".to_owned()),
            remediation: Some(
                "call flush_state or sync_workspace to re-index stale files".to_owned(),
            ),
        },
        Some(_) => HealthCheck {
            name: "offline_scan".to_owned(),
            status: HealthStatus::Green,
            message: Some("no offline changes detected".to_owned()),
            remediation: None,
        },
    }
}

fn check_session_resume(inputs: &HealthInputs) -> HealthCheck {
    match (inputs.dispatch.is_some(), inputs.last_indexed_at) {
        (false, _) => HealthCheck {
            name: "session_resume".to_owned(),
            status: HealthStatus::Yellow,
            message: Some("no active session — workspace not bound".to_owned()),
            remediation: Some("call set_workspace to start a session".to_owned()),
        },
        (true, None) => HealthCheck {
            name: "session_resume".to_owned(),
            status: HealthStatus::Yellow,
            message: Some("workspace bound but not yet indexed".to_owned()),
            remediation: Some(
                "call flush_state or index_workspace to complete session setup".to_owned(),
            ),
        },
        (true, Some(indexed_at)) => HealthCheck {
            name: "session_resume".to_owned(),
            status: HealthStatus::Green,
            message: Some(format!("session indexed at {}", indexed_at.to_rfc3339())),
            remediation: None,
        },
    }
}

fn check_telemetry_health(inputs: &HealthInputs) -> HealthCheck {
    let tool_calls = inputs.tool_call_count;
    let watcher_events = inputs.watcher_events;

    if tool_calls == 0 && watcher_events == 0 {
        HealthCheck {
            name: "telemetry_health".to_owned(),
            status: HealthStatus::Yellow,
            message: Some("no telemetry recorded yet (daemon just started)".to_owned()),
            remediation: None,
        }
    } else {
        HealthCheck {
            name: "telemetry_health".to_owned(),
            status: HealthStatus::Green,
            message: Some(format!(
                "{tool_calls} tool calls, {watcher_events} watcher events recorded"
            )),
            remediation: None,
        }
    }
}

// ── Overall roll-up ──────────────────────────────────────────────────────────

fn derive_overall(checks: &[HealthCheck]) -> HealthStatus {
    checks
        .iter()
        .fold(HealthStatus::Green, |worst, c| match (worst, c.status) {
            (_, HealthStatus::Red) | (HealthStatus::Red, _) => HealthStatus::Red,
            (_, HealthStatus::Yellow | HealthStatus::Unknown)
            | (HealthStatus::Yellow | HealthStatus::Unknown, _) => HealthStatus::Yellow,
            _ => worst,
        })
}

// ── Public API ───────────────────────────────────────────────────────────────

/// Build a structured health report for the daemon covering all eight
/// 029-F failure modes.
///
/// Returns a [`HealthReport`] with one [`HealthCheck`] per failure mode:
/// `binary_version`, `pid_liveness`, `workspace_identity`,
/// `pipe_reachability`, `registry_validity`, `offline_scan`,
/// `session_resume`, `telemetry_health`.
pub async fn get_health_report_for_daemon(state: &AppState) -> Result<HealthReport, EngramError> {
    let inputs = snapshot_health_inputs(state, "get_health_report_for_daemon").await;
    let checks = vec![
        check_binary_version(),
        check_pid_liveness(),
        check_workspace_identity(&inputs),
        check_pipe_reachability(),
        check_registry_validity(&inputs),
        check_offline_scan(&inputs),
        check_session_resume(&inputs),
        check_telemetry_health(&inputs),
    ];

    let overall = derive_overall(&checks);
    Ok(HealthReport { overall, checks })
}

#[doc(hidden)]
pub fn smoke_request_methods_for_mode(mode: DaemonMode) -> Vec<&'static str> {
    match mode {
        DaemonMode::Managed => vec!["get_daemon_status", "set_workspace", "_shutdown"],
        DaemonMode::ReadServer => vec!["get_daemon_status", "get_workspace_status"],
    }
}

#[doc(hidden)]
/// Return the smoke methods that remain after the initial daemon-status probe.
pub fn smoke_request_methods_for_run(mode: DaemonMode, daemon_started: bool) -> Vec<&'static str> {
    match (mode, daemon_started) {
        (DaemonMode::Managed, true) => vec!["set_workspace", "_shutdown"],
        (DaemonMode::Managed, false) => Vec::new(),
        (DaemonMode::ReadServer, _) => vec!["get_workspace_status"],
    }
}

/// Run a full shim→daemon handshake round-trip for the `doctor --smoke` CLI flag.
///
/// Spawns or reuses the daemon, then connects as a shim and exchanges the
/// version handshake via `get_daemon_status`. Doctor never binds or shuts down
/// a reused daemon. Reused `ReadServer` daemons still receive the read-only
/// `get_workspace_status` readiness probe.
/// The daemon mode is read from the live response rather than on-disk
/// configuration. Returns a [`SmokeResult`] indicating pass or fail with
/// latency measurement.
pub async fn run_smoke_test(workspace: &Path) -> Result<SmokeResult, EngramError> {
    run_smoke_test_with_timeout(workspace, std::time::Duration::from_secs(10)).await
}

/// Run the doctor smoke workflow with the caller-selected IPC request timeout.
///
/// The timeout applies to smoke IPC requests after daemon lifecycle readiness
/// is established. Daemon startup retains its independent ready-wait budget.
///
/// # Errors
///
/// Returns the lifecycle or IPC error encountered while running the smoke
/// workflow.
pub async fn run_smoke_test_with_timeout(
    workspace: &Path,
    ipc_timeout: std::time::Duration,
) -> Result<SmokeResult, EngramError> {
    use serde_json::{Value, json};

    use crate::daemon::ipc_server::ipc_endpoint;
    use crate::daemon::protocol::IpcRequest;
    use crate::shim::ipc_client::send_request;
    use crate::shim::lifecycle::{
        DaemonOwnership, cleanup_owned_daemon, combine_cleanup_error,
        ensure_daemon_running_for_doctor,
    };

    let start = std::time::Instant::now();

    // Step 1: Spawn or reuse the daemon.
    let mut ownership = ensure_daemon_running_for_doctor(workspace)
        .await
        .map_err(|error| match error {
            refused @ crate::errors::EngramError::Ipc(
                crate::errors::IpcError::VersionMismatch { .. }
                | crate::errors::IpcError::ConnectionFailed { .. },
            ) => refused,
            cleanup @ crate::errors::EngramError::Daemon(
                crate::errors::DaemonError::CleanupFailed { .. },
            ) => cleanup,
            error => crate::errors::EngramError::Daemon(crate::errors::DaemonError::SpawnFailed {
                reason: format!("smoke test: daemon start failed: {error}"),
            }),
        })?;

    let endpoint = ipc_endpoint(workspace)?;
    let smoke_outcome = async {
        let workspace_str = workspace
            .to_str()
            .ok_or_else(|| {
                crate::errors::EngramError::Daemon(crate::errors::DaemonError::SpawnFailed {
                    reason: "workspace path contains non-UTF-8 characters".to_owned(),
                })
            })?
            .to_owned();

        // Step 2: always probe get_daemon_status first and read the *actual live
        // daemon's* reported mode from its response, rather than trusting
        // on-disk configuration via `resolve_daemon_mode`. `ensure_daemon_running`
        // may have reused an already-running daemon whose mode was fixed at its
        // own startup; if configuration changed on disk afterward, selecting the
        // remaining smoke sequence from stale on-disk config could send a
        // write-capable `set_workspace` + `_shutdown` sequence to a live
        // ReadServer daemon, violating its non-destructive smoke contract.
        let status_request = IpcRequest {
            jsonrpc: "2.0".to_owned(),
            id: Some(Value::Number(serde_json::Number::from(1))),
            method: "get_daemon_status".to_owned(),
            params: None,
        };
        let status_response = send_request(&endpoint, &status_request, ipc_timeout).await?;
        if let Some(err) = &status_response.error {
            let latency_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
            return Ok(SmokeResult {
                passed: false,
                message: format!("get_daemon_status failed: {}", err.message),
                latency_ms: Some(latency_ms),
            });
        }
        let observed_mode_str = status_response
            .result
            .as_ref()
            .and_then(|result| result.get("mode"))
            .and_then(Value::as_str)
            .ok_or_else(|| {
                crate::errors::EngramError::Daemon(crate::errors::DaemonError::SpawnFailed {
                    reason: "smoke test: get_daemon_status response missing mode field".to_owned(),
                })
            })?;
        let mode = DaemonMode::resolve(Some(observed_mode_str)).map_err(|e| {
            crate::errors::EngramError::Daemon(crate::errors::DaemonError::SpawnFailed {
                reason: format!("smoke test: daemon reported unrecognized mode: {e}"),
            })
        })?;
        // Step 3: reused Managed daemons stop after the status probe. ReadServer
        // always receives its read-only workspace-status readiness probe, while
        // invocation-started Managed daemons also receive bind and shutdown.
        let remaining_methods = smoke_request_methods_for_run(mode, ownership.is_started());
        for (offset, method) in remaining_methods.iter().enumerate() {
            let index = offset + 2;
            let request = match *method {
                "get_workspace_status" => IpcRequest {
                    jsonrpc: "2.0".to_owned(),
                    id: Some(Value::Number(serde_json::Number::from(index))),
                    method: "get_workspace_status".to_owned(),
                    params: None,
                },
                "set_workspace" => IpcRequest {
                    jsonrpc: "2.0".to_owned(),
                    id: Some(Value::Number(serde_json::Number::from(index))),
                    method: "set_workspace".to_owned(),
                    params: Some(json!({ "path": workspace_str })),
                },
                "_shutdown" => IpcRequest {
                    jsonrpc: "2.0".to_owned(),
                    id: Some(Value::Number(serde_json::Number::from(index))),
                    method: "_shutdown".to_owned(),
                    params: None,
                },
                other => unreachable!("unexpected smoke workflow method: {other}"),
            };

            let response = send_request(&endpoint, &request, ipc_timeout).await?;
            if let Some(err) = &response.error {
                let latency_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
                return Ok(SmokeResult {
                    passed: false,
                    message: format!("{method} failed: {}", err.message),
                    latency_ms: Some(latency_ms),
                });
            }
        }

        let latency_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
        let message = match (&ownership, mode) {
            (DaemonOwnership::Reused, DaemonMode::Managed) => {
                "Smoke test passed: version exchange succeeded and the reused daemon was left untouched"
                    .to_owned()
            }
            (DaemonOwnership::Reused, DaemonMode::ReadServer) => {
                "Smoke test passed: version exchange and workspace-status readiness succeeded and the reused daemon was left untouched"
                    .to_owned()
            }
            (DaemonOwnership::Started { .. }, DaemonMode::Managed) => {
                "Smoke test passed: version exchange, workspace bind, and shutdown all succeeded"
                    .to_owned()
            }
            (DaemonOwnership::Started { .. }, DaemonMode::ReadServer) => {
                "Smoke test passed: version exchange and workspace-status readiness succeeded without write or shutdown"
                    .to_owned()
            }
        };
        Ok(SmokeResult {
            passed: true,
            message,
            latency_ms: Some(latency_ms),
        })
    }
    .await;

    let cleanup_outcome = match &mut ownership {
        DaemonOwnership::Started { child, pid_file } => {
            cleanup_owned_daemon(workspace, &endpoint, pid_file, child).await
        }
        DaemonOwnership::Reused => Ok(()),
    };

    match (smoke_outcome, cleanup_outcome) {
        (Ok(result), Ok(())) => Ok(result),
        (Err(original_error), Err(cleanup_error)) => {
            Err(combine_cleanup_error(original_error, cleanup_error))
        }
        (Ok(_), Err(cleanup_error)) | (Err(cleanup_error), Ok(())) => Err(cleanup_error),
    }
}
