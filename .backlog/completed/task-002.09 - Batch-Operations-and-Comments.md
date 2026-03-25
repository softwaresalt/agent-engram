---
id: TASK-002.09
title: '002-09: Batch Operations and Comments'
status: Done
assignee: []
created_date: '2026-02-07'
labels:
  - feature
  - 002
  - userstory
  - p9
dependencies: []
references:
  - specs/002-enhanced-task-management/spec.md
parent_task_id: TASK-002
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
As an orchestrator performing bulk task management, I update multiple tasks in a single call and attach discussion comments to tasks so that batch workflows are efficient and task discussions are preserved.

**Why this priority**: Agents frequently need to update multiple tasks in sequence (e.g., marking all subtasks done when a parent completes). Batch operations reduce round-trips. Comments provide discussion threads separate from the append-only context notes.

**Independent Test**: Create 10 tasks, call `batch_update_tasks` to set all to "in_progress" in one call, and verify all 10 are updated with individual context notes. Add multiple comments to a task and verify retrieval in chronological order.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 **Given** a list of task IDs and updates, **When** `batch_update_tasks(updates: [{id, status, notes}])` is called, **Then** all tasks are updated and individual context notes are created for each
- [x] #2 **Given** a batch update where one task ID is invalid, **When** the batch is executed, **Then** valid updates succeed, the invalid one returns an error, and the response includes per-item results
- [x] #3 **Given** a task, **When** `add_comment(task_id, content, author)` is called, **Then** a comment is stored with timestamp and author, separate from context notes
- [x] #4 **Given** a task with multiple comments, **When** task details are retrieved, **Then** comments are returned in chronological order ---
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Phase 11: User Story 9 — Batch Operations and Comments (Priority: P9)

**Goal**: `batch_update_tasks` with per-item results, `add_comment` with chronological retrieval, `.engram/comments.md` serialization.

**Independent Test**: Batch 10 tasks in one call, verify all updated; add comments, verify chronological order.

### Red Phase (Tests First — Expect Failure)

- [X] T073 [P] [US9] Write contract tests for batch_update_tasks and add_comment in tests/contract/write_test.rs: workspace-not-set (1003), valid batch returns per-item results, batch with one invalid ID returns partial failure (3007), valid comment returns comment_id (FR-058, FR-059, FR-062)

### Green Phase (Implementation)

- [X] T074 [US9] Implement batch_update_tasks tool handler in src/tools/write.rs: validate batch.max_size from config (FR-060), iterate updates calling existing update_task logic per item, collect per-item success/failure results, return succeeded + failed counts (FR-058, FR-059)
- [X] T075 [US9] Implement comment queries in src/db/queries.rs: insert_comment(task_id, content, author), get_comments_for_task(task_id) ordered by created_at ASC (FR-061, FR-062, FR-063)
- [X] T076 [US9] Implement add_comment tool handler in src/tools/write.rs: parse task_id + content + author, validate task exists, call insert_comment, return comment_id + task_id + author + created_at (FR-062)
- [X] T077 [US9] Implement comments.md hydration in src/services/hydration.rs: parse ## task:\* section headers, ### timestamp — author comment headers, body content until next header; populate comment table (FR-063b)
- [X] T078 [US9] Implement comments.md dehydration in src/services/dehydration.rs: query comments per task grouped chronologically, write .engram/comments.md with ## task:\* and ### timestamp — author format (FR-063b)
- [X] T079 [US9] Integration test in tests/integration/enhanced_features_test.rs: batch_update_tasks on 10 tasks (one invalid → partial failure), verify per-item results; add 3 comments to one task, verify chronological order; flush → rehydrate → verify comments preserved (SC-019)
- [X] T080 [US9] Edge case test: batch with duplicate task IDs → last update wins, each generates its own context note

**Checkpoint**: Batch operations and comments functional including `.engram/comments.md` serialization. US9 independently testable.

---
<!-- SECTION:PLAN:END -->

