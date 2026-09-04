//! Canonical tool descriptor registry (plan unit F19, 142.005-T / 134-S).
//!
//! This module is the single declarative place where every Engram method or
//! workflow records the facts a caller-side gate needs before dispatch:
//!
//! * the method / workflow name,
//! * its [`CapabilityClass`] (read, write, or control),
//! * its declared JSON input schema,
//! * the [`ToolSurface`]s it is reachable through,
//! * whether it stays available when the daemon runs in
//!   [`DaemonMode::ReadServer`](crate::models::config::DaemonMode::ReadServer),
//! * and which subsystem owns validating its input ([`InputOwnership`]).
//!
//! # Behavior neutrality
//!
//! The registry only *records* today's reachability. It does not gate, refuse,
//! or route anything on its own — wiring consumers (request entry, dispatch
//! enforcement, catalog derivation) belongs to later plan units. Adding or
//! changing a surface here is a declaration change, never a behavior change.
//!
//! # Schema ownership
//!
//! Tools served through the MCP `tools/list` catalog take their schema from
//! [`crate::shim::tools_catalog::all_tools`], which remains the single source
//! of truth for agent-visible schemas. Methods that are *not* MCP tools
//! (`_health`, `_shutdown`, the `doctor --smoke` readiness workflow, and the
//! `git-graph` feature-gated dispatch tools that the default catalog omits)
//! declare their schema locally here.
//!
//! # What is intentionally not registered
//!
//! Local CLI process entrypoints and installer maintenance commands (`engram
//! shim`, `engram daemon`, `engram install`, `engram update`, `engram
//! reinstall`, `engram uninstall`, `engram manifest`, `engram verify`, `engram
//! migrate-down`) are not daemon methods or workflows: they never reach IPC
//! dispatch and have no capability class to gate. See
//! `docs/cli-mcp-parity.md` for the canonical surface map.

use std::collections::BTreeMap;
use std::sync::Arc;

use serde_json::{Map, Value, json};

use crate::shim::tools_catalog;

// ── Surface ──────────────────────────────────────────────────────────────────

/// A transport surface a method or workflow is reachable through.
///
/// Canonical setting strings match the plan's declared vocabulary and the
/// snake_case convention used by
/// [`DaemonMode::as_str`](crate::models::config::DaemonMode::as_str).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ToolSurface {
    /// The daemon's line-oriented IPC server (named pipe / Unix socket).
    DirectIpc,
    /// The human-facing `engram` command-line interface.
    Cli,
    /// The MCP stdio shim's `tools/list` and `tools/call` surface.
    StdioMcp,
}

impl ToolSurface {
    /// Canonical setting string for this surface.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DirectIpc => "direct_ipc",
            Self::Cli => "cli",
            Self::StdioMcp => "stdio_mcp",
        }
    }
}

impl std::fmt::Display for ToolSurface {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ── Capability class ─────────────────────────────────────────────────────────

/// The effect class of a method or workflow.
///
/// A read-server refuses `Write` and `Control` dispatch (see
/// [`ReadServerRefusalError`](crate::errors::ReadServerRefusalError)); `Read`
/// is the only class that is unconditionally serviceable there.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CapabilityClass {
    /// Observes state without mutating it.
    Read,
    /// Mutates workspace or generation state on disk.
    Write,
    /// Changes daemon lifecycle or binding rather than workspace data.
    Control,
}

impl CapabilityClass {
    /// Canonical setting string for this capability class.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Control => "control",
        }
    }
}

impl std::fmt::Display for CapabilityClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ── Input ownership ──────────────────────────────────────────────────────────

/// The subsystem that owns deserializing and validating a method's input.
///
/// Recording ownership explicitly keeps validation from being duplicated (or
/// silently skipped) as the request-entry path grows mode-aware gates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InputOwnership {
    /// The daemon tool handler deserializes and validates the params object.
    DaemonHandler,
    /// The IPC request-entry layer owns the input; the method is answered
    /// before tool dispatch and accepts no caller-supplied parameters.
    IpcServer,
    /// The CLI front end parses and validates arguments before invoking the
    /// workflow entrypoint.
    CliFrontend,
}

impl InputOwnership {
    /// Canonical setting string for this ownership assignment.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DaemonHandler => "daemon_handler",
            Self::IpcServer => "ipc_server",
            Self::CliFrontend => "cli_frontend",
        }
    }
}

impl std::fmt::Display for InputOwnership {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ── Descriptor ───────────────────────────────────────────────────────────────

/// The canonical declaration for one method or workflow.
#[derive(Debug, Clone)]
pub struct ToolDescriptor {
    /// Method name (`set_workspace`, `_health`) or workflow name
    /// (`doctor --smoke`).
    pub name: &'static str,
    /// Effect class used by mode-aware gates.
    pub capability: CapabilityClass,
    /// Declared JSON input schema (a JSON Schema object body, matching the
    /// `Arc<Map<String, Value>>` representation used by the MCP catalog).
    pub input_schema: Arc<Map<String, Value>>,
    /// Every surface the method or workflow is reachable through today.
    pub surfaces: Vec<ToolSurface>,
    /// Whether the method stays serviceable in read-server mode.
    pub read_server_available: bool,
    /// Which subsystem validates this method's input.
    pub input_ownership: InputOwnership,
}

impl ToolDescriptor {
    /// Whether this method is reachable through `surface`.
    #[must_use]
    pub fn supports(&self, surface: ToolSurface) -> bool {
        self.surfaces.contains(&surface)
    }

    /// Names of required attributes this descriptor fails to declare.
    ///
    /// An empty result means the descriptor is complete. A descriptor is
    /// incomplete when it has a blank name, declares no surface, or carries a
    /// schema that is not a well-formed JSON Schema object body (missing or
    /// non-`"object"` `type`).
    #[must_use]
    pub fn missing_attributes(&self) -> Vec<&'static str> {
        let mut missing = Vec::new();
        if self.name.trim().is_empty() {
            missing.push("name");
        }
        if self.surfaces.is_empty() {
            missing.push("surfaces");
        }
        if self.input_schema.is_empty() {
            missing.push("input_schema");
        } else if self.input_schema.get("type").and_then(Value::as_str) != Some("object") {
            missing.push("input_schema.type");
        }
        missing
    }
}

// ── Declaration table ────────────────────────────────────────────────────────

const IPC_CLI_MCP: &[ToolSurface] = &[
    ToolSurface::DirectIpc,
    ToolSurface::Cli,
    ToolSurface::StdioMcp,
];
const IPC_AND_MCP: &[ToolSurface] = &[ToolSurface::DirectIpc, ToolSurface::StdioMcp];
const IPC_ONLY: &[ToolSurface] = &[ToolSurface::DirectIpc];
const CLI_ONLY: &[ToolSurface] = &[ToolSurface::Cli];

/// Where a declaration's schema comes from.
#[derive(Clone, Copy)]
enum SchemaSource {
    /// Reuse the agent-visible schema declared by the MCP catalog entry of the
    /// same name — the catalog stays the single source of truth for schemas.
    McpCatalog,
    /// Declare the schema locally: the method is not an MCP catalog tool.
    Local(fn() -> Value),
}

/// One row of the declaration table, before schema resolution.
struct Declaration {
    name: &'static str,
    capability: CapabilityClass,
    surfaces: &'static [ToolSurface],
    read_server_available: bool,
    input_ownership: InputOwnership,
    schema: SchemaSource,
}

/// Schema for a method that accepts no caller-supplied parameters.
fn no_params_schema() -> Value {
    json!({
        "type": "object",
        "properties": {},
        "additionalProperties": false
    })
}

/// Schema for the `doctor --smoke` readiness workflow.
fn smoke_workflow_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "path": {
                "type": "string",
                "description": "Absolute path to the workspace root to probe"
            }
        },
        "required": ["path"]
    })
}

#[cfg(feature = "git-graph")]
fn query_changes_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "file_path": { "type": "string", "description": "Filter commits that touched this file path" },
            "symbol": { "type": "string", "description": "Filter commits that affected this named symbol" },
            "since": { "type": "string", "description": "Return only commits on or after this ISO-8601 timestamp" },
            "until": { "type": "string", "description": "Return only commits on or before this ISO-8601 timestamp" },
            "limit": { "type": "integer", "description": "Maximum number of commits to return (default: 20)" }
        }
    })
}

#[cfg(feature = "git-graph")]
fn index_git_history_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "depth": { "type": "integer", "description": "Number of commits to walk from HEAD (default: 500)" },
            "force": { "type": "boolean", "description": "Re-index all commits even if already stored" }
        }
    })
}

/// Every declared method and workflow, in catalog order followed by the
/// non-catalog IPC methods and workflows.
///
/// Read-server availability tracks the capability class for dispatched tools:
/// a read-server refuses `Write` and `Control` dispatch. `_health` and
/// `_shutdown` both skip the request-entry activation path (F20) before
/// dispatch, but that is independent of capability-based refusal: `_health`
/// is `Read`, so it stays available on a read-server by the general rule;
/// `_shutdown` is `Control` and is refused like any other Control tool (plan
/// P22: "Refuse non-read capabilities before side effects, including raw
/// `_shutdown`") — it is not a deliberate exception.
const DECLARATIONS: &[Declaration] = &[
    // ── Workspace / lifecycle ────────────────────────────────────────────
    Declaration {
        name: "set_workspace",
        capability: CapabilityClass::Control,
        surfaces: IPC_CLI_MCP,
        read_server_available: false,
        input_ownership: InputOwnership::DaemonHandler,
        schema: SchemaSource::McpCatalog,
    },
    Declaration {
        name: "get_daemon_status",
        capability: CapabilityClass::Read,
        surfaces: IPC_CLI_MCP,
        read_server_available: true,
        input_ownership: InputOwnership::DaemonHandler,
        schema: SchemaSource::McpCatalog,
    },
    Declaration {
        name: "get_workspace_status",
        capability: CapabilityClass::Read,
        surfaces: IPC_CLI_MCP,
        read_server_available: true,
        input_ownership: InputOwnership::DaemonHandler,
        schema: SchemaSource::McpCatalog,
    },
    Declaration {
        name: "flush_state",
        capability: CapabilityClass::Write,
        surfaces: IPC_CLI_MCP,
        read_server_available: false,
        input_ownership: InputOwnership::DaemonHandler,
        schema: SchemaSource::McpCatalog,
    },
    // ── Indexing ─────────────────────────────────────────────────────────
    Declaration {
        name: "index_workspace",
        capability: CapabilityClass::Write,
        surfaces: IPC_CLI_MCP,
        read_server_available: false,
        input_ownership: InputOwnership::DaemonHandler,
        schema: SchemaSource::McpCatalog,
    },
    Declaration {
        name: "sync_workspace",
        capability: CapabilityClass::Write,
        surfaces: IPC_CLI_MCP,
        read_server_available: false,
        input_ownership: InputOwnership::DaemonHandler,
        schema: SchemaSource::McpCatalog,
    },
    // ── Retrieval / graph reads ──────────────────────────────────────────
    Declaration {
        name: "query_memory",
        capability: CapabilityClass::Read,
        surfaces: IPC_CLI_MCP,
        read_server_available: true,
        input_ownership: InputOwnership::DaemonHandler,
        schema: SchemaSource::McpCatalog,
    },
    Declaration {
        name: "get_workspace_statistics",
        capability: CapabilityClass::Read,
        surfaces: IPC_CLI_MCP,
        read_server_available: true,
        input_ownership: InputOwnership::DaemonHandler,
        schema: SchemaSource::McpCatalog,
    },
    Declaration {
        name: "map_code",
        capability: CapabilityClass::Read,
        surfaces: IPC_CLI_MCP,
        read_server_available: true,
        input_ownership: InputOwnership::DaemonHandler,
        schema: SchemaSource::McpCatalog,
    },
    Declaration {
        name: "list_symbols",
        capability: CapabilityClass::Read,
        surfaces: IPC_CLI_MCP,
        read_server_available: true,
        input_ownership: InputOwnership::DaemonHandler,
        schema: SchemaSource::McpCatalog,
    },
    Declaration {
        name: "unified_search",
        capability: CapabilityClass::Read,
        surfaces: IPC_CLI_MCP,
        read_server_available: true,
        input_ownership: InputOwnership::DaemonHandler,
        schema: SchemaSource::McpCatalog,
    },
    Declaration {
        name: "impact_analysis",
        capability: CapabilityClass::Read,
        surfaces: IPC_CLI_MCP,
        read_server_available: true,
        input_ownership: InputOwnership::DaemonHandler,
        schema: SchemaSource::McpCatalog,
    },
    Declaration {
        name: "query_graph",
        capability: CapabilityClass::Read,
        surfaces: IPC_CLI_MCP,
        read_server_available: true,
        input_ownership: InputOwnership::DaemonHandler,
        schema: SchemaSource::McpCatalog,
    },
    // ── Lint ─────────────────────────────────────────────────────────────
    Declaration {
        name: "lint_dax",
        capability: CapabilityClass::Read,
        surfaces: IPC_CLI_MCP,
        read_server_available: true,
        input_ownership: InputOwnership::DaemonHandler,
        schema: SchemaSource::McpCatalog,
    },
    // ── Reports / metrics ────────────────────────────────────────────────
    Declaration {
        name: "get_health_report",
        capability: CapabilityClass::Read,
        surfaces: IPC_CLI_MCP,
        read_server_available: true,
        input_ownership: InputOwnership::DaemonHandler,
        schema: SchemaSource::McpCatalog,
    },
    Declaration {
        name: "get_branch_metrics",
        capability: CapabilityClass::Read,
        surfaces: IPC_CLI_MCP,
        read_server_available: true,
        input_ownership: InputOwnership::DaemonHandler,
        schema: SchemaSource::McpCatalog,
    },
    Declaration {
        name: "get_token_savings_report",
        capability: CapabilityClass::Read,
        surfaces: IPC_CLI_MCP,
        read_server_available: true,
        input_ownership: InputOwnership::DaemonHandler,
        schema: SchemaSource::McpCatalog,
    },
    Declaration {
        name: "get_evaluation_report",
        capability: CapabilityClass::Read,
        surfaces: IPC_CLI_MCP,
        read_server_available: true,
        input_ownership: InputOwnership::DaemonHandler,
        schema: SchemaSource::McpCatalog,
    },
    Declaration {
        name: "get_mutable_script_retry_metrics",
        capability: CapabilityClass::Read,
        surfaces: IPC_CLI_MCP,
        read_server_available: true,
        input_ownership: InputOwnership::DaemonHandler,
        schema: SchemaSource::McpCatalog,
    },
    // ── Retrieval evaluation ─────────────────────────────────────────────
    Declaration {
        name: "run_retrieval_eval",
        capability: CapabilityClass::Write,
        surfaces: IPC_CLI_MCP,
        read_server_available: false,
        input_ownership: InputOwnership::DaemonHandler,
        schema: SchemaSource::McpCatalog,
    },
    // MCP-only: `docs/cli-mcp-parity.md` records this as a deliberate CLI gap
    // — the CLI exposes `engram eval` (run + emit) but no read-only accessor
    // for the latest persisted report. No approved requirement adds a CLI
    // surface here, so none is declared.
    Declaration {
        name: "get_retrieval_eval_report",
        capability: CapabilityClass::Read,
        surfaces: IPC_AND_MCP,
        read_server_available: true,
        input_ownership: InputOwnership::DaemonHandler,
        schema: SchemaSource::McpCatalog,
    },
    // ── git-graph feature-gated dispatch tools ───────────────────────────
    // Excluded from the default MCP catalog (`TOOL_COUNT` covers the default
    // build only), so they declare local schemas and are compiled only when
    // the feature that makes them reachable is enabled.
    #[cfg(feature = "git-graph")]
    Declaration {
        name: "query_changes",
        capability: CapabilityClass::Read,
        surfaces: IPC_AND_MCP,
        read_server_available: true,
        input_ownership: InputOwnership::DaemonHandler,
        schema: SchemaSource::Local(query_changes_schema),
    },
    #[cfg(feature = "git-graph")]
    Declaration {
        name: "index_git_history",
        capability: CapabilityClass::Write,
        surfaces: IPC_AND_MCP,
        read_server_available: false,
        input_ownership: InputOwnership::DaemonHandler,
        schema: SchemaSource::Local(index_git_history_schema),
    },
    // ── Non-dispatch IPC methods ─────────────────────────────────────────
    // `_health` is the direct-IPC liveness probe the shim polls while the
    // daemon hydrates. It is never published in the MCP catalog and has no CLI
    // command, and the IPC request-entry layer answers it before dispatch.
    Declaration {
        name: "_health",
        capability: CapabilityClass::Read,
        surfaces: IPC_ONLY,
        read_server_available: true,
        input_ownership: InputOwnership::IpcServer,
        schema: SchemaSource::Local(no_params_schema),
    },
    // `_shutdown` changes daemon lifecycle rather than workspace data, so it
    // is Control. It skips the request-entry activation path like `_health`
    // (F20), but that does not exempt it from capability-based refusal: plan
    // P22 requires refusing non-read capabilities before side effects,
    // including raw `_shutdown`, so it must not be reachable on a
    // read-server.
    Declaration {
        name: "_shutdown",
        capability: CapabilityClass::Control,
        surfaces: IPC_ONLY,
        read_server_available: false,
        input_ownership: InputOwnership::IpcServer,
        schema: SchemaSource::Local(no_params_schema),
    },
    // ── Workflows ────────────────────────────────────────────────────────
    // `doctor --smoke` drives a full shim -> daemon handshake round-trip
    // (`crate::tools::doctor::run_smoke_test`). It binds a workspace and stops
    // the daemon it started, mutating nothing in the workspace, so it is a
    // non-destructive readiness workflow that stays available on a
    // read-server. It is a CLI-surface workflow, not an MCP tool and not an
    // IPC method name.
    Declaration {
        name: DOCTOR_SMOKE,
        capability: CapabilityClass::Read,
        surfaces: CLI_ONLY,
        read_server_available: true,
        input_ownership: InputOwnership::CliFrontend,
        schema: SchemaSource::Local(smoke_workflow_schema),
    },
];

/// Canonical name of the `doctor --smoke` readiness workflow.
pub const DOCTOR_SMOKE: &str = "doctor --smoke";

/// Canonical name of the direct-IPC liveness probe.
pub const HEALTH_METHOD: &str = "_health";

/// Canonical name of the direct-IPC graceful-stop control method.
pub const SHUTDOWN_METHOD: &str = "_shutdown";

// ── Registry accessors ───────────────────────────────────────────────────────

/// Build a name-keyed map of the agent-visible MCP catalog schemas.
fn catalog_schemas() -> BTreeMap<String, Arc<Map<String, Value>>> {
    tools_catalog::all_tools()
        .into_iter()
        .map(|tool| (tool.name.to_string(), Arc::clone(&tool.input_schema)))
        .collect()
}

/// Convert a JSON value into the catalog's schema representation.
///
/// A non-object value yields an empty map, which
/// [`ToolDescriptor::missing_attributes`] reports as a missing schema rather
/// than silently accepting a malformed declaration.
fn schema_map(value: Value) -> Arc<Map<String, Value>> {
    Arc::new(match value {
        Value::Object(map) => map,
        _ => Map::new(),
    })
}

/// Every declared tool descriptor for the current build.
///
/// The returned descriptors are ordered as declared: MCP catalog tools first,
/// then feature-gated dispatch tools, then the non-dispatch IPC methods and
/// workflows.
#[must_use]
pub fn all_descriptors() -> Vec<ToolDescriptor> {
    let schemas = catalog_schemas();
    DECLARATIONS
        .iter()
        .map(|declaration| {
            let input_schema = match declaration.schema {
                SchemaSource::McpCatalog => schemas
                    .get(declaration.name)
                    .map_or_else(|| Arc::new(Map::new()), Arc::clone),
                SchemaSource::Local(build) => schema_map(build()),
            };
            ToolDescriptor {
                name: declaration.name,
                capability: declaration.capability,
                input_schema,
                surfaces: declaration.surfaces.to_vec(),
                read_server_available: declaration.read_server_available,
                input_ownership: declaration.input_ownership,
            }
        })
        .collect()
}

/// Look up a single descriptor by method or workflow name.
#[must_use]
pub fn descriptor(name: &str) -> Option<ToolDescriptor> {
    all_descriptors()
        .into_iter()
        .find(|descriptor| descriptor.name == name)
}

/// Every declared name, in declaration order.
#[must_use]
pub fn declared_names() -> Vec<&'static str> {
    DECLARATIONS
        .iter()
        .map(|declaration| declaration.name)
        .collect()
}
