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
struct ToolEntry {
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

/// Reduce a JSON array of tool objects to a name-keyed map. The input schema is
/// read from the MCP camelCase `inputSchema` key, which both the human-authored
/// fixture and the serialized rmcp `Tool` use; an absent schema becomes an empty
/// object so a malformed entry surfaces as a schema-shape mismatch, not a panic.
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
            name,
            ToolEntry {
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

// ── U4/U6: declared-shape schema comparison and classified drift diffs ──────

/// A per-tool difference classified by the facet an agent would notice.
#[derive(Debug, PartialEq, Eq)]
enum ToolDiff {
    /// Served but not declared.
    Added(String),
    /// Declared but not served.
    Removed(String),
    /// Same name, different description.
    DescriptionChanged { name: String },
    /// Same name, different declared input-schema shape; `facet` names the
    /// specific differing property or schema facet.
    SchemaChanged { name: String, facet: String },
}

/// The reusable, pure comparison. Given the declared (expected) and observed
/// (actual) catalogs keyed by name, return every difference classified as
/// added / removed / description-changed / schema-changed. Because it is pure
/// and takes both sides as arguments, drift scenarios can exercise it without
/// mutating the real fixture or the real catalog.
fn classify_diffs(
    expected: &BTreeMap<String, ToolEntry>,
    actual: &BTreeMap<String, ToolEntry>,
) -> Vec<ToolDiff> {
    let mut diffs = Vec::new();
    let names: BTreeSet<&String> = expected.keys().chain(actual.keys()).collect();
    for name in names {
        match (expected.get(name), actual.get(name)) {
            (None, Some(_)) => diffs.push(ToolDiff::Added(name.clone())),
            (Some(_), None) => diffs.push(ToolDiff::Removed(name.clone())),
            (Some(expected_entry), Some(actual_entry)) => {
                if expected_entry.description != actual_entry.description {
                    diffs.push(ToolDiff::DescriptionChanged { name: name.clone() });
                }
                if let Some(facet) =
                    compare_schema_shape(&expected_entry.input_schema, &actual_entry.input_schema)
                {
                    diffs.push(ToolDiff::SchemaChanged {
                        name: name.clone(),
                        facet,
                    });
                }
            }
            (None, None) => unreachable!("name came from the union of both maps"),
        }
    }
    diffs
}

/// Compare two input schemas by DECLARED SHAPE (never raw bytes or key order),
/// returning the first differing facet if any. Compared facets: top-level
/// `type`, the exact property-name set, each shared property's declared `type`,
/// the `required` list (absent treated as empty), and `additionalProperties`
/// handling. NOT compared: `default`, `enum`, `items`, `description`, and the
/// `anyOf` disjunction (e.g. `impact_analysis`) — those are declared-detail
/// facets outside the shape contract.
fn compare_schema_shape(expected: &Value, actual: &Value) -> Option<String> {
    let expected_type = expected.get("type").and_then(Value::as_str);
    let actual_type = actual.get("type").and_then(Value::as_str);
    if expected_type != actual_type {
        return Some("type".to_owned());
    }

    // A present `properties` that is not an object is malformed. Surface the
    // asymmetry rather than letting `schema_properties` normalize it to an empty
    // map (which would let an observed `"properties": []` compare equal to a
    // declared empty object and miss the corruption).
    let expected_props_malformed = expected.get("properties").is_some_and(|v| !v.is_object());
    let actual_props_malformed = actual.get("properties").is_some_and(|v| !v.is_object());
    if expected_props_malformed != actual_props_malformed {
        return Some("properties".to_owned());
    }

    let expected_props = schema_properties(expected);
    let actual_props = schema_properties(actual);
    let expected_names: BTreeSet<&String> = expected_props.keys().collect();
    let actual_names: BTreeSet<&String> = actual_props.keys().collect();
    if let Some(name) = expected_names.symmetric_difference(&actual_names).next() {
        return Some(format!("property `{name}`"));
    }

    for (property, expected_schema) in &expected_props {
        let want_type = expected_schema.get("type").and_then(Value::as_str);
        let got_type = actual_props
            .get(property)
            .and_then(|schema| schema.get("type"))
            .and_then(Value::as_str);
        if want_type != got_type {
            return Some(format!("property `{property}` type"));
        }
    }

    if required_repr(expected) != required_repr(actual) {
        return Some("required".to_owned());
    }

    if expected.get("additionalProperties") != actual.get("additionalProperties") {
        return Some("additionalProperties".to_owned());
    }

    None
}

/// The `properties` object of a schema as a name-keyed map (absent -> empty).
fn schema_properties(schema: &Value) -> BTreeMap<String, Value> {
    schema
        .get("properties")
        .and_then(Value::as_object)
        .map(|object| {
            object
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect()
        })
        .unwrap_or_default()
}

/// A canonical, comparable representation of a schema's `required` list.
///
/// Absent is the empty set. A well-formed list yields its member names. A
/// malformed `required` — a non-array, or an array containing a non-string —
/// yields a sentinel that cannot equal any valid required-name set, so a
/// malformed observed schema surfaces as a `required` mismatch instead of
/// silently normalizing to equal a well-formed one.
fn required_repr(schema: &Value) -> BTreeSet<String> {
    match schema.get("required") {
        None => BTreeSet::new(),
        Some(Value::Array(items)) => items
            .iter()
            .map(|value| match value {
                Value::String(name) => name.clone(),
                other => format!("\0non-string:{other}"),
            })
            .collect(),
        Some(other) => {
            let mut set = BTreeSet::new();
            set.insert(format!("\0non-array:{other}"));
            set
        }
    }
}

/// U4 scenario: every observed tool's declared input-schema shape matches the
/// fixture — object type, exact property-name set, per-property type, required
/// list, and `additionalProperties` handling.
#[tokio::test]
async fn agent_visible_tool_schemas_match_fixture_shape() {
    let expected = load_expected();
    let response = capture::capture_tools_list_response().await;
    let observed = observed_from_response(&response);

    for (name, expected_entry) in &expected {
        assert_eq!(
            expected_entry
                .input_schema
                .get("type")
                .and_then(Value::as_str),
            Some("object"),
            "fixture schema for `{name}` must declare an object type"
        );
        let observed_entry = observed
            .get(name)
            .unwrap_or_else(|| panic!("tool `{name}` declared in fixture but not served"));
        if let Some(facet) =
            compare_schema_shape(&expected_entry.input_schema, &observed_entry.input_schema)
        {
            panic!("schema-shape drift for tool `{name}` at facet: {facet}");
        }
    }
}

/// U4 scenario: the whole agent-visible catalog has zero classified drift
/// against the declared fixture.
#[tokio::test]
async fn agent_visible_catalog_has_zero_drift() {
    let expected = load_expected();
    let response = capture::capture_tools_list_response().await;
    let observed = observed_from_response(&response);

    let diffs = classify_diffs(&expected, &observed);
    assert!(
        diffs.is_empty(),
        "agent-visible catalog must exhibit zero drift; classified diffs: {diffs:?}"
    );
}

/// U6 scenario: an induced mismatch — constructed entirely in-test so the real
/// fixture is untouched — produces the correctly classified per-tool diff,
/// naming the specific differing property for a schema change.
#[test]
fn classify_diffs_reports_each_drift_class_with_the_specific_property() {
    fn entry(description: &str, schema: Value) -> ToolEntry {
        ToolEntry {
            description: description.to_owned(),
            input_schema: schema,
        }
    }
    let object_schema =
        |properties: Value| serde_json::json!({ "type": "object", "properties": properties });

    let mut expected: BTreeMap<String, ToolEntry> = BTreeMap::new();
    expected.insert(
        "renamed_old".to_owned(),
        entry("stable summary", object_schema(serde_json::json!({}))),
    );
    expected.insert(
        "desc_tool".to_owned(),
        entry("original summary", object_schema(serde_json::json!({}))),
    );
    expected.insert(
        "schema_tool".to_owned(),
        entry(
            "stable summary",
            object_schema(serde_json::json!({ "limit": { "type": "integer" } })),
        ),
    );

    let mut actual: BTreeMap<String, ToolEntry> = BTreeMap::new();
    // renamed_old -> renamed_new (a rename is a removed + an added).
    actual.insert(
        "renamed_new".to_owned(),
        entry("stable summary", object_schema(serde_json::json!({}))),
    );
    // desc_tool: description changed only.
    actual.insert(
        "desc_tool".to_owned(),
        entry("reworded summary", object_schema(serde_json::json!({}))),
    );
    // schema_tool: `limit` property type changed integer -> string.
    actual.insert(
        "schema_tool".to_owned(),
        entry(
            "stable summary",
            object_schema(serde_json::json!({ "limit": { "type": "string" } })),
        ),
    );

    let diffs = classify_diffs(&expected, &actual);

    assert!(
        diffs.contains(&ToolDiff::Removed("renamed_old".to_owned())),
        "a rename must yield a Removed for the old name: {diffs:?}"
    );
    assert!(
        diffs.contains(&ToolDiff::Added("renamed_new".to_owned())),
        "a rename must yield an Added for the new name: {diffs:?}"
    );
    assert!(
        diffs.contains(&ToolDiff::DescriptionChanged {
            name: "desc_tool".to_owned()
        }),
        "a reworded description must be classified DescriptionChanged: {diffs:?}"
    );
    assert!(
        diffs.contains(&ToolDiff::SchemaChanged {
            name: "schema_tool".to_owned(),
            facet: "property `limit` type".to_owned(),
        }),
        "a property type change must be classified SchemaChanged naming the property: {diffs:?}"
    );
}

// ── U5: mechanically enforced oracle independence (in-test mirror) ──────────

/// Regression: a malformed observed `required` — a non-array, or an array with
/// a non-string member — must surface as a `required` facet difference rather
/// than normalizing to equal a well-formed declared list.
#[test]
fn compare_schema_shape_flags_malformed_required() {
    let declared = serde_json::json!({
        "type": "object",
        "properties": { "path": { "type": "string" } },
        "required": ["path"]
    });
    let non_array = serde_json::json!({
        "type": "object",
        "properties": { "path": { "type": "string" } },
        "required": "path"
    });
    let null_member = serde_json::json!({
        "type": "object",
        "properties": { "path": { "type": "string" } },
        "required": ["path", null]
    });
    assert_eq!(
        compare_schema_shape(&declared, &non_array).as_deref(),
        Some("required"),
        "a non-array `required` must be reported as a required-facet difference"
    );
    assert_eq!(
        compare_schema_shape(&declared, &null_member).as_deref(),
        Some("required"),
        "a `required` array with a non-string member must be reported as a difference"
    );
}

/// Regression: a present but malformed `properties` value (not an object, e.g.
/// `"properties": []`) must surface as a `properties` difference rather than
/// normalizing to an empty property set and comparing equal to a declared
/// empty-object schema.
#[test]
fn compare_schema_shape_flags_malformed_properties() {
    let declared = serde_json::json!({ "type": "object", "properties": {} });
    let non_object = serde_json::json!({ "type": "object", "properties": [] });
    assert_eq!(
        compare_schema_shape(&declared, &non_object).as_deref(),
        Some("properties"),
        "a non-object `properties` must be reported as a properties-facet difference"
    );
}

/// U5 scenario: the oracle's own Rust sources never reach the production
/// catalog derivation path. This mirrors `scripts/check-oracle-independence.*`
/// so the invariant fails the test suite, not only an out-of-band script.
///
/// The forbidden tokens are assembled from fragments at runtime so this
/// assertion does not itself embed them as literals — otherwise the scan would
/// flag its own source file. The human-authored JSON fixture is intentionally
/// NOT scanned here: it is data, may name the source contract in its policy
/// note, and its independence is guaranteed by the regeneration scan and its
/// header rather than by token absence.
#[test]
fn oracle_sources_are_independent_of_production_catalog() {
    let module_token = ["tools", "catalog"].join("_");
    let constructor_token = ["all", "tools"].join("_");
    let sources = [
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/contract/mcp_catalog_oracle_test.rs"
        ),
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/helpers/mcp_catalog_capture.rs"
        ),
    ];
    for path in sources {
        let body = std::fs::read_to_string(path)
            .unwrap_or_else(|error| panic!("oracle source must be readable at {path}: {error}"));
        assert!(
            !body.contains(&module_token),
            "oracle source {path} must not reference the production catalog module"
        );
        assert!(
            !body.contains(&constructor_token),
            "oracle source {path} must not reference the production catalog enumeration constructor"
        );
    }
}
