---
id: TASK-001.02
title: '001-02: Task State Management'
status: Done
assignee: []
created_date: '2026-02-05'
labels:
  - feature
  - 001
  - userstory
  - p2
dependencies: []
references:
  - specs/001-core-mcp-daemon/spec.md
parent_task_id: TASK-001
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
As an orchestrator or agent, I create, update, and query tasks within my workspace so that work progress is tracked and persisted across sessions.

**Why this priority**: Task management is the core value proposition. Once connected, clients need to read and write task state to coordinate work.

**Independent Test**: Connect to workspace, call `create_task` to add a new task, call `update_task` to modify its status, call `get_task_graph` to verify the change, then call `flush_state` and verify the `.engram/tasks.md` file reflects the update.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 **Given** an ACTIVE workspace, **When** `create_task(title, description, parent_id?)` is called, **Then** a new task is created with status `todo` and a unique ID is returned
- [x] #2 **Given** an ACTIVE workspace with existing tasks, **When** `update_task(id, "in_progress", "Starting work")` is called, **Then** the task status changes and a context note is appended
- [x] #3 **Given** a task in progress, **When** `add_blocker(task_id, "Waiting for API response")` is called, **Then** the task status becomes "blocked" and a blocker context node is created
- [x] #4 **Given** an ACTIVE workspace, **When** `get_task_graph(root_id)` is called, **Then** a tree view of subtasks and dependencies is returned with current status
- [x] #5 **Given** an ACTIVE workspace, **When** `register_decision("auth", "Use OAuth2")` is called, **Then** an architectural decision record is stored in the graph ---
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Phase 4: User Story 2 - Task State Management (Priority: P2)

**Goal**: Clients can create, update, and query tasks with graph relationships

**Independent Test**: Connect, update_task to change status, get_task_graph to verify, add_blocker to block

### Tests for User Story 2

- [X] T041 [P] [US2] Contract test for update_task in tests/contract/write_test.rs
- [X] T042 [P] [US2] Contract test for add_blocker in tests/contract/write_test.rs
- [X] T043 [P] [US2] Contract test for register_decision in tests/contract/write_test.rs
- [X] T044 [P] [US2] Contract test for get_task_graph in tests/contract/read_test.rs
- [X] T045 [P] [US2] Contract test for check_status in tests/contract/read_test.rs
- [X] T046 [P] [US2] Unit test for cyclic dependency detection in src/db/queries.rs
- [X] T047 [P] [US2] Property test for Task serialization round-trip in tests/unit/proptest_models.rs
- [X] T129 [P] [US2] Contract test for create_task returning WorkspaceNotSet (1003) when workspace not bound in tests/contract/write_test.rs
- [X] T130 [P] [US2] Contract test for create_task with empty title returning TaskTitleEmpty (3005) in tests/contract/write_test.rs
- [X] T131 [P] [US2] Integration test for create_task with parent_task_id creating depends_on edge in tests/integration/hydration_test.rs

### Implementation for User Story 2

- [X] T048 [US2] Implement task CRUD operations in src/db/queries.rs
- [X] T049 [US2] Implement graph edge operations (depends_on, implements, relates_to) in src/db/queries.rs
- [X] T050 [US2] Implement cyclic dependency detection before edge insert in src/db/queries.rs
- [X] T051 [US2] Implement update_task tool in src/tools/write.rs
- [X] T052 [US2] Implement add_blocker tool in src/tools/write.rs
- [X] T053 [US2] Implement register_decision tool in src/tools/write.rs
- [X] T054 [US2] Implement get_task_graph tool in src/tools/read.rs (recursive graph traversal)
- [X] T055 [US2] Implement check_status tool in src/tools/read.rs
- [X] T056 [US2] Add context note creation on task update in src/services/connection.rs

### Create Task Tool (FR-013a, Session 2026-02-12)

- [X] T132 [US2] Add TaskTitleEmpty (3005) error variant to TaskError enum and wire to error code mapping in src/errors/mod.rs
- [X] T133 [US2] Add error code constant `TASK_TITLE_EMPTY: u16 = 3005` in src/errors/codes.rs
- [X] T134 [US2] Add `create_task` query method to Queries struct: insert task with generated UUID, `todo` status, optional parent via depends_on edge in src/db/queries.rs
- [X] T135 [US2] Implement `create_task` tool: validate title (non-empty, max 200 chars), accept optional description/parent_task_id/work_item_id, call Queries, return created task in src/tools/write.rs
- [X] T136 [US2] Add `create_task` dispatch route to tools::dispatch() match arm in src/tools/mod.rs

### Gap Analysis Updates (Session 2026-02-09)

- [X] T121 [US2] Implement task status transition validation per data-model.md state machine (reject invalid transitions like done→blocked) in src/tools/write.rs
- [X] T122 [P] [US2] Contract test for invalid task status transition (error 3002) in tests/contract/write_test.rs
- [X] T127 [P] [US2] Contract test for work_item_id assignment and retrieval via update_task and get_task_graph in tests/contract/write_test.rs (FR-017 coverage)

**Checkpoint**: Full task CRUD (including create), graph operations, and state transition validation functional

---
<!-- SECTION:PLAN:END -->

