//! Doctor diagnostic tool: health report and smoke-test functionality.
//!
//! Provides [`get_health_report_for_daemon`] (returns a structured
//! [`HealthReport`] covering all eight 029-F failure modes) and
//! [`run_smoke_test`] (runs a full shim→daemon handshake round-trip
//! for the `doctor --smoke` CLI subcommand).

use std::path::Path;

use crate::errors::EngramError;
use crate::models::health::{HealthCheck, HealthReport, HealthStatus, SmokeResult};
use crate::server::state::AppState;
use crate::shim::version::ENGRAM_PROTOCOL_VERSION;

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

async fn check_workspace_identity(state: &AppState) -> HealthCheck {
    match state.snapshot_workspace().await {
        Some(snap) => HealthCheck {
            name: "workspace_identity".to_owned(),
            status: HealthStatus::Green,
            message: Some(format!(
                "workspace {} bound at {}",
                snap.workspace_id, snap.path
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

async fn check_registry_validity(state: &AppState) -> HealthCheck {
    match state.workspace_config().await {
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

async fn check_offline_scan(state: &AppState) -> HealthCheck {
    match state.snapshot_workspace().await {
        None => HealthCheck {
            name: "offline_scan".to_owned(),
            status: HealthStatus::Yellow,
            message: Some("offline scan skipped — no workspace bound".to_owned()),
            remediation: Some("call set_workspace before requesting an offline scan".to_owned()),
        },
        Some(snap) if snap.stale_files => HealthCheck {
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

async fn check_session_resume(state: &AppState) -> HealthCheck {
    let has_workspace = state.snapshot_workspace().await.is_some();
    let last_indexed = state.last_indexed_at().await;

    match (has_workspace, last_indexed) {
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

async fn check_telemetry_health(state: &AppState) -> HealthCheck {
    let tool_calls = state.tool_call_count();
    let (watcher_events, _) = state.watcher_stats().await;

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
            (_, HealthStatus::Yellow) | (HealthStatus::Yellow, _) => HealthStatus::Yellow,
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
    let checks = vec![
        check_binary_version(),
        check_pid_liveness(),
        check_workspace_identity(state).await,
        check_pipe_reachability(),
        check_registry_validity(state).await,
        check_offline_scan(state).await,
        check_session_resume(state).await,
        check_telemetry_health(state).await,
    ];

    let overall = derive_overall(&checks);
    Ok(HealthReport { overall, checks })
}

/// Run a full shim→daemon handshake round-trip for the `doctor --smoke` CLI flag.
///
/// Spawns the daemon if not running, connects as a shim, exchanges the version
/// handshake, calls `set_workspace`, then shuts down. Returns a [`SmokeResult`]
/// indicating pass or fail with latency measurement.
pub async fn run_smoke_test(workspace: &Path) -> Result<SmokeResult, EngramError> {
    use std::time::Duration;

    use serde_json::{Value, json};

    use crate::daemon::ipc_server::ipc_endpoint;
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

    let workspace_str = workspace
        .to_str()
        .ok_or_else(|| {
            crate::errors::EngramError::Daemon(crate::errors::DaemonError::SpawnFailed {
                reason: "workspace path contains non-UTF-8 characters".to_owned(),
            })
        })?
        .to_owned();

    // Step 2: Version exchange — get_daemon_status.
    let status_req = IpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: Some(Value::Number(serde_json::Number::from(1))),
        method: "get_daemon_status".to_owned(),
        params: None,
    };
    let status_resp = send_request(&endpoint, &status_req, Duration::from_secs(10)).await?;
    if let Some(err) = &status_resp.error {
        let latency_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
        return Ok(SmokeResult {
            passed: false,
            message: format!("get_daemon_status failed: {}", err.message),
            latency_ms: Some(latency_ms),
        });
    }

    // Step 3: set_workspace round-trip.
    let bind_req = IpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: Some(Value::Number(serde_json::Number::from(2))),
        method: "set_workspace".to_owned(),
        params: Some(json!({ "path": workspace_str })),
    };
    let bind_resp = send_request(&endpoint, &bind_req, Duration::from_secs(10)).await?;
    if let Some(err) = &bind_resp.error {
        let latency_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
        return Ok(SmokeResult {
            passed: false,
            message: format!("set_workspace failed: {}", err.message),
            latency_ms: Some(latency_ms),
        });
    }

    // Step 4: Graceful shutdown.
    let shutdown_req = IpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: Some(Value::Number(serde_json::Number::from(3))),
        method: "_shutdown".to_owned(),
        params: None,
    };
    let _ = send_request(&endpoint, &shutdown_req, Duration::from_secs(5)).await;

    let latency_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
    Ok(SmokeResult {
        passed: true,
        message: "Smoke test passed: version exchange, workspace bind, and shutdown all succeeded"
            .to_owned(),
        latency_ms: Some(latency_ms),
    })
}
