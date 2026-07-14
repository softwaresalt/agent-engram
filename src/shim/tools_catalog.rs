//! Static tool catalog for the shim's `tools/list` response.
//!
//! The shim proxies all tool calls to the workspace daemon, but it can answer
//! `tools/list` locally from this compile-time catalog so that MCP clients
//! (IDEs, agents) get accurate schema information before the daemon is ready
//! and without an extra round-trip.
//!
//! All tools in the **default feature set** (`cozo-backend`) that are registered
//! in [`crate::tools::dispatch`] must appear here. Feature-gated tools (e.g.,
//! those compiled only under `cfg(feature = "git-graph")`) are intentionally
//! excluded; [`TOOL_COUNT`] and this catalog reflect only the default build.
//! The [`TOOL_COUNT`] constant is asserted by the `tool_count_matches_catalog`
//! contract test so that catalog and dispatch stay in sync.

use std::sync::Arc;

use rmcp::model::Tool;
use serde_json::{Map, Value, json};

/// Total number of tools registered in the dispatch table and this catalog.
pub const TOOL_COUNT: usize = 21;

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
                    },
                    "content_type": {
                        "type": "string",
                        "description": "Filter by content type (e.g. spec, docs, tests, backlog)"
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
                        "description": "Maximum traversal depth (default 1)",
                        "default": 1
                    },
                    "max_nodes": {
                        "type": "integer",
                        "description": "Maximum number of graph nodes to return (default 50)",
                        "default": 50
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
                    "node_type": {
                        "type": "string",
                        "description": "Filter by symbol kind (function, struct, enum, trait, impl, ...)"
                    },
                    "name_prefix": {
                        "type": "string",
                        "description": "Filter to symbols whose name starts with this prefix"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of symbols to return (default 50)"
                    },
                    "offset": {
                        "type": "integer",
                        "description": "Pagination offset (default 0)"
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
                    "region": {
                        "type": "string",
                        "enum": ["all", "code"],
                        "default": "all",
                        "description": "Limit search to a specific region (default: all). 'code' restricts to code symbols only; 'all' searches across all available sources."
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum total results to return (default 10)"
                    },
                    "content_type": {
                        "type": "string",
                        "description": "Filter context results by content type (e.g. spec, docs, tests)"
                    },
                    "scope_to_symbol": {
                        "type": "string",
                        "description": "Restrict code symbol results to the graph neighbourhood of this symbol"
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
                        "description": "How many hops in the call graph to explore (default 1)",
                        "default": 1
                    },
                    "max_nodes": {
                        "type": "integer",
                        "description": "Maximum number of graph nodes to return (default 50)",
                        "default": 50
                    },
                    "concept": {
                        "type": "string",
                        "description": "Optional semantic concept to narrow the analysis"
                    }
                },
                "required": ["symbol_name"]
            })),
        ),
        Tool::new(
            "lint_dax",
            "Lint the DAX in the bound workspace's indexed Power BI model(s), returning { conformant, findings[] }. Applies Tier-1 syntactic rules and Tier-2 schema-aware rules (broken column/measure refs, unqualified-column and qualified-measure style findings, and measure-to-measure cycles) by reparsing measure and calculated-column expressions against the model-scope-aggregated schema.",
            schema(json!({
                "type": "object",
                "properties": {
                    "model_path": {
                        "type": "string",
                        "description": "Optional TMDL model path, canonicalised to one model scope. Omitted lints every indexed model; a path matching no indexed model is an error."
                    }
                }
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
        Tool::new(
            "get_branch_metrics",
            "Return the persisted metrics summary for a branch, or compare two branches.",
            schema(json!({
                "type": "object",
                "properties": {
                    "branch_name": {
                        "type": "string",
                        "description": "Branch to summarize; defaults to the current branch"
                    },
                    "compare_to": {
                        "type": "string",
                        "description": "Optional second branch to compare against"
                    }
                }
            })),
        ),
        Tool::new(
            "get_token_savings_report",
            "Return a concise text summary of the current branch's tracked token delivery.",
            schema(json!({
                "type": "object",
                "properties": {}
            })),
        ),
        Tool::new(
            "get_evaluation_report",
            "Compute an agent efficiency evaluation report from recorded usage events. Returns per-agent scoring, anomaly flags (token ratio spikes, error bursts, tool hammering), and actionable recommendations.",
            schema(json!({
                "type": "object",
                "properties": {}
            })),
        ),
        // ── Retrieval + graph-recall evaluation (081-F) ────────────────────
        Tool::new(
            "run_retrieval_eval",
            "Run the portable retrieval + graph-recall evaluation over the indexed workspace. Derives ground truth automatically (semantic self-retrieval from indexed function docstrings, falling back to the function name; graph resolution recall from the parser call-site inventory) and returns a structured RetrievalEvalReport with semantic (precision@k, recall@k, MRR, nDCG) and graph (resolution_recall, false_edge_rate) metrics. Evaluates functions only — not arbitrary symbols or qualified-name uniqueness. Disabled by default; returns an empty report unless enabled via the [retrieval_eval] config section. Distinct from get_evaluation_report (agent efficiency).",
            schema(json!({
                "type": "object",
                "properties": {}
            })),
        ),
        Tool::new(
            "get_retrieval_eval_report",
            "Return the latest persisted retrieval + graph-recall evaluation report (RetrievalEvalReport) for the current branch, or a well-formed empty report when no run exists. Distinct from get_evaluation_report (agent efficiency).",
            schema(json!({
                "type": "object",
                "properties": {}
            })),
        ),
        // ── Sandboxed Query ────────────────────────────────────────────────
        Tool::new(
            "query_graph",
            "Execute a structured graph query against the workspace code and backlog graph. \
             Three operations are supported: \
             `neighborhood` (BFS from a root node), \
             `find_path` (shortest path between two nodes), and \
             `transitive_closure` (all nodes reachable from a root). \
             Edge types: code (`calls`, `imports`, `defines`, `inherits_from`, `concerns`, `references`) \
             and backlog (`parent_of`, `depends_on`, `backlog_references`). \
             Results are capped at 500 nodes.",
            schema(json!({
                "type": "object",
                "properties": {
                    "operation": {
                        "type": "string",
                        "enum": ["neighborhood", "find_path", "transitive_closure"],
                        "description": "Graph operation to execute"
                    },
                    "root": {
                        "type": "string",
                        "description": "Root node ID for `neighborhood` and `transitive_closure` (e.g. `fn:abc123`)"
                    },
                    "from": {
                        "type": "string",
                        "description": "Start node ID for `find_path`"
                    },
                    "to": {
                        "type": "string",
                        "description": "End node ID for `find_path`"
                    },
                    "direction": {
                        "type": "string",
                        "enum": ["both", "outgoing", "incoming"],
                        "description": "Traversal direction for `neighborhood` — defaults to `both`"
                    },
                    "max_depth": {
                        "type": "integer",
                        "description": "Maximum hop depth — defaults to 3"
                    },
                    "max_nodes": {
                        "type": "integer",
                        "description": "Maximum result nodes for `neighborhood` and `transitive_closure` — defaults to 50, hard-capped at 500"
                    },
                    "edge_types": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Edge types to traverse — empty array means all types"
                    }
                },
                "required": ["operation"]
            })),
        ),
        // ── DB observability ──────────────────────────────────────────────
        Tool::new(
            "get_mutable_script_retry_metrics",
            "Return mutable-script SQLITE_BUSY retry telemetry: monotonic retry count and timestamp of the most recent retry. Does not require a workspace to be bound.",
            schema(json!({
                "type": "object",
                "properties": {}
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
            "get_branch_metrics",
            "get_token_savings_report",
            "get_evaluation_report",
            "query_graph",
            "get_mutable_script_retry_metrics",
        ];
        for name in &required {
            assert!(names.contains(name), "tool '{name}' missing from catalog");
        }
    }
}
