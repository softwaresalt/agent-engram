---
id: TASK-001.01
title: '001-01: Daemon Connection & Workspace Binding'
status: Done
assignee: []
created_date: '2026-02-05'
labels:
  - feature
  - 001
  - userstory
  - p1
dependencies: []
references:
  - specs/001-core-mcp-daemon/spec.md
parent_task_id: TASK-001
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
As an MCP client (CLI, IDE, or agent), I connect to the engram daemon and bind to a specific Git repository workspace so that all subsequent operations are scoped to that project's state.

**Why this priority**: This is the foundational capability. Without connection and workspace binding, no other features can function. Every client interaction begins here.

**Independent Test**: Start the daemon, connect via SSE, call `set_workspace` with a valid Git repo path, and verify the connection enters ACTIVE state with workspace metadata returned.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 **Given** the daemon is running, **When** a client connects to the SSE endpoint, **Then** the daemon assigns a unique connection ID and the connection enters CONNECTED state
- [x] #2 **Given** a CONNECTED client, **When** `set_workspace("/path/to/git/repo")` is called, **Then** the daemon validates the path has a `.git/` directory and returns workspace metadata
- [x] #3 **Given** a client with ACTIVE workspace, **When** `get_workspace_status()` is called, **Then** the daemon returns task count, context count, last flush timestamp, and stale-file detection status
- [x] #4 **Given** a client calls `set_workspace` with an invalid path, **When** the path does not exist, **Then** the daemon returns error code 1001 (WorkspaceNotFound) ---
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Phase 3: User Story 1 - Daemon Connection & Workspace Binding (Priority: P1) 🎯 MVP

**Goal**: MCP clients can connect via SSE and bind to a Git repository workspace

**Independent Test**: Start daemon, connect via SSE curl, call set_workspace, verify ACTIVE state returned

### Tests for User Story 1

> **TDD: Write tests FIRST, ensure they FAIL, then implement**

- [X] T022 [P] [US1] Contract test for set_workspace in tests/contract/lifecycle_test.rs
- [X] T023 [P] [US1] Contract test for get_daemon_status in tests/contract/lifecycle_test.rs
- [X] T024 [P] [US1] Contract test for get_workspace_status in tests/contract/lifecycle_test.rs
- [X] T025 [P] [US1] Integration test for SSE connection lifecycle in tests/integration/connection_test.rs
- [X] T026 [P] [US1] Unit test for workspace path validation in src/services/connection.rs

### Implementation for User Story 1

- [X] T027 Create src/server/mod.rs with module structure
- [X] T028 Create src/server/router.rs with axum Router setup and routes
- [X] T029 Create src/server/sse.rs with SSE connection handling and connection ID assignment
- [X] T030 Create src/server/mcp.rs with MCP JSON-RPC request/response handling
- [X] T031 Create src/db/workspace.rs with workspace path hashing and namespace isolation
- [X] T032 Create src/services/mod.rs with module structure
- [X] T033 Create src/services/connection.rs with ConnectionState enum and lifecycle management
- [X] T034 [US1] Implement set_workspace tool in src/tools/lifecycle.rs (path validation, hydration trigger)
- [X] T035 [US1] Implement get_daemon_status tool in src/tools/lifecycle.rs
- [X] T036 [US1] Implement get_workspace_status tool in src/tools/lifecycle.rs
- [X] T037 [US1] Create src/tools/mod.rs with MCP tool registry and dispatch
- [X] T038 [US1] Add SSE keepalive ping (15s interval) in src/server/sse.rs
- [X] T039 [US1] Add connection timeout handling (60s configurable) in src/server/sse.rs
- [X] T040 [US1] Wire up daemon main() in src/bin/engram.rs with graceful shutdown (SIGTERM/SIGINT)

### Clarification Updates (Session 2026-02-09)

- [X] T111 [US1] Implement workspace limit check in set_workspace tool (FR-009a) returning error 1005 when max_workspaces exceeded in src/tools/lifecycle.rs
- [X] T112 [P] [US1] Contract test for workspace limit exceeded (error 1005) in tests/contract/lifecycle_test.rs

### Analyze Remediation (Session 2026-02-12)

- [X] T128 [US1] Implement HTTP GET /health endpoint in src/server/router.rs returning daemon status and active workspace count (FR-026, constitution VII; moved from Phase 8)

**Checkpoint**: Daemon starts, accepts SSE connections, binds workspaces, enforces workspace limits, exposes /health

---
<!-- SECTION:PLAN:END -->

