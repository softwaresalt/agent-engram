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

/// Run a full shim→daemon handshake round-trip for the `doctor --smoke` CLI flag.
///
/// Spawns the daemon if not running, then connects as a shim and exchanges the
/// version handshake. In [`DaemonMode::Managed`], additionally calls
/// `set_workspace` and shuts the daemon down. In [`DaemonMode::ReadServer`],
/// only re-checks readiness via `get_workspace_status` and performs neither a
/// workspace bind nor a shutdown, since a read-server smoke run must stay
/// non-destructive. Returns a [`SmokeResult`] indicating pass or fail with
/// latency measurement.
pub async fn run_smoke_test(workspace: &Path) -> Result<SmokeResult, EngramError> {
    use std::time::Duration;

    use serde_json::{Value, json};

    use crate::daemon::ipc_server::{ipc_endpoint, resolve_daemon_mode};
    use crate::daemon::protocol::IpcRequest;
    use crate::shim::ipc_client::send_request;
    use crate::shim::lifecycle::ensure_daemon_running;

    let start = std::time::Instant::now();

    // Step 1: Spawn or reuse the daemon.
    ensure_daemon_running(workspace).await.map_err(|e| {
        crate::errors::EngramError::Daemon(crate::errors::DaemonError::SpawnFailed {
            reason: format!("smoke test: daemon start failed: {e}"),
        })
    })?;

    let endpoint = ipc_endpoint(workspace)?;
    let mode = resolve_daemon_mode(workspace)?;

    let workspace_str = workspace
        .to_str()
        .ok_or_else(|| {
            crate::errors::EngramError::Daemon(crate::errors::DaemonError::SpawnFailed {
                reason: "workspace path contains non-UTF-8 characters".to_owned(),
            })
        })?
        .to_owned();

    for (index, method) in smoke_request_methods_for_mode(mode).iter().enumerate() {
        let request = match *method {
            "get_daemon_status" => IpcRequest {
                jsonrpc: "2.0".to_owned(),
                id: Some(Value::Number(serde_json::Number::from(index + 1))),
                method: "get_daemon_status".to_owned(),
                params: None,
            },
            "get_workspace_status" => IpcRequest {
                jsonrpc: "2.0".to_owned(),
                id: Some(Value::Number(serde_json::Number::from(index + 1))),
                method: "get_workspace_status".to_owned(),
                params: None,
            },
            "set_workspace" => IpcRequest {
                jsonrpc: "2.0".to_owned(),
                id: Some(Value::Number(serde_json::Number::from(index + 1))),
                method: "set_workspace".to_owned(),
                params: Some(json!({ "path": workspace_str })),
            },
            "_shutdown" => IpcRequest {
                jsonrpc: "2.0".to_owned(),
                id: Some(Value::Number(serde_json::Number::from(index + 1))),
                method: "_shutdown".to_owned(),
                params: None,
            },
            other => unreachable!("unexpected smoke workflow method: {other}"),
        };

        let response = send_request(&endpoint, &request, Duration::from_secs(10)).await?;
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
    let message = match mode {
        DaemonMode::Managed => {
            "Smoke test passed: version exchange, workspace bind, and shutdown all succeeded"
                .to_owned()
        }
        DaemonMode::ReadServer => {
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
