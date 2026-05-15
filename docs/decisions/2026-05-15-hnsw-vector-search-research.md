---
title: "059-F — HNSW vector-search current-state audit and adoption recommendation"
type: decision
date: 2026-05-15
status: in_review
shipment_id: "044-S"
feature_id: "059-F"
task_ids:
  - "059.001-T"
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
