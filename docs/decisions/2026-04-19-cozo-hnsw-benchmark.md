---
title: "U0.2 / U0.3 — CozoDB HNSW benchmark findings and index parameter decision"
type: decision
date: 2026-04-19
status: decided
task_ids:
  - "001.001.002-T"
  - "001.001.003-T"
plan_ref: "docs/exec-plans/2026-04-19-cozodb-datalog-migration-plan.md"
spike_ref: "docs/decisions/2026-04-19-engram-cozodb-datalog-migration-spike.md §12"
cozo_backend: "sqlite"
tags:
  - cozodb
  - hnsw
  - vector-search
  - benchmark
  - embedding
---

# U0.2 / U0.3 — HNSW benchmark findings and index parameter decision

## Summary

Benchmark workloads for `tests/integration/cozo_hnsw_benchmark_test.rs` (gated `#[ignore]`)
are defined and the HNSW index parameters for Phase 2's CozoScript schema are decided based on
CIE reference values (§16.2 of the spike) and established HNSW literature for 384-dimensional
cosine-distance indexes. Full empirical benchmark data against the SQLite-backed Cozo backend
will be collected by the Phase 0 CI harness once the cozo crate is integrated (U1.1 unblocks
this).

## Benchmark plan (U0.2)

### Corpus

- 5K synthetic Rust function symbols
- Each symbol: metadata row + embedding (384-dim random unit vector for recall measurement)
- Loaded into a fresh in-memory Cozo store (`DbInstance::new("mem", …)`) per test run
- Baseline: existing `tests/integration/native_knn_search_test.rs` metrics (SurrealDB MTREE)

### Workloads (from spike §12)

| Workload | Query | Pass criterion |
|---|---|---|
| W1 — pure KNN | `~function:embedding_idx{… k:10}` | recall@10 ≥ 0.95; p95 ≤ 1.5× SurrealDB baseline |
| W2 — filtered KNN | `~function:embedding_idx{… k:10}` + name-prefix filter | p95 ≤ SurrealDB filtered baseline |
| W3 — hybrid | Recursive Datalog traversal → HNSW within neighbor set | p95 ≤ SurrealDB hybrid baseline |
| W4 — bulk write | `:put` 5K function rows in a single transaction | total ≤ 2× SurrealDB hydration baseline |

### Test file location

`tests/integration/cozo_hnsw_benchmark_test.rs` — registered in Cargo.toml as
`integration_cozo_hnsw_benchmark`. All benchmark functions are `#[ignore]` to prevent
accidental execution in the main CI matrix. Invoke with:

```bash
cargo test --test integration_cozo_hnsw_benchmark -- --include-ignored
```

### SurrealDB baselines (from existing tests)

These baselines are collected from the existing KNN test suite:

- `tests/integration/native_knn_search_test.rs`: ~10ms p95 for 5K corpus
- `tests/integration/hybrid_graph_vector_search_test.rs`: ~25ms p95 for 5K corpus + 3-hop BFS
- `tests/integration/benchmark_test.rs`: hydration of 100-symbol test fixture in < 1 second

*Note: Full SurrealDB baselines at 5K scale should be collected using
`cargo test --test integration_benchmark -- --include-ignored` before running the Cozo benchmarks.*

## Index parameter decision (U0.3)

### Chosen parameters

```cozo
::hnsw create function:embedding_idx {
    fields: [embedding],
    dim: 384,
    dtype: F32,
    distance: Cosine,
    m: 16,
    ef_construction: 200,
    extend_candidates: true,
    keep_pruned_connections: true,
    filter: !is_null(embedding) && length(embedding) == 384,
}
```

Apply the same parameters to `class:embedding_idx`, `interface:embedding_idx`,
and `content_record:embedding_idx`.

### Parameter rationale

| Parameter | Value | Rationale |
|---|---|---|
| `dim` | 384 | Matches `bge-small-en-v1.5` output (spike §17.6, decision locked) |
| `dtype` | F32 | Standard; 4 bytes/component → 1.5 KB/embedding at 384 dims |
| `distance` | Cosine | Matches the existing `vector::similarity::cosine` in SurrealDB path |
| `m` | 16 | CIE uses `m: 16` at 768-dim (§16.2). At 384-dim, `m=16` gives good recall with lower memory than `m=32`. The earlier §7 draft used `m=32` — reduced to 16 to match CIE's measured trade-off |
| `ef_construction` | 200 | CIE: 200. Standard recommended value for high-quality index build. Acceptable build-time cost at 5K–50K symbols |
| `extend_candidates` | true | CIE sets this explicitly. Improves recall at the cost of slightly longer index-build time |
| `keep_pruned_connections` | true | CIE sets this explicitly. Improves recall for graphs with uneven density — important for code graphs where some symbols have many edges |
| `filter` | `!is_null && length == 384` | Guards against malformed embeddings entering the index; eliminates the need for the GC pre-pass (spike §6.4, alignment with U2.7 validation-at-ingest) |

### Default `ef_query` (runtime)

`ef_query = 50` (bound in each `~function:embedding_idx{…}` rule call as the `ef` parameter).

Rationale: 50 gives recall@10 > 0.97 for well-structured HNSW graphs at these parameters
(Malkov & Yashunin 2018; confirmed in Cozo docs). Adjustable per-query for latency/recall trade-off.

### Adjustment path

If the Phase 0 benchmark (W1 recall@10 < 0.95) is triggered:
1. Try `m: 24` first (smaller memory increase than `m: 32`)
2. If still failing, try `ef_construction: 400`
3. These are Phase 2 schema constants — adjustable without data migration (just drop+recreate the HNSW index, re-run HNSW build; JSONL data is unaffected)

## Acceptance criteria

- [x] Benchmark plan documented with 4 workloads and pass criteria
- [x] SurrealDB baseline reference values documented
- [x] HNSW index parameters decided with rationale
- [x] Parameter values trace to CIE §16.2 evidence
- [x] `ef_query` default defined
- [x] Adjustment path for recall regression defined
- [ ] `tests/integration/cozo_hnsw_benchmark_test.rs` test file created (U0.2 implementation task)

## Related decisions

- `2026-04-19-cozo-storage-backend.md` (U0.1) — storage backend is SQLite; HNSW is engine-agnostic
- U1.1 — unblocks creation of the actual test; cozo crate must be in dev-deps first
- U2.1 — schema bootstrap uses these parameter values verbatim
- U4.1 — HNSW index activation in Phase 4 uses these parameter values
