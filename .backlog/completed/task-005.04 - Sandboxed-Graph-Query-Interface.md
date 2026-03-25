---
id: TASK-005.04
title: '005-04: Sandboxed Graph Query Interface'
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
As an AI coding assistant, I need the ability to perform complex, exact-match graph queries across my workspace memory — beyond what semantic search provides — so I can answer precise structural questions like "which tasks block this task?" or "what are all in-progress tasks assigned to this agent?"

The system exposes a read-only query interface that allows structured graph traversals and filtered lookups. All queries are sandboxed: they cannot modify data, access other workspaces, or execute arbitrary operations. The query interface provides the analytical power to navigate the full task-file-context graph.

**Why this priority**: Semantic search is excellent for fuzzy retrieval but cannot answer precise structural questions. Agents frequently need to understand dependency chains, filter by exact status, or traverse relationships — capabilities that require structured querying. This unlocks a new class of agent self-awareness about workspace state.

**Independent Test**: Can be fully tested by populating a workspace with tasks, dependencies, and labels, then issuing read-only queries and verifying correct results. Delivers value: agents can answer precise structural questions about workspace state without relying on file parsing.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 **Given** a workspace with 5 tasks in various statuses, **When** an agent queries for all tasks with status `in_progress`, **Then** the query returns exactly the matching tasks with their full details.
- [x] #2 **Given** a task with three outgoing `hard_blocker` edges, **When** an agent queries for all tasks blocked by this task, **Then** the query returns exactly the three downstream tasks.
- [x] #3 **Given** a query that attempts a write operation (INSERT, UPDATE, DELETE), **When** the query is submitted, **Then** the system rejects it with a clear error explaining that only read operations are permitted.
- [x] #4 **Given** a query with valid syntax but referencing a table that does not exist, **When** the query is submitted, **Then** the system returns an empty result set without exposing internal schema details. ---
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Phase 7: User Story 4 — Sandboxed Graph Query Interface (Priority: P2)

**Goal**: Expose read-only SurrealQL queries with sandboxing, timeout, and row limits

**Independent Test**: Populate workspace with tasks and dependencies, issue read-only queries, verify correct results and write rejection

### Tests for User Story 4 ⚠️

- [X] T062 [P] [US4] Contract test: SELECT query returns correct results (S031) in tests/contract/query_test.rs
- [X] T063 [P] [US4] Contract test: graph traversal query returns correct results (S032) in tests/contract/query_test.rs
- [X] T064 [P] [US4] Contract test: INSERT query rejected (S033) in tests/contract/query_test.rs
- [X] T065 [P] [US4] Contract test: DELETE query rejected (S034) in tests/contract/query_test.rs
- [X] T066 [P] [US4] Contract test: UPDATE query rejected (S035) in tests/contract/query_test.rs
- [X] T067 [P] [US4] Contract test: DEFINE statement rejected (S041) in tests/contract/query_test.rs
- [X] T068 [P] [US4] Contract test: RELATE statement rejected (S042) in tests/contract/query_test.rs
- [X] T069 [P] [US4] Contract test: invalid syntax returns QUERY_INVALID (S038) in tests/contract/query_test.rs
- [X] T070 [P] [US4] Contract test: non-existent table returns empty result (S039) in tests/contract/query_test.rs
- [X] T071 [P] [US4] Contract test: row limit enforced (S037) in tests/contract/query_test.rs
- [X] T072 [P] [US4] Contract test: query without workspace returns WORKSPACE_NOT_SET (S043) in tests/contract/query_test.rs

### Implementation for User Story 4

- [X] T073 [US4] Implement query sanitizer in src/services/gate.rs — word-boundary keyword blocklist validation (INSERT, UPDATE, DELETE, CREATE, DEFINE, REMOVE, RELATE, KILL, SLEEP, THROW); MUST use word-boundary detection and MUST NOT match keywords inside quoted string literals
- [X] T074 [US4] Implement query_graph tool in src/tools/read.rs — sanitize, execute with timeout, enforce row limit, return results
- [X] T075 [US4] Register query_graph in src/tools/mod.rs dispatch

**Checkpoint**: Agents can query workspace graph with full sandboxing

---
<!-- SECTION:PLAN:END -->

