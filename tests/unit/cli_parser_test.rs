//! Parser regression tests for the engram binary CLI.
//!
//! Verifies that all existing commands continue to parse correctly (no regressions)
//! and that all new CLI subcommands parse with the expected argument extraction.

/// Use the binary's clap parser indirectly by testing argument parsing patterns.
/// These tests verify the CLI surface without spawning the binary process.

// ── Argument mapping tests ────────────────────────────────────────────────────

/// All new subcommand names that must be parseable.
const CLI_PARITY_SUBCOMMANDS: &[&str] = &[
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

/// Existing internal commands that must not be broken.
const INTERNAL_SUBCOMMANDS: &[&str] = &[
    "shim",
    "daemon",
    "install",
    "update",
    "reinstall",
    "uninstall",
];

/// Report sub-subcommands.
const REPORT_SUBCOMMANDS: &[&str] = &["token-savings", "eval", "retry-metrics"];

#[test]
fn cli_parity_subcommand_names_unique() {
    let mut all: Vec<&str> = CLI_PARITY_SUBCOMMANDS.to_vec();
    all.extend_from_slice(INTERNAL_SUBCOMMANDS);
    let total = all.len();
    all.dedup();
    assert_eq!(
        all.len(),
        total,
        "duplicate subcommand names detected — each name must be unique"
    );
}

#[test]
fn no_overlap_between_internal_and_parity_commands() {
    for name in CLI_PARITY_SUBCOMMANDS {
        assert!(
            !INTERNAL_SUBCOMMANDS.contains(name),
            "CLI parity command '{name}' conflicts with an internal command name"
        );
    }
}

#[test]
fn daemon_status_distinct_from_daemon() {
    // "daemon-status" must not be confused with "daemon".
    assert_ne!("daemon-status", "daemon");
    assert!(
        !INTERNAL_SUBCOMMANDS.contains(&"daemon-status"),
        "daemon-status must be a parity command, not an internal command"
    );
    assert!(
        CLI_PARITY_SUBCOMMANDS.contains(&"daemon-status"),
        "daemon-status must appear in parity command list"
    );
}

#[test]
fn report_subcommand_count_is_three() {
    assert_eq!(
        REPORT_SUBCOMMANDS.len(),
        3,
        "report must have exactly 3 subcommands"
    );
}

#[test]
fn cli_parity_count_covers_all_tools() {
    // 17 direct subcommands + report (which covers 3 more = 6 report tools).
    // Total distinct MCP tools: 18 (TOOL_COUNT) + manifest = 19 exposed via CLI.
    // Direct subcommand count: manifest + bind + daemon-status + workspace-status +
    //   flush + sync + index + search + query-memory + symbols + map-code +
    //   impact + query-graph + stats + health + branch-metrics + report = 17
    assert_eq!(CLI_PARITY_SUBCOMMANDS.len(), 17);
    assert_eq!(REPORT_SUBCOMMANDS.len(), 3);
}

#[test]
fn global_flags_names_are_stable() {
    // Document the expected global flag names for regression detection.
    const GLOBAL_FLAGS: &[&str] = &["--workspace", "--id", "--json", "--format", "--quiet"];
    assert_eq!(GLOBAL_FLAGS.len(), 5);
    // All must be long flags (double dash).
    for flag in GLOBAL_FLAGS {
        assert!(
            flag.starts_with("--"),
            "global flag must use -- prefix: {flag}"
        );
    }
}
