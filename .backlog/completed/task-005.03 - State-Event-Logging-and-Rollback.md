---
id: TASK-005.03
title: '005-03: State Event Logging and Rollback'
status: Done
assignee: []
created_date: '2026-03-09'
labels:
  - feature
  - 005
  - userstory
  - p2
dependencies: []
references:
  - specs/005-lifecycle-observability/spec.md
parent_task_id: TASK-005
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
As an AI coding assistant operating on a workspace, I need the ability to review a history of all state changes and roll back to a previous known-good state, so that corrupted or hallucinated state modifications can be undone without losing the entire workspace history.

Every discrete state change (task creation, status transition, edge addition, context storage) is recorded as an immutable event in a ledger. An authorized user or oversight agent can replay events in reverse to restore the workspace graph to a previous point in time, reverting only the affected nodes and edges.

**Why this priority**: Agents operating unattended can produce cascading state corruption through hallucinated updates. Without rollback capability, the only recovery path is manual file editing or full workspace reset, both of which lose valuable accumulated context.

**Independent Test**: Can be fully tested by creating a task, modifying it several times, then issuing a rollback to a specific event, and verifying the task returns to its state at that point. Delivers value: safe undo for any state corruption without manual intervention.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 **Given** a workspace with an event ledger containing 10 recorded state changes, **When** the user requests a rollback to event 7, **Then** events 8–10 are reversed in order and the workspace state matches the state after event 7.
- [x] #2 **Given** a workspace with a task that was created (event 5) and later modified (events 6, 8), **When** a rollback to event 5 occurs, **Then** the task reflects its original creation state and the modifications are undone.
- [x] #3 **Given** a rollback that would remove a dependency edge between two tasks, **When** the rollback is applied, **Then** both the edge record and any derived blocking state are reverted.
- [x] #4 **Given** a request to rollback beyond the oldest available event, **When** the rollback is attempted, **Then** the system rejects it with an error explaining the earliest available rollback point. ---
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Phase 6: User Story 3 — State Event Logging and Rollback (Priority: P2)

**Goal**: Record all state changes in an append-only ledger with rollback capability

**Independent Test**: Create task, modify several times, rollback to earlier event, verify state restored

### Tests for User Story 3 ✅

- [X] T045 [P] [US3] Contract test: task creation records event with kind=task_created (S013) in tests/contract/event_test.rs
- [X] T046 [P] [US3] Contract test: task update records event with previous/new values (S014) in tests/contract/event_test.rs
- [X] T047 [P] [US3] Contract test: edge creation records event (S015) in tests/contract/event_test.rs
- [X] T048 [P] [US3] Contract test: rolling retention prunes oldest events (S016) in tests/contract/event_test.rs
- [X] T049 [P] [US3] Contract test: get_event_history returns filtered results (S017, S018) in tests/contract/event_test.rs
- [X] T050 [P] [US3] Contract test: rollback_to_event restores previous state (S023) in tests/contract/event_test.rs
- [X] T051 [P] [US3] Contract test: rollback denied when allow_agent_rollback=false (S025) in tests/contract/event_test.rs
- [X] T052 [P] [US3] Contract test: rollback to non-existent event returns error (S027) in tests/contract/event_test.rs
- [X] T053 [P] [US3] Integration test: rollback reverses edge creation (S024) in tests/integration/rollback_test.rs
- [X] T054 [P] [US3] Integration test: rollback conflict when entity deleted (S028) in tests/integration/rollback_test.rs

### Implementation for User Story 3

- [X] T055 [US3] Implement event recording functions in src/services/event_ledger.rs — record_event(), prune_events()
- [X] T056 [US3] Implement event ledger queries in src/db/queries.rs — insert_event, list_events, count_events, delete_oldest_events, get_events_after
- [X] T057 [US3] Integrate event recording into all write tools in src/tools/write.rs — create_task, update_task, add_dependency, add_blocker, etc.
- [X] T058 [US3] Implement get_event_history tool in src/tools/read.rs — filtered retrieval with pagination
- [X] T059 [US3] Implement rollback validation in src/services/event_ledger.rs — check event existence, detect conflicts
- [X] T060 [US3] Implement rollback_to_event tool in src/tools/write.rs — reverse events, restore previous values, record rollback event
- [X] T061 [US3] Register get_event_history and rollback_to_event in src/tools/mod.rs dispatch

**Checkpoint**: Event ledger recording all changes, rollback functional

---
<!-- SECTION:PLAN:END -->

