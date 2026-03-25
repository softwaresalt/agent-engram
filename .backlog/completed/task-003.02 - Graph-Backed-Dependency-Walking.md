---
id: TASK-003.02
title: '003-02: Graph-Backed Dependency Walking'
status: Done
assignee: []
created_date: '2026-02-11'
labels:
  - feature
  - 003
  - userstory
  - p2
dependencies: []
references:
  - specs/003-unified-code-graph/spec.md
parent_task_id: TASK-003
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
As an AI agent preparing to modify code, I retrieve the structural neighborhood of a symbol so that my prompt contains exactly the functions, classes, and files that are directly relevant, drastically reducing context pollution.

**Why this priority**: This is the primary value delivery. Once the code graph exists, dependency walking converts it from static storage into an active context-pruning engine. An agent asking "what do I need to know about `process_payment`?" receives precisely the call tree and dependents rather than 10 loosely-related vector chunks.

**Independent Test**: Index a workspace, then call `map_code("process_payment")`. Verify the result contains the function definition plus all direct callers, callees, and type dependencies within the requested traversal depth. Verify that increasing depth to 2 includes transitive dependencies.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 **Given** an indexed workspace with a function `process_payment`, **When** `map_code("process_payment")` is called, **Then** the system returns the function node plus all nodes reachable via 1-hop `calls`, `imports`, and `inherits_from` edges
- [x] #2 **Given** a `map_code` call with `depth: 2`, **When** the function has transitive dependencies, **Then** the system traverses 2 hops and returns the expanded neighborhood
- [x] #3 **Given** a symbol name that matches multiple nodes (e.g., overloaded functions in different files), **When** `map_code` is called, **Then** the system returns results for all matches, grouped by file
- [x] #4 **Given** a symbol name that does not exist in the graph, **When** `map_code` is called, **Then** the system falls back to vector search across function summaries and returns the closest semantic matches
- [x] #5 **Given** a `map_code` call with `depth: 3` on a highly connected node, **When** the traversal result exceeds a configurable node limit (default: 50), **Then** the system truncates results at the limit, prioritizing direct dependencies, and includes a `truncated: true` indicator ---
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Phase 4: User Story 2 — Graph-Backed Dependency Walking (Priority: P2)

**Goal**: Enable retrieval of a symbol's definition plus its graph neighborhood via BFS traversal. Falls back to vector search when the exact symbol name is not found. Provide `list_symbols` for agent discoverability (FR-150).

**Independent Test**: Index a workspace, call `map_code("dispatch", depth: 2)`. Verify the result contains the function definition plus all nodes reachable within 2 hops. Verify truncation at max_nodes. Verify vector search fallback for unknown symbol names. Call `list_symbols` and verify paginated results.

### Tests for User Story 2

- [x] T036 [P] [US2] Add contract test for `map_code` (workspace-not-set returns 1003, empty graph returns 7004 with suggestion) in tests/contract/read_test.rs
- [x] T037 [P] [US2] Add contract test for `list_symbols` (workspace-not-set returns 1003, empty graph returns 7004) in tests/contract/read_test.rs

### Implementation for User Story 2

- [x] T038 [US2] Add application-level BFS traversal queries (1-hop and multi-hop with max_nodes truncation) and symbol listing/filtering queries (by file_path, node_type, name_prefix with pagination) to src/db/queries.rs
- [x] T039 [US2] Implement `map_code` tool handler with exact-name lookup, BFS neighborhood expansion, vector-search fallback (FR-130), depth/max_nodes clamping (FR-149), and full source body loading (FR-148) in src/tools/read.rs
- [x] T040 [US2] Implement `list_symbols` tool handler with file_path, node_type, name_prefix filters and limit/offset pagination (FR-150) in src/tools/read.rs
- [x] T041 [US2] Add `map_code` and `list_symbols` match arms to `dispatch()` in src/tools/mod.rs

**Checkpoint**: `map_code` returns precise structural neighborhoods. `list_symbols` enables agents to discover valid symbol names. Agents can request exactly the code context they need.

---
<!-- SECTION:PLAN:END -->

