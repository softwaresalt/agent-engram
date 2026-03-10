# Log Observation Guide

How to read and interpret engram daemon logs to verify correct operation.

## Quick Start

```powershell
# Run the daemon with full debug logging
$env:RUST_LOG = "engram=debug"
$env:ENGRAM_LOG_FORMAT = "pretty"

# Option A: Use the local-run script
.\scripts\run-local.ps1

# Option B: Run the daemon manually
cargo run --release -- daemon --workspace .
```

Or use the `ENGRAM_LOG_FORMAT=json` variant for machine-parseable output
piped to `jq`.

## Log Levels

| Level | RUST_LOG filter | What you see |
|-------|----------------|--------------|
| ERROR | `engram=error` | Only failures (IPC errors, watcher crashes) |
| WARN  | `engram=warn`  | + Degraded operation (stale locks, corrupt files) |
| INFO  | `engram=info`  | + Lifecycle events (startup, shutdown, tool calls) |
| DEBUG | `engram=debug` | + Connection tracking, file watcher events |
| TRACE | `engram=trace` | + Low-level frame data (very verbose) |

**Recommended for validation**: `RUST_LOG=engram=debug`

## Phase 1: Startup

When the daemon starts, you should see these events in order:

### Expected log sequence

```
INFO  daemon lock acquired                    workspace="D:\\Source\\GitHub\\agent-engram"
INFO  structured log directory ready          log_dir=".../.engram/logs"
INFO  idle TTL configured                     idle_timeout_ms=14400000
INFO  IPC listener bound                      endpoint="\\\\.\pipe\\engram-a1b2c3d4e5f67890"
```

### What each event means

| Event | Level | Fields | Meaning |
|-------|-------|--------|---------|
| `daemon lock acquired` | INFO | `workspace` | Lockfile claimed; no other daemon owns this workspace |
| `structured log directory ready` | INFO | `log_dir` | `.engram/logs/` created; file appender active |
| `idle TTL configured` | INFO | `idle_timeout_ms` | Daemon will self-terminate after this many ms of inactivity |
| `IPC listener bound` | INFO | `endpoint` | Named pipe (Windows) or Unix socket (macOS/Linux) is accepting connections |

### Startup warnings (non-fatal)

| Event | Level | Meaning | Action |
|-------|-------|---------|--------|
| `failed to create .engram/logs/ directory` | WARN | Log dir inaccessible | Daemon continues; logs only to stderr |
| `file watcher failed to start` | ERROR | `notify` backend error | Daemon runs in degraded mode (no file events) |
| `found stale lockfile, cleaning up` | WARN | Previous daemon crashed | Normal after crash recovery |
| `removed stale IPC socket from previous daemon run` | INFO | Socket cleanup | Normal after crash recovery |
| `daemon lock held by live process, cannot start` | WARN | Another daemon is running | Check for duplicate daemon processes |

## Phase 2: Steady State

During normal operation, each tool call produces a tracing span and
connection events appear at DEBUG level.

### Tool dispatch (every MCP tool call)

```
INFO  tool_dispatch{tool="create_task"}
INFO  tool_dispatch{tool="get_ready_work"}
INFO  tool_dispatch{tool="flush_state"}
```

The `tool_dispatch` span wraps every tool call. The `tool` field contains
the method name. Latency is recorded internally (visible via
`get_health_report`).

### IPC connections

```
DEBUG ipc_connection_established              connection_id="a1b2c3d4-..."
DEBUG ipc_connection_closed                   connection_id="a1b2c3d4-..."
```

Each IPC call opens a fresh connection (stateless per-connection protocol).
You should see established/closed pairs. Orphaned `established` without a
matching `closed` indicates a connection leak.

### File watcher events

```
DEBUG watcher_event_detected                  event_kind=Modify path="src/lib.rs"
DEBUG watcher_event_sent                      path="src/lib.rs" kind=Modified
```

Or for excluded paths:

```
DEBUG watcher_event_excluded                  path=".engram/tasks.md"
```

**Excluded patterns**: `.engram/`, `.git/`, `node_modules/`, `target/`, `.env*`

### Event recording warnings (non-fatal)

```
WARN  event recording failed for create_task  error="..." task_id="task:abc"
```

These indicate the event ledger failed to persist an operation record.
The operation itself succeeded — only the audit trail entry was lost.

## Phase 3: Shutdown

### Graceful shutdown (IPC `_shutdown` or Ctrl-C)

```
INFO  daemon received _shutdown IPC request — initiating graceful shutdown
INFO  shutdown signal received — stopping IPC listener
INFO  daemon exiting cleanly
```

Or via Ctrl-C / SIGTERM:

```
INFO  Ctrl-C / SIGTERM received — signalling graceful shutdown
INFO  shutdown signal received — stopping IPC listener
INFO  daemon exiting cleanly
```

### Idle timeout shutdown

When the daemon has no activity for the configured idle period:

```
INFO  idle TTL configured                     idle_timeout_ms=500
  ... (no activity) ...
INFO  daemon exiting cleanly
```

### Workspace moved/deleted

```
WARN  workspace path no longer exists         path="D:\\..."
INFO  daemon exiting cleanly
```

This check runs every 60 seconds.

## Health Report

Query the daemon's self-reported metrics at any time:

```json
// IPC request:
{"jsonrpc": "2.0", "id": 1, "method": "get_health_report", "params": null}

// Response:
{
  "version": "0.0.1",
  "uptime_seconds": 1234,
  "active_connections": 1,
  "workspace_id": "abc123...",
  "tool_call_count": 42,
  "latency_us": {
    "p50": 1250,
    "p95": 3840,
    "p99": 8900
  },
  "memory_mb": 45,
  "watcher_events": 234,
  "last_watcher_event": "2026-03-10T07:00:00.000Z"
}
```

### What to check

| Metric | Healthy | Concerning |
|--------|---------|------------|
| `uptime_seconds` | Increasing | Resets to 0 (daemon restarted) |
| `tool_call_count` | Increasing with agent usage | Stuck at 0 (agent not calling engram) |
| `latency_us.p50` | < 5,000 (5ms) | > 50,000 (50ms) |
| `latency_us.p99` | < 50,000 (50ms) | > 500,000 (500ms) |
| `memory_mb` | < 200 | > 500 (possible leak) |
| `watcher_events` | Increasing during edits | 0 (file watcher may be degraded) |

## Targeted Debugging

Use `RUST_LOG` filter syntax to focus on specific components:

```powershell
# Only IPC server logs
$env:RUST_LOG = "engram::daemon::ipc_server=debug"

# Only file watcher
$env:RUST_LOG = "engram::daemon::watcher=debug"

# Only tool dispatch
$env:RUST_LOG = "engram::tools=debug"

# Combined: tools + IPC at debug, everything else at warn
$env:RUST_LOG = "engram=warn,engram::tools=debug,engram::daemon::ipc_server=debug"
```

## Validating the Reliability Gate

To demonstrate that the daemon is functioning correctly in a real
workspace, look for all of the following in a single session:

- [ ] `daemon lock acquired` with correct workspace path
- [ ] `IPC listener bound` with the expected endpoint
- [ ] At least one `tool_dispatch{tool="set_workspace"}` span
- [ ] Task creation and retrieval (`create_task`, `get_ready_work`)
- [ ] `flush_state` producing `.engram/tasks.md` on disk
- [ ] `get_health_report` showing `tool_call_count > 0`
- [ ] Clean shutdown via `_shutdown` or Ctrl-C
- [ ] No ERROR-level log entries during the session

The E2E smoke test (`cargo test --test integration_smoke`) validates all
of these automatically.
