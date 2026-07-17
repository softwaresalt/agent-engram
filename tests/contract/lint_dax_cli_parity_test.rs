//! Contract — canonical CLI ↔ MCP parity drift guard (090.002-T).
//!
//! The canonical mapping lives in `docs/cli-mcp-parity.md`. These tests compare
//! that table against the real MCP catalog and the real Clap help surface so new
//! tools or subcommands cannot drift silently.

use std::collections::{BTreeMap, BTreeSet};
use std::process::Command;

use engram::shim::tools_catalog;

const DOC_PATH: &str = "docs/cli-mcp-parity.md";
/// Externally resolvable canonical parity-doc URL that CLI help and MCP tool
/// descriptions must reference. The released binary is installed globally and
/// runs against arbitrary workspaces, so a repository-relative path would not
/// resolve for MCP clients or operators — the reference must be a stable URL.
const DOC_URL: &str =
    "https://github.com/softwaresalt/agent-engram/blob/main/docs/cli-mcp-parity.md";
const PARITY_DOC: &str = include_str!("../../docs/cli-mcp-parity.md");
const TOOLS_MOD: &str = include_str!("../../src/tools/mod.rs");
const MCP_WITHOUT_CLI_ALLOWLIST: &[&str] = &[
    "get_retrieval_eval_report",
    "query_changes",
    "index_git_history",
];
const LOCAL_CLI_ALLOWLIST: &[&str] = &[
    "shim",
    "daemon",
    "install",
    "update",
    "reinstall",
    "uninstall",
    "manifest",
    "verify",
    "migrate-down",
];

#[derive(Debug)]
struct MappingRow {
    mcp_tool: String,
    cli_command: String,
    surface: String,
    notes: String,
}

#[must_use]
fn clean_cell(cell: &str) -> String {
    cell.replace('`', "").trim().to_owned()
}

#[must_use]
fn parse_mapping_rows() -> Vec<MappingRow> {
    let mut rows = Vec::new();
    let mut in_table = false;

    for line in PARITY_DOC.lines() {
        if line == "| MCP tool | CLI command | daemon/local | notes |" {
            in_table = true;
            continue;
        }
        if !in_table {
            continue;
        }
        if !line.starts_with('|') || line.trim().is_empty() {
            break;
        }
        if line.starts_with("|---") {
            continue;
        }

        let cells = line
            .trim()
            .trim_matches('|')
            .split('|')
            .map(clean_cell)
            .collect::<Vec<_>>();
        assert_eq!(
            cells.len(),
            4,
            "mapping table rows must have exactly four cells: {line}"
        );
        rows.push(MappingRow {
            mcp_tool: cells[0].clone(),
            cli_command: cells[1].clone(),
            surface: cells[2].clone(),
            notes: cells[3].clone(),
        });
    }

    assert!(
        !rows.is_empty(),
        "canonical mapping table must not be empty"
    );
    let mut seen_mcp = BTreeSet::new();
    for row in rows.iter().filter(|row| row.mcp_tool != "-") {
        assert!(
            seen_mcp.insert(row.mcp_tool.as_str()),
            "duplicate MCP mapping row for '{}' in {DOC_PATH}",
            row.mcp_tool
        );
    }
    rows
}

#[must_use]
fn command_path(cli_command: &str) -> Option<Vec<&str>> {
    if cli_command == "-" {
        return None;
    }
    let rest = cli_command.strip_prefix("engram ")?;
    let path = rest
        .split_whitespace()
        .take_while(|part| !part.starts_with('<') && !part.starts_with('['))
        .collect::<Vec<_>>();
    if path.is_empty() { None } else { Some(path) }
}

#[must_use]
fn primary_cli_surface(cli_command: &str) -> Option<String> {
    command_path(cli_command).map(|parts| format!("engram {}", parts.join(" ")))
}

#[must_use]
fn dispatch_pattern_literals(line: &str) -> Vec<String> {
    let trimmed = line.trim();
    let Some((patterns, _handler)) = trimmed.split_once("=>") else {
        return Vec::new();
    };
    patterns
        .split('|')
        .filter_map(|pattern| {
            let pattern = pattern.trim();
            pattern
                .strip_prefix('"')
                .and_then(|rest| rest.split_once('"'))
                .map(|(name, _)| name.to_owned())
        })
        .collect()
}

// Line-based scan of the `match method` dispatch block. It relies on each arm
// keeping its tool literal on the same line as `=>` (the current one-tool-per-arm
// style). Any future under-count (for example a rustfmt-wrapped alternation arm)
// or a stale catalog entry is caught loudly by `dispatch_table_is_superset_of_catalog`,
// which uses the compiler-checked `tools_catalog::all_tools()` set as an oracle.
#[must_use]
fn dispatch_tool_names() -> BTreeSet<String> {
    let dispatch_start = TOOLS_MOD
        .find("let result = match method {")
        .expect("dispatch match must exist");
    let dispatch_tail = &TOOLS_MOD[dispatch_start..];
    let dispatch_end = dispatch_tail
        .find("// Record latency")
        .expect("dispatch match must end before latency recording");
    dispatch_tail[..dispatch_end]
        .lines()
        .flat_map(dispatch_pattern_literals)
        .collect()
}

#[test]
fn dispatch_pattern_parser_collects_alternative_literals() {
    let names = dispatch_pattern_literals(r#""old_tool" | "new_tool" => handler(params)"#);
    assert_eq!(
        names,
        vec!["old_tool".to_owned(), "new_tool".to_owned()],
        "dispatch parser must collect every literal in an alternation arm"
    );
}

#[must_use]
fn run_cli(args: &[&str]) -> (i32, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_engram"))
        .args(args)
        .env_remove("ENGRAM_DATA_DIR")
        .output()
        .expect("engram CLI must execute");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

#[must_use]
fn help_command_names(args: &[&str]) -> BTreeSet<String> {
    let (code, stdout, stderr) = run_cli(args);
    assert_eq!(code, 0, "help command must exit 0; stderr:\n{stderr}");

    let mut names = BTreeSet::new();
    let mut in_commands = false;
    for line in stdout.lines() {
        let trimmed = line.trim_start();
        if trimmed == "Commands:" {
            in_commands = true;
            continue;
        }
        if !in_commands || trimmed.is_empty() {
            continue;
        }
        if !line.starts_with("  ") {
            if !names.is_empty() {
                break;
            }
            continue;
        }
        let Some(name) = trimmed.split_whitespace().next() else {
            continue;
        };
        if name != "help" {
            names.insert(name.to_owned());
        }
    }

    assert!(
        !names.is_empty(),
        "help output must include a Commands section; stdout:\n{stdout}"
    );
    names
}

#[test]
fn every_default_catalog_tool_is_documented_as_mapped_or_gap() {
    let rows = parse_mapping_rows();
    let by_mcp = rows
        .iter()
        .filter(|row| row.mcp_tool != "-")
        .map(|row| (row.mcp_tool.as_str(), row))
        .collect::<BTreeMap<_, _>>();

    for tool in tools_catalog::all_tools() {
        let name = tool.name.as_ref();
        let row = by_mcp
            .get(name)
            .unwrap_or_else(|| panic!("catalog tool '{name}' is missing from {DOC_PATH}"));
        assert_eq!(
            row.surface, "daemon",
            "catalog tool '{name}' must be marked daemon-backed in {DOC_PATH}"
        );
        if row.cli_command == "-" {
            assert!(
                MCP_WITHOUT_CLI_ALLOWLIST.contains(&name),
                "MCP-only row for '{name}' must be explicitly allowlisted in the drift guard"
            );
            assert!(
                row.notes.contains("gap") || row.notes.contains("MCP-only"),
                "MCP-only row for '{name}' must include a gap rationale"
            );
        } else {
            assert!(
                row.cli_command.starts_with("engram "),
                "mapped catalog tool '{name}' must use an engram CLI command"
            );
        }
    }
}

#[test]
fn documented_mcp_rows_match_catalog_or_dispatch_tools() {
    let catalog = tools_catalog::all_tools()
        .iter()
        .map(|tool| tool.name.to_string())
        .collect::<BTreeSet<_>>();
    let dispatch = dispatch_tool_names();

    for row in parse_mapping_rows()
        .iter()
        .filter(|row| row.mcp_tool != "-")
    {
        assert!(
            catalog.contains(&row.mcp_tool) || dispatch.contains(&row.mcp_tool),
            "documented MCP tool '{}' is neither in the catalog nor the dispatch table",
            row.mcp_tool
        );
    }
}

/// The text-scanned dispatch set must remain a superset of the structured runtime
/// catalog. `tools_catalog::all_tools()` is a compiler-checked source of tool names,
/// so it acts as an oracle: if the line-based dispatch parser ever under-counts (for
/// example a future rustfmt-wrapped alternation arm drops a literal) or a catalog
/// entry becomes stale after its dispatch arm is removed, this test fails loudly
/// instead of silently passing. Feature-gated dispatch-only tools (such as
/// `query_changes`/`index_git_history` under `git-graph`) are permitted because the
/// subset direction only constrains catalog ⊆ dispatch, never the reverse.
#[test]
fn dispatch_table_is_superset_of_catalog() {
    let dispatch = dispatch_tool_names();
    let missing = tools_catalog::all_tools()
        .iter()
        .map(|tool| tool.name.to_string())
        .filter(|name| !dispatch.contains(name))
        .collect::<Vec<_>>();
    assert!(
        missing.is_empty(),
        "catalog tools missing from the parsed dispatch table \
         (parser under-count or stale catalog entry): {missing:?}"
    );
}

#[test]
fn every_dispatch_tool_is_documented_as_mapped_or_allowlisted_gap() {
    let rows = parse_mapping_rows();
    let by_mcp = rows
        .iter()
        .filter(|row| row.mcp_tool != "-")
        .map(|row| (row.mcp_tool.as_str(), row))
        .collect::<BTreeMap<_, _>>();

    for name in dispatch_tool_names() {
        let row = by_mcp
            .get(name.as_str())
            .unwrap_or_else(|| panic!("dispatch tool '{name}' is missing from {DOC_PATH}"));
        if row.cli_command == "-" {
            assert!(
                MCP_WITHOUT_CLI_ALLOWLIST.contains(&name.as_str()),
                "dispatch tool '{name}' has no CLI command but is not explicitly allowlisted"
            );
            assert_eq!(
                row.surface, "daemon",
                "dispatch tool '{name}' must be documented as daemon-backed"
            );
            assert!(
                row.notes.contains("gap") || row.notes.contains("MCP-only"),
                "dispatch tool '{name}' gap row must keep its rationale"
            );
        } else {
            assert!(
                row.cli_command.starts_with("engram "),
                "dispatch tool '{name}' must map to an engram CLI command"
            );
        }
    }
}

#[test]
fn mcp_gap_allowlist_matches_documented_gap_rows() {
    let rows = parse_mapping_rows();
    let by_mcp = rows
        .iter()
        .filter(|row| row.mcp_tool != "-")
        .map(|row| (row.mcp_tool.as_str(), row))
        .collect::<BTreeMap<_, _>>();

    for gap in MCP_WITHOUT_CLI_ALLOWLIST {
        let row = by_mcp.get(gap).unwrap_or_else(|| {
            panic!("MCP gap allowlist entry '{gap}' is missing from {DOC_PATH}")
        });
        assert_eq!(
            row.cli_command, "-",
            "MCP gap allowlist entry '{gap}' must remain documented as MCP-only"
        );
    }
}

#[test]
fn every_real_cli_command_is_documented_as_mapped_or_local() {
    let rows = parse_mapping_rows();
    let documented_top_level = rows
        .iter()
        .filter_map(|row| command_path(&row.cli_command))
        .filter_map(|path| path.first().map(|part| (*part).to_owned()))
        .collect::<BTreeSet<_>>();

    for command in help_command_names(&["--help"]) {
        assert!(
            documented_top_level.contains(&command),
            "top-level CLI command '{command}' is missing from {DOC_PATH}"
        );
    }

    let documented_report_children = rows
        .iter()
        .filter_map(|row| command_path(&row.cli_command))
        .filter(|path| path.first().is_some_and(|part| *part == "report"))
        .filter_map(|path| path.get(1).map(|part| (*part).to_owned()))
        .collect::<BTreeSet<_>>();

    for command in help_command_names(&["report", "--help"]) {
        assert!(
            documented_report_children.contains(&command),
            "report CLI subcommand '{command}' is missing from {DOC_PATH}"
        );
    }
}

#[test]
fn every_documented_cli_command_resolves_to_help() {
    for row in parse_mapping_rows() {
        let Some(path) = command_path(&row.cli_command) else {
            continue;
        };
        let mut args = path;
        args.push("--help");
        let (code, _stdout, stderr) = run_cli(&args);
        assert_eq!(
            code, 0,
            "documented CLI command '{}' must resolve with --help; stderr:\n{stderr}",
            row.cli_command
        );
    }
}

#[test]
fn local_cli_only_rows_are_explicitly_allowlisted_with_rationale() {
    for row in parse_mapping_rows()
        .iter()
        .filter(|row| row.mcp_tool == "-" && row.cli_command != "-")
    {
        assert_eq!(
            row.surface, "local",
            "CLI-only command '{}' must be marked local",
            row.cli_command
        );
        let Some(path) = command_path(&row.cli_command) else {
            panic!(
                "CLI-only command '{}' must include an engram command",
                row.cli_command
            );
        };
        let command_key = path.join(" ");
        assert!(
            LOCAL_CLI_ALLOWLIST.contains(&command_key.as_str()),
            "CLI-only command '{}' must be explicitly allowlisted in the drift guard",
            row.cli_command
        );
        let rationale = row.notes.to_lowercase();
        assert!(
            rationale.contains("local")
                || rationale.contains("internal")
                || rationale.contains("operator"),
            "CLI-only command '{}' must include a local/internal rationale",
            row.cli_command
        );
    }
}

#[test]
fn local_cli_allowlist_matches_documented_local_rows() {
    let documented = parse_mapping_rows()
        .iter()
        .filter(|row| row.mcp_tool == "-" && row.cli_command != "-")
        .filter_map(|row| command_path(&row.cli_command))
        .map(|path| path.join(" "))
        .collect::<BTreeSet<_>>();
    let allowlisted = LOCAL_CLI_ALLOWLIST
        .iter()
        .map(|entry| (*entry).to_owned())
        .collect::<BTreeSet<_>>();

    assert_eq!(
        documented, allowlisted,
        "local CLI allowlist must exactly match documented local-only rows"
    );
}

#[test]
fn cli_help_and_mcp_catalog_reference_canonical_doc() {
    let (code, stdout, stderr) = run_cli(&["--help"]);
    assert_eq!(code, 0, "engram --help must exit 0; stderr:\n{stderr}");
    assert!(
        stdout.contains(DOC_URL),
        "top-level CLI help must reference {DOC_URL}; stdout:\n{stdout}"
    );

    let rows = parse_mapping_rows();
    let by_mcp = rows
        .iter()
        .filter(|row| row.mcp_tool != "-")
        .map(|row| (row.mcp_tool.as_str(), row))
        .collect::<BTreeMap<_, _>>();

    for tool in tools_catalog::all_tools() {
        let name = tool.name.as_ref();
        let description = tool.description.as_deref().unwrap_or_default();
        assert!(
            description.contains(DOC_URL),
            "MCP tool '{name}' description must reference {DOC_URL}: {description}"
        );
        let row = by_mcp
            .get(name)
            .unwrap_or_else(|| panic!("catalog tool '{name}' is missing from {DOC_PATH}"));
        if let Some(cli_surface) = primary_cli_surface(&row.cli_command) {
            assert!(
                description.contains(&cli_surface),
                "MCP tool '{name}' description must reference CLI surface '{cli_surface}': {description}"
            );
        } else {
            assert!(
                description.contains("MCP-only") || description.contains("daemon-only"),
                "MCP-only tool '{name}' must document that distinction: {description}"
            );
        }
    }
}
