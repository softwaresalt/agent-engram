---
title: "059-F — HNSW vector-search current-state audit and adoption recommendation"
type: decision
date: 2026-05-15
status: decided
shipment_id: "044-S"
feature_id: "059-F"
task_ids:
  - "059.001-T"
  - "059.002-T"
tags:
  - hnsw
  - vector-search
  - search
  - cozodb
  - embeddings
---

# 059-F — HNSW vector-search current-state audit and adoption recommendation

## Summary

Engram does **not** currently use HNSW as its authoritative query path for
vector search. The repository bootstraps Cozo HNSW indexes for symbol embedding
tables, but the active search implementation still performs explicit linear
scans and cosine scoring in Rust.

## Current state

### What Engram uses today

| Surface | Current behavior | Evidence |
|---|---|---|
| Database backend | CozoDB is the only supported runtime backend | `src/db/mod.rs` |
| Embedding stack | `fastembed` with `bge-small-en-v1.5` at 384 dimensions | `Cargo.toml`, `src/services/embedding.rs` |
| Symbol vector search | `vector_search_symbols_native()` linearly scans `function_embedding`, `class_embedding`, and `interface_embedding`, then computes cosine similarity in process | `src/db/cozo_queries.rs` |
| Hybrid graph + vector search | BFS neighborhood first, then cosine re-ranking in process | `src/db/cozo_queries.rs`, `src/services/search.rs` |
| HNSW bootstrap | Schema bootstrap attempts to create Cozo HNSW indexes for symbol embedding tables and suppresses several benign failures | `src/db/cozo_backend/schema.rs` |
| Content-record vector search | No HNSW index is bootstrapped for `content_record` embeddings | `src/db/cozo_backend/schema.rs` |
| Alternate ANN store | No `lance` or `lancedb` dependency is present in the active runtime | `Cargo.toml`, repository search |

### Evidence details

1. `Cargo.toml` enables `embeddings` and `cozo-backend` by default, and
   `src/db/mod.rs` hard-fails builds that omit `cozo-backend`
2. `src/services/embedding.rs` defines `EMBEDDING_DIM = 384` and
   `EMBEDDING_MODEL = "bge-small-en-v1.5"`
3. `src/db/cozo_backend/schema.rs` creates HNSW indexes for:
   * `function_embedding`
   * `class_embedding`
   * `interface_embedding`
4. `src/db/cozo_queries.rs` documents and implements the live query path as a
   full linear scan:

   > "Performs a full linear scan across all symbol embedding tables and computes
   > cosine similarity against `query_embedding` ... callers should not assume
   > HNSW acceleration."

5. `src/db/cozo_queries.rs` implements `hybrid_graph_vector_search()` as
   graph traversal plus cosine re-ranking over the discovered neighborhood, not
   as an HNSW-first retrieval flow

## Findings

### HNSW is present, but not authoritative

The strongest current-state answer is:

* **HNSW is configured in the Cozo schema**
* **HNSW is not the authoritative retrieval path that Engram callers rely on**
* **Current query behavior is still correct even if HNSW creation fails**

That means Engram is best described as **HNSW-provisioned but linear-scan
driven**.

### Current implementation risk

Schema bootstrap suppresses several HNSW creation failures, including
unsupported-vector-index and generic HNSW-related errors. That keeps startup
robust, but it also means "HNSW exists in schema code" is not the same as
"HNSW is definitely live in the workspace database."

### Parameter drift exists

The current runtime schema uses:

* `m: 50`
* `ef_construction: 20`

The earlier benchmark decision in
`docs/decisions/2026-04-19-cozo-hnsw-benchmark.md` recommends:

* `m: 16`
* `ef_construction: 200`

This drift weakens the case for deeper HNSW adoption work until the intended
parameter set is reconciled with the shipped schema.

## Trade-offs

| Concern | Adopt HNSW more explicitly now | Keep the current path for now |
|---|---|---|
| Query performance | Better upside once symbol counts are large enough to punish linear scans | Simpler and deterministic for today's shipped path |
| Build and update cost | HNSW creation and maintenance add bootstrap and mutation overhead | Linear scan avoids index-dependence during workspace bring-up |
| Persistence and reliability | Cozo can persist HNSW state, but bootstrap currently tolerates unsupported or failed index creation | Current path still works when index creation is absent or degraded |
| Hybrid search fit | Could accelerate candidate retrieval for symbol search | Existing hybrid flow is graph-first and still needs cosine re-ranking logic |
| Complexity | Requires explicit Cozo HNSW query integration, new tests, and parameter reconciliation | No extra search-path complexity beyond today's code |
| Dependency impact | No new library is required because Cozo is already present | Also avoids tightening runtime coupling to Cozo-specific ANN semantics |

## Recommendation

**Decision: defer explicit HNSW adoption work for the active Engram search path.**

### Why defer

1. The runtime already attempts to provision HNSW, so there is no discovery gap
   to close before a broader product decision
2. The shipped query path still uses linear scan, which means the actual product
   behavior has not yet committed to HNSW semantics
3. Parameter drift between the benchmark decision and the live schema means any
   deeper HNSW investment should start by reconciling what "intended HNSW" means
4. The repository does not yet show production-grade evidence that vector-search
   latency is a bottleneck large enough to justify extra query-path complexity

### Adopt later only if these criteria are met

Adopt HNSW as an explicit, first-class retrieval path when all of the following
are true:

* representative workspace benchmarks show vector-search latency or scale is a
  real operator-facing problem
* schema bootstrap reports HNSW availability as a reliable health signal rather
  than a best-effort side effect
* the query path is updated to issue explicit Cozo HNSW retrieval queries rather
  than depending on linear scan
* recall, ordering, and hybrid-search behavior are covered by tests against
  realistic Rust, Go, and Python or TypeScript corpora

### Avoid for now when these conditions hold

Continue avoiding explicit HNSW adoption when:

* most workspaces remain small enough that linear scan is acceptable
* graph traversal and content-record retrieval dominate relevance more than raw
  vector nearest-neighbor speed
* index-bootstrap reliability remains soft-fail instead of hard-guaranteed

## Disposition

* **Current-state answer**: Engram is not currently HNSW-driven at query time
* **Recommendation**: defer
* **Next-step posture**: treat HNSW query-path adoption as a future performance
  engineering task, not an immediate architecture correction
