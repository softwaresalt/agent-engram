---
id: TASK-008.02
title: '008-02: Native Vector KNN Search (MTREE Index)'
status: Done
assignee: []
created_date: '2026-03-21'
labels:
  - feature
  - '008'
  - vector-search
  - surreal-db
dependencies: []
references:
  - src/db/queries.rs
  - src/services/search.rs
  - src/tools/read.rs
  - src/db/schema.rs
parent_task_id: TASK-008
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Replace the "load all, score in Rust" pattern with SurrealDB's built-in KNN operator that leverages the existing MTREE indexes.

**Beads ID**: agent-engram-dxo.2

### Current State (the problem)
- `vector_search_symbols()` at `queries.rs:3897` does `SELECT * FROM function` (loads ALL rows into memory)
- Then iterates and computes `cosine_similarity()` in Rust for each row
- Same pattern repeated for `class`, `interface`, and `content_record` tables
- O(n) in total symbol count; ignores MTREE indexes entirely
- Critical bottleneck: scales poorly beyond the current ~900 functions

### Target State
Single KNN query using existing MTREE indexes:
```surql
SELECT *, vector::similarity::cosine(embedding, $query_vec) AS score
FROM function
WHERE embedding <|$limit, COSINE|> $query_vec
ORDER BY score DESC;
```

**Expected impact**: O(log n) index lookup instead of O(n) full scan. Eliminates loading all symbols into memory.

### References
- Current vector queries: `src/db/queries.rs:3897-3999` (full scan)
- `cosine_similarity()`: `src/services/search.rs:110`
- MTREE index definitions: `src/db/schema.rs:122, 140, 158, 235`
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 `unified_search` uses MTREE index (verifiable via SurrealDB EXPLAIN output)
- [x] #2 `vector_search_symbols()` no longer loads all rows into memory
- [x] #3 Similarity scores are returned by SurrealDB, not computed in Rust
- [x] #4 Results match (within floating-point tolerance) previous Rust-side cosine similarity
- [x] #5 `cosine_similarity()` in `services/search.rs` is removed or deprecated
- [x] #6 Contract tests pass: `unified_search` input/output schemas unchanged
- [x] #7 Integration tests pass for all symbol tables: function, class, interface, content_record
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Tasks (Beads: agent-engram-dxo.2.*)

### dxo.2.1 — Replace vector_search_symbols() with native KNN queries

**File**: `src/db/queries.rs`

Replace the full-table-scan implementation with native KNN for all symbol tables:
```surql
SELECT *, vector::similarity::cosine(embedding, $query_vec) AS score
FROM function
WHERE embedding <|$limit, COSINE|> $query_vec
ORDER BY score DESC;
```
Apply same pattern to `class`, `interface`, and `content_record` tables.

**Leaf task — dxo.2.1.2**: Implement `vector_search_symbols_native()` using SurrealDB KNN operator with MTREE indexes for O(log n) search.

### dxo.2.2 — Remove or deprecate application-level cosine_similarity()

**File**: `src/services/search.rs`

- If no other callers exist after dxo.2.1, remove `cosine_similarity()` at `search.rs:110` entirely
- If other callers exist, apply `#[deprecated]` attribute and migrate remaining callers to DB-side scoring
- Ensure all similarity scoring flows through SurrealDB's `vector::similarity::cosine()`

**Dependencies**: dxo.2.1

### dxo.2.3 — Update unified_search handler for DB-returned similarity scores

**File**: `src/tools/read.rs`

- Consume scores returned directly from SurrealDB KNN queries (the `score` field from `vector::similarity::cosine()`)
- Ensure score values are in the same range [0, 1] for backward compatibility
- Update any score thresholding or filtering logic to use DB-returned values

**Dependencies**: dxo.2.1

### dxo.2.4 — Contract tests for native KNN search query schemas

**File**: `tests/contract/`

Test coverage:
- `unified_search` tool input schema (query text, limit, threshold) remains unchanged
- `unified_search` tool output schema (results with scores, symbol metadata) remains backward compatible
- Score field is present and in valid range [0, 1]
- Results count respects the limit parameter
- Error responses for invalid inputs use correct error codes

**Dependencies**: dxo.2.1, dxo.2.3

### dxo.2.5 — Integration tests for native vector KNN search

**File**: `tests/integration/`

Test coverage:
- Index symbols with known embedding vectors; verify KNN returns top-K correctly ordered
- Results match (within floating-point tolerance) previous Rust-side computation
- Verify search does not load all rows into memory (no O(n) behavior)
- Test across all symbol tables: `function`, `class`, `interface`, `content_record`
- Edge cases: table with fewer rows than K, all zero vectors, duplicate embeddings

**Dependencies**: dxo.2.1, dxo.2.3
<!-- SECTION:PLAN:END -->
