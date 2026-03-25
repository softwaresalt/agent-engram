---
id: TASK-003.06
title: '003-06: Impact Analysis Queries'
status: Done
assignee: []
created_date: '2026-02-11'
labels:
  - feature
  - 003
  - userstory
  - p6
dependencies: []
references:
  - specs/003-unified-code-graph/spec.md
parent_task_id: TASK-003
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
As an orchestrator planning a refactor, I query the system to discover which active tasks are affected by changes to a specific code symbol so that I can assess risk and coordinate work across the team.

**Why this priority**: Impact analysis is where the unified graph delivers its highest strategic value. A query like "which active tasks are blocked by the `UserAuth` class refactor?" is impossible for task-only systems or code-only vector stores to answer. It requires traversing cross-region edges.

**Independent Test**: Create 5 tasks, link 3 of them to functions that depend on `UserAuth`. Call `impact_analysis("UserAuth")`. Verify the response lists all 3 linked tasks with their status, priority, and the specific dependency path from `UserAuth` to the linked function.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 **Given** a code symbol with `concerns` edges from tasks, **When** `impact_analysis("UserAuth")` is called, **Then** the system returns all tasks linked (directly or transitively via code dependencies) to that symbol, with the dependency path for each
- [x] #2 **Given** a code symbol with no task links, **When** `impact_analysis` is called, **Then** the system returns the code dependency neighborhood with a note that no tasks reference this symbol
- [x] #3 **Given** a symbol with transitive task links (task → function A → calls → function B, and `UserAuth` → called_by → function B), **When** `impact_analysis("UserAuth", depth: 2)` is called, **Then** the task linked to function A is included because function A transitively depends on `UserAuth`
- [x] #4 **Given** an `impact_analysis` call with `status_filter: "in_progress"`, **When** executed, **Then** only tasks with matching status are included in results ---
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Phase 8: User Story 6 — Impact Analysis Queries (Priority: P6)

**Goal**: Traverse code dependencies and cross-region concerns edges to find all tasks affected by changes to a specific code symbol.

**Independent Test**: Create 5 tasks, link 3 to functions that depend on `EngramError`. Call `impact_analysis("EngramError", depth: 2)`. Verify all 3 tasks appear with dependency paths. Verify status_filter narrows results.

### Tests for User Story 6

- [x] T059 [P] [US6] Add contract test for `impact_analysis` (workspace-not-set 1003, symbol-not-found 7004) in tests/contract/read_test.rs

### Implementation for User Story 6

- [x] T060 [US6] Add cross-region traversal queries (code BFS → collect node IDs → concerns edge lookup → task filtering by status) to src/db/queries.rs
- [x] T061 [US6] Implement `impact_analysis` tool handler with code neighborhood BFS, cross-region edge resolution, dependency path tracking, depth/max_nodes clamping (FR-149), task status filtering, and full source body loading (FR-148) in src/tools/read.rs
- [x] T062 [US6] Add `impact_analysis` match arm to `dispatch()` in src/tools/mod.rs

**Checkpoint**: Agents can assess the blast radius of a code change across both code and task domains.

---
<!-- SECTION:PLAN:END -->

