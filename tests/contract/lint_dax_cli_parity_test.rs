//! Contract — bounded CLI ↔ MCP parity guard for `lint_dax` / `lint-dax`
//! (P7, `085.007-T`).
//!
//! The `engram lint-dax` subcommand mirrors the `lint_dax` MCP tool. This test
//! pins that mirror and guards against silent drift **without** absorbing the
//! broader full-surface CLI ↔ MCP parity audit (stash `30F372C8`).
//!
//! The guard is deliberately bounded:
//! - [`CLI_MCP_MIRROR`] enumerates every tool in the compile-time
//!   [`tools_catalog::all_tools`] catalog paired with the `engram` CLI surface
//!   that mirrors it. Two catalog tools legitimately share one CLI surface: the
//!   daemon-only `get_retrieval_eval_report` has no dedicated subcommand and is
//!   mirrored onto the same `eval` retrieval-eval surface as `run_retrieval_eval`.
//! - [`PARITY_ALLOWLIST`] records the 5 AC-fixed, pre-existing gaps that are NOT
//!   in the default catalog and must never trip this guard.
//!
//! Together these let the guard fail loudly when a *new* catalog tool is added
//! without either a CLI mirror or an explicit allowlist entry, while staying
//! scoped to the `lint_dax` deliverable.

use engram::shim::tools_catalog;

/// Every catalog (`all_tools()`) MCP tool paired with its mirroring `engram`
/// CLI surface. The CLI surface is the subcommand path (`report eval` is the
/// `report` subcommand's `eval` child). All 21 catalog tools are accounted for.
const CLI_MCP_MIRROR: &[(&str, &str)] = &[
    ("lint_dax", "lint-dax"),
    ("set_workspace", "bind"),
    ("get_daemon_status", "daemon-status"),
    ("get_workspace_status", "workspace-status"),
    ("flush_state", "flush"),
    ("query_memory", "query-memory"),
    ("get_workspace_statistics", "stats"),
    ("index_workspace", "index"),
    ("sync_workspace", "sync"),
    ("map_code", "map-code"),
    ("list_symbols", "symbols"),
    ("unified_search", "search"),
    ("impact_analysis", "impact"),
    ("get_health_report", "health"),
    ("get_branch_metrics", "branch-metrics"),
    ("get_token_savings_report", "report token-savings"),
    ("get_evaluation_report", "report eval"),
    ("run_retrieval_eval", "eval"),
    // Daemon-only report accessor: no dedicated CLI subcommand; shares the
    // `eval` retrieval-eval surface with `run_retrieval_eval`.
    ("get_retrieval_eval_report", "eval"),
    ("query_graph", "query-graph"),
    ("get_mutable_script_retry_metrics", "report retry-metrics"),
];

/// The 5 AC-fixed, known pre-existing CLI ↔ MCP parity gaps. These names are NOT
/// in the default `all_tools()` catalog (git-graph-gated dispatch-only or
/// removed legacy task tools) and must never trip the bounded drift guard. The
/// broader parity feature (`30F372C8`) owns closing these; this guard only
/// documents and pins them so `lint_dax` parity work does not silently absorb
/// or regress them.
const PARITY_ALLOWLIST: &[&str] = &[
    "query_graph_neighborhood",
    "create_task",
    "update_task",
    "query_changes",
    "index_git_history",
];

/// Test A (focused) — the `lint_dax` ↔ `lint-dax` mirror is present and correct.
///
/// Directly exercises the P7 deliverable: `lint_dax` is a real catalog tool, the
/// `("lint_dax", "lint-dax")` pair is pinned in the mirror table, and `lint_dax`
/// is deliberately NOT allowlisted (it must be mirrored, not waived).
#[test]
fn lint_dax_is_mirrored_by_lint_dax_cli() {
    let tools = tools_catalog::all_tools();
    let catalog: std::collections::HashSet<&str> = tools.iter().map(|t| t.name.as_ref()).collect();

    assert!(
        catalog.contains("lint_dax"),
        "lint_dax must be a registered catalog tool (P6/085.006-T)"
    );

    let mirror = CLI_MCP_MIRROR
        .iter()
        .find(|(mcp, _)| *mcp == "lint_dax")
        .expect("CLI_MCP_MIRROR must pin the lint_dax mirror");
    assert_eq!(
        mirror.1, "lint-dax",
        "lint_dax must mirror the `engram lint-dax` CLI subcommand"
    );

    assert!(
        !PARITY_ALLOWLIST.contains(&"lint_dax"),
        "lint_dax must be mirrored, never allowlisted as a parity gap"
    );
}

/// Test B (bounded drift) — every catalog tool is either mirrored or allowlisted.
///
/// Adding a new `all_tools()` tool without a `CLI_MCP_MIRROR` entry or a
/// `PARITY_ALLOWLIST` entry fails here, catching CLI ↔ MCP drift the moment it
/// is introduced — without auditing the full surface beyond the catalog.
#[test]
fn every_catalog_tool_is_mirrored_or_allowlisted() {
    let mirrored: std::collections::HashSet<&str> =
        CLI_MCP_MIRROR.iter().map(|(mcp, _)| *mcp).collect();
    let allowlisted: std::collections::HashSet<&str> = PARITY_ALLOWLIST.iter().copied().collect();

    for tool in tools_catalog::all_tools() {
        let name = tool.name.as_ref();
        assert!(
            mirrored.contains(name) || allowlisted.contains(name),
            "catalog tool '{name}' has no CLI mirror and is not allowlisted — \
             add a CLI_MCP_MIRROR entry or (for a deliberate pre-existing gap) a \
             PARITY_ALLOWLIST entry"
        );
    }
}

/// Test B' — the mirror table stays in lockstep with the catalog.
///
/// Every mirror-table MCP name must be a real catalog tool, so the table can
/// never drift ahead of (or reference tools absent from) `all_tools()`.
#[test]
fn mirror_table_only_references_catalog_tools() {
    let tools = tools_catalog::all_tools();
    let catalog: std::collections::HashSet<&str> = tools.iter().map(|t| t.name.as_ref()).collect();

    for (mcp, _cli) in CLI_MCP_MIRROR {
        assert!(
            catalog.contains(mcp),
            "CLI_MCP_MIRROR references '{mcp}', which is not a catalog tool"
        );
    }
}

/// Test C — the allowlist is disjoint from the mirror targets and out-of-catalog.
///
/// Exercises the allowlist itself: an allowlisted name must never shadow a real
/// mirror (disjoint from mirror MCP targets) and must genuinely be a
/// pre-existing gap (absent from `all_tools()`), so the allowlist can only ever
/// waive tools that truly lack a CLI surface.
#[test]
fn allowlist_is_disjoint_and_out_of_catalog() {
    let mirrored: std::collections::HashSet<&str> =
        CLI_MCP_MIRROR.iter().map(|(mcp, _)| *mcp).collect();
    let tools = tools_catalog::all_tools();
    let catalog: std::collections::HashSet<&str> = tools.iter().map(|t| t.name.as_ref()).collect();

    for gap in PARITY_ALLOWLIST {
        assert!(
            !mirrored.contains(gap),
            "allowlisted gap '{gap}' must not also be a mirror target"
        );
        assert!(
            !catalog.contains(gap),
            "allowlisted gap '{gap}' must be a genuine out-of-catalog parity gap"
        );
    }
}
