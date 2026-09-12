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

fn declared_stdio_mcp_names() -> Vec<String> {
    capabilities::surface_names(ToolSurface::StdioMcp)
        .into_iter()
        .map(str::to_owned)
        .collect()
}

#[test]
fn the_catalog_membership_equals_the_declared_stdio_mcp_surface() {
    let declared: BTreeSet<String> = declared_stdio_mcp_names().into_iter().collect();
    let advertised: BTreeSet<String> = catalog_names().into_iter().collect();

    assert_eq!(
        advertised, declared,
        "the MCP catalog and the descriptor registry must describe the same set of tools"
    );
}

#[test]
fn the_catalog_preserves_declaration_order() {
    // Order is derived, not curated. Asserting it keeps the derivation honest:
    // a catalog that merely happened to contain the right names while being
    // built from its own list would pass a set comparison but fail this.
    let declared = declared_stdio_mcp_names();

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
        declared_stdio_mcp_names().len(),
        TOOL_COUNT,
        "TOOL_COUNT must track the declared stdio-MCP surface"
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
