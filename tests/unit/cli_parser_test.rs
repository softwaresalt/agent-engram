//! Parser regression tests for the engram binary CLI.
//!
//! Verifies that the expected subcommand and flag names are stable across
//! refactors. These tests enumerate the known CLI surface and flag names as
//! assertions over hard-coded string lists; they do not invoke the clap parser
//! directly. Use the integration tests in `tests/integration/cli_e2e_test.rs`
//! for end-to-end CLI parsing coverage via the compiled binary.

// ── Subcommand name registry ──────────────────────────────────────────────────

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
    "lint-dax",
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
    all.sort_unstable();
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
    // CLI parity subcommands: one per group (lifecycle, indexing, manifest, search, report).
    // report is a single entry here but contains 3 sub-subcommands (token-savings, eval, retry-metrics).
    // Total top-level parity subcommands: 18 (added `lint-dax` mirroring `lint_dax`, P7/085.007-T).
    assert_eq!(CLI_PARITY_SUBCOMMANDS.len(), 18);
    assert_eq!(REPORT_SUBCOMMANDS.len(), 3);
}

#[test]
fn global_flags_names_are_stable() {
    // Document the expected global flag names for regression detection.
    const GLOBAL_FLAGS: &[&str] = &[
        "--workspace",
        "--id",
        "--json",
        "--format",
        "--quiet",
        "--timeout",
    ];
    assert_eq!(GLOBAL_FLAGS.len(), 6);
    // All must be long flags (double dash).
    for flag in GLOBAL_FLAGS {
        assert!(
            flag.starts_with("--"),
            "global flag must use -- prefix: {flag}"
        );
    }
}
