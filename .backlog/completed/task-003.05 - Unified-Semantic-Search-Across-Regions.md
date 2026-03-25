---
id: TASK-003.05
title: '003-05: Unified Semantic Search Across Regions'
status: Done
assignee: []
created_date: '2026-02-11'
labels:
  - feature
  - 003
  - userstory
  - p5
dependencies: []
references:
  - specs/003-unified-code-graph/spec.md
parent_task_id: TASK-003
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
As an AI agent, I perform a single natural language query that searches across both code symbols and task/context data so that I receive a holistic view of the workspace relevant to my question.

**Why this priority**: Unified search is the convergence point. Rather than issuing separate `query_memory` (tasks/specs) and `map_code` (code) calls, the agent can ask one question and receive ranked results spanning both regions, weighted by relevance.

**Independent Test**: Populate a workspace with tasks about "billing" and code containing `process_payment`, `TaxCalculator`, and `PaymentGateway` functions. Query `unified_search("billing logic")`. Verify results include both the billing-related tasks and the semantically related code symbols, ranked by combined relevance.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 **Given** an indexed workspace with tasks and code, **When** `unified_search("billing logic")` is called, **Then** the system returns ranked results from both code nodes (functions, classes) and task/context nodes, merged by relevance score
- [x] #2 **Given** a unified search result containing a code node, **When** the result is returned, **Then** it includes the node type (function/class/file), file path, line range, and summary
- [x] #3 **Given** a unified search result containing a task node, **When** the result is returned, **Then** it includes the task title, status, priority, and linked code symbols (if any)
- [x] #4 **Given** a query that matches only code nodes, **When** `unified_search` is called, **Then** only code results are returned (no empty task section padding)
- [x] #5 **Given** a `unified_search` call with `region: "code"` filter, **When** executed, **Then** only code graph nodes are searched, bypassing the task region entirely ---
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Phase 7: User Story 5 — Unified Semantic Search (Priority: P5)

**Goal**: Perform a single natural language query that searches across both code symbols and task/context data, returning merged ranked results.

**Independent Test**: Populate workspace with billing-related tasks and payment-related code. Call `unified_search("billing logic")`. Verify results include both tasks and code symbols ranked by relevance. Verify region filter works.

### Tests for User Story 5

- [x] T054 [P] [US5] Add contract test for `unified_search` (workspace-not-set 1003, empty query 4001 per FR-157) in tests/contract/read_test.rs

### Implementation for User Story 5

- [x] T055 [US5] Add hybrid vector search queries across code tables (function, class, interface) and task tables (task, context, spec) with cosine similarity scoring to src/db/queries.rs
- [x] T056 [US5] Extend search service with cross-region result merging, ranking by descending cosine score, and region filtering in src/services/search.rs
- [x] T057 [US5] Implement `unified_search` tool handler with query embedding, empty query validation (FR-157), region dispatch, and merged response assembly (summary text only, not full bodies per FR-148 exemption) in src/tools/read.rs
- [x] T058 [US5] Add `unified_search` match arm to `dispatch()` in src/tools/mod.rs

**Checkpoint**: Single query spans both code and task domains. Agents get holistic workspace results.

---
<!-- SECTION:PLAN:END -->

