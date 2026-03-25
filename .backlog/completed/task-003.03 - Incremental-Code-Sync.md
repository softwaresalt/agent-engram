---
id: TASK-003.03
title: '003-03: Incremental Code Sync'
status: Done
assignee: []
created_date: '2026-02-11'
labels:
  - feature
  - 003
  - userstory
  - p3
dependencies: []
references:
  - specs/003-unified-code-graph/spec.md
parent_task_id: TASK-003
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
As a developer committing code changes, I expect the code graph to stay current without re-indexing the entire workspace, so that retrieval always reflects the latest code state.

**Why this priority**: Full re-indexing is expensive for large codebases. Incremental sync keeps the graph fresh by processing only changed files, making the system practical for active development workflows.

**Independent Test**: Index a workspace, modify 3 files and delete 1 file, then call `sync_workspace`. Verify that only the 4 affected files are re-parsed, their old nodes and edges are replaced, the deleted file's nodes are removed, and all other nodes remain unchanged.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 **Given** a previously indexed workspace with file modifications, **When** `sync_workspace()` is called, **Then** the system detects changed files (via mtime or content hash), re-parses only those files, and updates their graph nodes and edges
- [x] #2 **Given** a deleted source file, **When** `sync_workspace()` is called, **Then** all nodes and edges originating from that file are removed from the graph
- [x] #3 **Given** a newly added source file, **When** `sync_workspace()` is called, **Then** the new file is parsed and its nodes and edges are added to the graph
- [x] #4 **Given** a renamed file (detected as delete + add), **When** `sync_workspace()` is called, **Then** the old file's nodes are removed and the new file's nodes are created, with edges reflecting the new file path
- [x] #5 **Given** no files have changed since the last index, **When** `sync_workspace()` is called, **Then** the system reports "no changes detected" and performs no graph mutations ---
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Phase 5: User Story 3 — Incremental Code Sync (Priority: P3)

**Goal**: Detect changed, added, and deleted files since last index and update only affected nodes. Use two-level hashing (file + symbol) to minimize re-embedding. Preserve concerns edges across file moves via hash-resilient identity.

**Independent Test**: Index a workspace, modify 3 files and delete 1, call `sync_workspace`. Verify only 4 files are re-processed, unchanged symbols keep their embeddings, deleted file nodes are removed, and concerns edges survive file moves.

### Tests for User Story 3

- [x] T042 [P] [US3] Add contract test for `sync_workspace` (workspace-not-set returns 1003, sync while indexing returns 7003) in tests/contract/write_test.rs

### Implementation for User Story 3

- [x] T043 [US3] Add two-level hash comparison logic (file-level content_hash then per-symbol body_hash) and selective re-embedding to src/services/code_graph.rs
- [x] T044 [US3] Add hash-resilient concerns edge relinking logic using `(name, body_hash)` tuple identity matching with orphan cleanup and context notes (FR-124) to src/services/code_graph.rs
- [x] T045 [US3] Implement `sync_workspace` tool handler that detects file changes, delegates to incremental sync, records sync context note (FR-125), and returns structured summary in src/tools/write.rs
- [x] T046 [US3] Add `sync_workspace` match arm to `dispatch()` in src/tools/mod.rs

**Checkpoint**: Code graph stays current with minimal cost. Only changed symbols are re-embedded.

---
<!-- SECTION:PLAN:END -->

