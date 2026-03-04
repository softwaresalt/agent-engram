# Data Model: 004-refactor-engram-server-as-plugin

**Date**: 2026-03-04  
**Branch**: `004-refactor-engram-server-as-plugin`

## Overview

This feature introduces new architectural entities for the shim/daemon model. Existing data models (Task, Spec, Context, DependencyType) are **unchanged** — the refactoring affects only process architecture and communication, not the domain model.

## New Entities

### DaemonState

Represents the current state of the workspace daemon process.

| Field | Type | Description |
|-------|------|-------------|
| `workspace_path` | `PathBuf` | Canonical absolute path to the workspace root |
| `workspace_hash` | `String` | SHA-256 hash of canonical workspace path (reuses `db::workspace::hash_workspace_path`) |
| `pid` | `u32` | Process ID of the running daemon |
| `ipc_address` | `String` | IPC endpoint address (socket path or pipe name) |
| `started_at` | `DateTime<Utc>` | Timestamp when daemon started |
| `last_activity` | `DateTime<Utc>` | Timestamp of most recent activity (tool call or file event) |
| `idle_timeout` | `Duration` | Configured idle timeout duration |
| `status` | `DaemonStatus` | Current lifecycle state |

### DaemonStatus (enum)

| Variant | Description |
|---------|-------------|
| `Starting` | Daemon is initializing (hydrating data, binding IPC) |
| `Ready` | Daemon is accepting connections and processing events |
| `ShuttingDown` | Daemon is flushing state and cleaning up before exit |

### IpcRequest

JSON-RPC request message sent from shim to daemon over IPC.

| Field | Type | Description |
|-------|------|-------------|
| `jsonrpc` | `String` | Always `"2.0"` |
| `id` | `Value` | Request ID (number or string, echoed in response) |
| `method` | `String` | MCP tool name (e.g., `"set_workspace"`, `"update_task"`) |
| `params` | `Option<Value>` | Tool parameters (JSON object or null) |

### IpcResponse

JSON-RPC response message sent from daemon to shim over IPC.

| Field | Type | Description |
|-------|------|-------------|
| `jsonrpc` | `String` | Always `"2.0"` |
| `id` | `Value` | Request ID matching the request |
| `result` | `Option<Value>` | Success response payload (mutually exclusive with `error`) |
| `error` | `Option<IpcError>` | Error response (mutually exclusive with `result`) |

### IpcError

| Field | Type | Description |
|-------|------|-------------|
| `code` | `i32` | JSON-RPC error code |
| `message` | `String` | Human-readable error description |
| `data` | `Option<Value>` | Additional error data |

### WatcherEvent

Represents a debounced file system change event.

| Field | Type | Description |
|-------|------|-------------|
| `path` | `PathBuf` | Relative path from workspace root (primary path for Created/Modified/Deleted) |
| `old_path` | `Option<PathBuf>` | Previous path for Renamed events (None for all other event kinds) |
| `kind` | `WatchEventKind` | Type of change |
| `timestamp` | `DateTime<Utc>` | When the debounced event was emitted |

### WatchEventKind (enum)

| Variant | Description |
|---------|-------------|
| `Created` | New file or directory created |
| `Modified` | File content changed |
| `Deleted` | File or directory removed |
| `Renamed` | File moved or renamed — `old_path` contains the previous location, `path` contains the new location |

### PluginConfig

User-configurable settings loaded from `.engram/config.toml`.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `idle_timeout_minutes` | `u64` | `240` (4 hours) | Minutes of inactivity before daemon shuts down |
| `debounce_ms` | `u64` | `500` | Milliseconds to debounce file events |
| `watch_patterns` | `Vec<String>` | `["**/*"]` | Glob patterns for files to watch |
| `exclude_patterns` | `Vec<String>` | `[".engram/", ".git/", "node_modules/", "target/", ".env*"]` | Glob patterns for files to exclude from watching |
| `log_level` | `String` | `"info"` | Daemon log verbosity |
| `log_format` | `String` | `"pretty"` | Log output format (`pretty` or `json`) |

## Relationships

```text
MCP Client ──stdio──> Shim ──IPC──> Daemon ──> SurrealDB (existing)
                                       │
                                       ├──> File Watcher ──> Existing Pipelines
                                       │      (code graph, embeddings)
                                       │
                                       └──> TTL Timer
```

## Unchanged Entities

The following existing entities are **not modified** by this feature:

- `Task` (models/task.rs) — all fields, status transitions, and serialization preserved
- `Spec` (models/spec.rs) — unchanged
- `Context` (models/context.rs) — unchanged
- `DependencyType` (models/graph.rs) — unchanged
- `CodeFile`, `Function`, `Class`, `Interface`, `Comment`, `CodeEdge` (models/) — unchanged
- `EngramError` (errors/mod.rs) — extended with new variants (see below)

## New Error Variants

| Variant | Code Range | Description |
|---------|-----------|-------------|
| `IpcConnection` | 8xxx | IPC socket/pipe errors (connect, send, receive) |
| `DaemonSpawn` | 8xxx | Daemon process spawning failures |
| `LockAcquisition` | 8xxx | PID file lock failures |
| `WatcherInit` | 8xxx | File watcher initialization failures |
| `ConfigParse` | 8xxx | Plugin configuration parsing errors |
| `InstallError` | 9xxx | Plugin install/update/uninstall failures |

*Note: 6xxx (config) and 7xxx (code graph) are already occupied in `src/errors/codes.rs`. These new variants use 8xxx and 9xxx to avoid collision.*

## State Transitions

### Daemon Lifecycle

```text
                     spawn
  [Not Running] ────────────> [Starting]
       ▲                          │
       │                          │ hydrate + bind IPC
       │                          ▼
       │  TTL expired        [Ready] <──── tool call / file event
       │  or SIGTERM             │          (resets TTL)
       │                         │
       └──── [ShuttingDown] <────┘
                  │
                  │ flush + cleanup
                  ▼
             [Not Running]
```

### Shim Request Flow

```text
  [Receive stdio] ──> [Check IPC] ──> [Connected?]
                                          │
                           Yes ◄──────────┤
                            │             No
                            │              │
                            │         [Acquire lock]
                            │              │
                            │         [Spawn daemon]
                            │              │
                            │         [Wait for ready]
                            │              │
                            ▼              ▼
                       [Forward via IPC]
                            │
                            ▼
                       [Return response via stdout]
                            │
                            ▼
                         [Exit]
```
