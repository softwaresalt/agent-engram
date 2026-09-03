//! Descriptor registry completeness contract (plan unit F19, 142.005.003-ST).
//!
//! The canonical descriptor registry in `src/tools/capabilities.rs` is only
//! trustworthy if it cannot silently drift as tools are added or removed. This
//! contract enforces three things:
//!
//! 1. **Completeness** — every tool reachable today has exactly one descriptor.
//! 2. **Attribute integrity** — no descriptor omits a required attribute.
//! 3. **Named declarations** — the four declarations the plan calls out by name
//!    (`_health`, `_shutdown`, `doctor --smoke`, `get_retrieval_eval_report`)
//!    hold their specified surfaces, capability class, and mode availability.
//!
//! The reachability oracle is the production MCP catalog
//! (`engram::shim::tools_catalog::all_tools`) plus the two non-dispatch IPC
//! methods and the one CLI readiness workflow that the catalog never lists.
//! Removing a descriptor, or adding a catalog tool without one, turns this
//! contract RED.

use std::collections::BTreeSet;

use engram::shim::tools_catalog;
use engram::tools::capabilities::{
    self, CapabilityClass, DOCTOR_SMOKE, HEALTH_METHOD, InputOwnership, SHUTDOWN_METHOD,
    ToolDescriptor, ToolSurface,
};
use serde_json::{Value, json};

/// Names that are reachable through MCP dispatch but are deliberately absent
/// from the default `tools/list` catalog because they are feature-gated.
/// `TOOL_COUNT` and the catalog cover the default build only.
const FEATURE_GATED_MCP_TOOLS: &[&str] = &["query_changes", "index_git_history"];

/// Non-dispatch methods and workflows the MCP catalog never lists but that are
/// still reachable: two direct-IPC methods answered at request entry, and one
/// CLI readiness workflow.
const NON_CATALOG_DECLARATIONS: &[&str] = &[HEALTH_METHOD, SHUTDOWN_METHOD, DOCTOR_SMOKE];

/// The agent-visible MCP catalog tool names for the current build.
fn catalog_names() -> BTreeSet<String> {
    tools_catalog::all_tools()
        .into_iter()
        .map(|tool| tool.name.to_string())
        .collect()
}

/// Every declared descriptor name for the current build.
fn descriptor_names() -> BTreeSet<String> {
    capabilities::all_descriptors()
        .into_iter()
        .map(|descriptor| descriptor.name.to_owned())
        .collect()
}

/// Fetch a descriptor by name or fail with a legible message.
fn require(name: &str) -> ToolDescriptor {
    capabilities::descriptor(name)
        .unwrap_or_else(|| panic!("`{name}` must have a descriptor in the canonical registry"))
}

// ── Completeness ─────────────────────────────────────────────────────────────

/// Every MCP catalog tool has a descriptor that declares the stdio-MCP surface.
///
/// This is the RED trigger for "a tool exists without a descriptor": deleting a
/// declaration, or adding a catalog tool without one, fails here.
#[test]
fn every_mcp_catalog_tool_has_a_descriptor() {
    let declared = descriptor_names();
    let missing: Vec<String> = catalog_names()
        .into_iter()
        .filter(|name| !declared.contains(name))
        .collect();
    assert!(
        missing.is_empty(),
        "every MCP catalog tool must have a descriptor; undeclared tools: {missing:?}"
    );

    for name in catalog_names() {
        let descriptor = require(&name);
        assert!(
            descriptor.supports(ToolSurface::StdioMcp),
            "catalog tool `{name}` must declare the stdio_mcp surface"
        );
    }
}

/// No descriptor claims the stdio-MCP surface for a tool that is not actually
/// served: every MCP-surfaced descriptor is either in the catalog or is a known
/// feature-gated dispatch tool compiled into this build.
#[test]
fn no_descriptor_declares_an_unserved_mcp_tool() {
    let catalog = catalog_names();
    let gated: BTreeSet<&str> = FEATURE_GATED_MCP_TOOLS.iter().copied().collect();

    for descriptor in capabilities::all_descriptors() {
        if !descriptor.supports(ToolSurface::StdioMcp) {
            continue;
        }
        assert!(
            catalog.contains(descriptor.name) || gated.contains(descriptor.name),
            "descriptor `{}` declares stdio_mcp but is neither in the MCP catalog \
             nor a known feature-gated dispatch tool",
            descriptor.name
        );
    }
}

/// The non-catalog reachable methods and workflows are all declared, and the
/// registry declares nothing beyond catalog tools, feature-gated dispatch
/// tools, and those non-catalog entries.
#[test]
fn registry_covers_exactly_the_reachable_surface() {
    let declared = descriptor_names();

    for name in NON_CATALOG_DECLARATIONS {
        assert!(
            declared.contains(*name),
            "`{name}` must have a descriptor in the canonical registry"
        );
    }

    let mut expected: BTreeSet<String> = catalog_names();
    expected.extend(NON_CATALOG_DECLARATIONS.iter().map(|s| (*s).to_owned()));
    expected.extend(FEATURE_GATED_MCP_TOOLS.iter().map(|s| (*s).to_owned()));

    let unexpected: Vec<&String> = declared.difference(&expected).collect();
    assert!(
        unexpected.is_empty(),
        "registry declares names outside the known reachable surface: {unexpected:?}"
    );
}

/// Names are unique — a duplicated declaration would make lookups ambiguous.
#[test]
fn descriptor_names_are_unique() {
    let all = capabilities::all_descriptors();
    let unique: BTreeSet<&str> = all.iter().map(|descriptor| descriptor.name).collect();
    assert_eq!(
        unique.len(),
        all.len(),
        "descriptor names must be unique; duplicates present in {:?}",
        capabilities::declared_names()
    );
}

// ── Attribute integrity ──────────────────────────────────────────────────────

/// No descriptor omits a required attribute: every one carries a non-blank
/// name, at least one surface, and a well-formed JSON Schema object body.
#[test]
fn no_descriptor_omits_a_required_attribute() {
    for descriptor in capabilities::all_descriptors() {
        let missing = descriptor.missing_attributes();
        assert!(
            missing.is_empty(),
            "descriptor `{}` omits required attribute(s): {missing:?}",
            descriptor.name
        );
    }
}

/// The completeness check is not vacuous: a descriptor missing its surfaces or
/// carrying a malformed schema is reported, so the assertion above can fail.
#[test]
fn missing_attributes_detects_an_incomplete_descriptor() {
    let complete = require(HEALTH_METHOD);
    assert!(
        complete.missing_attributes().is_empty(),
        "the reference descriptor must be complete"
    );

    let mut no_surfaces = complete.clone();
    no_surfaces.surfaces = Vec::new();
    assert_eq!(
        no_surfaces.missing_attributes(),
        vec!["surfaces"],
        "a descriptor with no declared surface must report `surfaces` as missing"
    );

    let mut empty_schema = complete.clone();
    empty_schema.input_schema = std::sync::Arc::new(serde_json::Map::new());
    assert_eq!(
        empty_schema.missing_attributes(),
        vec!["input_schema"],
        "a descriptor with an empty schema must report `input_schema` as missing"
    );

    let mut wrong_schema_type = complete;
    let Value::Object(body) = json!({ "type": "array" }) else {
        unreachable!("literal is an object")
    };
    wrong_schema_type.input_schema = std::sync::Arc::new(body);
    assert_eq!(
        wrong_schema_type.missing_attributes(),
        vec!["input_schema.type"],
        "a schema that is not an object body must report `input_schema.type` as missing"
    );
}

/// Read-server availability tracks the capability class for dispatched tools:
/// a read-server refuses write and control dispatch. `_health` and `_shutdown`
/// are the declared exceptions — request entry answers both before the
/// mode-gated dispatch path.
#[test]
fn write_and_control_tools_are_unavailable_on_a_read_server() {
    for descriptor in capabilities::all_descriptors() {
        if descriptor.name == HEALTH_METHOD || descriptor.name == SHUTDOWN_METHOD {
            continue;
        }
        match descriptor.capability {
            CapabilityClass::Read => assert!(
                descriptor.read_server_available,
                "read tool `{}` must stay available on a read-server",
                descriptor.name
            ),
            CapabilityClass::Write | CapabilityClass::Control => assert!(
                !descriptor.read_server_available,
                "{} tool `{}` must not be declared available on a read-server",
                descriptor.capability, descriptor.name
            ),
        }
    }
}

// ── Named declarations ───────────────────────────────────────────────────────

/// `_health` is the direct-IPC liveness probe: direct-IPC surface only, never
/// MCP or CLI, and it stays serviceable on a read-server.
#[test]
fn health_is_declared_direct_ipc_only_liveness() {
    let health = require(HEALTH_METHOD);
    assert_eq!(
        health.surfaces,
        vec![ToolSurface::DirectIpc],
        "`_health` must declare the direct_ipc surface and nothing else"
    );
    assert_eq!(
        health.capability,
        CapabilityClass::Read,
        "`_health` is a non-mutating liveness probe"
    );
    assert!(
        health.read_server_available,
        "`_health` liveness must remain available on a read-server"
    );
    assert_eq!(
        health.input_ownership,
        InputOwnership::IpcServer,
        "`_health` input is owned by the IPC request-entry layer"
    );
}

/// `_shutdown` is a Control method on the direct-IPC surface.
#[test]
fn shutdown_is_declared_control() {
    let shutdown = require(SHUTDOWN_METHOD);
    assert_eq!(
        shutdown.capability,
        CapabilityClass::Control,
        "`_shutdown` must be declared with the Control capability class"
    );
    assert_eq!(
        shutdown.surfaces,
        vec![ToolSurface::DirectIpc],
        "`_shutdown` must declare the direct_ipc surface and nothing else"
    );
    assert_eq!(
        shutdown.input_ownership,
        InputOwnership::IpcServer,
        "`_shutdown` input is owned by the IPC request-entry layer"
    );
}

/// `doctor --smoke` is the non-destructive readiness workflow that stays
/// available in `ReadServer` mode.
#[test]
fn doctor_smoke_is_a_read_server_readiness_workflow() {
    let smoke = require(DOCTOR_SMOKE);
    assert_eq!(
        smoke.capability,
        CapabilityClass::Read,
        "`doctor --smoke` must be non-destructive, so it is a Read capability"
    );
    assert!(
        smoke.read_server_available,
        "`doctor --smoke` must remain available in ReadServer mode"
    );
    assert_eq!(
        smoke.surfaces,
        vec![ToolSurface::Cli],
        "`doctor --smoke` is a CLI-surface workflow, not an MCP tool or IPC method"
    );
    assert_eq!(
        smoke.input_ownership,
        InputOwnership::CliFrontend,
        "`doctor --smoke` arguments are validated by the CLI front end"
    );
}

/// `get_retrieval_eval_report` is MCP-only: it is served over the MCP surface
/// and has no CLI command (`docs/cli-mcp-parity.md` records the gap).
#[test]
fn retrieval_eval_report_is_declared_mcp_only() {
    let report = require("get_retrieval_eval_report");
    assert!(
        report.supports(ToolSurface::StdioMcp),
        "`get_retrieval_eval_report` must declare the stdio_mcp surface"
    );
    assert!(
        !report.supports(ToolSurface::Cli),
        "`get_retrieval_eval_report` is MCP-only and must not declare a CLI surface"
    );
    assert_eq!(
        report.capability,
        CapabilityClass::Read,
        "`get_retrieval_eval_report` reads the latest persisted report"
    );
}

// ── Vocabulary ───────────────────────────────────────────────────────────────

/// The surface enum exposes exactly the three declared canonical names.
#[test]
fn surface_vocabulary_is_exactly_three_variants() {
    let surfaces = [
        ToolSurface::DirectIpc,
        ToolSurface::Cli,
        ToolSurface::StdioMcp,
    ];
    let names: Vec<&str> = surfaces.iter().map(|s| s.as_str()).collect();
    assert_eq!(names, vec!["direct_ipc", "cli", "stdio_mcp"]);
}

/// The capability class vocabulary distinguishes read, write, and control.
#[test]
fn capability_vocabulary_distinguishes_read_write_control() {
    let classes = [
        CapabilityClass::Read,
        CapabilityClass::Write,
        CapabilityClass::Control,
    ];
    let names: Vec<&str> = classes.iter().map(|c| c.as_str()).collect();
    assert_eq!(names, vec!["read", "write", "control"]);
}
