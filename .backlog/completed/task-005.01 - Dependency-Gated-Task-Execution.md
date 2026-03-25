---
id: TASK-005.01
title: '005-01: Dependency-Gated Task Execution'
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
As an AI coding assistant working through a multi-step feature implementation, I need the memory service to enforce task dependency ordering automatically, so that I cannot start work on a task whose prerequisites are incomplete — preventing wasted effort, hallucinated out-of-order execution, and broken downstream workflows.

When the assistant attempts to modify a task that has unresolved blocking dependencies, the system intercepts the operation and returns a clear, actionable error explaining which upstream tasks must be completed first. This forces the assistant to redirect attention to the correct task, drastically reducing wasted tokens and preventing cascading failures.

**Why this priority**: Without dependency enforcement, agents routinely attempt tasks out of order, producing work that must be discarded when prerequisites are later completed differently. This is the single highest-impact improvement for agent productivity — every other feature assumes tasks are executed in the correct order.

**Independent Test**: Can be fully tested by creating two tasks with a blocking dependency, attempting to transition the blocked task to `in_progress`, and verifying the system rejects the transition with a descriptive error citing the blocker. Delivers immediate value: agents stop thrashing on blocked work.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 **Given** task B depends on task A via a `hard_blocker` edge and task A has status `todo`, **When** an agent attempts to transition task B to `in_progress`, **Then** the system rejects the transition with an error message naming task A as the unresolved blocker.
- [x] #2 **Given** task B depends on task A via a `hard_blocker` edge and task A has status `done`, **When** an agent transitions task B to `in_progress`, **Then** the transition succeeds normally.
- [x] #3 **Given** a chain of three tasks A → B → C with `hard_blocker` edges, **When** an agent attempts to transition task C to `in_progress` while task A is still `todo`, **Then** the system rejects the transition citing the entire upstream chain (both A and B) as unresolved.
- [x] #4 **Given** task B depends on task A via a `soft_dependency` edge and task A has status `todo`, **When** an agent transitions task B to `in_progress`, **Then** the transition succeeds but includes a warning that task A is an incomplete soft dependency. ---
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Phase 3: User Story 1 — Dependency-Gated Task Execution (Priority: P1) 🎯 MVP

**Goal**: Enforce task dependency ordering — reject transitions when hard_blocker prerequisites are incomplete

**Independent Test**: Create two tasks with a hard_blocker edge, attempt to transition the blocked task to in_progress, verify rejection with descriptive error

### Tests for User Story 1 ⚠️

> **Write these tests FIRST, ensure they FAIL before implementation**

- [X] T014 [P] [US1] Contract test: update_task rejects in_progress transition when hard_blocker incomplete (S001) in tests/contract/gate_test.rs
- [X] T015 [P] [US1] Contract test: update_task succeeds when hard_blocker complete (S002) in tests/contract/gate_test.rs
- [X] T016 [P] [US1] Contract test: transitive blocking across 3-task chain (S003) in tests/contract/gate_test.rs
- [X] T017 [P] [US1] Contract test: soft_dependency emits warning not rejection (S004) in tests/contract/gate_test.rs
- [X] T018 [P] [US1] Contract test: add_dependency rejects cyclic dependency (S006, S007, S008) in tests/contract/gate_test.rs
- [X] T019 [P] [US1] Integration test: multiple blockers reported in single error (S010) in tests/integration/gate_integration_test.rs
- [X] T020 [P] [US1] Integration test: gate performance under 100-task chain within 50ms (S012) in tests/integration/gate_integration_test.rs

### Implementation for User Story 1

- [X] T021 [US1] Implement check_blockers() recursive graph query in src/db/queries.rs — walks upstream depends_on edges filtering hard_blocker type, returns Vec<BlockerInfo>
- [X] T022 [US1] Implement check_cycle() path-existence query in src/db/queries.rs — detects if adding an edge would create a cycle
- [X] T023 [US1] Implement gate evaluation logic in src/services/gate.rs — calls check_blockers, returns GateResult (pass/fail with blocker details and soft_dependency warnings)
- [X] T024 [US1] Integrate gate check into update_task in src/tools/write.rs — call gate evaluation before applying status transition to in_progress
- [X] T025 [US1] Integrate cycle detection into add_dependency in src/tools/write.rs — call check_cycle before creating edge
- [X] T026 [US1] Add warnings field to update_task response for soft_dependency notifications in src/tools/write.rs

**Checkpoint**: Gate enforcement working — agents cannot start blocked tasks

---
<!-- SECTION:PLAN:END -->

