---
id: TASK-002.05
title: '002-05: Task Claiming and Assignment'
status: Done
assignee: []
created_date: '2026-02-07'
labels:
  - feature
  - 002
  - userstory
  - p5
dependencies: []
references:
  - specs/002-enhanced-task-management/spec.md
parent_task_id: TASK-002
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
As one of multiple agents or developers working on the same workspace, I claim a task so that parallel workers do not duplicate effort on the same item.

**Why this priority**: Multi-client workspaces need coordination. Claiming provides a lightweight locking mechanism that prevents two agents from working on the same task simultaneously without heavyweight locking protocols.

**Independent Test**: Connect two clients, have both call `get_ready_work`, have Client A claim a task, then verify Client B's unfiltered `get_ready_work` still includes the claimed task, and verify Client B's `get_ready_work(assignee: "agent-1")` returns only Client A's claimed tasks.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 **Given** an unclaimed ready task, **When** `claim_task(task_id, claimant: "agent-1")` is called, **Then** the task's assignee field is set and a context note records the claim
- [x] #2 **Given** a task already claimed by "agent-1", **When** `claim_task(task_id, claimant: "agent-2")` is called, **Then** the system rejects with a "task already claimed" error including the current claimant
- [x] #3 **Given** a claimed task, **When** `release_task(task_id)` is called by any client, **Then** the assignee is cleared, the task becomes available, and the context note records both the releaser and the previous claimant
- [x] #4 **Given** a claimed task, **When** `get_ready_work(assignee: "agent-1")` is called, **Then** only tasks claimed by "agent-1" are returned
- [x] #5 **Given** no filter specified, **When** `get_ready_work()` is called, **Then** both claimed and unclaimed tasks are returned (claiming does not remove from ready queue by default) ---
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Phase 7: User Story 5 — Task Claiming and Assignment (Priority: P5)

**Goal**: `claim_task` and `release_task` with conflict rejection, context note audit trail, ready-work assignee filter.

**Independent Test**: Two clients, Client A claims, Client B rejected, third-party release, audit trail verified.

### Red Phase (Tests First — Expect Failure)

- [x] T049 [P] [US5] Write contract tests for claim_task and release_task in tests/contract/write_test.rs: workspace-not-set (1003), valid claim sets assignee, already-claimed returns error 3005 with current claimant, release unclaimed returns error 3012, release records previous claimant in context note (FR-044, FR-045, FR-046)

### Green Phase (Implementation)

- [x] T050 [US5] Implement claim/release queries in src/db/queries.rs: claim_task with atomic assignee IS NULL check (return current claimant on conflict), release_task clears assignee and returns previous claimant (FR-044, FR-045)
- [x] T051 [US5] Implement claim_task tool handler in src/tools/write.rs: parse task_id + claimant, call claim query, create context note "Claimed by {claimant}", return task_id + claimant + context_id + claimed_at (FR-044, FR-046)
- [x] T052 [US5] Implement release_task tool handler in src/tools/write.rs: parse task_id, call release query, create context note "Released by {releaser}, previously claimed by {previous}", return task_id + previous_claimant + context_id (FR-044, FR-046)
- [x] T053 [US5] Integration test in tests/integration/enhanced_features_test.rs: Client A claims task, Client B claim rejected (3005), Client B releases Client A's claim, verify context notes record both events with identities, verify get_ready_work(assignee: "agent-1") returns only agent-1's tasks

**Checkpoint**: Task claiming functional with audit trail and ready-work integration. US5 independently testable.

---
<!-- SECTION:PLAN:END -->

