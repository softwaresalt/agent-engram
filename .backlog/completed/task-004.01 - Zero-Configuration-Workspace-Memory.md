---
id: TASK-004.01
title: '004-01: Zero-Configuration Workspace Memory'
status: Done
assignee: []
created_date: '2026-03-04'
labels:
  - feature
  - 004
  - userstory
  - p1
dependencies: []
references:
  - specs/004-refactor-engram-server-as-plugin/spec.md
parent_task_id: TASK-004
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
As an AI coding assistant invoked in a workspace, I need persistent memory that is automatically available without manual setup, so I can retrieve and store context across sessions without the developer configuring ports, starting servers, or managing processes.

When the assistant issues an MCP tool call (e.g., `query_memory`, `update_task`), the system automatically starts a workspace-scoped memory service if one is not already running. The assistant never needs to know about the underlying process lifecycle — it simply calls tools and gets answers. The memory service is scoped exclusively to the current workspace, preventing cross-project context leakage.

**Why this priority**: This is the fundamental value proposition. Without automatic, zero-configuration workspace memory, the refactoring has no purpose. Every subsequent feature depends on this working seamlessly.

**Independent Test**: Can be fully tested by installing the plugin in a fresh workspace, invoking a memory tool from an MCP client, and verifying the response returns valid data. Delivers immediate value: AI assistants gain persistent workspace memory without any developer intervention.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 **Given** a workspace with the engram plugin installed but no running memory service, **When** an MCP client issues a `set_workspace` tool call, **Then** the memory service starts automatically within 2 seconds and returns a successful response.
- [x] #2 **Given** a workspace with an active memory service, **When** an MCP client issues a tool call, **Then** the response arrives within 50ms for read operations and 10ms for write operations.
- [x] #3 **Given** a workspace with the engram plugin installed, **When** the developer opens a new terminal and invokes an MCP client, **Then** the client connects to the same active memory service without starting a duplicate. ---
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Phase 3: User Story 1 + User Story 2 — Zero-Config Workspace Memory & Workspace Isolation (Priority: P1) 🎯 MVP

**Goal**: MCP clients invoke tools via stdio, shim auto-starts daemon, workspace isolation via per-workspace IPC channels. This is the core value proposition.

**Independent Test**: Install plugin in fresh workspace, invoke `set_workspace` from an MCP client via stdio, verify daemon auto-starts and returns valid response. Open two workspaces simultaneously and verify zero cross-contamination.

### Tests for US1 + US2 (write first, verify they fail)

- [X] T020 [P] [US1] Contract test for shim cold start in tests/contract/shim_lifecycle_test.rs — no daemon running, shim spawns daemon, forwards request, returns response (S001); covers S005 cold start <2s
- [X] T021 [P] [US1] Contract test for shim warm start in tests/contract/shim_lifecycle_test.rs — daemon already running, shim connects and forwards (S002)
- [X] T022 [P] [US1] Contract test for shim error forwarding in tests/contract/shim_lifecycle_test.rs — daemon returns error, shim forwards faithfully (S004, S008)
- [X] T023 [P] [US1] Integration test for shim malformed input in tests/integration/shim_error_test.rs — invalid JSON (S006), empty stdin (S007), daemon timeout (S009), daemon crash (S010)
- [X] T024 [P] [US2] Integration test for multi-workspace isolation in tests/integration/multi_workspace_test.rs — two workspaces with separate data, verify no leakage (S088-S089); covers S091 symlink resolution
- [X] T025 [P] [US2] Integration test for concurrent workspace scaling in tests/integration/multi_workspace_test.rs — 20 workspaces running concurrently (S090 boundary test)

### Implementation for US1 + US2

- [X] T026 [US1] Implement shim IPC client in src/shim/ipc_client.rs — connect to daemon via interprocess LocalSocketStream, send JSON-RPC request, read response with timeout; covers S003, S009-S010
- [X] T027 [US1] Implement shim lifecycle in src/shim/lifecycle.rs — daemon health check (_health IPC message), spawn via std::process::Command, exponential backoff wait for ready; covers S001, S012, S013
- [X] T028 [US1] Implement daemon spawn guard in src/shim/lifecycle.rs — acquire lock before spawn, detect existing daemon, connect to existing if running; covers S028-S029
- [X] T029 [US1] Implement rmcp StdioTransport + ServerHandler in src/shim/transport.rs — rmcp ServerHandler trait impl, call_tool forwards to IPC client, tools/list returns compiled-in registry; covers S003, S015-S016
- [X] T030 [US1] Wire shim subcommand in src/bin/engram.rs — default subcommand invokes shim transport, connects stdio to daemon via IPC
- [X] T031 [US1] Wire daemon subcommand in src/bin/engram.rs — starts daemon process with --workspace arg, binds IPC, enters ready state; covers S034-S036
- [X] T032 [US2] Implement workspace-scoped IPC addressing in src/daemon/ipc_server.rs — each workspace gets unique IPC endpoint via SHA-256 hash prefix; covers S089, S091
- [X] T033 [US1] Implement _health IPC handler in src/daemon/ipc_server.rs — returns status, uptime, workspace, active connections per contracts/ipc-protocol.md (S021)
- [X] T034 [US1] Verify `cargo test` passes for all Phase 3 tests

**Checkpoint**: Core MVP functional — MCP clients can invoke tools via stdio, daemon auto-starts, workspaces fully isolated. User Stories 1 & 2 independently testable.

---
<!-- SECTION:PLAN:END -->

