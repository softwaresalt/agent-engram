//! Canonical tool descriptor schema (plan unit F19, 142.005-T / 134-S).
//!
//! This module defines the descriptor attribute schema every Engram method or
//! workflow is declared against — the facts a caller-side gate needs before
//! dispatch:
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
//! A descriptor only *records* today's reachability. It does not gate, refuse,
//! or route anything on its own — wiring consumers (request entry, dispatch
//! enforcement, catalog derivation) belongs to later plan units. Adding or
//! changing a surface here is a declaration change, never a behavior change.
//!
//! The tool declarations themselves are populated against this schema in
//! 142.005.002-ST; this unit defines only the shape.

use std::sync::Arc;

use serde_json::{Map, Value};

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
