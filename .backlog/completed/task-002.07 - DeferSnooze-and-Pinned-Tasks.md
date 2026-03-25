---
id: TASK-002.07
title: '002-07: Defer/Snooze and Pinned Tasks'
status: Done
assignee: []
created_date: '2026-02-07'
labels:
  - feature
  - 002
  - userstory
  - p7
dependencies: []
references:
  - specs/002-enhanced-task-management/spec.md
parent_task_id: TASK-002
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
As an agent or developer, I defer a task to a future date or pin an important task so that deferred work resurfaces automatically and critical context stays visible regardless of priority ordering.

**Why this priority**: Deferral prevents agents from repeatedly considering tasks they cannot act on yet (waiting for external input, scheduled for a future sprint). Pinning ensures critical context tasks (architectural constraints, must-read decisions) always appear at the top of results.

**Independent Test**: Defer a task to tomorrow, verify it is excluded from today's ready work. Pin a low-priority task, verify it appears at the top of ready work results ahead of higher-priority unpinned tasks.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 **Given** a task, **When** `defer_task(task_id, until: "2026-03-01")` is called, **Then** the task's `defer_until` field is set and it is excluded from ready-work results until that date
- [x] #2 **Given** a deferred task whose `defer_until` date has passed, **When** `get_ready_work()` is called, **Then** the task reappears in the ready-work queue at its normal priority
- [x] #3 **Given** a task, **When** `pin_task(task_id)` is called, **Then** the task's `pinned` flag is set and it appears at the top of ready-work results
- [x] #4 **Given** a pinned task, **When** `unpin_task(task_id)` is called, **Then** the task returns to its normal priority position ---
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Phase 9: User Story 7 — Defer/Snooze and Pinned Tasks (Priority: P7)

**Goal**: `defer_task`, `undefer_task`, `pin_task`, `unpin_task` tools with ready-work interaction.

**Independent Test**: Defer to tomorrow (excluded from ready-work), pin low-priority (appears first).

### Red Phase (Tests First — Expect Failure)

- [x] T059 [P] [US7] Write contract tests for defer_task, undefer_task, pin_task, unpin_task in tests/contract/write_test.rs: workspace-not-set (1003), valid defer sets field, valid pin sets flag, each creates context note (FR-050, FR-051, FR-052, FR-053)

### Green Phase (Implementation)

- [x] T060 [US7] Implement defer_task tool handler in src/tools/write.rs: parse task_id + until (ISO 8601), set defer_until, create context note "Deferred until {date}" (FR-050, FR-051)
- [x] T061 [US7] Implement undefer_task tool handler in src/tools/write.rs: parse task_id, clear defer_until, create context note with previous defer date (FR-051)
- [x] T062 [US7] Implement pin_task and unpin_task tool handlers in src/tools/write.rs: set/clear pinned flag, create context notes (FR-052, FR-053)
- [x] T063 [US7] Extend hydration to parse defer_until (ISO 8601 datetime) and pinned (boolean) from YAML frontmatter in src/services/hydration.rs (FR-050, FR-052)
- [x] T064 [US7] Extend dehydration to write defer_until and pinned to YAML frontmatter in src/services/dehydration.rs (FR-050, FR-052)
- [x] T065 [US7] Integration test in tests/integration/enhanced_features_test.rs: defer task to tomorrow → excluded from ready-work; undefer → reappears; pin low-priority p4 task → appears above p0 unpinned; unpin → returns to p4 position; pinned tasks sorted by priority among themselves (FR-054)
- [x] T066 [US7] Edge case test: defer_until in the past at hydration time → task immediately eligible for ready-work queue

**Checkpoint**: Defer and pin fully functional with ready-work interaction. US7 independently testable.

---
<!-- SECTION:PLAN:END -->

