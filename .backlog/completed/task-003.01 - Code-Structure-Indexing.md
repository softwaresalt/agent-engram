---
id: TASK-003.01
title: '003-01: Code Structure Indexing'
status: Done
assignee: []
created_date: '2026-02-11'
labels:
  - feature
  - 003
  - userstory
  - p1
dependencies: []
references:
  - specs/003-unified-code-graph/spec.md
parent_task_id: TASK-003
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
As a developer or orchestrator, I index a project workspace so that the code structure (files, functions, classes, interfaces, and their relationships) is stored as a navigable graph, enabling precise retrieval without reading entire source files.

**Why this priority**: This is the foundational capability. Without a populated code graph, none of the downstream traversal, linking, or retrieval features can function. Indexing transforms raw source code into structured, queryable knowledge.

**Independent Test**: Point the indexer at a workspace containing 50 Rust source files. Call `index_workspace`. Verify that file, function, class, and interface nodes exist in the graph with correct `calls`, `imports`, and `inherits_from` edges. Verify that each function node has an embedding generated from its summary.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 **Given** a workspace with source files, **When** `index_workspace()` is called, **Then** the system parses each file and creates graph nodes for files, functions, classes, and interfaces with their attributes (name, file path, line range, signature, docstring)
- [x] #2 **Given** a parsed source file with function calls, **When** indexing completes, **Then** `calls` edges exist between caller and callee function nodes
- [x] #3 **Given** a parsed source file with import statements, **When** indexing completes, **Then** `imports` edges exist between the importing file node and the imported module or symbol nodes
- [x] #4 **Given** a class that extends another class, **When** indexing completes, **Then** an `inherits_from` edge exists between the child class and the parent class
- [x] #5 **Given** a function node whose source body fits within the embedding model's token limit, **When** indexing completes, **Then** the node is tagged `explicit_code` and its embedding is generated from the raw source body
- [x] #6 **Given** a function node whose source body exceeds the embedding model's token limit, **When** indexing completes, **Then** the node is tagged `summary_pointer`, its embedding is generated from the function signature and docstring summary only, and the full source body is stored separately for retrieval
- [x] #7 **Given** a `summary_pointer` node matched by a vector search, **When** the result is returned to the caller, **Then** the system returns the full stored source body (not the summary used for embedding)
- [x] #8 **Given** a workspace with files in unsupported languages, **When** `index_workspace()` is called, **Then** unsupported files are skipped with a warning and indexing continues for supported files ---
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Phase 3: User Story 1 — Code Structure Indexing (Priority: P1) MVP

**Goal**: Parse all workspace source files via tree-sitter, create code graph nodes (code_file, function, class, interface) with structural edges (calls, imports, inherits_from, defines), and generate per-symbol embeddings using the tiered strategy.

**Independent Test**: Call `index_workspace` on a workspace with Rust source files. Verify nodes exist in the graph with correct edges and embeddings. Verify Tier 1 vs Tier 2 classification is correct. Verify unsupported/oversized files are skipped.

### Tests for User Story 1

- [X] T031 [P] [US1] Add contract test for `index_workspace` (workspace-not-set returns error 1003, index-in-progress returns error 7003) in tests/contract/write_test.rs

### Implementation for User Story 1

- [X] T032 [US1] Create code_graph indexing orchestration service with file discovery (`ignore` crate for .gitignore + `code_graph.exclude_patterns` from config), parallel file parsing (`spawn_blocking`, concurrency bounded by `code_graph.parse_concurrency`), character-based token counting (4 chars/token), tiered embedding via batched `embed_texts()`, SSE progress event emission (FR-120), and batch edge creation in src/services/code_graph.rs
- [X] T033 [US1] Implement `index_workspace` tool handler that validates workspace, checks in-progress flag, delegates to code_graph service, and returns structured summary in src/tools/write.rs
- [X] T034 [US1] Add `index_workspace` match arm to `dispatch()` in src/tools/mod.rs
- [X] T035 [US1] Add integration test for full index round-trip (index workspace → verify nodes and edges in DB → verify embeddings generated → verify tier classification) in tests/integration/code_graph_test.rs

**Checkpoint**: `index_workspace` is functional. Workspace code is parsed into a navigable graph with embeddings. MVP is testable.

---
<!-- SECTION:PLAN:END -->

