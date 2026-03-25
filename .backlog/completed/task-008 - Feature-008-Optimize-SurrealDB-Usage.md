---
id: TASK-008
title: '008: Optimize SurrealDB Usage — Native Graph Traversal & Vector Search'
status: Done
type: feature
assignee: []
created_date: '2026-03-21'
labels:
  - feature
  - '008'
  - surreal-db
  - performance
  - graph
  - vector-search
milestone: m-0
dependencies: []
references:
  - .context/backlog.md
  - src/db/queries.rs
  - src/services/search.rs
  - src/services/embedding.rs
  - src/tools/read.rs
  - src/db/schema.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
# Feature 008: Optimize SurrealDB Usage — Native Graph Traversal & Vector Search

**Discovered**: 2026-03-18 during code review of code graph tools
**Beads ID**: agent-engram-dxo

Agent-engram uses SurrealDB 2 as its persistence layer, which natively supports both graph traversal (via `RELATE` edges
and `->edge->` syntax) and vector/KNN search (via `MTREE` indexes with cosine distance). However, the original
implementation treated SurrealDB as a flat key-value store — all graph traversal and vector similarity scoring happened
in application-level Rust code, bypassing the database's purpose-built query engine.

## Problems Addressed

- **Graph queries** (`map_code`, `impact_analysis`) used a manual BFS with one `SELECT out FROM {edge_table} WHERE in = $node`
  per hop per edge type, resulting in N×5 round-trips for an N-hop traversal across 5 edge types.
- **Vector search** (`unified_search`, `vector_search_symbols`) loaded **all rows** from symbol tables into memory, then
  computed cosine similarity in Rust — O(n) in total symbol count, ignoring the existing `MTREE DIMENSION 384 DIST COSINE`
  indexes entirely.
- **Embedding feature flag** silently returned empty results when the `embeddings` Cargo feature was disabled, with no
  user-facing indication of degraded capability.

## Five Areas of Optimization

1. **Native Graph Traversal (SurrealQL)** — replace manual BFS with `->edge->` syntax
2. **Native Vector KNN Search (MTREE Index)** — use `<|K, COSINE|>` KNN operator
3. **Hybrid Graph + Vector Queries** — combine graph traversal with vector similarity in single queries
4. **Embedding Feature Flag Hardening** — surface embedding status, improve defaults
5. **Query Performance Observability** — tracing spans and timing for all DB queries

## Current Architecture Before This Feature

### Graph Storage (correctly implemented)
- 4 node tables: `code_file`, `function`, `class`, `interface` (all `SCHEMAFULL`)
- 5 edge tables: `calls`, `imports`, `defines`, `inherits_from`, `concerns` (all `TYPE RELATION`)
- Edges created via `RELATE $from->calls->$to` in `src/db/queries.rs`

### Vector Storage (correctly defined, never queried natively)
- All symbol tables have `embedding` fields (`array<float>`, 384 dimensions)
- MTREE indexes defined: `DEFINE INDEX function_embedding ON TABLE function COLUMNS embedding MTREE DIMENSION 384 DIST COSINE`
- Equivalent indexes on `class`, `interface`, `spec`, `context`, `content_record`

### Query Layer (the problem, now fixed)
- `bfs_neighborhood()` at `queries.rs:3472` — manual `VecDeque`-based BFS, flat SELECTs per hop
- `vector_search_symbols()` at `queries.rs:3897` — `SELECT * FROM function` (loads all), Rust-side scoring
- `cosine_similarity()` at `services/search.rs:110` — manual dot product calculation in Rust

## References

- Current graph queries: `src/db/queries.rs:3472-3598` (BFS), `3319-3373` (RELATE)
- Current vector queries: `src/db/queries.rs:3897-3999` (full scan)
- MTREE index definitions: `src/db/schema.rs:122, 140, 158, 235`
- Embedding service: `src/services/embedding.rs:1-167`
- Code graph indexer: `src/services/code_graph.rs:70-398`
- SurrealDB graph docs: https://surrealdb.com/docs/surrealql/statements/relate
- SurrealDB vector docs: https://surrealdb.com/docs/surrealql/functions/vector
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 `map_code` with depth=3 completes in <100ms (previously: N×5 round-trips per hop)
- [x] #2 `impact_analysis` uses a single SurrealQL graph query, not a manual BFS loop
- [x] #3 `unified_search` uses MTREE index (verifiable via SurrealDB EXPLAIN)
- [x] #4 Hybrid graph+vector query works for combined structural/semantic search
- [x] #5 Embedding status is visible in health report and workspace statistics
- [x] #6 All existing tool contracts (input/output schemas) remain backward compatible
- [x] #7 Performance improvement is measurable via tracing spans (before/after)
- [x] #8 `unified_search` returns a clear error when embeddings are unavailable rather than silent empty results
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] Native SurrealQL graph traversal replaces all application-level BFS code
- [x] Native KNN operator replaces full-table-scan vector search for all symbol tables
- [x] Hybrid graph+vector query methods implemented and exposed through MCP tool handlers
- [x] `EmbeddingStatus` struct with coverage metrics implemented and surfaced in health report
- [x] `#[tracing::instrument]` spans added to all public query methods
- [x] Contract tests pass for all modified tool schemas
- [x] Integration tests pass for graph traversal, KNN search, and hybrid queries
- [x] `cargo test` and `cargo clippy` pass with zero warnings
<!-- DOD:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
## Native Graph Traversal — Before/After

**Before** (multiple round-trips per hop):
```rust
// 5 queries per node, per hop
let query = format!("SELECT out FROM {edge_table} WHERE in = $node");
```

**After** (single query, multi-hop):
```surql
-- 1 query for full neighborhood, all edge types, configurable depth
SELECT
  ->calls->function AS called_functions,
  ->defines->function AS defined_functions,
  ->imports->code_file AS imported_files,
  <-calls<-function AS callers
FROM $root_symbol
FETCH called_functions, defined_functions, imported_files, callers;
```

For variable-depth traversal:
```surql
SELECT ->calls[WHERE depth <= $max_depth]->function FROM $root;
```

## Native KNN Search — Before/After

**Before** (O(n) full scan in Rust):
```rust
let func_rows: Vec<FunctionRow> = db.query("SELECT * FROM function").await?;
for f in func_rows {
    let score = cosine_similarity(query_embedding, &f.embedding);
}
```

**After** (single query, index-accelerated):
```surql
SELECT *, vector::similarity::cosine(embedding, $query_vec) AS score
FROM function
WHERE embedding <|$limit, COSINE|> $query_vec
ORDER BY score DESC;
```

## Hybrid Graph + Vector Query

SurrealDB uniquely enables combining graph traversal with vector search in a single query:
```surql
-- Find functions semantically similar to X that are within 2 hops of Y
LET $neighbors = SELECT VALUE ->calls[..2]->function FROM $root_symbol;
SELECT *, vector::similarity::cosine(embedding, $query_vec) AS score
FROM function
WHERE id IN $neighbors
  AND embedding <|10, COSINE|> $query_vec
ORDER BY score DESC;
```

## Dependency Graph (Beads IDs)

```
dxo.1 (Native Graph)     → dxo.3 (Hybrid), dxo.5 (Observability)
dxo.2 (Native KNN)       → dxo.3 (Hybrid), dxo.5 (Observability)
dxo.3 (Hybrid)           depends on dxo.1, dxo.2
dxo.4 (Embedding)        independent
dxo.5 (Observability)    depends on dxo.1, dxo.2
```
<!-- SECTION:NOTES:END -->
