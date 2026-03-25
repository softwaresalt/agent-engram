---
id: TASK-002.06
title: '002-06: Issue Types and Task Classification'
status: Done
assignee: []
created_date: '2026-02-07'
labels:
  - feature
  - 002
  - userstory
  - p6
dependencies: []
references:
  - specs/002-enhanced-task-management/spec.md
parent_task_id: TASK-002
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
As an orchestrator, I classify tasks by type (task, bug, spike, decision, milestone) so that different kinds of work can be filtered and tracked with type-appropriate workflows.

**Why this priority**: Type classification enables differentiated handling. Bugs may need reproduction steps, spikes have time-boxes, milestones aggregate child tasks. This metadata enriches reporting and query capabilities.

**Independent Test**: Create tasks of different types, filter by type, and verify correct results. Create a milestone with child tasks and verify the milestone reflects aggregate child status.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 **Given** a new task, **When** created without an explicit type, **Then** the system assigns the default type "task"
- [x] #2 **Given** a task, **When** `update_task(id, issue_type: "bug")` is called, **Then** the task type changes and a context note records the change
- [x] #3 **Given** multiple tasks of various types, **When** `get_ready_work(issue_type: "bug")` is called, **Then** only bug-type tasks are returned
- [x] #4 **Given** the workspace configuration defines custom types, **When** a task is created with a custom type, **Then** the system accepts and stores the custom type ---
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Phase 8: User Story 6 — Issue Types and Task Classification (Priority: P6)

**Goal**: `issue_type` field with defaults, update support, type filtering on ready-work, custom types from config.

**Independent Test**: Create tasks of different types, filter by type, verify custom type from config.

### Red Phase (Tests First — Expect Failure)

- [x] T054 [P] [US6] Write contract tests for update_task with issue_type param in tests/contract/write_test.rs: valid type change creates context note, invalid type returns error 3010 when allowed_types configured (FR-047, FR-048)

### Green Phase (Implementation)

- [x] T055 [US6] Extend update_task handler in src/tools/write.rs to accept issue_type param: validate against WorkspaceConfig.allowed_types if set (error 3010), update field, create context note recording type change (FR-047, FR-048)
- [x] T056 [US6] Extend hydration to parse issue_type from YAML frontmatter (default "task" when missing) in src/services/hydration.rs (FR-047)
- [x] T057 [US6] Extend dehydration to write issue_type to YAML frontmatter in src/services/dehydration.rs (FR-047)
- [x] T058 [US6] Integration test in tests/integration/enhanced_features_test.rs: create tasks as "task", "bug", "spike"; filter get_ready_work(issue_type: "bug") returns only bugs; custom type from config accepted; type change creates context note

**Checkpoint**: Issue types functional with filtering and config validation. US6 independently testable.

---
<!-- SECTION:PLAN:END -->

