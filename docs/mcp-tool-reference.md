---
title: Engram MCP Tool Reference
description: Complete reference for all MCP tools registered by the Engram daemon, organized by category with parameters, return schemas, error codes, and examples.
---

## Overview

This reference documents every MCP tool registered by the Engram daemon. Tools are organized by category. Each entry describes the tool's purpose, parameters, return schema, error codes, and an example request.

All tools are called via the MCP JSON-RPC protocol over the HTTP/SSE endpoint (`http://localhost:7437/sse`).

---

## Table of Contents

1. [Lifecycle Tools](#lifecycle-tools)
   - [set_workspace](#set_workspace)
   - [get_daemon_status](#get_daemon_status)
   - [get_workspace_status](#get_workspace_status)
2. [Code Graph Tools](#code-graph-tools)
   - [index_workspace](#index_workspace)
   - [sync_workspace](#sync_workspace)
   - [map_code](#map_code)
   - [list_symbols](#list_symbols)
   - [impact_analysis](#impact_analysis)
   - [get_workspace_statistics](#get_workspace_statistics)
3. [Search and Query Tools](#search-and-query-tools)
   - [query_memory](#query_memory)
   - [unified_search](#unified_search)
   - [query_graph](#query_graph)
4. [Persistence Tools](#persistence-tools)
   - [flush_state](#flush_state)

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

Returns the current state of the bound workspace: last flush time, staleness flag, and code graph statistics.

**Parameters**: None

**Returns**

```json
{
  "path": "/home/user/my-project",
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

## Code Graph Tools

These tools index, navigate, and analyze the code symbol graph.

---

### `index_workspace`

Parse and index all source files in the workspace into the code graph using tree-sitter. On completion, all functions, classes, interfaces, and their relationships are available for query.

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

**Example**

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "index_workspace",
    "arguments": {
      "force": false
    }
  }
}
```

---

### `sync_workspace`

Incrementally re-index source files that have changed since the last index run, without performing a full re-parse of the entire workspace.

**Parameters**: None

**Error Codes**

| Code | Condition |
|---|---|
| `1003` | No workspace set |
| `7007` | Sync conflict detected |

**Example**

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "sync_workspace",
    "arguments": {}
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

### `impact_analysis`

Estimates the blast radius of a change to a code symbol: which other symbols call or reference it, and which files are transitively affected.

**Parameters**

| Name | Type | Required | Default | Description |
|---|---|---|---|---|
| `symbol_name` | `string` | Yes | — | Symbol whose change impact is being analyzed |
| `depth` | `integer` | No | `2` | Graph traversal depth |
| `max_nodes` | `integer` | No | `50` | Maximum nodes in the impact graph |

**Returns**: Impact graph with affected symbols and files.

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
      "depth": 3
    }
  }
}
```

---

### `get_workspace_statistics`

Returns aggregate statistics for the workspace: code file count, indexed symbol counts by type, and edge count.

**Parameters**: None

**Returns**: Statistics object.

**Error Codes**

| Code | Condition |
|---|---|
| `1003` | No workspace set |

---

## Search and Query Tools

These tools search and query workspace data without modifying state.

---

### `query_memory`

Semantic similarity search over workspace content records and indexed commit history. Uses vector embeddings to find results semantically related to the query string.

**Parameters**

| Name | Type | Required | Default | Description |
|---|---|---|---|---|
| `query` | `string` | Yes | — | Natural language search query |
| `limit` | `integer` | No | `10` | Maximum number of results to return |
| `content_type` | `string` | No | `null` | Filter by content type (e.g., `"spec"`, `"docs"`, `"tests"`) |

**Returns**: Array of ranked content records with similarity scores.

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

Cross-domain semantic search that queries content records, code symbols, and commit history simultaneously, returning ranked results from all sources in a single response.

**Parameters**

| Name | Type | Required | Default | Description |
|---|---|---|---|---|
| `query` | `string` | Yes | — | Natural language search query |
| `regions` | `string[]` | No | `["tasks","context","code"]` | Limit search to specific regions: `"tasks"`, `"context"`, `"code"` |
| `limit` | `integer` | No | `10` | Maximum results per domain |
| `content_type` | `string` | No | `null` | Filter content records by type when region includes context |

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

### `query_graph`

Execute a sandboxed SurrealQL SELECT query against the workspace database. Subject to `query_timeout_ms` and `query_row_limit` constraints from `.engram/config.toml`. Only SELECT statements are permitted; write operations are rejected.

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
      "query": "SELECT id, name, file_path FROM function WHERE file_path CONTAINS 'auth' LIMIT 20"
    }
  }
}
```

---

## Persistence Tools

---

### `flush_state`

Persist the current in-memory workspace state to the `.engram/` directory files. This is a safe operation; existing files are updated atomically via temp-file-then-rename.

**Parameters**: None (the `params` argument is accepted but ignored)

**Returns**: Flush result with timestamp.

**Error Codes**

| Code | Condition |
|---|---|
| `1003` | No workspace set |
| `5002` | Flush failed (I/O error) |

**Example**

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "flush_state",
    "arguments": {}
  }
}
```
