//! Contract coverage for CLI workflow surface derivation (plan unit F23,
//! 142.032-T).
//!
//! The CLI's surface used to be implicit: it was whatever method-name string
//! literals the command modules passed to `run_tool`. That made drift
//! invisible — a method retired from the registry stayed callable from the CLI
//! until a user tripped over it. These assertions make the registry the only
//! decider, and scrape the real call sites so the check cannot be satisfied by
//! a second hand-maintained list that happens to agree.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use engram::cli::runner::{cli_workflow_methods, is_cli_workflow_method};
use engram::tools::capabilities::{self, ToolSurface};

fn cli_command_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("cli")
}

/// Method names actually passed to `run_tool*` anywhere under `src/cli/`.
///
/// Scraping the sources rather than restating them is the whole point: the
/// test compares the code's real behaviour against the registry, not one list
/// against another.
fn invoked_method_names() -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut stack = vec![cli_command_dir()];

    while let Some(dir) = stack.pop() {
        let entries = std::fs::read_dir(&dir)
            .unwrap_or_else(|error| panic!("read {}: {error}", dir.display()));
        for entry in entries {
            let entry = entry.expect("directory entry");
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().is_none_or(|ext| ext != "rs") {
                continue;
            }
            let source = std::fs::read_to_string(&path)
                .unwrap_or_else(|_| panic!("read {}", path.display()));
            collect_invocations(&source, &mut names);
        }
    }

    names
}

/// Pull the first string literal following each `run_tool…(` call.
fn collect_invocations(source: &str, names: &mut BTreeSet<String>) {
    let mut rest = source;
    while let Some(offset) = rest.find("run_tool") {
        rest = &rest[offset + "run_tool".len()..];
        let Some(open) = rest.find('(') else { break };
        // Only a call site: anything other than an identifier tail before the
        // paren means this was a different token (a doc reference, an import).
        if !rest[..open]
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            continue;
        }
        let tail = &rest[open + 1..];
        let Some(quote) = tail.find('"') else {
            continue;
        };
        // The literal must be the first argument: reject a match that crossed
        // into a later call.
        if tail[..quote].contains(')') {
            continue;
        }
        let literal = &tail[quote + 1..];
        let Some(end) = literal.find('"') else {
            continue;
        };
        names.insert(literal[..end].to_owned());
    }
}

#[test]
fn the_cli_surface_equals_the_declared_cli_descriptors() {
    let declared: Vec<&str> = capabilities::surface_names(ToolSurface::Cli);
    assert!(
        !declared.is_empty(),
        "the descriptor registry must declare a CLI surface"
    );
    assert_eq!(
        cli_workflow_methods(),
        declared,
        "the CLI workflow surface must be the descriptor registry's CLI surface, in order"
    );
}

#[test]
fn every_method_the_cli_invokes_is_declared_on_the_cli_surface() {
    let invoked = invoked_method_names();
    assert!(
        !invoked.is_empty(),
        "the scraper found no run_tool call sites; the check would be vacuous"
    );

    let undeclared: Vec<&String> = invoked
        .iter()
        .filter(|name| !is_cli_workflow_method(name))
        .collect();

    assert!(
        undeclared.is_empty(),
        "these methods are invoked by the CLI but are not declared on the CLI surface: \
         {undeclared:?}"
    );
}

#[test]
fn the_scraper_found_the_call_sites_it_is_meant_to_guard() {
    // Anchor the scraper itself. A regex that silently stops matching would
    // make the previous test pass for the wrong reason.
    let invoked = invoked_method_names();
    for expected in [
        "set_workspace",
        "get_daemon_status",
        "unified_search",
        "index_workspace",
        "query_graph",
    ] {
        assert!(
            invoked.contains(expected),
            "scraper missed the '{expected}' call site; the drift guard is not actually running"
        );
    }
}

#[test]
fn an_undeclared_method_is_refused_rather_than_sent_to_the_daemon() {
    assert!(
        !is_cli_workflow_method("definitely_not_a_cli_method"),
        "an undeclared method must not be accepted by the CLI surface gate"
    );
    assert!(
        is_cli_workflow_method("unified_search"),
        "a declared CLI method must be accepted"
    );
}

#[test]
fn the_cli_surface_is_a_strict_subset_of_the_declared_registry() {
    let all: BTreeSet<&str> = capabilities::declared_names().into_iter().collect();
    for name in cli_workflow_methods() {
        assert!(
            all.contains(name),
            "CLI method '{name}' is not present in the descriptor registry at all"
        );
    }
}

#[test]
fn cli_and_stdio_mcp_surfaces_are_derived_independently() {
    // Both surfaces derive from the same registry but must not be forced to be
    // identical: the CLI legitimately exposes workflows the MCP catalog does
    // not, and conflating them would hide a real divergence.
    let cli: BTreeSet<&str> = cli_workflow_methods().into_iter().collect();
    let mcp: BTreeSet<&str> = capabilities::surface_names(ToolSurface::StdioMcp)
        .into_iter()
        .collect();

    assert!(!cli.is_empty() && !mcp.is_empty());
    assert!(
        cli.intersection(&mcp).next().is_some(),
        "the two surfaces must share the common tool methods"
    );
}
