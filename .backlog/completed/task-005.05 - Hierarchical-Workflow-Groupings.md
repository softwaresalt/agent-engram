---
id: TASK-005.05
title: '005-05: Hierarchical Workflow Groupings'
status: Done
assignee: []
created_date: '2026-03-09'
labels:
  - feature
  - 005
  - userstory
  - p3
dependencies: []
references:
  - specs/005-lifecycle-observability/spec.md
parent_task_id: TASK-005
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
As a developer managing a large feature with dozens of related tasks spanning design, implementation, and testing, I need the ability to group tasks into named collections (also known as epics or workflows), so that an AI assistant can hydrate all relevant context for a feature in a single operation rather than hunting for individual tasks.

A collection node aggregates related tasks, files, and contexts under a named hierarchy. When the assistant requests context for a collection, the system recursively fetches all contained sub-tasks, their associated files, and relevant context entries — assembling a cohesive prompt payload that covers the entire feature scope.

**Why this priority**: Without grouping, agents must discover related tasks through search or explicit references, leading to fragmented context and missed dependencies. Collections solve the "context stuffing" problem by providing curated, feature-scoped views of workspace state. This becomes critical at scale but is not required for basic workflow enforcement.

**Independent Test**: Can be fully tested by creating a collection, adding tasks and sub-tasks to it, then requesting the collection's context and verifying it returns all contained items recursively. Delivers value: feature-scoped context retrieval in a single operation.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 **Given** a collection "Feature X" containing 5 tasks, **When** an agent requests the collection's context, **Then** the system returns all 5 tasks with their descriptions, statuses, and associated file references.
- [x] #2 **Given** a collection with nested sub-collections (e.g., "Design" and "Implementation" under "Feature X"), **When** an agent requests the parent collection, **Then** the system recursively includes tasks from all nested sub-collections.
- [x] #3 **Given** a task that belongs to two different collections, **When** either collection is queried, **Then** the task appears in both result sets.
- [x] #4 **Given** a collection with 50 tasks, **When** an agent requests the collection with a filter for only `in_progress` tasks, **Then** only matching tasks are returned, reducing payload size. ---
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Phase 8: User Story 5 — Hierarchical Workflow Groupings (Priority: P3)

**Goal**: Named collections that group tasks hierarchically with recursive context retrieval

**Independent Test**: Create collection, add tasks, retrieve collection context, verify recursive results

### Tests for User Story 5 ⚠️

- [x] T076 [P] [US5] Contract test: create_collection succeeds (S044) in tests/contract/collection_test.rs
- [x] T077 [P] [US5] Contract test: duplicate collection name rejected (S045) in tests/contract/collection_test.rs
- [x] T078 [P] [US5] Contract test: add_to_collection creates contains edges (S046) in tests/contract/collection_test.rs
- [x] T079 [P] [US5] Contract test: recursive context retrieval (S048) in tests/contract/collection_test.rs
- [x] T080 [P] [US5] Contract test: collection context with status filter (S049) in tests/contract/collection_test.rs
- [x] T081 [P] [US5] Contract test: cyclic collection nesting rejected (S053) in tests/contract/collection_test.rs
- [x] T082 [P] [US5] Contract test: remove_from_collection removes contains edges (S051) in tests/contract/collection_test.rs
- [x] T083 [P] [US5] Contract test: collection_not_found error (S054) in tests/contract/collection_test.rs

### Implementation for User Story 5

- [x] T084 [US5] Implement collection CRUD queries in src/db/queries.rs — create_collection, get_collection, add_member, remove_member, list_members_recursive
- [x] T085 [US5] Implement collection cycle detection in src/db/queries.rs — check_collection_cycle
- [x] T086 [US5] Implement create_collection tool in src/tools/write.rs
- [x] T087 [US5] Implement add_to_collection tool in src/tools/write.rs
- [x] T088 [US5] Implement remove_from_collection tool in src/tools/write.rs
- [x] T089 [US5] Implement get_collection_context tool in src/tools/read.rs — recursive traversal with optional filters
- [x] T090 [US5] Register all collection tools in src/tools/mod.rs dispatch
- [x] T091 [US5] Add collection dehydration to src/services/dehydration.rs — serialize to .engram/collections.md
- [x] T092 [US5] Add collection hydration to src/services/hydration.rs — parse from .engram/collections.md

**Checkpoint**: Collections working with full recursive context retrieval

---
<!-- SECTION:PLAN:END -->

