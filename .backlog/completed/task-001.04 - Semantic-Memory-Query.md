---
id: TASK-001.04
title: '001-04: Semantic Memory Query'
status: Done
assignee: []
created_date: '2026-02-05'
labels:
  - feature
  - 001
  - userstory
  - p4
dependencies: []
references:
  - specs/001-core-mcp-daemon/spec.md
parent_task_id: TASK-001
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
As an AI agent, I query the workspace memory using natural language so that I receive relevant context from specs, tasks, and prior decisions to ground my responses.

**Why this priority**: Semantic search adds intelligence to context retrieval. Functional without it (can use task graph), but significantly enhanced with vector search.

**Independent Test**: Populate a workspace with specs and context, call `query_memory("authentication flow")`, and verify results include semantically related content ranked by relevance.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 **Given** a workspace with specs and context, **When** `query_memory("user login")` is called, **Then** the daemon returns ranked snippets combining vector similarity and keyword matching
- [x] #2 **Given** the embedding model is not yet downloaded, **When** `query_memory` is called for the first time, **Then** the model is lazily downloaded to `~/.local/share/engram/models/`
- [x] #3 **Given** no network access and model in cache, **When** `query_memory` is called, **Then** the search completes using cached model (offline-capable)
- [x] #4 **Given** a query exceeding 500 tokens, **When** `query_memory` is called, **Then** error code 4001 (QueryTooLong) is returned ---
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Phase 6: User Story 4 - Semantic Memory Query (Priority: P4)

**Goal**: Hybrid vector + keyword search returns relevant context for natural language queries

**Independent Test**: Populate specs/context, query_memory with natural language, verify ranked relevant results

### Tests for User Story 4

- [X] T073 [P] [US4] Contract test for query_memory in tests/contract/read_test.rs
- [X] T074 [P] [US4] Unit test for embedding generation in src/services/embedding.rs
- [X] T075 [P] [US4] Unit test for hybrid scoring (0.7 vector + 0.3 keyword) in src/services/search.rs
- [X] T076 [P] [US4] Integration test for lazy model download in tests/integration/embedding_test.rs

### Implementation for User Story 4

- [X] T077 [US4] Create src/services/embedding.rs with fastembed-rs integration
- [X] T078 [US4] Implement lazy model download to ~/.local/share/engram/models/
- [X] T079 [US4] Implement embedding generation for spec and context content
- [X] T080 [US4] Create src/services/search.rs with hybrid search logic
- [X] T081 [US4] Implement vector similarity search using SurrealDB MTREE index
- [X] T082 [US4] Implement keyword matching (BM25-style) for text content
- [X] T083 [US4] Implement weighted score combination (0.7 * vector + 0.3 * keyword)
- [X] T084 [US4] Implement query_memory tool in src/tools/read.rs
- [X] T085 [US4] Add query character limit validation (2000 characters max, error 4001)
- [X] T086 [US4] Wire embedding generation into hydration for missing embeddings

### Gap Analysis Updates (Session 2026-02-09)

- [X] T125 [US4] ~~Update query_memory character limit from 500 tokens to 2000 characters per updated spec (FR-018, SC-003) in src/tools/read.rs~~ — Already implemented in T085 (`MAX_QUERY_CHARS = 2000`)

**Checkpoint**: Semantic search returns relevant ranked results

---
<!-- SECTION:PLAN:END -->

