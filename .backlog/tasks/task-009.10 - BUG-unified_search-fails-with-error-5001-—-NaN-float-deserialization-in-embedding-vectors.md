---
id: TASK-009.10
title: >-
  BUG: unified_search fails with error 5001 — NaN float deserialization in
  embedding vectors
status: To Do
assignee: []
created_date: '2026-03-26 00:44'
labels:
  - bug
  - 009
  - search
  - embeddings
dependencies: []
references:
  - >-
    src/db/queries.rs (hybrid_graph_vector_search, list_symbols,
    upsert_content_record)
  - src/services/embedding.rs
  - src/db/schema.rs (embedding vector field definitions)
parent_task_id: TASK-009
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Bug Report

`unified_search` returns a hard error on every call, making the semantic search capability of the running daemon completely unavailable.

### Error

```
5001 — Database operation failed: failed to deserialize;
       expected a 32-bit floating point, found NaNf64
```

### Observed Behaviour

Every call to `unified_search` via the engram IPC channel returns error 5001 immediately. The tool does not return partial results — it fails entirely.

### Root Cause (Hypothesis)

One or more embedding vector records stored in the `content_record` or symbol tables in SurrealDB contain `NaN` (`f64::NAN`) values in their embedding vector fields. When SurrealDB attempts to deserialize these records into Rust `f32` values during a KNN or hybrid vector query, the cast from `NaNf64 → f32` is rejected by the deserializer.

Likely cause: a previous indexing run produced `NaN` embeddings (e.g., due to a zero-length input, an ORT model error, or a fastembed numerical instability), and those records were persisted to the database without a pre-write validation step.

### Affected Code

- `src/db/queries.rs` — `hybrid_graph_vector_search` and `list_symbols` query paths that join embedding vectors
- `src/services/embedding.rs` — embedding generation pipeline; no NaN guard before persistence
- `src/db/queries.rs` — `upsert_*` methods that write embedding vectors: no pre-write validation

### Impact

- Semantic search (`unified_search` MCP tool) is fully unavailable whenever corrupted records exist
- Agents operating under the engram-first protocol are forced to fall back to `list_symbols` + `map_code` + `impact_analysis` for all discovery work
- The fallback works but loses cross-cutting semantic search across commits and context records — a meaningful capability gap
- Any agent that treats a `unified_search` error as fatal (rather than degrading gracefully) will abort its workflow

### Reproduction

1. Start the engram daemon with an existing indexed workspace
2. Call `unified_search` with any query string
3. Observe error 5001 on every call

### Workarounds (available today)

- Use `list_symbols(name_contains=...)` for symbol discovery
- Use `map_code(symbol)` for call-graph traversal
- Use `impact_analysis(symbol)` for blast-radius analysis
- These three tools together cover the full blast-radius analysis workflow documented in AGENTS.md Principle IX
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 unified_search returns results (or an empty result set) for any valid query string — never error 5001
- [ ] #2 The embedding generation pipeline validates that no NaN values are present in a vector before persisting it to the database; records with NaN embeddings are rejected with a descriptive EngramError at write time
- [ ] #3 A database repair migration or tool (e.g., gc_embeddings or index_workspace --repair) can detect and delete corrupted records with NaN embedding vectors from an existing database without requiring a full re-index
- [ ] #4 After the repair tool runs, unified_search returns results without error 5001 on the previously affected database
- [ ] #5 cargo test --test integration_unified_search passes, including a regression test that inserts a record with a NaN embedding vector and asserts it is rejected at write time (not at read time)
- [ ] #6 cargo clippy -- -D warnings passes with zero new warnings
<!-- AC:END -->
