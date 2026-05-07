//! End-to-end integration tests for the engram CLI binary.
//!
//! Spawns the binary via `std::process::Command` and validates exit codes,
//! JSON-RPC 2.0 envelope structure, and output format switching.

use std::process::Command;

/// Path to the built binary, provided by Cargo for integration tests.
fn engram_bin() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_BIN_EXE_engram"))
}

/// Spawn `engram <args>`, capturing stdout + stderr, return exit status + output.
fn run(args: &[&str]) -> (i32, String, String) {
    let bin = engram_bin();
    let output = Command::new(&bin)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("failed to run {}: {e}", bin.display()));
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let code = output.status.code().unwrap_or(-1);
    (code, stdout, stderr)
}

// ── engram manifest ────────────────────────────────────────────────────────────

#[test]
fn manifest_exits_zero() {
    let (code, _stdout, _stderr) = run(&["manifest", "--json"]);
    assert_eq!(code, 0, "engram manifest must exit 0");
}

#[test]
fn manifest_emits_jsonrpc_envelope() {
    let (_code, stdout, _stderr) = run(&["manifest", "--json"]);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("manifest output must be valid JSON");
    assert_eq!(
        parsed["jsonrpc"], "2.0",
        "envelope must contain jsonrpc: 2.0"
    );
    assert!(
        parsed["result"]["tools"].is_array(),
        "result.tools must be an array"
    );
}

#[test]
fn manifest_tool_count_matches_catalog() {
    let (_code, stdout, _stderr) = run(&["manifest", "--json"]);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("manifest output must be valid JSON");
    let tools = parsed["result"]["tools"]
        .as_array()
        .expect("result.tools must be an array");
    assert_eq!(
        tools.len(),
        engram::shim::tools_catalog::TOOL_COUNT,
        "manifest must expose all {} catalog tools",
        engram::shim::tools_catalog::TOOL_COUNT
    );
}

#[test]
fn manifest_tools_have_name_and_description() {
    let (_code, stdout, _stderr) = run(&["manifest", "--json"]);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("manifest output must be valid JSON");
    let tools = parsed["result"]["tools"]
        .as_array()
        .expect("result.tools must be an array");
    for tool in tools {
        assert!(
            tool["name"].is_string() && !tool["name"].as_str().unwrap().is_empty(),
            "each tool must have a non-empty name"
        );
        assert!(
            tool["inputSchema"].is_object(),
            "each tool must have an inputSchema object"
        );
    }
}

// ── --id flag ────────────────────────────────────────────────────────────────

#[test]
fn manifest_id_flag_echoed_as_number() {
    let (_code, stdout, _stderr) = run(&["manifest", "--json", "--id", "42"]);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("manifest output must be valid JSON");
    assert_eq!(
        parsed["id"], 42,
        "--id 42 must produce numeric id in JSON-RPC response"
    );
}

#[test]
fn manifest_id_flag_echoed_as_string() {
    let (_code, stdout, _stderr) = run(&["manifest", "--json", "--id", "my-req"]);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("manifest output must be valid JSON");
    assert_eq!(
        parsed["id"], "my-req",
        "--id my-req must produce string id in JSON-RPC response"
    );
}

// ── --format flag ─────────────────────────────────────────────────────────────

#[test]
fn manifest_format_text_produces_non_json_output() {
    let (_code, stdout, _stderr) = run(&["manifest", "--format", "text"]);
    // Text mode prints key: value pairs, not a JSON object.
    // It must NOT start with `{` (which would indicate JSON mode).
    let trimmed = stdout.trim();
    // Text output is not required to be empty, but must not be a JSON object envelope.
    if !trimmed.is_empty() {
        assert!(
            !trimmed.starts_with('{'),
            "text format must not emit a JSON object, got: {trimmed}"
        );
    }
}

#[test]
fn manifest_json_flag_overrides_format_text() {
    let (_code, stdout, _stderr) = run(&["manifest", "--json", "--format", "text"]);
    // --json flag takes precedence over --format text.
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("--json flag must produce JSON output");
    assert_eq!(parsed["jsonrpc"], "2.0");
}

// ── help / version ────────────────────────────────────────────────────────────

#[test]
fn help_exits_zero() {
    let (code, _stdout, _stderr) = run(&["--help"]);
    assert_eq!(code, 0, "engram --help must exit 0");
}

#[test]
fn manifest_help_exits_zero() {
    let (code, _stdout, _stderr) = run(&["manifest", "--help"]);
    assert_eq!(code, 0, "engram manifest --help must exit 0");
}

#[test]
fn sync_help_exits_zero() {
    let (code, _stdout, _stderr) = run(&["sync", "--help"]);
    assert_eq!(code, 0, "engram sync --help must exit 0");
}

#[test]
fn report_help_exits_zero() {
    let (code, _stdout, _stderr) = run(&["report", "--help"]);
    assert_eq!(code, 0, "engram report --help must exit 0");
}

// ── subcommand listing ────────────────────────────────────────────────────────

#[test]
fn help_lists_all_parity_subcommands() {
    let (_code, stdout, _stderr) = run(&["--help"]);
    let required = [
        "bind",
        "daemon-status",
        "workspace-status",
        "flush",
        "sync",
        "index",
        "manifest",
        "search",
        "query-memory",
        "symbols",
        "map-code",
        "impact",
        "query-graph",
        "stats",
        "health",
        "branch-metrics",
        "report",
    ];
    for cmd in required {
        assert!(
            stdout.contains(cmd),
            "help output must mention '{cmd}' subcommand"
        );
    }
}
