# MCP Tool Contracts: Lifecycle Observability

**Feature**: 005-lifecycle-observability
**Date**: 2026-03-09

## New Tools

### query_graph

Execute a sandboxed read-only query against the workspace graph.

**Input Schema**:
```json
{
  "query": "string (required) — SurrealQL SELECT statement",
  "params": "object (optional) — parameterized query bindings"
}
```

**Output Schema**:
```json
{
  "rows": "array — query result rows",
  "row_count": "integer — number of rows returned",
  "truncated": "boolean — true if results were limited by query_row_limit",
  "elapsed_ms": "integer — query execution time in milliseconds"
}
```

**Errors**:
- `WORKSPACE_NOT_SET` (1001): No workspace bound
- `QUERY_REJECTED` (4010): Query contains write operations
- `QUERY_TIMEOUT` (4011): Query exceeded execution timeout
- `QUERY_INVALID` (4012): Query syntax is invalid

---

### get_event_history

Retrieve recent events from the event ledger.

**Input Schema**:
```json
{
  "entity_id": "string (optional) — filter by target entity ID",
  "kind": "string (optional) — filter by event kind",
  "limit": "integer (optional, default 50) — max events to return"
}
```

**Output Schema**:
```json
{
  "events": [
    {
      "id": "string",
      "kind": "string",
      "entity_table": "string",
      "entity_id": "string",
      "source_client": "string",
      "created_at": "string (ISO 8601)"
    }
  ],
  "total_count": "integer",
  "limit": "integer — requested limit"
}
```

**Errors**:
- `WORKSPACE_NOT_SET` (1001): No workspace bound

---

### rollback_to_event

Roll workspace state back to a specific event.

**Input Schema**:
```json
{
  "event_id": "string (required) — event ID to rollback to"
}
```

**Output Schema**:
```json
{
  "rolled_back_events": "integer — number of events reversed",
  "conflicts": [
    {
      "event_id": "string",
      "entity_id": "string",
      "reason": "string"
    }
  ],
  "restored_entities": "integer — number of entities restored"
}
```

**Errors**:
- `WORKSPACE_NOT_SET` (1001): No workspace bound
- `ROLLBACK_DENIED` (3020): Agent rollback not permitted (allow_agent_rollback=false)
- `EVENT_NOT_FOUND` (3021): Specified event does not exist in ledger
- `ROLLBACK_CONFLICT` (3022): Rollback cannot be cleanly applied

---

### create_collection

Create a named collection (epic/workflow grouping).

**Input Schema**:
```json
{
  "name": "string (required) — collection name (unique within workspace)",
  "description": "string (optional) — collection description"
}
```

**Output Schema**:
```json
{
  "id": "string — collection record ID",
  "name": "string",
  "description": "string | null",
  "created_at": "string (ISO 8601)"
}
```

**Errors**:
- `WORKSPACE_NOT_SET` (1001): No workspace bound
- `COLLECTION_EXISTS` (3030): Collection with this name already exists

---

### add_to_collection

Add tasks or sub-collections to a collection.

**Input Schema**:
```json
{
  "collection_id": "string (required) — collection to add to",
  "member_ids": "array<string> (required) — task or collection IDs to add"
}
```

**Output Schema**:
```json
{
  "added": "integer — number of members successfully added",
  "already_members": "integer — number already in the collection (skipped)"
}
```

**Errors**:
- `WORKSPACE_NOT_SET` (1001): No workspace bound
- `COLLECTION_NOT_FOUND` (3031): Target collection does not exist
- `CYCLIC_COLLECTION` (3032): Adding would create a collection cycle

---

### remove_from_collection

Remove tasks or sub-collections from a collection.

**Input Schema**:
```json
{
  "collection_id": "string (required) — collection to remove from",
  "member_ids": "array<string> (required) — task or collection IDs to remove"
}
```

**Output Schema**:
```json
{
  "removed": "integer — number of members removed",
  "not_found": "integer — number that were not members (skipped)"
}
```

**Errors**:
- `WORKSPACE_NOT_SET` (1001): No workspace bound
- `COLLECTION_NOT_FOUND` (3031): Target collection does not exist

---

### get_collection_context

Recursively retrieve all tasks and context within a collection hierarchy.

**Input Schema**:
```json
{
  "collection_id": "string (required) — collection to retrieve",
  "status_filter": "array<string> (optional) — filter tasks by status",
  "include_files": "boolean (optional, default true) — include associated file references"
}
```

**Output Schema**:
```json
{
  "collection": { "id": "string", "name": "string", "description": "string | null" },
  "tasks": [
    { "id": "string", "title": "string", "status": "string", "priority": "string" }
  ],
  "sub_collections": [
    { "id": "string", "name": "string", "task_count": "integer" }
  ],
  "total_tasks": "integer",
  "files": ["string — file paths associated with contained tasks"]
}
```

**Errors**:
- `WORKSPACE_NOT_SET` (1001): No workspace bound
- `COLLECTION_NOT_FOUND` (3031): Target collection does not exist

---

### get_health_report

Extended daemon health with latency percentiles, watcher status, and memory.

**Input Schema**:
```json
{}
```

**Output Schema**:
```json
{
  "version": "string",
  "uptime_seconds": "integer",
  "active_connections": "integer",
  "workspace_id": "string | null",
  "tool_call_count": "integer",
  "latency_us": {
    "p50": "integer",
    "p95": "integer",
    "p99": "integer"
  },
  "memory_mb": "integer | null — process RSS; null if lookup fails",
  "watcher_events": "integer",
  "last_watcher_event": "string | null — ISO 8601 timestamp"
}
```

**Errors**:
- (none — always available even without workspace binding)

## Modified Tool Contracts

### update_task (modified)

**New error codes**:
- `TASK_BLOCKED` (3015): Task has unresolved hard_blocker dependencies

**New response fields** (added to existing response):
```json
{
  "warnings": [
    {
      "type": "soft_dependency_incomplete",
      "dependency_id": "string",
      "dependency_title": "string"
    }
  ]
}
```

### add_dependency (modified)

**New error codes**:
- `CYCLIC_DEPENDENCY` (3003): Adding this edge would create a dependency cycle
