---
id: TASK-002.01
title: '002-01: Priority-Based Ready-Work Queue'
status: Done
assignee: []
created_date: '2026-02-07'
labels:
  - feature
  - 002
  - userstory
  - p1
dependencies: []
references:
  - specs/002-enhanced-task-management/spec.md
parent_task_id: TASK-002
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
As an AI agent or orchestrator, I query the workspace for the next actionable task so that I always work on the highest-priority unblocked item without manually scanning all tasks.

**Why this priority**: This is the single highest-value feature gap. Without a ready-work queue, agents must fetch all tasks and manually filter for actionable items. A smart query that returns prioritized, unblocked, undeferred tasks transforms engram from passive storage into an active work coordinator.

**Independent Test**: Create a workspace with 20 tasks across multiple priority levels, block 5 of them, defer 3 to a future date, and call `get_ready_work`. Verify only unblocked, undeferred tasks are returned, sorted by priority then creation date.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 **Given** a workspace with tasks at various priority levels, **When** `get_ready_work()` is called, **Then** the system returns only tasks that are unblocked, not deferred, and not done, sorted by priority (p0 first) then by creation date
- [x] #2 **Given** a task with `defer_until` set to a future date, **When** `get_ready_work()` is called before that date, **Then** the deferred task is excluded from results
- [x] #3 **Given** a task blocked by an unresolved dependency, **When** `get_ready_work()` is called, **Then** the blocked task is excluded from results
- [x] #4 **Given** a task with `pinned: true`, **When** `get_ready_work()` is called, **Then** pinned tasks appear at the top of results regardless of priority level
- [x] #5 **Given** a request with `limit: 5`, **When** `get_ready_work(limit: 5)` is called, **Then** at most 5 tasks are returned ---
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Phase 3: User Story 1 — Priority-Based Ready-Work Queue (Priority: P1) MVP

**Goal**: `get_ready_work` returns unblocked, undeferred, incomplete tasks sorted by pinned → priority → creation date with limit and 4 filter dimensions.

**Independent Test**: Create 20 tasks across priority levels, block 5, defer 3, call `get_ready_work` and verify filtering + sort order.

### Red Phase (Tests First — Expect Failure)

- [X] T018 [US1] Write contract tests for get_ready_work in tests/contract/read_test.rs: workspace-not-set error (1003), basic call returns tasks, limit parameter caps results, empty workspace returns empty list (FR-027, FR-028)

### Green Phase (Implementation)

- [X] T019 [US1] Implement ready-work SurrealQL query in src/db/queries.rs: WHERE status NOT IN [done, blocked], defer_until IS NULL OR \<= now(), NOT IN blocking subquery (hard_blocker, blocked_by where out.status != done), NOT IN duplicate_of subquery; ORDER BY pinned DESC, priority_order ASC, created_at ASC; LIMIT $limit (FR-027, FR-028, FR-030, FR-037, FR-054)
- [X] T020 [US1] Implement get_ready_work tool handler in src/tools/read.rs: parse params (limit, label, priority, issue_type, assignee, brief, fields), call query, serialize to TaskSummary array, return total_eligible count (FR-027, FR-028)
- [X] T021 [P] [US1] Add label filter dimension to ready-work query via parameterized WHERE clause in src/db/queries.rs: AND-filter using label table join (FR-029, FR-033)
- [X] T022 [P] [US1] Add priority threshold filter to ready-work query in src/db/queries.rs: WHERE priority_order \<= compute_priority_order($threshold) (FR-029)
- [X] T023 [P] [US1] Add issue_type filter to ready-work query in src/db/queries.rs: WHERE issue_type = $type (FR-029)
- [X] T024 [P] [US1] Add assignee filter to ready-work query in src/db/queries.rs: WHERE assignee = $assignee (FR-029)
- [X] T025 [US1] Integration test in tests/integration/enhanced_features_test.rs: 20 tasks at p0–p4, block 5 with hard_blocker, defer 3 to future, pin 1 low-priority; verify get_ready_work returns 12 tasks, pinned first, sorted by priority then created_at; verify limit=5 caps results (SC-011)

**Checkpoint**: `get_ready_work` fully functional with all 4 filter dimensions. US1 independently testable.

---
<!-- SECTION:PLAN:END -->

