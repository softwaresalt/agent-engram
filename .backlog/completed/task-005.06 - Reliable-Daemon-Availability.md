---
id: TASK-005.06
title: '005-06: Reliable Daemon Availability'
status: Done
assignee: []
created_date: '2026-03-09'
labels:
  - feature
  - 005
  - userstory
  - p1
dependencies: []
references:
  - specs/005-lifecycle-observability/spec.md
parent_task_id: TASK-005
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
As a developer using AI coding assistants in active workspaces, I need the engram daemon to demonstrate reliable availability — starting on demand, maintaining stable connections during active use, and recovering gracefully from interruptions — so that the memory service can be trusted as a core part of the development workflow.

The daemon must prove it can sustain multi-hour sessions without dropped connections, handle concurrent tool calls without data corruption, survive workspace switches and IDE restarts, and provide clear diagnostics when problems occur. Tool selection guidance and integration templates ensure agents actively use engram as their primary context source rather than falling back to file search.

**Why this priority**: All other features are valueless if the daemon is unreliable. The current implementation has not demonstrated reliable availability and connection stability in active workspaces. This reliability gate must be cleared before advanced features can be trusted.

**Independent Test**: Can be fully tested by running the daemon in an active workspace for an extended session, performing concurrent tool calls, triggering file events, and verifying zero dropped connections and consistent state. Delivers value: the daemon becomes trustworthy enough to serve as the foundation for all other features.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 **Given** a daemon started in a workspace, **When** 100 sequential tool calls are issued over a 2-hour period, **Then** all calls receive correct responses with zero timeouts or connection errors.
- [x] #2 **Given** 3 concurrent AI assistants connected to the same workspace daemon, **When** all three issue tool calls simultaneously, **Then** all calls are processed correctly without data corruption or deadlocks.
- [x] #3 **Given** a daemon that has been idle for 30 minutes and then receives a tool call, **When** the call arrives, **Then** the daemon responds within 2 seconds, including any re-initialization time.
- [x] #4 **Given** an IDE that restarts while the daemon is running, **When** the IDE reconnects, **Then** the daemon accepts the new connection and serves all previously stored workspace state.
- [x] #5 **Given** a daemon crash during a write operation, **When** the daemon restarts, **Then** the workspace state is consistent (no half-written records) and the event ledger accurately reflects the last successful operation. --- ### Edge Cases - What happens when a dependency chain contains a cycle (A blocks B blocks A)? The system must detect and reject cycles at edge-creation time. - How does the system handle rollback of an event that created an entity now referenced by later events? Cascading undo must be explicit and bounded. - What happens when the file watcher cannot keep up with rapid file changes (e.g., `git checkout` switching hundreds of files)? The debounce window should absorb bursts; degraded mode is acceptable. - How does the system handle a sandboxed query that would scan the entire database? Query execution must have a timeout or row limit to prevent resource exhaustion. - What happens when an agent attempts to create a collection with the same name as an existing one? The system should return a descriptive conflict error. - How does the system behave when the event ledger grows very large (thousands of events)? Compaction or pruning of old events should be supported.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Phase 5: User Story 6 — Reliable Daemon Availability (Priority: P1)

**Goal**: Harden daemon for sustained multi-hour sessions with concurrent clients

**Independent Test**: Run daemon with 3 concurrent clients for extended period, verify zero dropped connections

### Tests for User Story 6 ⚠️

- [X] T037 [P] [US6] Integration test: 3 concurrent clients issuing tool calls without corruption (S061) in tests/integration/reliability_test.rs
- [X] T038 [P] [US6] Integration test: concurrent reads during write maintain consistency (S062) in tests/integration/reliability_test.rs
- [X] T039 [P] [US6] Integration test: client disconnect does not affect other clients (S063) in tests/integration/reliability_test.rs
- [X] T040 [P] [US6] Integration test: state consistent after simulated crash (S064) in tests/integration/reliability_test.rs

### Implementation for User Story 6

- [X] T041 [US6] Audit and harden RwLock usage in src/server/state.rs — verify no deadlock potential under concurrent access
- [X] T042 [US6] Add connection health monitoring spans in src/server/sse.rs and src/daemon/ipc_server.rs
- [X] T043 [US6] Verify atomic write-to-temp-then-rename in src/services/dehydration.rs survives simulated interruption
- [X] T044 [US6] Create agent integration template at .engram/agent-templates/tool-selection-guide.md — MCP tool usage examples for AI assistants

**Checkpoint**: Daemon proven reliable for extended concurrent sessions

---
<!-- SECTION:PLAN:END -->

