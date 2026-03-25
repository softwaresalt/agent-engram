---
id: TASK-008.01
title: '008-01: Native Graph Traversal (SurrealQL)'
status: Done
assignee: []
created_date: '2026-03-21'
labels:
  - feature
  - '008'
  - graph
  - surreal-db
dependencies: []
references:
  - src/db/queries.rs
  - src/tools/read.rs
parent_task_id: TASK-008
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Replace the application-level BFS in `bfs_neighborhood()` and `get_outbound_edges()`/`get_inbound_edges()` with SurrealQL
native graph traversal queries.

**Beads ID**: agent-engram-dxo.1

### Current State (the problem)
- `bfs_neighborhood()` at `queries.rs:3472` uses a manual `VecDeque`-based BFS with flat `SELECT` queries per hop
- `get_outbound_edges()` at `queries.rs:3550` iterates over each of 5 edge tables with separate queries
- This results in N×5 round-trips for an N-hop traversal across `calls`, `imports`, `defines`, `inherits_from`, `concerns`

### Target State
Single SurrealQL query using `->edge->` syntax for multi-hop traversal. For variable-depth: `SELECT ->calls[WHERE depth <= $max_depth]->function FROM $root`.

**Expected impact**: Reduce N×5 round-trips per traversal to 1-2 queries regardless of depth. Eliminates application-level BFS entirely.

### References
- Current BFS: `src/db/queries.rs:3472-3598`
- Current RELATE: `src/db/queries.rs:3319-3373`
- 4 node tables: `code_file`, `function`, `class`, `interface` (SCHEMAFULL)
- 5 edge tables: `calls`, `imports`, `defines`, `inherits_from`, `concerns` (TYPE RELATION)
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 `map_code` uses native SurrealQL `->edge->` traversal, not manual BFS
- [x] #2 `impact_analysis` uses single graph query, not per-hop SELECT loop
- [x] #3 Multi-hop traversal (depth=2, depth=3) returns correct transitive closure
- [x] #4 All 5 edge types are reachable in a single query
- [x] #5 Contract tests pass: map_code and impact_analysis output schemas unchanged
- [x] #6 Integration tests pass for single-hop and multi-hop traversal
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Tasks (Beads: agent-engram-dxo.1.*)

### dxo.1.1 — Replace bfs_neighborhood() and edge traversal with SurrealQL graph queries

**File**: `src/db/queries.rs`

Replace the manual BFS implementation:
- `bfs_neighborhood()` at `queries.rs:3472` — remove `VecDeque` BFS, replace with native SurrealQL `->edge->` query
- `get_outbound_edges()` at `queries.rs:3550` — remove per-edge-table iteration loop
- `get_inbound_edges()` — same treatment as outbound

Target implementation uses:
```surql
SELECT
  ->calls->function AS called_functions,
  ->defines->function AS defined_functions,
  ->imports->code_file AS imported_files,
  <-calls<-function AS callers
FROM $root_symbol
FETCH called_functions, defined_functions, imported_files, callers;
```

**Leaf task — dxo.1.1.1**: Implement `graph_neighborhood()` using SurrealQL `->edge->` syntax for single-query graph traversal replacing manual BFS.

### dxo.1.2 — Update map_code and impact_analysis handlers for native graph query results

**File**: `src/tools/read.rs`

Update the `map_code` and `impact_analysis` MCP tool handlers to consume the new native SurrealQL graph query result format:
- `map_code` handler: adapt from `bfs_neighborhood()` struct to native query result shape
- `impact_analysis` handler: similarly update to use traversal results
- All existing tool contracts (input/output schemas) must remain backward compatible

### dxo.1.3 — Contract tests for native graph traversal query schemas

**File**: `tests/contract/`

Test coverage:
- `map_code` tool input schema (symbol name, depth parameter) remains unchanged
- `map_code` tool output schema (`CodeGraphNeighborhood` structure) remains backward compatible
- `impact_analysis` tool input schema remains unchanged
- `impact_analysis` tool output schema remains backward compatible
- Error responses for invalid inputs use correct error codes

**Dependencies**: dxo.1.1, dxo.1.2

### dxo.1.4 — Integration tests for native graph traversal

**File**: `tests/integration/`

Test coverage:
- Index a small codebase with known call/import/define/inherit/concern relationships
- Verify single-hop traversal returns correct neighbors for all 5 edge types
- Verify multi-hop traversal (depth=2, depth=3) returns correct transitive closure
- Verify performance: single graph query instead of N×5 SELECT queries
- Test edge cases: root symbol with no outbound edges, circular references

**Dependencies**: dxo.1.1, dxo.1.2
<!-- SECTION:PLAN:END -->
