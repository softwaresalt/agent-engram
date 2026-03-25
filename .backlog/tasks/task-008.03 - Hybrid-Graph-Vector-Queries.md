---
id: TASK-008.03
title: '008-03: Hybrid Graph + Vector Queries'
status: Done
assignee: []
created_date: '2026-03-21'
labels:
  - feature
  - '008'
  - graph
  - vector-search
  - surreal-db
dependencies:
  - TASK-008.01
  - TASK-008.02
references:
  - src/db/queries.rs
  - src/tools/read.rs
parent_task_id: TASK-008
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Combine SurrealDB graph traversal with vector search in single queries — the real power for agent-engram's use case.

**Beads ID**: agent-engram-dxo.3

SurrealDB uniquely enables combining graph traversal with vector search in a single query. Example: "Find functions semantically similar to X that are within 2 hops of Y in the call graph":

```surql
LET $neighbors = SELECT VALUE ->calls[..2]->function FROM $root_symbol;
SELECT *, vector::similarity::cosine(embedding, $query_vec) AS score
FROM function
WHERE id IN $neighbors
  AND embedding <|10, COSINE|> $query_vec
ORDER BY score DESC;
```

This powers a much more effective `impact_analysis` that considers both structural relationships AND semantic meaning.

**Dependencies**: Requires Native Graph Traversal (TASK-008.01) and Native Vector KNN Search (TASK-008.02) to be completed first.

### Files Modified
- `src/db/queries.rs` — hybrid query methods
- `src/tools/read.rs` — updated `impact_analysis` and `unified_search` handlers
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 `hybrid_graph_vector_search()` method implemented in `src/db/queries.rs`
- [x] #2 `impact_analysis` supports combined mode when both root symbol and query concept provided
- [x] #3 `unified_search` accepts optional `scope_to_symbol` parameter for graph-scoped search
- [x] #4 Hybrid results are a strict subset of graph-only and vector-only result sets
- [x] #5 All existing tool contracts remain backward compatible (new parameters are optional)
- [x] #6 Integration tests pass for hybrid graph+vector queries
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Tasks (Beads: agent-engram-dxo.3.*)

### dxo.3.1 — Add hybrid graph+vector query methods

**File**: `src/db/queries.rs`

Implement `hybrid_graph_vector_search()`:
1. Finds graph neighbors within N hops of a root symbol using `->edge->` syntax
2. Filters those neighbors by semantic similarity using KNN operator
3. Returns results ordered by similarity score

```surql
LET $neighbors = SELECT VALUE ->calls[..2]->function FROM $root;
SELECT *, vector::similarity::cosine(embedding, $query_vec) AS score
FROM function
WHERE id IN $neighbors
  AND embedding <|10, COSINE|> $query_vec
ORDER BY score DESC;
```

**Dependencies**: dxo.1.1, dxo.2.1

### dxo.3.2 — Update impact_analysis and unified_search for combined graph+vector mode

**File**: `src/tools/read.rs`

- `impact_analysis`: when both a root symbol and a query concept are provided, use `hybrid_graph_vector_search()` to find structurally related AND semantically similar symbols
- `unified_search`: add an optional `scope_to_symbol` parameter that restricts vector search to the graph neighborhood of a given symbol
- Consider adding a new `relevance_search` MCP tool that explicitly exposes the combined capability
- All existing tool contracts must remain backward compatible (new parameters are optional)

**Dependencies**: dxo.3.1

### dxo.3.3 — Integration tests for hybrid graph+vector queries

**File**: `tests/integration/`

Test coverage:
- Index a codebase with known graph structure AND known embedding vectors
- Verify hybrid query returns only symbols within specified hop distance AND above similarity threshold
- Verify results are ordered by similarity score
- Verify combined mode returns a strict subset of graph-only and vector-only result sets
- Test `impact_analysis` handler in combined mode
- Test `unified_search` scoped to a symbol's neighborhood
- Edge cases: symbol with no graph neighbors, symbol with embeddings outside neighborhood

**Dependencies**: dxo.3.1, dxo.3.2
<!-- SECTION:PLAN:END -->
