---
id: TASK-003.04
title: '003-04: Cross-Region Task-to-Code Linking'
status: Done
assignee: []
created_date: '2026-02-11'
labels:
  - feature
  - 003
  - userstory
  - p4
dependencies: []
references:
  - specs/003-unified-code-graph/spec.md
parent_task_id: TASK-003
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
As an orchestrator or agent, I link tasks to the specific code symbols they concern so that the agent can retrieve both the task context and the relevant code in a single query, answering questions like "which files are affected by this bug fix?"

**Why this priority**: Cross-region linking is the "golden edge" that unifies the task graph (Region B, temporal memory) and the code graph (Region A, spatial memory). Without it, the two regions remain siloed and the agent must perform separate queries and manually correlate results.

**Independent Test**: Create a task "Fix authentication timeout," link it to functions `login_user` and `validate_token` via `link_task_to_code`, then call `get_active_context`. Verify the response includes both the task details and the linked function definitions with their dependency neighborhoods.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 **Given** a task and a code symbol, **When** `link_task_to_code(task_id, symbol_name)` is called, **Then** a `concerns` edge is created between the task node and the matching code node
- [x] #2 **Given** a task linked to 3 functions, **When** `get_active_context()` is called and that task has status `in_progress`, **Then** the response includes the task details plus the definitions and 1-hop dependency neighborhoods of all 3 linked functions
- [x] #3 **Given** a symbol name that resolves to multiple nodes, **When** `link_task_to_code` is called, **Then** the system links to all matching nodes and returns the count of links created
- [x] #4 **Given** a task with no code links, **When** `get_active_context()` is called, **Then** the response includes the task details with an empty `relevant_code` section
- [x] #5 **Given** a code node that is deleted during `sync_workspace`, **When** the node had `concerns` edges from tasks, **Then** the orphaned `concerns` edges are cleaned up and affected tasks receive a context note recording the broken link ---
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Phase 6: User Story 4 — Cross-Region Task-to-Code Linking (Priority: P4)

**Goal**: Create and manage `concerns` edges between tasks (Region B) and code symbols (Region A). Implement `get_active_context` to return linked code neighborhoods for the highest-priority in-progress task.

**Independent Test**: Create a task, link it to 2 functions via `link_task_to_code`, call `get_active_context`. Verify the response includes the task plus full code neighborhoods of both linked functions. Unlink one function, verify it disappears from context.

### Tests for User Story 4

- [x] T047 [P] [US4] Add contract tests for `link_task_to_code` (workspace-not-set 1003, invalid task 3001, symbol-not-found 7004) and `unlink_task_from_code` in tests/contract/write_test.rs
- [x] T048 [P] [US4] Add integration test for cross-region concerns edge lifecycle (create link → `get_active_context` → unlink → verify removed) in tests/integration/cross_region_test.rs

### Implementation for User Story 4

- [x] T049 [US4] Add concerns edge CRUD queries (create by task+symbol name with idempotency per FR-152, delete by task+symbol name, orphan cleanup, list by task) to src/db/queries.rs
- [x] T050 [US4] Implement `link_task_to_code` tool handler that resolves symbol names to node IDs and creates idempotent concerns edges (FR-152) in src/tools/write.rs
- [x] T051 [US4] Implement `unlink_task_from_code` tool handler that removes matching concerns edges in src/tools/write.rs
- [x] T052 [US4] Implement `get_active_context` tool handler that returns all in-progress tasks, expands full code neighborhoods (with source bodies) for highest-priority task only, and returns symbol names only for remaining tasks (FR-127) in src/tools/read.rs
- [x] T053 [US4] Add `link_task_to_code`, `unlink_task_from_code`, and `get_active_context` match arms to `dispatch()` in src/tools/mod.rs

**Checkpoint**: Tasks and code are unified via concerns edges. `get_active_context` returns grounded code context.

---
<!-- SECTION:PLAN:END -->

