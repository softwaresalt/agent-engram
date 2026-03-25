---
id: TASK-004.03
title: '004-03: Automatic Lifecycle Management'
status: Done
assignee: []
created_date: '2026-03-04'
labels:
  - feature
  - 004
  - userstory
  - p2
dependencies: []
references:
  - specs/004-refactor-engram-server-as-plugin/spec.md
parent_task_id: TASK-004
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
As a developer, I need the memory service to manage its own lifecycle — starting on demand, staying alive while active, and shutting down gracefully after a period of inactivity — so that idle workspaces do not consume system resources.

The memory service starts only when needed (triggered by a tool call) and automatically shuts down after a configurable idle timeout (default: 4 hours). Shutdown preserves all data integrity, flushing pending state to disk before exiting. On next invocation, the service restarts transparently.

**Why this priority**: Automatic lifecycle management prevents zombie processes from draining system resources (CPU, RAM, battery). This is critical for laptop developers who may have dozens of past workspaces.

**Independent Test**: Can be fully tested by starting a memory service, waiting for the idle timeout to expire, verifying clean shutdown occurred, then invoking a tool call and verifying seamless restart. Delivers value: zero resource waste from idle projects.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 **Given** an active memory service with no tool calls or file changes for the configured idle timeout, **When** the timeout expires, **Then** the service flushes all pending state to disk, cleans up runtime artifacts, and exits cleanly.
- [x] #2 **Given** a memory service that was previously shut down by idle timeout, **When** an MCP client issues a tool call, **Then** the service restarts within 2 seconds and responds with correct, un-corrupted data.
- [x] #3 **Given** an active memory service receiving periodic tool calls, **When** each call is received, **Then** the idle timeout resets and the service remains running.
- [x] #4 **Given** a system power loss or crash during memory service operation, **When** the service is next started, **Then** it recovers gracefully by rehydrating from the persisted `.engram/` files without data loss. ---
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Phase 5: User Story 3 — Automatic Lifecycle Management (Priority: P2)

**Goal**: Daemon self-manages its lifecycle with idle timeout, graceful shutdown, and crash recovery. Zero resource waste from idle workspaces.

**Independent Test**: Start daemon, wait for idle timeout, verify clean shutdown. Restart and verify data intact. Kill daemon, restart and verify recovery.

### Tests for US3 (write first, verify they fail)

- [X] T045 [P] [US3] Unit test for TTL timer in tests/unit/ttl_test.rs — expiry triggers shutdown (S045), activity resets timer (S046-S047), zero timeout = run forever (S049), rapid activity (S051)
- [X] T046 [P] [US3] Integration test for daemon lifecycle in tests/integration/daemon_lifecycle_test.rs — graceful shutdown flushes state (S037), shutdown during request (S038), restart after timeout (S050)
- [X] T047 [P] [US3] Integration test for crash recovery in tests/integration/daemon_lifecycle_test.rs — SIGKILL recovery (S039-S040), stale lock detection, data rehydration; covers S095-S096

### Implementation for US3

- [X] T048 [US3] Implement idle TTL timer in src/daemon/ttl.rs — activity timestamp tracking, periodic expiry check (S045), configurable duration; covers S048-S049
- [X] T049 [US3] Wire TTL reset into IPC request handler in src/daemon/ipc_server.rs — every tool call resets idle timer (S046)
- [X] T050 [US3] Wire TTL reset into file watcher event handler in src/daemon/watcher.rs — every file event resets idle timer (S047)
- [X] T051 [US3] Implement graceful shutdown sequence in src/daemon/mod.rs — transition to ShuttingDown, flush state, close IPC listener, remove lock file, remove socket, exit; covers S037
- [X] T052 [US3] Implement _shutdown IPC handler in src/daemon/ipc_server.rs — trigger graceful shutdown from shim command per contracts/ipc-protocol.md (S022)
- [X] T053 [US3] Implement crash recovery in src/daemon/lockfile.rs — detect stale lock (fd-lock not held), clean stale socket/pipe, allow fresh daemon start; covers S039-S040, S042
- [X] T054 [US3] Handle SIGTERM/SIGINT via tokio signal handler in src/daemon/mod.rs — trigger graceful shutdown on signal; covers S038
- [X] T055 [US3] Verify `cargo test` passes for all Phase 5 tests

**Checkpoint**: Lifecycle management complete — daemon auto-shuts down, recovers from crashes, zero resource waste. User Story 3 independently testable.

---
<!-- SECTION:PLAN:END -->

