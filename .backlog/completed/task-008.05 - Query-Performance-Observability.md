---
id: TASK-008.05
title: '008-05: Query Performance Observability'
status: Done
assignee: []
created_date: '2026-03-21'
labels:
  - feature
  - '008'
  - observability
  - tracing
dependencies:
  - TASK-008.01
  - TASK-008.02
references:
  - src/db/queries.rs
  - src/tools/lifecycle.rs
parent_task_id: TASK-008
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Add tracing spans around database queries to measure the actual impact of the SurrealDB optimizations.

**Beads ID**: agent-engram-dxo.5

### Target State
- Wrap all SurrealDB queries in `tracing::instrument` spans with query type, table, and result count
- Log slow queries (>100ms) at `WARN` level
- Include query timing in `get_health_report` response

### Files Modified
- `src/db/queries.rs` — `#[instrument]` on all public query methods
- `src/tools/lifecycle.rs` — aggregate query timing stats

**Dependencies**: Builds on Native Graph Traversal (TASK-008.01) and Native KNN Search (TASK-008.02) to provide measurable performance observability.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 All public query methods in `src/db/queries.rs` have `#[tracing::instrument]` spans
- [x] #2 Spans include `query_type`, `table`, and `result_count` fields
- [x] #3 Queries taking >100ms emit a `WARN` level log entry
- [x] #4 `get_health_report` includes `query_timing` section with aggregated stats
- [x] #5 Stats are organized by query type: graph traversal, KNN search, hybrid, CRUD
- [x] #6 Stats reset on workspace change
- [x] #7 Integration tests pass for query performance observability
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Tasks (Beads: agent-engram-dxo.5.*)

### dxo.5.1 — Add tracing instrument spans to all public query methods

**File**: `src/db/queries.rs`

Add `#[tracing::instrument(skip(db), fields(query_type, table, result_count))]` to all public methods on `CodeGraphQueries`:
- Record `query_type` (graph traversal, KNN search, hybrid, CRUD) in span fields
- Record `table` name(s) being queried
- Record `result_count` after query completes
- Log slow queries (>100ms) at `WARN` level using `tracing::warn!`
- Ensure spans are hierarchical: tool handler span > query span > DB operation span

**Leaf task — dxo.5.1.1**: Implement `record_query_metrics()` to emit tracing spans with `query_type`, `table`, `result_count` fields and `WARN` for slow queries >100ms.

### dxo.5.2 — Aggregate query timing stats in health report

**File**: `src/tools/lifecycle.rs`

- Track query execution times in a lightweight in-memory structure (fixed-size circular buffer or moving window)
- Expose aggregated stats in `get_health_report` response: total queries, avg latency, p95 latency, slow query count
- Organize stats by query type: graph traversal, KNN search, hybrid, CRUD
- Reset stats on workspace change (new workspace = new baseline)
- Keep memory overhead minimal

**Dependencies**: dxo.5.1

### dxo.5.3 — Integration tests for query performance observability

**File**: `tests/integration/`

Test coverage:
- Execute graph traversal, KNN search, and hybrid queries against embedded SurrealDB
- Verify tracing spans emitted with correct fields (`query_type`, `table`, `result_count`)
- Verify slow query warnings appear for artificially slow queries
- Verify health report includes `query_timing` section with aggregated stats
- Verify stats organized by query type
- Verify stats reset on workspace change
- Use `tracing-test` or similar crate to capture and assert on emitted spans

**Dependencies**: dxo.5.1, dxo.5.2
<!-- SECTION:PLAN:END -->
