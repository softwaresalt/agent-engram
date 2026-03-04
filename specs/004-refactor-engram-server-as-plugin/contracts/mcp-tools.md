# MCP Tools Contract: 004-refactor-engram-server-as-plugin

**Version**: 1.0.0 (unchanged from current)

## Overview

The MCP tool registry is **unchanged** by this refactoring. All tools, their parameters, return types, and error behaviors remain identical. The only change is the transport: tools are now invoked via stdio (shim) → IPC (daemon) instead of HTTP POST `/mcp`.

## Tool Registry

| Tool | Module | Parameters | Description |
|------|--------|------------|-------------|
| `set_workspace` | lifecycle | `{ workspace_path: string }` | Bind connection to a workspace, trigger hydration |
| `get_daemon_status` | lifecycle | none | Report uptime, connections, workspaces |
| `get_workspace_status` | lifecycle | none | Report task/context counts, flush state, staleness |
| `update_task` | write | `{ task_id: string, status?: string, description?: string }` | Change task status, creates context note |
| `add_blocker` | write | `{ task_id: string, blocker_id: string, reason?: string }` | Block a task with reason |
| `register_decision` | write | `{ title: string, content: string }` | Record architectural decision as context |
| `flush_state` | write | none | Serialize DB state to `.engram/` files |
| `get_task_graph` | read | `{ root_id?: string, depth?: number }` | Recursive dependency graph traversal |
| `check_status` | read | `{ work_item_ids: string[] }` | Batch work item status lookup |
| `query_memory` | read | `{ query: string, limit?: number }` | Semantic search (embedding-based) |

## Behavioral Guarantees

All existing behavioral contracts are preserved:

1. **Workspace binding**: `set_workspace` MUST be called before any workspace-scoped tool. Tools called without a bound workspace return error code `1001` (WORKSPACE_NOT_SET).
2. **Status transitions**: `update_task` validates transitions per the state machine (todo → in_progress → done, etc.). Invalid transitions return error code `3001`.
3. **Context notes**: Every `update_task` call creates a context note recording the transition (FR-015 from spec 001).
4. **Idempotency**: Write operations are idempotent where documented.
5. **Error codes**: All error codes from `errors/codes.rs` are unchanged.

### Known Behavioral Delta: Workspace Binding Semantics

The workspace binding model changes from **per-SSE-connection** to **per-daemon**:

| Aspect | Before (SSE) | After (Daemon) |
|--------|-------------|----------------|
| Binding scope | Each SSE connection has independent workspace binding | Daemon is bound to one workspace for its entire lifetime |
| Multiple clients | Two clients could bind to different workspaces on the same server | All clients connecting to a daemon share its workspace binding |
| `set_workspace` with different path | Allowed (each connection independent) | Returns error — daemon is already bound to a different workspace |
| Isolation guarantee | Logical (per-connection) | Physical (per-process + per-IPC-channel) |

**Impact**: Agents that previously relied on rebinding `set_workspace` to a different path within the same server session will now receive an error. This is a **stricter isolation guarantee** — each workspace has its own daemon process — but constitutes a semantic change that may affect multi-workspace tooling. The recommended migration is to let each workspace's shim auto-start its own daemon.

## Transport Change Only

| Aspect | Before (current) | After (refactored) |
|--------|-------------------|---------------------|
| Client → Server | HTTP POST `/mcp` | stdio → shim → IPC → daemon |
| Tool discovery | HTTP GET `/sse` (SSE event with tool list) | `tools/list` via MCP protocol (rmcp handles) |
| Connection model | Persistent SSE + POST requests | Per-invocation shim process |
| Workspace binding | Per-SSE-connection state | Per-daemon state (bound on first `set_workspace`) |
| Error format | JSON-RPC 2.0 over HTTP | JSON-RPC 2.0 over stdio (same schema) |

## Contract Test Compatibility

Existing contract tests in `tests/contract/` MUST continue to pass. The tests validate tool input/output schemas and error codes — these are transport-independent. Test setup may need adaptation to use the IPC transport instead of HTTP, but assertion logic remains identical.
