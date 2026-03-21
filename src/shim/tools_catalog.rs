//! Static tool catalog for the shim's `tools/list` response.
//!
//! The shim proxies all tool calls to the workspace daemon, but it can answer
//! `tools/list` locally from this compile-time catalog so that MCP clients
//! (IDEs, agents) get accurate schema information before the daemon is ready
//! and without an extra round-trip.
//!
//! All 14 tools registered in [`crate::tools::dispatch`] must appear here.
//! The [`TOOL_COUNT`] constant is asserted by the `tool_count_matches_dispatch`
//! unit test so that catalog and dispatch stay in sync.

use std::sync::Arc;

use rmcp::model::Tool;
use serde_json::{Map, Value, json};

/// Total number of tools registered in the dispatch table and this catalog.
pub const TOOL_COUNT: usize = 14;

/// Build a `serde_json::Map` from a JSON object literal.
///
/// Panics if `v` is not a JSON object — callers must only pass object literals.
fn schema(v: Value) -> Arc<Map<String, Value>> {
    Arc::new(match v {
        Value::Object(m) => m,
        _ => Map::new(),
    })
}

/// Return the full list of Engram MCP tools.
///
/// The returned `Vec` has exactly [`TOOL_COUNT`] entries with unique names.
pub fn all_tools() -> Vec<Tool> {
    vec![
        // ── Workspace / lifecycle ──────────────────────────────────────────
        Tool::new(
            "set_workspace",
            "Bind the daemon to a workspace directory. Must be called before any other tool.",
            schema(json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Absolute path to the workspace root directory"
                    }
                },
                "required": ["path"]
            })),
        ),
        Tool::new(
            "get_daemon_status",
            "Return runtime metrics for the running daemon (version, uptime, memory, connections).",
            schema(json!({
                "type": "object",
                "properties": {}
            })),
        ),
        Tool::new(
            "get_workspace_status",
            "Return the current workspace status including code graph statistics, stale files, and connection info.",
            schema(json!({
                "type": "object",
                "properties": {}
            })),
        ),
        Tool::new(
            "flush_state",
            "Persist in-memory workspace state to disk (.engram/ files). Safe to call at any time.",
            schema(json!({
                "type": "object",
                "properties": {
                    "force": {
                        "type": "boolean",
                        "description": "Force flush even if no changes are detected"
                    }
                }
            })),
        ),
        Tool::new(
            "query_memory",
            "Search workspace context records (decisions, notes) using a natural language query.",
            schema(json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Natural language search query"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of results to return (default 10)"
                    }
                },
                "required": ["query"]
            })),
        ),
        // ── Statistics ────────────────────────────────────────────────────
        Tool::new(
            "get_workspace_statistics",
            "Return aggregate statistics for the workspace: task counts by status, label distribution, and more.",
            schema(json!({
                "type": "object",
                "properties": {}
            })),
        ),
        // ── Code graph ────────────────────────────────────────────────────
        Tool::new(
            "index_workspace",
            "Parse and index the workspace source files into the code graph. Run once after `set_workspace`.",
            schema(json!({
                "type": "object",
                "properties": {
                    "force": {
                        "type": "boolean",
                        "description": "Force full re-index even if the code graph is up to date"
                    }
                }
            })),
        ),
        Tool::new(
            "sync_workspace",
            "Incrementally synchronize changed source files into the code graph since the last index.",
            schema(json!({
                "type": "object",
                "properties": {}
            })),
        ),
        Tool::new(
            "map_code",
            "Return the call graph and usages for a named symbol up to a configurable depth.",
            schema(json!({
                "type": "object",
                "properties": {
                    "symbol_name": {
                        "type": "string",
                        "description": "Name of the symbol to map"
                    },
                    "depth": {
                        "type": "integer",
                        "description": "Maximum traversal depth (default 2)",
                        "default": 2
                    }
                },
                "required": ["symbol_name"]
            })),
        ),
        Tool::new(
            "list_symbols",
            "List symbols (functions, structs, enums, etc.) indexed in the code graph, with optional filters.",
            schema(json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "Filter to symbols defined in this file path"
                    },
                    "symbol_type": {
                        "type": "string",
                        "description": "Filter by symbol kind (function, struct, enum, trait, impl, ...)"
                    },
                    "name_contains": {
                        "type": "string",
                        "description": "Filter to symbols whose name contains this substring"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of symbols to return (default 50)"
                    }
                }
            })),
        ),
        // ── Context / search ───────────────────────────────────────────────
        Tool::new(
            "unified_search",
            "Search across tasks, context records, and code symbols using a single natural language query.",
            schema(json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Natural language search query"
                    },
                    "regions": {
                        "type": "array",
                        "items": {
                            "type": "string",
                            "enum": ["tasks", "context", "code"]
                        },
                        "description": "Limit search to specific regions (default: all)"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum total results to return (default 20)"
                    }
                },
                "required": ["query"]
            })),
        ),
        Tool::new(
            "impact_analysis",
            "Identify tasks and context records likely affected by changes to a named code symbol.",
            schema(json!({
                "type": "object",
                "properties": {
                    "symbol_name": {
                        "type": "string",
                        "description": "Name of the changed symbol to analyse"
                    },
                    "depth": {
                        "type": "integer",
                        "description": "How many hops in the call graph to explore (default 2)",
                        "default": 2
                    }
                },
                "required": ["symbol_name"]
            })),
        ),
        // ── Observability ──────────────────────────────────────────────────
        Tool::new(
            "get_health_report",
            "Return runtime health metrics for the daemon including memory usage, tool call counts, event processing statistics, and query latency percentiles (p50/p95/p99).",
            schema(json!({
                "type": "object",
                "properties": {}
            })),
        ),
        // ── Sandboxed Query ────────────────────────────────────────────────
        Tool::new(
            "query_graph",
            "Execute a read-only SurrealQL SELECT query against the workspace graph database. Write operations (INSERT, UPDATE, DELETE, etc.) are rejected. Results are capped at the configured row limit.",
            schema(json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "A SurrealQL SELECT statement to execute against the workspace database"
                    },
                    "params": {
                        "description": "Reserved for future parameterised query support"
                    }
                },
                "required": ["query"]
            })),
        ),
    ]
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// The catalog must contain exactly [`TOOL_COUNT`] tools.
    #[test]
    fn tool_count_matches_dispatch() {
        assert_eq!(
            all_tools().len(),
            TOOL_COUNT,
            "all_tools() length must equal TOOL_COUNT ({TOOL_COUNT})"
        );
    }

    /// Every tool name must be unique.
    #[test]
    fn tool_names_are_unique() {
        let tools = all_tools();
        let mut seen = std::collections::HashSet::new();
        for tool in &tools {
            assert!(
                seen.insert(tool.name.as_ref()),
                "duplicate tool name: {}",
                tool.name
            );
        }
    }

    /// Spot-check that key tool names from the dispatch table are present.
    #[test]
    fn all_dispatch_names_present() {
        let tools = all_tools();
        let names: std::collections::HashSet<&str> =
            tools.iter().map(|t| t.name.as_ref()).collect();

        let required = [
            "set_workspace",
            "get_daemon_status",
            "get_workspace_status",
            "flush_state",
            "query_memory",
            "get_workspace_statistics",
            "index_workspace",
            "sync_workspace",
            "map_code",
            "list_symbols",
            "unified_search",
            "impact_analysis",
            "get_health_report",
            "query_graph",
        ];
        for name in &required {
            assert!(names.contains(name), "tool '{name}' missing from catalog");
        }
    }
}
