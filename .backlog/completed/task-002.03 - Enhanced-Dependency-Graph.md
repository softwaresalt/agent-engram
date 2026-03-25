---
id: TASK-002.03
title: '002-03: Enhanced Dependency Graph'
status: Done
assignee: []
created_date: '2026-02-07'
labels:
  - feature
  - 002
  - userstory
  - p3
dependencies: []
references:
  - specs/002-enhanced-task-management/spec.md
parent_task_id: TASK-002
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
As an orchestrator tracking complex projects, I model richer relationships between tasks (parent/child, blocks/blocked-by, duplicates, related) so that the task graph accurately represents real project structure.

**Why this priority**: The current 2-type dependency model (hard_blocker, soft_dependency) cannot express parent-child hierarchies, duplicate detection, or predecessor/successor relationships. Richer edge types unlock structured project decomposition and accurate blocking analysis.

**Independent Test**: Create a parent task with child subtasks, mark one as a duplicate of another, add a blocks/blocked-by relationship, and call `get_task_graph`. Verify all relationship types render correctly in the graph output.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 **Given** a parent task and a child task, **When** `add_dependency(child_id, parent_id, "child_of")` is called, **Then** the parent-child relationship is stored and both tasks reflect the hierarchy
- [x] #2 **Given** task A blocking task B, **When** `add_dependency(B, A, "blocked_by")` is called, **Then** Task B appears as blocked in the ready-work queue while Task A is incomplete
- [x] #3 **Given** task A and task B are duplicates, **When** `add_dependency(B, A, "duplicate_of")` is called, **Then** Task B is marked as a duplicate and excluded from the ready-work queue
- [x] #4 **Given** a dependency that would create a cycle, **When** the dependency is added, **Then** the system rejects it with a cyclic dependency error
- [x] #5 **Given** a parent task with 3 child tasks, **When** all children are marked done, **Then** the parent task is surfaced in get_ready_work as potentially completable (but not auto-closed) ---
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Phase 5: User Story 3 — Enhanced Dependency Graph (Priority: P3)

**Goal**: 8 dependency types via `add_dependency` tool, cycle detection across all types, `duplicate_of` exclusion from ready-work, parent-child surfacing.

**Independent Test**: Parent with children, duplicate, blocked_by — verify all render correctly in `get_task_graph`.

### Red Phase (Tests First — Expect Failure)

- [X] T034 [P] [US3] Write contract tests for add_dependency in tests/contract/write_test.rs: workspace-not-set (1003), valid add of each type, self-reference rejection, cycle rejection (3003) (FR-035b, FR-036)

### Green Phase (Implementation)

- [X] T035 [US3] Implement add_dependency query in src/db/queries.rs: validate dependency_type against 8-variant enum, reject self-reference (in == out), cycle detection via recursive graph traversal across all edge types, insert RELATE edge (FR-035b, FR-036)
- [X] T036 [US3] Implement add_dependency tool handler in src/tools/write.rs: parse from_task_id, to_task_id, dependency_type; call query; return edge details with created_at (FR-035b)
- [X] T037 [US3] Extend get_task_graph in src/tools/read.rs to include all 8 dependency types in graph output with type annotations (FR-035)
- [X] T038 [US3] Extend dehydration to serialize all 8 edge types in .engram/graph.surql in src/services/dehydration.rs (FR-035)
- [X] T039 [US3] Extend hydration to parse all 8 edge types from .engram/graph.surql RELATE statements in src/services/hydration.rs (FR-035)
- [X] T040 [US3] Integration test in tests/integration/enhanced_features_test.rs: parent task with 3 children (child_of), mark duplicate (duplicate_of → excluded from ready-work), add blocked_by (blocked in ready-work), mark all children done → parent surfaced in ready-work as completable (US3 scenario 5) (FR-037)

**Checkpoint**: All 8 dependency types functional with cycle detection and ready-work interaction. US3 independently testable.

---
<!-- SECTION:PLAN:END -->

