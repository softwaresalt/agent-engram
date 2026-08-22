//! Independent, agent-visible MCP catalog oracle (Feature 127-F).
//!
//! This contract test validates the serialized `tools/list` catalog that an MCP
//! client actually receives against a HUMAN-AUTHORED declarative fixture
//! (`tests/fixtures/mcp_tool_catalog.expected.json`).
//!
//! THE INDEPENDENCE INVARIANT (the whole point):
//!
//! The oracle's expectations must NOT derive from the production catalog it
//! validates. This file therefore:
//!   * never imports the production catalog module,
//!   * never calls its enumeration constructor, and
//!   * obtains the observed catalog only through the agent-visible capture
//!     helper (subprocess MCP stdio), not from in-process Rust structs.
//!
//! A future refactor cannot quietly reconnect the oracle to the production
//! derivation path: the guard scripts (`scripts/check-oracle-independence.*`)
//! and the in-test `oracle_sources_are_independent_of_production_catalog`
//! assertion both mechanically enforce the absence of the forbidden tokens.

#[path = "../helpers/mcp_catalog_capture.rs"]
mod capture;

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

/// Location of the declarative, human-authored expectation fixture.
const FIXTURE_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/mcp_tool_catalog.expected.json"
);

/// A single tool entry reduced to the facets the oracle asserts.
#[derive(Debug, Clone)]
#[allow(dead_code)] // `name`/`input_schema` are consumed by the U4 schema oracle.
struct ToolEntry {
    name: String,
    description: String,
    input_schema: Value,
}

/// Load the declarative fixture and reduce it to a name-keyed map.
fn load_expected() -> BTreeMap<String, ToolEntry> {
    let raw = std::fs::read_to_string(FIXTURE_PATH).unwrap_or_else(|error| {
        panic!("expectation fixture must exist at {FIXTURE_PATH}: {error}")
    });
    let doc: Value =
        serde_json::from_str(&raw).expect("expectation fixture must be well-formed JSON");
    let tools = doc["tools"]
        .as_array()
        .expect("expectation fixture must carry a `tools` array");
    entries_from_array(tools)
}

/// Reduce a captured `tools/list` JSON-RPC response to a name-keyed map.
fn observed_from_response(response: &Value) -> BTreeMap<String, ToolEntry> {
    let tools = response["result"]["tools"]
        .as_array()
        .expect("captured tools/list must carry a result.tools array");
    entries_from_array(tools)
}

/// Reduce a JSON array of tool objects to a name-keyed map, tolerating both
/// the fixture spelling and the MCP camelCase `inputSchema` spelling.
fn entries_from_array(tools: &[Value]) -> BTreeMap<String, ToolEntry> {
    let mut map = BTreeMap::new();
    for tool in tools {
        let name = tool["name"]
            .as_str()
            .expect("every tool must carry a string name")
            .to_owned();
        let description = tool["description"]
            .as_str()
            .expect("every tool must carry a string description")
            .to_owned();
        let input_schema = tool
            .get("inputSchema")
            .cloned()
            .unwrap_or_else(|| Value::Object(serde_json::Map::new()));
        map.insert(
            name.clone(),
            ToolEntry {
                name,
                description,
                input_schema,
            },
        );
    }
    map
}

// ── U2: the capture is well-formed and agent-visible ────────────────────────

/// U2 scenario: the capture obtained by driving the shim's MCP surface is
/// well-formed JSON containing a `tools` array.
#[tokio::test]
async fn captured_tools_list_is_well_formed_json_with_tools_array() {
    let response = capture::capture_tools_list_response().await;
    let tools = response["result"]["tools"]
        .as_array()
        .expect("captured tools/list must contain a result.tools array");
    assert!(
        !tools.is_empty(),
        "captured tools/list must expose at least one tool"
    );
    for tool in tools {
        assert!(
            tool["name"].as_str().is_some(),
            "every captured tool must carry a string name: {tool}"
        );
    }
}

// ── U1: exact name-set equality and per-tool description equality ───────────

/// U1 scenario: the observed tool-name set equals the declared set exactly —
/// no extras, no missing.
#[tokio::test]
async fn agent_visible_tool_names_match_fixture_exactly() {
    let expected = load_expected();
    let response = capture::capture_tools_list_response().await;
    let observed = observed_from_response(&response);

    let expected_names: BTreeSet<&String> = expected.keys().collect();
    let observed_names: BTreeSet<&String> = observed.keys().collect();

    let missing: Vec<&&String> = expected_names.difference(&observed_names).collect();
    let extra: Vec<&&String> = observed_names.difference(&expected_names).collect();

    assert!(
        missing.is_empty() && extra.is_empty(),
        "agent-visible tool-name set must equal the declared set exactly; \
         missing (declared but not served): {missing:?}; \
         extra (served but not declared): {extra:?}"
    );
}

/// U1 scenario: every observed tool's description equals the declared
/// description exactly (descriptions are the primary tool-selection signal).
#[tokio::test]
async fn agent_visible_tool_descriptions_match_fixture() {
    let expected = load_expected();
    let response = capture::capture_tools_list_response().await;
    let observed = observed_from_response(&response);

    for (name, expected_entry) in &expected {
        let observed_entry = observed
            .get(name)
            .unwrap_or_else(|| panic!("tool `{name}` declared in fixture but not served"));
        assert_eq!(
            observed_entry.description, expected_entry.description,
            "description drift for tool `{name}`"
        );
    }
}
