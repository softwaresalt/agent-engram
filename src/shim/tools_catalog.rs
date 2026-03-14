//! Static tool catalog for the shim's `tools/list` response.
//!
//! The shim proxies all tool calls to the workspace daemon, but it can answer
//! `tools/list` locally from this compile-time catalog so that MCP clients
//! (IDEs, agents) get accurate schema information before the daemon is ready
//! and without an extra round-trip.
//!
//! All 43 tools registered in [`crate::tools::dispatch`] must appear here.
//! The [`TOOL_COUNT`] constant is asserted by the `tool_count_matches_dispatch`
//! unit test so that catalog and dispatch stay in sync.

use std::sync::Arc;

use rmcp::model::Tool;
use serde_json::{Map, Value, json};

/// Total number of tools registered in the dispatch table and this catalog.
pub const TOOL_COUNT: usize = 43;

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
            "Return the current workspace status including task counts, stale files, and code graph statistics.",
            schema(json!({
                "type": "object",
                "properties": {}
            })),
        ),
        // ── Task write operations ──────────────────────────────────────────
        Tool::new(
            "create_task",
            "Create a new task in the workspace with a title, optional description, priority, and labels.",
            schema(json!({
                "type": "object",
                "properties": {
                    "title": {
                        "type": "string",
                        "description": "Short human-readable title for the task"
                    },
                    "description": {
                        "type": "string",
                        "description": "Detailed description of the task"
                    },
                    "priority": {
                        "type": "string",
                        "enum": ["critical", "high", "medium", "low"],
                        "description": "Task priority level"
                    },
                    "issue_type": {
                        "type": "string",
                        "description": "Optional issue type tag (e.g. bug, feature)"
                    },
                    "labels": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Optional list of label strings"
                    },
                    "parent_id": {
                        "type": "string",
                        "description": "Optional parent task ID for hierarchical tasks"
                    }
                },
                "required": ["title"]
            })),
        ),
        Tool::new(
            "update_task",
            "Update the status, notes, priority, or issue type of an existing task.",
            schema(json!({
                "type": "object",
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "Task ID (e.g. task:uuid)"
                    },
                    "status": {
                        "type": "string",
                        "enum": ["todo", "in_progress", "blocked", "done", "cancelled", "deferred"],
                        "description": "New status for the task"
                    },
                    "notes": {
                        "type": "string",
                        "description": "Optional notes or comment to record with the status change"
                    },
                    "priority": {
                        "type": "string",
                        "enum": ["critical", "high", "medium", "low"],
                        "description": "Updated priority level"
                    },
                    "issue_type": {
                        "type": "string",
                        "description": "Updated issue type tag"
                    }
                },
                "required": ["id", "status"]
            })),
        ),
        Tool::new(
            "add_blocker",
            "Record a blocking reason on a task, transitioning it to the blocked status.",
            schema(json!({
                "type": "object",
                "properties": {
                    "task_id": {
                        "type": "string",
                        "description": "ID of the task to mark as blocked"
                    },
                    "reason": {
                        "type": "string",
                        "description": "Human-readable description of the blocking reason"
                    }
                },
                "required": ["task_id", "reason"]
            })),
        ),
        Tool::new(
            "register_decision",
            "Persist an architectural or design decision as a context record in the workspace.",
            schema(json!({
                "type": "object",
                "properties": {
                    "topic": {
                        "type": "string",
                        "description": "Short topic or subject of the decision"
                    },
                    "decision": {
                        "type": "string",
                        "description": "Full text of the decision including rationale"
                    }
                },
                "required": ["topic", "decision"]
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
        // ── Task read operations ───────────────────────────────────────────
        Tool::new(
            "get_task_graph",
            "Return the dependency graph rooted at a task, traversed up to a configurable depth.",
            schema(json!({
                "type": "object",
                "properties": {
                    "root_task_id": {
                        "type": "string",
                        "description": "Root task ID for graph traversal"
                    },
                    "depth": {
                        "type": "integer",
                        "description": "Maximum traversal depth (default 3)",
                        "default": 3
                    }
                },
                "required": ["root_task_id"]
            })),
        ),
        Tool::new(
            "check_status",
            "Retrieve the current status and metadata for one or more tasks by ID.",
            schema(json!({
                "type": "object",
                "properties": {
                    "work_item_ids": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "List of task IDs to look up"
                    },
                    "brief": {
                        "type": "boolean",
                        "description": "Return a compact summary instead of full task details"
                    },
                    "fields": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Limit response to these specific fields"
                    }
                },
                "required": ["work_item_ids"]
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
        Tool::new(
            "get_ready_work",
            "Return tasks that have no unresolved blockers and are ready to start.",
            schema(json!({
                "type": "object",
                "properties": {
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of tasks to return (default 20)"
                    },
                    "labels": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Filter to tasks matching any of these labels"
                    },
                    "priority": {
                        "type": "string",
                        "enum": ["critical", "high", "medium", "low"],
                        "description": "Filter to tasks at or above this priority"
                    }
                }
            })),
        ),
        // ── Label / dependency management ──────────────────────────────────
        Tool::new(
            "add_label",
            "Add a label string to an existing task.",
            schema(json!({
                "type": "object",
                "properties": {
                    "task_id": {
                        "type": "string",
                        "description": "Task ID to label"
                    },
                    "label": {
                        "type": "string",
                        "description": "Label string to add"
                    }
                },
                "required": ["task_id", "label"]
            })),
        ),
        Tool::new(
            "remove_label",
            "Remove a label string from an existing task.",
            schema(json!({
                "type": "object",
                "properties": {
                    "task_id": {
                        "type": "string",
                        "description": "Task ID to remove label from"
                    },
                    "label": {
                        "type": "string",
                        "description": "Label string to remove"
                    }
                },
                "required": ["task_id", "label"]
            })),
        ),
        Tool::new(
            "add_dependency",
            "Add a directed dependency edge between two tasks (e.g. task A blocks task B).",
            schema(json!({
                "type": "object",
                "properties": {
                    "from_id": {
                        "type": "string",
                        "description": "ID of the blocking task"
                    },
                    "to_id": {
                        "type": "string",
                        "description": "ID of the task that is blocked"
                    },
                    "dependency_type": {
                        "type": "string",
                        "enum": ["blocks", "relates_to", "duplicates"],
                        "description": "Type of dependency relationship"
                    }
                },
                "required": ["from_id", "to_id"]
            })),
        ),
        // ── Compaction ─────────────────────────────────────────────────────
        Tool::new(
            "get_compaction_candidates",
            "Return tasks that are candidates for history compaction (done or cancelled with long notes).",
            schema(json!({
                "type": "object",
                "properties": {
                    "max_notes_chars": {
                        "type": "integer",
                        "description": "Tasks with notes longer than this are candidates (default 500)"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of candidates to return (default 20)"
                    }
                }
            })),
        ),
        Tool::new(
            "apply_compaction",
            "Compact the notes on a completed or cancelled task to a concise summary.",
            schema(json!({
                "type": "object",
                "properties": {
                    "task_id": {
                        "type": "string",
                        "description": "ID of the task to compact"
                    },
                    "summary": {
                        "type": "string",
                        "description": "Compact summary text to replace the full notes"
                    }
                },
                "required": ["task_id", "summary"]
            })),
        ),
        // ── Task lifecycle (claim / release / pin / defer) ─────────────────
        Tool::new(
            "claim_task",
            "Claim a task for the current agent session, transitioning it to in_progress.",
            schema(json!({
                "type": "object",
                "properties": {
                    "task_id": {
                        "type": "string",
                        "description": "ID of the task to claim"
                    }
                },
                "required": ["task_id"]
            })),
        ),
        Tool::new(
            "release_task",
            "Release a previously claimed task, returning it to todo status.",
            schema(json!({
                "type": "object",
                "properties": {
                    "task_id": {
                        "type": "string",
                        "description": "ID of the task to release"
                    },
                    "notes": {
                        "type": "string",
                        "description": "Optional notes to record on the task before releasing"
                    }
                },
                "required": ["task_id"]
            })),
        ),
        Tool::new(
            "defer_task",
            "Defer a task until a future date or condition is met.",
            schema(json!({
                "type": "object",
                "properties": {
                    "task_id": {
                        "type": "string",
                        "description": "ID of the task to defer"
                    },
                    "reason": {
                        "type": "string",
                        "description": "Reason for deferring the task"
                    }
                },
                "required": ["task_id"]
            })),
        ),
        Tool::new(
            "undefer_task",
            "Move a deferred task back to todo status, making it eligible for work again.",
            schema(json!({
                "type": "object",
                "properties": {
                    "task_id": {
                        "type": "string",
                        "description": "ID of the deferred task to reactivate"
                    }
                },
                "required": ["task_id"]
            })),
        ),
        Tool::new(
            "pin_task",
            "Pin a task so it always appears at the top of ready-work queries.",
            schema(json!({
                "type": "object",
                "properties": {
                    "task_id": {
                        "type": "string",
                        "description": "ID of the task to pin"
                    }
                },
                "required": ["task_id"]
            })),
        ),
        Tool::new(
            "unpin_task",
            "Remove the pin from a previously pinned task.",
            schema(json!({
                "type": "object",
                "properties": {
                    "task_id": {
                        "type": "string",
                        "description": "ID of the task to unpin"
                    }
                },
                "required": ["task_id"]
            })),
        ),
        // ── Statistics / batch ─────────────────────────────────────────────
        Tool::new(
            "get_workspace_statistics",
            "Return aggregate statistics for the workspace: task counts by status, label distribution, and more.",
            schema(json!({
                "type": "object",
                "properties": {}
            })),
        ),
        Tool::new(
            "batch_update_tasks",
            "Apply the same status update to multiple tasks in a single atomic operation.",
            schema(json!({
                "type": "object",
                "properties": {
                    "task_ids": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "List of task IDs to update"
                    },
                    "status": {
                        "type": "string",
                        "enum": ["todo", "in_progress", "blocked", "done", "cancelled", "deferred"],
                        "description": "New status to apply to all listed tasks"
                    },
                    "notes": {
                        "type": "string",
                        "description": "Optional notes to record on each updated task"
                    }
                },
                "required": ["task_ids", "status"]
            })),
        ),
        Tool::new(
            "add_comment",
            "Append a timestamped comment to a task's notes without changing its status.",
            schema(json!({
                "type": "object",
                "properties": {
                    "task_id": {
                        "type": "string",
                        "description": "ID of the task to comment on"
                    },
                    "comment": {
                        "type": "string",
                        "description": "Comment text to append"
                    }
                },
                "required": ["task_id", "comment"]
            })),
        ),
        // ── Code graph ────────────────────────────────────────────────────
        Tool::new(
            "index_workspace",
            "Parse and index the workspace source files into the code graph. Run once after set_workspace.",
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
            "link_task_to_code",
            "Associate a task with a source symbol so code changes can be traced back to tasks.",
            schema(json!({
                "type": "object",
                "properties": {
                    "task_id": {
                        "type": "string",
                        "description": "Task ID to link"
                    },
                    "symbol_name": {
                        "type": "string",
                        "description": "Fully-qualified symbol name to associate with the task"
                    }
                },
                "required": ["task_id", "symbol_name"]
            })),
        ),
        Tool::new(
            "unlink_task_from_code",
            "Remove the association between a task and a source symbol.",
            schema(json!({
                "type": "object",
                "properties": {
                    "task_id": {
                        "type": "string",
                        "description": "Task ID to unlink"
                    },
                    "symbol_name": {
                        "type": "string",
                        "description": "Symbol name to dissociate from the task"
                    }
                },
                "required": ["task_id", "symbol_name"]
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
            "get_active_context",
            "Return the tasks currently in_progress and recently modified context records for the active session.",
            schema(json!({
                "type": "object",
                "properties": {}
            })),
        ),
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
        // ── Event Ledger ───────────────────────────────────────────────────
        Tool::new(
            "get_event_history",
            "Return recent events from the workspace event ledger, optionally filtered by event kind or entity ID.",
            schema(json!({
                "type": "object",
                "properties": {
                    "kind": {
                        "type": "string",
                        "description": "Filter events by kind (e.g. task_created, task_updated, rollback_applied)"
                    },
                    "entity_id": {
                        "type": "string",
                        "description": "Filter events by the ID of the affected entity"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of events to return (default 50)",
                        "default": 50
                    }
                }
            })),
        ),
        Tool::new(
            "rollback_to_event",
            "Revert workspace state to a previous event snapshot by reversing all events recorded after the target event. Requires allow_agent_rollback to be enabled in workspace config.",
            schema(json!({
                "type": "object",
                "properties": {
                    "event_id": {
                        "type": "string",
                        "description": "ID of the event to roll back to (all later events are reversed)"
                    }
                },
                "required": ["event_id"]
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
        // ── Collections ────────────────────────────────────────────────────
        Tool::new(
            "create_collection",
            "Create a named collection to group tasks and sub-collections into a logical hierarchy.",
            schema(json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Unique name for the collection"
                    },
                    "description": {
                        "type": "string",
                        "description": "Optional human-readable description of the collection's purpose"
                    }
                },
                "required": ["name"]
            })),
        ),
        Tool::new(
            "add_to_collection",
            "Add tasks or sub-collections as members of a collection. Cycle detection prevents circular containment hierarchies.",
            schema(json!({
                "type": "object",
                "properties": {
                    "collection_id": {
                        "type": "string",
                        "description": "ID of the parent collection to add members to"
                    },
                    "member_ids": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "IDs of tasks or collections to add as members"
                    }
                },
                "required": ["collection_id", "member_ids"]
            })),
        ),
        Tool::new(
            "remove_from_collection",
            "Remove tasks or sub-collections from a collection.",
            schema(json!({
                "type": "object",
                "properties": {
                    "collection_id": {
                        "type": "string",
                        "description": "ID of the collection to remove members from"
                    },
                    "member_ids": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "IDs of tasks or collections to remove"
                    }
                },
                "required": ["collection_id", "member_ids"]
            })),
        ),
        Tool::new(
            "get_collection_context",
            "Return all tasks recursively contained in a collection hierarchy, with an optional status filter for narrowing results.",
            schema(json!({
                "type": "object",
                "properties": {
                    "collection_id": {
                        "type": "string",
                        "description": "ID of the collection to expand"
                    },
                    "status_filter": {
                        "type": "string",
                        "description": "Optional task status to filter by (e.g. in_progress, todo, done)"
                    }
                },
                "required": ["collection_id"]
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
            "create_task",
            "update_task",
            "add_blocker",
            "register_decision",
            "flush_state",
            "get_task_graph",
            "check_status",
            "query_memory",
            "get_ready_work",
            "add_label",
            "remove_label",
            "add_dependency",
            "get_compaction_candidates",
            "apply_compaction",
            "claim_task",
            "release_task",
            "defer_task",
            "undefer_task",
            "pin_task",
            "unpin_task",
            "get_workspace_statistics",
            "batch_update_tasks",
            "add_comment",
            "index_workspace",
            "sync_workspace",
            "link_task_to_code",
            "unlink_task_from_code",
            "map_code",
            "list_symbols",
            "get_active_context",
            "unified_search",
            "impact_analysis",
            // Phase 005 additions
            "get_health_report",
            "get_event_history",
            "rollback_to_event",
            "query_graph",
            "create_collection",
            "add_to_collection",
            "remove_from_collection",
            "get_collection_context",
        ];
        assert_eq!(
            required.len(),
            TOOL_COUNT,
            "required array must match TOOL_COUNT"
        );
        for name in required {
            assert!(names.contains(name), "missing tool: {name}");
        }
    }
}
