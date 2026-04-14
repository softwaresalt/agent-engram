---
title: "Multi-Session Shim: Concurrent Agent Support"
date: 2026-03-28
scope: standard
status: draft
---

# Multi-Session Shim: Concurrent Agent Support

## Problem Frame

The shim's current design is sound for the common case of a single agent session:
it checks whether the daemon is running, spawns it if absent, then proxies stdio
MCP calls over IPC. The daemon's `accept_loop` already spawns a `tokio::task` per
accepted connection, so the daemon itself is capable of serving multiple concurrent
IPC connections. However, several edge cases and gaps emerge when two or more agent
processes connect simultaneously or in quick succession to the same workspace:

1. **Spawn race**: If two shims start within milliseconds of each other and neither
   finds a healthy daemon, both may attempt to spawn. The current lifecycle code
   handles the "one winner" case with a final health check after backoff exhaustion,
   but there is no explicit test proving this works under realistic load.

2. **Long-running shim sessions**: The shim's stdio transport holds a connection open
   for the lifetime of the MCP session (potentially hours). The daemon's TTL timer
   must not evict a workspace while active shim connections are still live.

3. **Connection-count visibility**: Agents querying `get_daemon_status` cannot see
   how many peer shim sessions are currently connected. This makes it hard to reason
   about shared daemon state or detect orphaned connections.

4. **No test coverage for concurrency**: The existing `shim_lifecycle_test.rs`
   validates single-session lifecycle but does not exercise two shims connecting
   to the same workspace simultaneously.

## Requirements

### Spawn Race Hardening

1. When a shim cannot reach a healthy daemon after exhausting backoff, it MUST make
   one final health check before returning `DaemonError::NotReady`. This already
   exists; the requirement is that it be covered by a concurrent integration test.
2. A concurrent integration test MUST spawn two shim processes against the same
   workspace simultaneously and assert that exactly one daemon is running at steady
   state (verified via the daemon's lockfile or `_health` IPC response).
3. The daemon lockfile (`DaemonLock`) MUST serve as the authoritative "one daemon
   wins" arbiter. The second daemon that loses the lockfile race MUST exit cleanly
   with a structured log message (not a panic).

### TTL / Active-Session Interaction

4. The daemon TTL timer MUST NOT fire while one or more shim connections are actively
   holding an IPC stream open. The `TtlTimer::reset()` call on each accepted
   connection (already present in `accept_loop`) satisfies this for periodic
   requests; the requirement is that long-lived idle connections (no tool calls for
   minutes) also suppress TTL expiry.
5. A `keep_alive_interval` configuration (default 60 s) MUST be added to the shim
   transport. When no tool call has been forwarded in `keep_alive_interval`, the
   shim MUST send an `_health` IPC request to reset the daemon's TTL timer.
6. If the daemon TTL expires while a shim is connected (rare but possible), the shim
   MUST detect the disconnection, attempt to respawn the daemon via the standard
   `ensure_daemon_running` path, and reconnect without surfacing an error to the
   MCP client.

### Connection Visibility

7. The `get_daemon_status` MCP tool response MUST include an `active_ipc_connections`
   count reflecting the number of currently open IPC streams.
8. The `ConnectionRegistry` (already tracking SSE connections) MUST be extended or a
   parallel `IpcConnectionRegistry` MUST be created to track active IPC sessions
   with connection ID, workspace path, and connect timestamp.

### Test Coverage

9. A contract test MUST verify that `get_daemon_status` reports `active_ipc_connections`
   correctly as connections are opened and closed.
10. An integration test MUST verify that two shims connecting to the same workspace
    both receive valid tool call responses for concurrent `get_workspace_status` calls.
11. The existing `shim_lifecycle_test.rs` tests MUST continue to pass.

## Success Criteria

1. Two agent processes (e.g., build orchestrator + code reviewer) can connect to the
   same workspace daemon simultaneously and call tools without interference.
2. Starting 5 shim processes at exactly the same time results in exactly 1 running
   daemon and 5 successful health checks.
3. A shim idle for 90 seconds automatically sends a keep-alive and the daemon does
   not TTL-expire.
4. `get_daemon_status` accurately reports the number of active connections.
5. All existing shim lifecycle tests pass.

## Scope Boundaries

### In Scope

- Concurrent spawn race test (two simultaneous shim starts)
- TTL keep-alive from shim for long-lived sessions
- Daemon reconnection on unexpected TTL expiry
- `active_ipc_connections` in `get_daemon_status`
- IPC connection tracking registry

### Non-Goals

- HTTP/SSE multiplexing (SSE connections are already tracked separately)
- Cross-workspace daemon sharing (each workspace has exactly one daemon)
- Shim connection pooling (each shim maintains exactly one IPC connection)
- Load balancing across multiple daemon instances
- Authentication or per-agent access control

## Key Decisions

### D1: Daemon already handles concurrent IPC connections

The `accept_loop` spawns a `tokio::spawn` per connection. No architectural change
is needed for basic concurrency. The work is hardening edge cases and adding
test coverage.

### D2: Keep-alive from the shim, not the daemon

The TTL timer is reset by IPC activity. Rather than making the daemon reach out
to connected clients, the shim takes responsibility for sending periodic
`_health` requests to keep the TTL clock reset. This preserves the daemon's
simple accept-respond model.

### D3: IPC connection registry is separate from SSE registry

IPC connections are short-request-then-done or long-lived (shim session), while
SSE connections are long-lived HTTP streams. They have different lifecycle
semantics and warrant separate tracking structures.

## Outstanding Questions

### Resolve Before Planning

1. **Keep-alive interval default**: Should the keep-alive interval be 60 s (matches
   existing SSE keepalive) or shorter? Shorter reduces TTL expiry risk but adds IPC
   traffic.

2. **IPC connection registry scope**: Should the registry be in-memory only (lost on
   restart) or persisted to `.engram/`? In-memory is sufficient for status reporting;
   persisted enables detecting orphaned sessions across daemon restarts.

### Deferred to Implementation

3. **Reconnection backoff**: The exact backoff parameters for shim-initiated daemon
   reconnection after unexpected TTL expiry.

4. **Connection ID format**: Whether IPC connection IDs are UUID v4 (consistent with
   SSE connection IDs) or a simpler counter.
