//! End-to-end integration proofs for dual-source correlation-id emission
//! (067.004-T / t4).
//!
//! These drive the REAL daemon dispatch choke point (`tools::dispatch`) and the
//! REAL metrics writer in-process, proving:
//!   (a) MCP `_meta.correlation_id` → written record,
//!   (b) CLI `--correlation-id` (parsed → injected into `_meta`) → written record,
//!   (d) every record carries a pinned ISO-8601-UTC timestamp + schema_version 2
//!       on the branch-aware `.engram/metrics/<branch>/usage.jsonl` path.
//! The daemonless direct path (proof c) is covered by
//! `tests/integration/cli_direct_usage_emit.rs`.
//!
//! `set_workspace` binds the workspace AND initializes the process-global metrics
//! writer (lifecycle.rs), and is in `should_record_metrics`, so re-binding with a
//! correlation id both routes and records through the production path. All
//! singleton usage is consolidated into a single test to avoid cross-test races
//! on the process-global writer.

use std::fs;
use std::path::Path;
use std::sync::Arc;

use clap::Parser;
use serde_json::{Value, json};

use engram::cli::flags::GlobalFlags;
use engram::cli::runner::inject_correlation_id;
use engram::models::metrics::MetricsConfig;
use engram::server::state::AppState;
use engram::services::metrics::resolve_usage_path;
use engram::tools;

/// Minimal clap harness to parse the shared global flags from an argv slice.
#[derive(Debug, Parser)]
struct FlagHarness {
    #[command(flatten)]
    flags: GlobalFlags,
}

fn init_git(dir: &Path) {
    let git_dir = dir.join(".git");
    fs::create_dir_all(&git_dir).expect("create .git");
    fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").expect("write HEAD");
}

fn read_records(usage_path: &Path) -> Vec<Value> {
    let content = fs::read_to_string(usage_path)
        .unwrap_or_else(|e| panic!("usage.jsonl must exist at {}: {e}", usage_path.display()));
    content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str::<Value>(l).expect("valid JSON line"))
        .collect()
}

fn assert_utc_schema2(rec: &Value) {
    assert_eq!(rec["schema_version"], Value::from(2), "schema_version must be 2");
    let ts = rec["timestamp"].as_str().expect("timestamp present");
    let parsed = chrono::DateTime::parse_from_rfc3339(ts).expect("timestamp is ISO-8601");
    assert_eq!(
        parsed.offset().local_minus_utc(),
        0,
        "timestamp must be pinned to UTC"
    );
}

/// Proofs (a) + (b) + (d): both the MCP envelope id and the CLI-parsed id reach
/// distinct written records via the real dispatch + writer, each UTC-stamped and
/// schema_version 2, on the branch-aware path.
#[tokio::test]
async fn t067_004_dual_source_correlation_id_emits_records() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path().canonicalize().expect("canonicalize");
    init_git(&ws);
    let path = ws.to_string_lossy().to_string();

    let state = Arc::new(AppState::new(10));

    // Bind once: no record (pre-bind snapshot is None) but the metrics writer is
    // now initialized against this workspace.
    tools::dispatch(state.clone(), "set_workspace", Some(json!({ "path": path })))
        .await
        .expect("initial set_workspace bind");

    // (a) MCP _meta.correlation_id → record.
    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path, "_meta": { "correlation_id": "mcp-corr-a" } })),
    )
    .await
    .expect("set_workspace with _meta id");

    // (b) CLI --correlation-id parsed, then injected into _meta exactly as the
    // runner does, then dispatched → record.
    let flags = FlagHarness::parse_from(["engram", "--correlation-id", "cli-corr-b"]).flags;
    let cli_id = flags.resolve_correlation_id().expect("valid cli id");
    let params_b = inject_correlation_id(Some(json!({ "path": path })), cli_id.as_deref());
    tools::dispatch(state.clone(), "set_workspace", params_b)
        .await
        .expect("set_workspace with injected cli id");

    // Drain the writer before reading.
    engram::services::metrics::shutdown()
        .await
        .expect("drain metrics writer");

    let usage_path = ws
        .join(".engram")
        .join("metrics")
        .join("main")
        .join("usage.jsonl");
    let records = read_records(&usage_path);

    let ids: Vec<&str> = records
        .iter()
        .filter_map(|r| r["correlation_id"].as_str())
        .collect();
    assert!(
        ids.contains(&"mcp-corr-a"),
        "MCP _meta correlation id must be recorded; ids: {ids:?}"
    );
    assert!(
        ids.contains(&"cli-corr-b"),
        "CLI --correlation-id must be recorded; ids: {ids:?}"
    );

    for rec in &records {
        assert_eq!(rec["tool_name"], Value::String("set_workspace".into()));
        assert_utc_schema2(rec);
    }
}

/// Proof (d), path shape: the resolved usage path is branch-aware and built with
/// the native OS separator (Windows `\` vs POSIX `/`).
#[test]
fn t067_004_branch_aware_path_is_cross_platform() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();

    let resolved = resolve_usage_path(root, "release-1", &MetricsConfig::default())
        .expect("default branch-scoped path");

    let sep = std::path::MAIN_SEPARATOR;
    let tail = format!("metrics{sep}release-1{sep}usage.jsonl");
    let resolved_str = resolved.to_string_lossy();
    assert!(
        resolved_str.ends_with(&tail),
        "path must be branch-aware with native separators; got: {resolved_str}"
    );
    assert!(resolved.starts_with(root));
}
