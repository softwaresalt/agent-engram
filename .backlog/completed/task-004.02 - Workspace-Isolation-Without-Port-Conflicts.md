---
id: TASK-004.02
title: '004-02: Workspace Isolation Without Port Conflicts'
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
As a developer working on multiple projects simultaneously, I need each project's memory service to run independently without port collisions, shared state, or cross-contamination, so I can safely use AI assistants across all my workspaces at the same time.

Each workspace's memory service communicates through a workspace-local channel (not a network port), making conflicts impossible regardless of how many projects are open. No workspace can access another workspace's data, even if both services are running concurrently.

**Why this priority**: Port collisions and cross-workspace data leaks are the primary problems motivating this refactoring. Without isolation, the architecture change has no value over the current centralized model.

**Independent Test**: Can be fully tested by installing the plugin in two separate workspaces, starting both simultaneously, storing data in each, and verifying that queries in one workspace never return data from the other. Delivers value: developers can run 20+ workspaces without conflicts.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 **Given** two workspaces (A and B) each with the engram plugin, **When** both memory services are started simultaneously, **Then** neither service interferes with the other and both respond normally.
- [x] #2 **Given** workspace A with task "Fix login bug" stored, **When** an agent in workspace B queries for tasks, **Then** the query returns only workspace B's tasks with no leakage from workspace A.
- [x] #3 **Given** 20 workspaces with active memory services, **When** a new workspace starts its service, **Then** it starts successfully without displacement or conflict with existing services. ---
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

