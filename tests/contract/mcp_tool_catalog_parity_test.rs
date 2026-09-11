//! Contract coverage for stdio MCP catalog derivation (plan unit F22,
//! 142.031-T).
//!
//! The catalog the shim advertises over `tools/list` must be *derived* from
//! the F19 descriptor registry, not maintained beside it. These assertions
//! pin that derivation: if the two surfaces are ever allowed to disagree, the
//! disagreement shows up here rather than as a tool an agent can see but not
//! call (or can call but never sees).

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

use engram::shim::tools_catalog::{TOOL_COUNT, all_tools};
use engram::tools::capabilities::{self, ToolSurface};

fn catalog_names() -> Vec<String> {
    all_tools()
        .into_iter()
        .map(|tool| tool.name.to_string())
        .collect()
}

/// Tools that `src/shim/tools_catalog.rs`'s module doc comment documents as
/// *intentionally* excluded from the derived MCP catalog even though they
/// declare the `StdioMcp` surface: the `git-graph`-feature-gated dispatch
/// tools use a local/direct schema source (`SchemaSource::Local`) instead of
/// a catalog literal, so `catalog_entries()` has no entry for them by design.
///
/// Under the default feature set neither name is compiled in at all, so this
/// allowlist is a no-op there and the parity checks below still assert real,
/// unconditional equality for every tool that *is* present. It only becomes
/// load-bearing under `cargo test --features git-graph` (or `--all-features`),
/// where `capabilities::surface_names(StdioMcp)` grows to include these two
/// names but the catalog correctly continues to omit them.
const EXCLUDED_FROM_CATALOG: &[&str] = &["query_changes", "index_git_history"];

fn declared_stdio_mcp_names() -> Vec<String> {
    capabilities::surface_names(ToolSurface::StdioMcp)
        .into_iter()
        .map(str::to_owned)
        .collect()
}

fn declared_catalog_eligible_names() -> Vec<String> {
    declared_stdio_mcp_names()
        .into_iter()
        .filter(|name| !EXCLUDED_FROM_CATALOG.contains(&name.as_str()))
        .collect()
}

#[test]
fn the_catalog_membership_equals_the_declared_stdio_mcp_surface() {
    let declared: BTreeSet<String> = declared_catalog_eligible_names().into_iter().collect();
    let advertised: BTreeSet<String> = catalog_names().into_iter().collect();

    assert_eq!(
        advertised, declared,
        "the MCP catalog and the descriptor registry must describe the same set of tools \
         (excluding the deliberately-excluded git-graph tools: {EXCLUDED_FROM_CATALOG:?})"
    );
}

#[test]
fn the_catalog_never_advertises_a_deliberately_excluded_git_graph_tool() {
    // Guards the other direction of the allowlist: if `catalog_entries()` ever
    // grows a literal for one of these names, this fails loudly instead of
    // the allowlist silently absorbing an accidental re-inclusion.
    let advertised: BTreeSet<String> = catalog_names().into_iter().collect();
    for excluded in EXCLUDED_FROM_CATALOG {
        assert!(
            !advertised.contains(*excluded),
            "'{excluded}' is documented as excluded from the MCP catalog but is advertised anyway"
        );
    }
}

#[test]
fn the_catalog_preserves_declaration_order() {
    // Order is derived, not curated. Asserting it keeps the derivation honest:
    // a catalog that merely happened to contain the right names while being
    // built from its own list would pass a set comparison but fail this.
    let declared = declared_catalog_eligible_names();

    assert_eq!(
        catalog_names(),
        declared,
        "the catalog must be emitted in descriptor declaration order"
    );
}

#[test]
fn tool_count_matches_the_derived_catalog() {
    assert_eq!(
        all_tools().len(),
        TOOL_COUNT,
        "TOOL_COUNT must track the derived catalog length"
    );
    assert_eq!(
        declared_catalog_eligible_names().len(),
        TOOL_COUNT,
        "TOOL_COUNT must track the declared stdio-MCP surface, excluding the \
         deliberately-excluded git-graph tools: {EXCLUDED_FROM_CATALOG:?}"
    );
}

#[test]
fn every_advertised_tool_carries_a_usable_schema_and_description() {
    for tool in all_tools() {
        let schema = tool.input_schema.as_ref();
        assert!(
            !schema.is_empty(),
            "tool '{}' is advertised with an empty input schema",
            tool.name
        );
        assert!(
            schema.contains_key("type"),
            "tool '{}' input schema declares no JSON Schema type",
            tool.name
        );
        let description = tool.description.as_deref().unwrap_or_default();
        assert!(
            !description.trim().is_empty(),
            "tool '{}' is advertised with no description",
            tool.name
        );
    }
}

#[test]
fn no_tool_is_advertised_twice() {
    let names = catalog_names();
    let unique: BTreeSet<&String> = names.iter().collect();
    assert_eq!(
        unique.len(),
        names.len(),
        "the derived catalog must not advertise a duplicate tool name"
    );
}

#[test]
fn every_advertised_tool_resolves_back_to_a_descriptor() {
    // The round trip is what makes "cannot drift" literal: an advertised name
    // with no descriptor would be a tool the capability gate has never
    // classified.
    for tool in all_tools() {
        let descriptor = capabilities::descriptor(&tool.name)
            .unwrap_or_else(|| panic!("advertised tool '{}' has no descriptor", tool.name));
        assert!(
            descriptor.supports(ToolSurface::StdioMcp),
            "advertised tool '{}' does not declare the stdio MCP surface",
            tool.name
        );
    }
}
