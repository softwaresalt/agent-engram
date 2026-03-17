# Engram MCP Tool Reference

This reference documents every MCP tool registered by the Engram daemon. Tools are organized by category. Each entry describes the tool's purpose, parameters, return schema, error codes, and an example request.

All tools are called via the MCP JSON-RPC protocol over the HTTP/SSE endpoint (`http://localhost:7437/sse`).

---

## Table of Contents

1. [Lifecycle Tools](#lifecycle-tools)
   - [set_workspace](#set_workspace)
   - [get_daemon_status](#get_daemon_status)
   - [get_workspace_status](#get_workspace_status)
2. [Read Tools](#read-tools)
   - [query_memory](#query_memory)
   - [unified_search](#unified_search)
   - [get_task_graph](#get_task_graph)
   - [check_status](#check_status)
   - [get_ready_work](#get_ready_work)
   - [get_compaction_candidates](#get_compaction_candidates)
   - [map_code](#map_code)
   - [list_symbols](#list_symbols)
   - [get_active_context](#get_active_context)
   - [impact_analysis](#impact_analysis)
   - [get_health_report](#get_health_report)
   - [get_event_history](#get_event_history)
   - [query_graph](#query_graph)
   - [get_collection_context](#get_collection_context)
   - [get_workspace_statistics](#get_workspace_statistics)
3. [Write Tools](#write-tools)
   - [create_task](#create_task)
   - [update_task](#update_task)
   - [add_blocker](#add_blocker)
   - [register_decision](#register_decision)
   - [flush_state](#flush_state)
   - [add_label](#add_label)
   - [remove_label](#remove_label)
   - [add_dependency](#add_dependency)
   - [apply_compaction](#apply_compaction)
   - [claim_task](#claim_task)
   - [release_task](#release_task)
   - [defer_task](#defer_task)
   - [undefer_task](#undefer_task)
   - [pin_task](#pin_task)
   - [unpin_task](#unpin_task)
   - [batch_update_tasks](#batch_update_tasks)
   - [add_comment](#add_comment)
   - [index_workspace](#index_workspace)
   - [sync_workspace](#sync_workspace)
   - [link_task_to_code](#link_task_to_code)
   - [unlink_task_from_code](#unlink_task_from_code)
   - [rollback_to_event](#rollback_to_event)
   - [create_collection](#create_collection)
   - [add_to_collection](#add_to_collection)
   - [remove_from_collection](#remove_from_collection)
4. [Git Graph Tools](#git-graph-tools)
   - [query_changes](#query_changes)
   - [index_git_history](#index_git_history)

---

## Error Code Quick Reference

| Code | Name | Category |
|---|---|---|
| 1001 | `WORKSPACE_NOT_FOUND` | Workspace |
| 1002 | `NOT_A_GIT_ROOT` | Workspace |
| 1003 | `WORKSPACE_NOT_SET` | Workspace |
| 1004 | `WORKSPACE_ALREADY_ACTIVE` | Workspace |
| 1005 | `WORKSPACE_LIMIT_REACHED` | Workspace |
| 2001 | `HYDRATION_FAILED` | Hydration |
| 2002 | `SCHEMA_MISMATCH` | Hydration |
| 2003 | `CORRUPTED_STATE` | Hydration |
| 2004 | `STALE_WORKSPACE` | Hydration |
| 3001 | `TASK_NOT_FOUND` | Task |
| 3002 | `INVALID_STATUS` | Task |
| 3003 | `CYCLIC_DEPENDENCY` | Task |
| 3004 | `BLOCKER_EXISTS` | Task |
| 3005 | `TASK_ALREADY_CLAIMED` | Task |
| 3006 | `LABEL_VALIDATION` | Task |
| 3007 | `BATCH_PARTIAL_FAILURE` | Task |
| 3008 | `COMPACTION_FAILED` | Task |
| 3009 | `INVALID_PRIORITY` | Task |
| 3010 | `INVALID_ISSUE_TYPE` | Task |
| 3011 | `DUPLICATE_LABEL` | Task |
| 3012 | `TASK_NOT_CLAIMABLE` | Task |
| 3013 | `TASK_TITLE_EMPTY` | Task |
| 3014 | `TASK_TITLE_TOO_LONG` | Task |
| 3015 | `TASK_BLOCKED` | Task |
| 3020 | `ROLLBACK_DENIED` | Task |
| 3021 | `EVENT_NOT_FOUND` | Task |
| 3022 | `ROLLBACK_CONFLICT` | Task |
| 3030 | `COLLECTION_EXISTS` | Collection |
| 3031 | `COLLECTION_NOT_FOUND` | Collection |
| 3032 | `CYCLIC_COLLECTION` | Collection |
| 4001 | `QUERY_TOO_LONG` | Query |
| 4002 | `MODEL_NOT_LOADED` | Query |
| 4003 | `SEARCH_FAILED` | Query |
| 4004 | `QUERY_EMPTY` | Query |
| 4010 | `QUERY_REJECTED` | Query |
| 4011 | `QUERY_TIMEOUT` | Query |
| 4012 | `QUERY_INVALID` | Query |
| 5001 | `DATABASE_ERROR` | System |
| 5003 | `RATE_LIMITED` | System |
| 5004 | `SHUTTING_DOWN` | System |
| 5005 | `INVALID_PARAMS` | System |
| 7001 | `PARSE_ERROR` | Code Graph |
| 7002 | `UNSUPPORTED_LANGUAGE` | Code Graph |
| 7003 | `INDEX_IN_PROGRESS` | Code Graph |
| 7004 | `SYMBOL_NOT_FOUND` | Code Graph |
| 7006 | `FILE_TOO_LARGE` | Code Graph |
| 7007 | `SYNC_CONFLICT` | Code Graph |
| 10001 | `REGISTRY_PARSE_FAILED` | Registry |
| 10002 | `REGISTRY_VALIDATION_FAILED` | Registry |
| 11001 | `INGESTION_FAILED` | Ingestion |
| 12001 | `GIT_NOT_FOUND` | Git Graph |
| 12002 | `GIT_ACCESS_ERROR` | Git Graph |

---

## Lifecycle Tools

These tools manage workspace binding and daemon status. They do not require an active workspace (except `get_workspace_status`).

---

### `set_workspace`

Bind the daemon to a workspace directory. Parses and hydrates `.engram/` files into the embedded database, making all task and context data available for querying.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `path` | `string` | Yes | Absolute path to the git repository root. Must contain a `.git/` directory. |

**Returns**

```json
{
  "workspace_id": "string",   // SHA-derived workspace identifier
  "path": "string",           // canonicalized absolute path
  "task_count": 0,            // number of tasks loaded
  "hydrated": true            // always true on success
}
```

**Error Codes**

| Code | Condition |
|---|---|
| `1002` | Path is not a git repository root |
| `1005` | Daemon has reached `max_workspaces` limit |
| `2001` | `.engram/` files could not be parsed |
| `2002` | Schema version mismatch in stored data |
| `5005` | `path` parameter missing or invalid type |

**Example**

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "set_workspace",
    "arguments": {
      "path": "/home/user/my-project"
    }
  }
}
```

---

### `get_daemon_status`

Returns runtime information about the daemon process: version, uptime, active workspaces, memory usage, and embedding model state.

**Parameters**: None

**Returns**

```json
{
  "version": "0.1.0",
  "uptime_seconds": 3600,
  "active_workspaces": 1,
  "active_connections": 2,
  "memory_bytes": 104857600,
  "model_loaded": true,
  "model_name": "nomic-embed-text"
}
```

**Error Codes**: None (always succeeds while daemon is running)

**Example**

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "get_daemon_status",
    "arguments": {}
  }
}
```

---

### `get_workspace_status`

Returns the current state of the bound workspace: task count, context count, last flush time, staleness, and code graph statistics.

**Parameters**: None

**Returns**

```json
{
  "path": "/home/user/my-project",
  "task_count": 42,
  "context_count": 5,
  "last_flush": "2024-01-15T10:30:00Z",
  "stale_files": false,
  "connection_count": 1,
  "code_graph": {
    "code_files": 45,
    "functions": 312,
    "classes": 28,
    "interfaces": 14,
    "edges": 891
  }
}
```

**Error Codes**

| Code | Condition |
|---|---|
| `1003` | No workspace has been set via `set_workspace` |

**Example**

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "get_workspace_status",
    "arguments": {}
  }
}
```

---

## Read Tools

These tools query workspace data without modifying state.

---

### `query_memory`

Semantic similarity search over workspace memory: tasks, context records, and spec content. Uses vector embeddings to find results semantically related to the query string.

**Parameters**

| Name | Type | Required | Default | Description |
|---|---|---|---|---|
| `query` | `string` | Yes | — | Natural language search query |
| `limit` | `integer` | No | `10` | Maximum number of results to return |
| `content_type` | `string` | No | `null` | Filter by content type (e.g., `"spec"`, `"docs"`, `"tests"`) |

**Returns**: Array of ranked memory records with similarity scores.

**Error Codes**

| Code | Condition |
|---|---|
| `1003` | No workspace set |
| `4002` | Embedding model not loaded |
| `4003` | Search operation failed |
| `4004` | Query string is empty |

**Example**

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "query_memory",
    "arguments": {
      "query": "authentication flow",
      "limit": 5,
      "content_type": "spec"
    }
  }
}
```

---

### `unified_search`

Cross-domain semantic search that queries tasks, context/specs, and code symbols simultaneously, returning ranked results from all domains in a single response.

**Parameters**

| Name | Type | Required | Default | Description |
|---|---|---|---|---|
| `query` | `string` | Yes | — | Natural language search query |
| `region` | `string` | No | `"all"` | Search scope: `"all"`, `"tasks"`, `"context"`, or `"code"` |
| `limit` | `integer` | No | `10` | Maximum results per domain |
| `content_type` | `string` | No | `null` | Filter content records by type when `region` includes context |

**Returns**: Ranked array of results with `kind` (`"task"`, `"context"`, `"code"`), `id`, `title`/`name`, and `score`.

**Error Codes**

| Code | Condition |
|---|---|
| `1003` | No workspace set |
| `4002` | Embedding model not loaded |
| `4003` | Search failed |
| `4004` | Query is empty |

**Example**

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "unified_search",
    "arguments": {
      "query": "error handling in auth module",
      "region": "all",
      "limit": 10
    }
  }
}
```

---

### `get_task_graph`

Returns a dependency graph rooted at the specified task, traversed to a configurable depth. Useful for understanding how tasks relate to each other and what must complete before a given task can start.

**Parameters**

| Name | Type | Required | Default | Description |
|---|---|---|---|---|
| `root_task_id` | `string` | Yes | — | ID of the root task (e.g., `"task:abc123"`) |
| `depth` | `integer` | No | `3` | Traversal depth from the root task |

**Returns**: Graph object with nodes (tasks) and edges (dependency relationships).

**Error Codes**

| Code | Condition |
|---|---|
| `1003` | No workspace set |
| `3001` | Root task not found |

**Example**

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "get_task_graph",
    "arguments": {
      "root_task_id": "task:abc123",
      "depth": 2
    }
  }
}
```

---

### `check_status`

Returns the current status of one or more tasks by ID. Efficient batch status lookup for agents tracking work item progress.

**Parameters**

| Name | Type | Required | Default | Description |
|---|---|---|---|---|
| `work_item_ids` | `string[]` | Yes | — | Array of task IDs to check |
| `brief` | `boolean` | No | `false` | Return compact representation |
| `fields` | `string[]` | No | `null` | Return only specified fields |

**Returns**: Array of task status records.

**Error Codes**

| Code | Condition |
|---|---|
| `1003` | No workspace set |
| `3001` | One or more task IDs not found |

**Example**

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "check_status",
    "arguments": {
      "work_item_ids": ["task:abc123", "task:def456"]
    }
  }
}
```

---

### `get_ready_work`

Returns tasks that are eligible to start: no incomplete hard blockers, not claimed, not deferred, and in a startable status. Supports filtering by label, priority, issue type, and assignee.

**Parameters**

| Name | Type | Required | Default | Description |
|---|---|---|---|---|
| `limit` | `integer` | No | `10` | Maximum tasks to return |
| `label` | `string[]` | No | `null` | Filter: task must have all specified labels |
| `priority` | `string` | No | `null` | Filter by priority (`"P0"` through `"P5"`) |
| `issue_type` | `string` | No | `null` | Filter by type (`"feature"`, `"bug"`, `"chore"`, etc.) |
| `assignee` | `string` | No | `null` | Filter by assigned agent/user |
| `brief` | `boolean` | No | `false` | Return compact representation |
| `fields` | `string[]` | No | `null` | Return only specified fields |

**Returns**: Array of ready tasks ordered by priority.

**Error Codes**

| Code | Condition |
|---|---|
| `1003` | No workspace set |

**Example**

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "get_ready_work",
    "arguments": {
      "limit": 5,
      "priority": "P0",
      "label": ["backend"]
    }
  }
}
```

---

### `get_compaction_candidates`

Returns tasks that are candidates for compaction: completed tasks older than a threshold that can be summarized and removed from the active ledger to reduce workspace size.

**Parameters**

| Name | Type | Required | Default | Description |
|---|---|---|---|---|
| `threshold_days` | `integer` | No | `null` | Only return tasks completed more than N days ago |
| `max_candidates` | `integer` | No | `null` | Maximum number of candidates to return |

**Returns**: Array of compaction candidate tasks.

**Error Codes**

| Code | Condition |
|---|---|
| `1003` | No workspace set |

**Example**

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "get_compaction_candidates",
    "arguments": {
      "threshold_days": 30,
      "max_candidates": 50
    }
  }
}
```

---

### `map_code`

Returns a call/reference graph centered on a named code symbol, traversed to a configurable depth. Use this to understand a function's callers and callees or a type's usages.

**Parameters**

| Name | Type | Required | Default | Description |
|---|---|---|---|---|
| `symbol_name` | `string` | Yes | — | Name of the symbol to map (function, class, interface) |
| `depth` | `integer` | No | `2` | Traversal depth from the symbol |
| `max_nodes` | `integer` | No | `50` | Maximum graph nodes to return |

**Returns**: Graph with symbol nodes and relationship edges.

**Error Codes**

| Code | Condition |
|---|---|
| `1003` | No workspace set |
| `7004` | Symbol not found in code graph |

**Example**

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "map_code",
    "arguments": {
      "symbol_name": "handle_auth_error",
      "depth": 3,
      "max_nodes": 30
    }
  }
}
```

---

### `list_symbols`

Lists code symbols indexed in the code graph, with optional filters for file, node type, and name prefix. Supports pagination via `offset`.

**Parameters**

| Name | Type | Required | Default | Description |
|---|---|---|---|---|
| `file_path` | `string` | No | `null` | Filter to symbols in a specific file |
| `node_type` | `string` | No | `null` | Filter by node type: `"function"`, `"class"`, `"interface"`, `"file"` |
| `name_prefix` | `string` | No | `null` | Filter to symbols whose name starts with this prefix |
| `limit` | `integer` | No | `100` | Maximum symbols to return |
| `offset` | `integer` | No | `0` | Pagination offset |

**Returns**: Array of symbol records.

**Error Codes**

| Code | Condition |
|---|---|
| `1003` | No workspace set |

**Example**

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "list_symbols",
    "arguments": {
      "file_path": "src/auth/mod.rs",
      "node_type": "function",
      "limit": 20
    }
  }
}
```

---

### `get_active_context`

Returns the current active context for the workspace: relevant tasks, recent decisions, and pinned items that an agent should be aware of before starting work.

**Parameters**: None

**Returns**: Active context bundle including pinned tasks, recent decisions, and high-priority ready work.

**Error Codes**

| Code | Condition |
|---|---|
| `1003` | No workspace set |

**Example**

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "get_active_context",
    "arguments": {}
  }
}
```

---

### `impact_analysis`

Estimates the blast radius of a change to a code symbol: which tasks are linked to files that call or reference the symbol, and which other symbols depend on it.

**Parameters**

| Name | Type | Required | Default | Description |
|---|---|---|---|---|
| `symbol_name` | `string` | Yes | — | Symbol whose change impact is being analyzed |
| `depth` | `integer` | No | `2` | Graph traversal depth |
| `status_filter` | `string` | No | `null` | Only include linked tasks with this status |
| `max_nodes` | `integer` | No | `50` | Maximum nodes in the impact graph |

**Returns**: Impact graph with affected symbols, files, and linked tasks.

**Error Codes**

| Code | Condition |
|---|---|
| `1003` | No workspace set |
| `7004` | Symbol not found |

**Example**

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "impact_analysis",
    "arguments": {
      "symbol_name": "AuthService",
      "depth": 3,
      "status_filter": "in-progress"
    }
  }
}
```

---

### `get_health_report`

Returns a structured health report for the workspace: task distribution by status, blocker count, stale file status, and recent tool latency statistics.

**Parameters**: None

**Returns**: Health report object.

**Error Codes**

| Code | Condition |
|---|---|
| `1003` | No workspace set |

---

### `get_event_history`

Returns the rolling event ledger for the current workspace, optionally filtered by event kind or entity ID.

**Parameters**

| Name | Type | Required | Default | Description |
|---|---|---|---|---|
| `kind` | `string` | No | `null` | Filter by event kind (e.g., `"task_created"`, `"task_updated"`) |
| `entity_id` | `string` | No | `null` | Filter to events affecting a specific entity ID |
| `limit` | `integer` | No | `50` | Maximum events to return |

**Returns**: Array of event records in reverse-chronological order.

**Error Codes**

| Code | Condition |
|---|---|
| `1003` | No workspace set |

**Example**

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "get_event_history",
    "arguments": {
      "kind": "task_updated",
      "limit": 20
    }
  }
}
```

---

### `query_graph`

Execute a sandboxed SurrealQL SELECT query against the workspace database. Subject to `ENGRAM_QUERY_TIMEOUT_MS` and `ENGRAM_QUERY_ROW_LIMIT` constraints. Only SELECT statements are permitted.

**Parameters**

| Name | Type | Required | Default | Description |
|---|---|---|---|---|
| `query` | `string` | Yes | — | SurrealQL SELECT statement |
| `params` | `object` | No | `null` | Reserved for future parameterized queries |

**Returns**: Array of result rows.

**Error Codes**

| Code | Condition |
|---|---|
| `1003` | No workspace set |
| `4010` | Query was rejected (non-SELECT statement) |
| `4011` | Query exceeded `query_timeout_ms` |
| `4012` | Query is syntactically invalid |

**Example**

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "query_graph",
    "arguments": {
      "query": "SELECT id, title, status FROM tasks WHERE status = 'in-progress' LIMIT 10"
    }
  }
}
```

---

### `get_collection_context`

Returns all tasks and context records belonging to a named collection, optionally filtered by task status.

**Parameters**

| Name | Type | Required | Default | Description |
|---|---|---|---|---|
| `collection_id` | `string` | Yes | — | ID of the collection |
| `status_filter` | `string` | No | `null` | Filter collection members by task status |

**Returns**: Collection metadata and member records.

**Error Codes**

| Code | Condition |
|---|---|
| `1003` | No workspace set |
| `3031` | Collection not found |

**Example**

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "get_collection_context",
    "arguments": {
      "collection_id": "collection:sprint-42",
      "status_filter": "in-progress"
    }
  }
}
```

---

### `get_workspace_statistics`

Returns aggregate statistics for the workspace: task counts by status/priority/type, context record counts, code graph metrics.

**Parameters**: None

**Returns**: Statistics object.

**Error Codes**

| Code | Condition |
|---|---|
| `1003` | No workspace set |

---

## Write Tools

These tools modify workspace state. All mutating operations are recorded in the event ledger.

---

### `create_task`

Create a new task in the workspace task ledger.

**Parameters**

| Name | Type | Required | Default | Description |
|---|---|---|---|---|
| `title` | `string` | Yes | — | Task title. Must be non-empty and ≤ 255 characters. |
| `description` | `string` | No | `null` | Extended task description |
| `parent_task_id` | `string` | No | `null` | ID of the parent task (creates a subtask) |
| `work_item_id` | `string` | No | `null` | External work item identifier |
| `issue_type` | `string` | No | `null` | Issue type: `"feature"`, `"bug"`, `"chore"`, `"spike"`, `"epic"` |

**Returns**: Created task record with generated ID.

**Error Codes**

| Code | Condition |
|---|---|
| `1003` | No workspace set |
| `3001` | `parent_task_id` does not exist |
| `3010` | Invalid `issue_type` value |
| `3013` | Title is empty |
| `3014` | Title exceeds maximum length |

**Example**

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "create_task",
    "arguments": {
      "title": "Implement OAuth2 PKCE flow",
      "description": "Add PKCE support to the authorization endpoint",
      "issue_type": "feature"
    }
  }
}
```

---

### `update_task`

Update a task's status, notes, priority, or issue type.

**Parameters**

| Name | Type | Required | Default | Description |
|---|---|---|---|---|
| `id` | `string` | Yes | — | Task ID to update |
| `status` | `string` | Yes | — | New status: `"todo"`, `"in-progress"`, `"done"`, `"cancelled"` |
| `notes` | `string` | No | `null` | Progress notes or completion summary |
| `priority` | `string` | No | `null` | Priority: `"P0"` through `"P5"` |
| `issue_type` | `string` | No | `null` | Override issue type |

**Returns**: Updated task record.

**Error Codes**

| Code | Condition |
|---|---|
| `1003` | No workspace set |
| `3001` | Task not found |
| `3002` | Invalid status value |
| `3009` | Invalid priority value |
| `3015` | Task is blocked by incomplete hard blockers |

**Example**

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "update_task",
    "arguments": {
      "id": "task:abc123",
      "status": "done",
      "notes": "Implemented and tested with 100% branch coverage"
    }
  }
}
```

---

### `add_blocker`

Add a blocking reason to a task, preventing it from being moved to `done` status until the blocker is resolved.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `task_id` | `string` | Yes | ID of the task to block |
| `reason` | `string` | Yes | Human-readable description of the blocker |

**Returns**: Updated task with blocker recorded.

**Error Codes**

| Code | Condition |
|---|---|
| `1003` | No workspace set |
| `3001` | Task not found |
| `3004` | Blocker already exists with identical reason |

---

### `register_decision`

Record an architectural or design decision in the workspace's decision log. Decisions are persisted across sessions and available for agent context retrieval.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `topic` | `string` | Yes | Short topic name for the decision |
| `decision` | `string` | Yes | Full decision text |

**Returns**: Recorded decision with timestamp.

**Error Codes**

| Code | Condition |
|---|---|
| `1003` | No workspace set |

**Example**

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "register_decision",
    "arguments": {
      "topic": "Database choice",
      "decision": "Use SurrealDB embedded for zero-dependency local storage"
    }
  }
}
```

---

### `flush_state`

Persist the current in-memory workspace state to the `.engram/` directory files. This is a safe operation — existing files are updated atomically.

**Parameters**: None (the `params` argument is accepted but ignored)

**Returns**: Flush result with timestamp.

**Error Codes**

| Code | Condition |
|---|---|
| `1003` | No workspace set |
| `5002` | Flush failed (I/O error) |

---

### `add_label` / `remove_label`

Add or remove a string label tag on a task. Labels are used for categorization and filtering via `get_ready_work`.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `task_id` | `string` | Yes | Task ID |
| `label` | `string` | Yes | Label string to add or remove |

**Error Codes**

| Code | Condition |
|---|---|
| `1003` | No workspace set |
| `3001` | Task not found |
| `3006` | Label validation failed (invalid characters) |
| `3011` | Label already exists (add_label) |

---

### `add_dependency`

Add a dependency relationship between two tasks.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `from_task_id` | `string` | Yes | The dependent task (the one that requires the other) |
| `to_task_id` | `string` | Yes | The prerequisite task |
| `dependency_type` | `string` | Yes | Type: `"soft"` (advisory) or `"hard"` (blocks completion) |

**Error Codes**

| Code | Condition |
|---|---|
| `1003` | No workspace set |
| `3001` | One or both task IDs not found |
| `3003` | Adding this dependency would create a cycle |

---

### `apply_compaction`

Apply a compaction plan, summarizing and archiving completed tasks to reduce workspace size. Use `get_compaction_candidates` first to identify eligible tasks.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `compactions` | `array` | Yes | Array of compaction items, each with `task_id` and `summary` |

**Error Codes**

| Code | Condition |
|---|---|
| `1003` | No workspace set |
| `3007` | One or more compaction items failed (partial failure) |
| `3008` | Compaction operation failed entirely |

---

### `claim_task` / `release_task`

Claim a task for exclusive assignment to a specific agent or user, or release a claim.

**`claim_task` Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `task_id` | `string` | Yes | Task to claim |
| `claimant` | `string` | Yes | Identifier of the agent or user claiming the task |

**`release_task` Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `task_id` | `string` | Yes | Task to release |

**Error Codes** (claim_task)

| Code | Condition |
|---|---|
| `3005` | Task is already claimed by another claimant |
| `3012` | Task is not in a claimable status |

---

### `defer_task` / `undefer_task`

Defer a task until a future date, hiding it from `get_ready_work` results until then.

**`defer_task` Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `task_id` | `string` | Yes | Task to defer |
| `until` | `string` | Yes | ISO-8601 datetime after which the task becomes eligible again |

**`undefer_task` Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `task_id` | `string` | Yes | Task to undefer immediately |

---

### `pin_task` / `unpin_task`

Pin a task so it always appears in `get_active_context`, regardless of status. Useful for tasks requiring constant agent attention.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `task_id` | `string` | Yes | Task to pin or unpin |

---

### `batch_update_tasks`

Update multiple tasks' statuses in a single call. More efficient than multiple `update_task` calls when processing many tasks.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `updates` | `array` | Yes | Array of `{id, status, notes?}` update objects |

**Error Codes**

| Code | Condition |
|---|---|
| `3007` | One or more updates failed (partial failure — successful updates are applied) |

---

### `add_comment`

Add a timestamped comment to a task's comment thread.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `task_id` | `string` | Yes | Task to comment on |
| `content` | `string` | Yes | Comment text |
| `author` | `string` | Yes | Author identifier |

---

### `index_workspace`

Trigger workspace content indexing: scan the workspace for spec and doc files, parse them through the content registry, generate embeddings, and store in the database.

**Parameters**

| Name | Type | Required | Default | Description |
|---|---|---|---|---|
| `force` | `boolean` | No | `false` | Re-index all files even if already indexed |

**Error Codes**

| Code | Condition |
|---|---|
| `1003` | No workspace set |
| `7003` | Index operation already in progress |
| `10001` | Registry file could not be parsed |
| `10002` | Registry validation failed |
| `11001` | Content ingestion failed |

---

### `sync_workspace`

Synchronize the workspace: detect changes to `.engram/` files since last hydration and update the database without a full re-hydration.

**Parameters**: None

**Error Codes**

| Code | Condition |
|---|---|
| `1003` | No workspace set |
| `7007` | Sync conflict detected |

---

### `link_task_to_code` / `unlink_task_from_code`

Create or remove an explicit association between a task and a code symbol. These links are used by `impact_analysis` to show which tasks are affected by code changes.

**Parameters** (both tools)

| Name | Type | Required | Description |
|---|---|---|---|
| `task_id` | `string` | Yes | Task ID |
| `symbol_name` | `string` | Yes | Code symbol name to link/unlink |

---

### `rollback_to_event`

Roll back workspace state to the point immediately after a specific event. Requires `ENGRAM_ALLOW_AGENT_ROLLBACK=true`.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `event_id` | `string` | Yes | ID of the event to roll back to |

**Error Codes**

| Code | Condition |
|---|---|
| `1003` | No workspace set |
| `3020` | Rollback denied (`allow_agent_rollback` is false) |
| `3021` | Event ID not found in ledger |
| `3022` | Rollback conflict (concurrent modification) |

---

### `create_collection`

Create a named collection for grouping related tasks and context records.

**Parameters**

| Name | Type | Required | Default | Description |
|---|---|---|---|---|
| `name` | `string` | Yes | — | Collection name |
| `description` | `string` | No | `null` | Collection description |

**Error Codes**

| Code | Condition |
|---|---|
| `3030` | Collection with this name already exists |

---

### `add_to_collection` / `remove_from_collection`

Add or remove members from a collection.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `collection_id` | `string` | Yes | Collection ID |
| `member_ids` | `string[]` | Yes | Task or context record IDs to add/remove |

**Error Codes**

| Code | Condition |
|---|---|
| `3031` | Collection not found |
| `3032` | Adding members would create a cyclic collection |

---

## Git Graph Tools

These tools are available when the binary is compiled with the `git-graph` feature (`cargo build --features git-graph`).

---

### `query_changes`

Query the indexed git commit history for commits matching file path, symbol, or date range filters.

**Parameters**

| Name | Type | Required | Default | Description |
|---|---|---|---|---|
| `file_path` | `string` | No | `null` | Filter to commits that touched this file |
| `symbol` | `string` | No | `null` | Filter to commits that affected this named symbol (cross-referenced with code graph) |
| `since` | `string` | No | `null` | ISO-8601 timestamp — only return commits on or after this time |
| `until` | `string` | No | `null` | ISO-8601 timestamp — only return commits on or before this time |
| `limit` | `integer` | No | `20` | Maximum commits to return |

**Returns**: Array of commit records with hash, author, timestamp, message, and changed files.

**Error Codes**

| Code | Condition |
|---|---|
| `1003` | No workspace set |
| `12001` | Git repository not found or not indexed |
| `12002` | Git repository access error |

**Example**

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "query_changes",
    "arguments": {
      "file_path": "src/auth/mod.rs",
      "since": "2024-01-01T00:00:00Z",
      "limit": 10
    }
  }
}
```

---

### `index_git_history`

Walk and index git commit history from HEAD, storing commits as graph nodes cross-referenced with the code graph.

**Parameters**

| Name | Type | Required | Default | Description |
|---|---|---|---|---|
| `depth` | `integer` | No | `500` | Number of commits to walk from HEAD |
| `force` | `boolean` | No | `false` | Re-index all commits even if already stored |

**Returns**: Summary of indexed commits.

**Error Codes**

| Code | Condition |
|---|---|
| `1003` | No workspace set |
| `12001` | Git repository not found |
| `12002` | Git repository access error |

**Example**

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "index_git_history",
    "arguments": {
      "depth": 1000,
      "force": false
    }
  }
}
```
