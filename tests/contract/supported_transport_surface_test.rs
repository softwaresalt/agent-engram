//! Contract test for the supported Engram transport surfaces (plan unit F47,
//! 142.024-T).
//!
//! Engram supports exactly three agent-facing transport surfaces: direct
//! daemon IPC, the `engram` CLI (normally routed over IPC via
//! `cli::runner`, except the `sync --direct` / `index --direct` daemonless
//! mode, which bypasses IPC — see `src/cli/direct.rs`), and stdio MCP via
//! `engram shim`. The legacy HTTP/SSE transport (`legacy-sse` feature, the
//! axum router, MCP HTTP handler, and SSE handler) was retired in 135-S.
//! See ADR-0016 and ADR-0003 (both superseded), and
//! docs/exec-plans/2026-09-02-separate-indexer-read-server-plan.md (P15).
//!
//! The tests below prove `cli::runner`'s IPC-only behavior; they do not
//! cover the `--direct` daemonless bypass, which is exercised separately by
//! `tests/integration/cli_direct_test.rs` and
//! `tests/integration/direct_sync_mode_test.rs`.

use std::sync::Weak;
use std::time::Duration;

use engram::daemon::ipc_server::ipc_endpoint;
use engram::shim::StartupOutcome;
use engram::shim::transport::ShimHandler;

/// Compile-time proof that `T` implements the MCP `rmcp::ServerHandler`
/// trait — i.e. it is a real stdio MCP server surface, not a stub.
fn assert_implements_server_handler<T: rmcp::ServerHandler>(_: &T) {}

// ─── Surface 1: direct daemon IPC ───────────────────────────────────────────

#[test]
fn direct_ipc_endpoint_is_not_http() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    std::fs::create_dir(workspace.path().join(".git")).expect("create .git");

    let endpoint = ipc_endpoint(workspace.path()).expect("ipc endpoint resolves");

    assert!(
        !endpoint.starts_with("http://") && !endpoint.starts_with("https://"),
        "direct IPC endpoint must never be an HTTP URL, got: {endpoint}"
    );

    #[cfg(windows)]
    assert!(
        endpoint.starts_with(r"\\.\pipe\"),
        "expected a Windows named pipe endpoint, got: {endpoint}"
    );
    #[cfg(unix)]
    assert!(
        endpoint.contains("engram"),
        "expected a Unix domain socket path, got: {endpoint}"
    );
}

// ─── Surface 2: the `engram` CLI, routed over IPC ───────────────────────────

#[test]
fn cli_runner_routes_over_ipc_not_http() {
    let runner_source = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/cli/runner.rs"));

    assert!(
        runner_source.contains("ipc_endpoint") && runner_source.contains("IpcRequest"),
        "cli::runner must route CLI commands through the daemon IPC endpoint"
    );
    assert!(
        !runner_source.contains("axum") && !runner_source.contains("http://"),
        "cli::runner must not reference the retired HTTP transport"
    );
}

// ─── Surface 3: stdio MCP via `engram shim` ─────────────────────────────────

#[test]
fn shim_handler_is_a_stdio_mcp_server() {
    let (_tx, rx) = tokio::sync::watch::channel(None::<StartupOutcome>);
    let handler = ShimHandler::new(Weak::new(), rx, Duration::from_secs(1));

    assert_implements_server_handler(&handler);
}

// ─── Retired surface: HTTP/SSE is gone ──────────────────────────────────────

#[test]
fn legacy_http_sse_transport_is_fully_retired() {
    let manifest = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));

    assert!(
        !manifest.contains("legacy-sse"),
        "the legacy-sse feature must be fully removed from Cargo.toml"
    );
    assert!(
        !manifest.contains("axum"),
        "axum must be fully removed as a dependency"
    );
    assert!(
        !manifest.contains("tower-http"),
        "tower-http must be fully removed as a dependency"
    );
    assert!(
        !manifest.contains("tower ="),
        "tower must be fully removed as a dependency"
    );
    assert!(
        !manifest.contains("tokio-stream"),
        "tokio-stream must be fully removed as a dependency"
    );
    assert!(
        manifest.contains("sysinfo"),
        "sysinfo must remain — it has remaining users outside the retired transport"
    );
}
