---
id: TASK-002.02
title: '002-02: Task Priorities and Labels'
status: Done
assignee: []
created_date: '2026-02-07'
labels:
  - feature
  - 002
  - userstory
  - p2
dependencies: []
references:
  - specs/002-enhanced-task-management/spec.md
parent_task_id: TASK-002
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
As a project manager or orchestrator, I assign priority levels and descriptive labels to tasks so that work can be triaged, filtered, and categorized effectively.

**Why this priority**: Priorities and labels provide the metadata foundation for the ready-work queue and all filtering operations. Without them, tasks are an undifferentiated flat list with no way to express urgency or categorization.

**Independent Test**: Create tasks with different priority levels and labels, then filter for tasks matching specific labels and priority thresholds. Verify correct results returned.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 **Given** a new task, **When** created without an explicit priority, **Then** the system assigns the workspace default priority
- [x] #2 **Given** an existing task, **When** `update_task` is called with a new priority value, **Then** the task priority updates and a context note records the change
- [x] #3 **Given** a task, **When** `add_label(task_id, "frontend")` is called, **Then** the label is associated with the task
- [x] #4 **Given** a task with labels "frontend" and "urgent", **When** `remove_label(task_id, "urgent")` is called, **Then** only "urgent" is removed and "frontend" remains
- [x] #5 **Given** multiple tasks with various labels, **When** filtering by label with AND logic (e.g., "frontend" AND "bug"), **Then** only tasks matching all specified labels are returned ---
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Phase 4: User Story 2 — Task Priorities and Labels (Priority: P2)

**Goal**: Assign priorities, add/remove labels with validation, AND-filter by labels, serialize labels in YAML frontmatter.

**Independent Test**: Create tasks with different priorities and labels, filter for multi-label AND match, verify correct results.

### Red Phase (Tests First — Expect Failure)

- [X] T026 [P] [US2] Write contract tests for add_label and remove_label in tests/contract/write_test.rs: workspace-not-set (1003), valid add returns label_count, duplicate label returns error 3011, label not in allowed_labels returns error 3006 (FR-032, FR-034)

### Green Phase (Implementation)

- [X] T027 [US2] Implement label CRUD queries in src/db/queries.rs: insert_label with UNIQUE check (error 3011 on conflict), delete_label, get_labels_for_task, filter_tasks_by_labels using GROUP BY + HAVING count() for AND logic (FR-031, FR-032, FR-033)
- [X] T028 [US2] Implement add_label tool handler in src/tools/write.rs: parse task_id and label, validate against WorkspaceConfig.allowed_labels if set (error 3006), call insert_label, return task_id + label + label_count (FR-032, FR-034)
- [X] T029 [US2] Implement remove_label tool handler in src/tools/write.rs: parse task_id and label, call delete_label, return task_id + label + remaining label_count (FR-032)
- [X] T030 [US2] Extend update_task handler in src/tools/write.rs to accept priority param: compute priority_order via compute_priority_order(), validate if needed, create context note recording priority change (FR-026)
- [X] T031 [US2] Extend hydration to parse labels array from task YAML frontmatter and populate label table via insert_label in src/services/hydration.rs (FR-031b)
- [X] T032 [US2] Extend dehydration to query labels per task and write labels array into YAML frontmatter in src/services/dehydration.rs (FR-031b)
- [X] T033 [US2] Integration test in tests/integration/enhanced_features_test.rs: create 5 tasks with varying labels, add_label, remove_label; filter by \["frontend", "bug"\] AND logic verifies intersection; flush → rehydrate → verify labels preserved (SC-019)

**Checkpoint**: Priorities and labels fully functional including AND-filtering and round-trip serialization. US2 independently testable.

---
<!-- SECTION:PLAN:END -->

