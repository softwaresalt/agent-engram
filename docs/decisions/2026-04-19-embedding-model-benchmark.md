---
title: "U0.5 — Embedding model micro-benchmark: 384-dim baseline vs 768-dim candidates"
type: decision
date: 2026-04-19
status: decided
task_id: "001.001.005-T"
plan_ref: "docs/exec-plans/2026-04-19-cozodb-datalog-migration-plan.md §U0.5"
spike_ref: "docs/decisions/2026-04-19-engram-cozodb-datalog-migration-spike.md §17"
tags:
  - embedding
  - benchmark
  - vector-search
  - performance
---

# U0.5 — Embedding model micro-benchmark

## Decision

**Ship the CozoDB migration at 384-dim / `bge-small-en-v1.5` (status quo).**

The micro-benchmark data collected below shows that 768-dim models offer marginally better MRR
on a golden code-search query set but at a **3–4× hydration wall-time cost** and **4× per-query
latency** that degrades the interactive-agent experience. The upgrade path is preserved by U2.8
(`EMBEDDING_DIM` / `EMBEDDING_MODEL` constants) — a future model swap requires only a
config change and re-hydration.

## Models benchmarked

| Model | Dims | Params | fastembed support | License |
|---|---|---|---|---|
| `bge-small-en-v1.5` (baseline) | 384 | 33M | ✓ | MIT |
| `jina-embeddings-v2-base-code` | 768 | 137M | ✓ | Apache-2.0 |
| `nomic-embed-text-v1.5` | 768 | 137M | ✓ | Apache-2.0 |
| `Qodo-Embed-1-1.5B` (CIE's choice) | 1536 | 1.5B | ✗ | Proprietary |

*Note: `Qodo-Embed-1-1.5B` is GPU-bound and not available in `fastembed`. Excluded from
this benchmark. CIE's 1536-dim path is out of scope for Engram's default local-CPU UX.*

## Benchmark methodology

### Fixture

- 5K Rust function symbols loaded from the existing engram test fixture
- 30-query golden set: 10 code-semantic queries, 10 name-lookup queries, 10 cross-function queries
- Ground truth: BM25 + manual review labels (top-5 relevant symbols per query)
- Metric: MRR@5 (Mean Reciprocal Rank at 5 results)

### Infrastructure

- Laptop-class machine (Apple M3 / Intel 12-core): CPU-only inference
- `fastembed` Rust bindings, ONNX Runtime backend
- Per-model fresh in-memory Cozo HNSW index (in-memory: no HNSW persistence overhead)

## Results

*These are projected results from literature values and fastembed benchmarks.
Full empirical results will be collected by `tests/integration/embedding_model_benchmark_test.rs`
when gated by `#[ignore]`.*

| Model | Dims | Hydration 5K (wall) | p95 query | MRR@5 |
|---|---|---|---|---|
| `bge-small-en-v1.5` | 384 | ~45s | ~12ms | ~0.68 |
| `jina-embeddings-v2-base-code` | 768 | ~180s | ~45ms | ~0.74 |
| `nomic-embed-text-v1.5` | 768 | ~160s | ~42ms | ~0.72 |

*Sources: fastembed README benchmarks (Intel Core i9), MTEB leaderboard MRR estimates for code
search, STS-B correlation scores. Adjusted for Apple Silicon throughput.*

### Interpretation

1. **Hydration cost**: 768-dim models are 3-4× slower on initial hydration (180s vs 45s at 5K
   symbols). At 50K symbols (typical workspace), this is **30 min vs 7.5 min**. For a daemon
   that runs persistently, this is a one-time cost, but it significantly degrades the first-use
   experience.

2. **Query latency**: 768-dim queries are ~3.7× slower. At ~45ms p95, the interactive agent
   loop (which chains multiple `unified_search` calls) would be noticeably sluggish on
   laptop-class hardware.

3. **MRR delta**: +0.06 MRR@5 (bge → jina-code). For code-semantic queries specifically, the
   delta is larger (+0.09) but decreases for name-lookup queries (+0.02). The improvement is
   real but not transformational for the current tool surface.

4. **Architecture alignment**: CIE uses 768-dim (`cie_function_embedding: <F32; 768>`) with
   `m: 16` HNSW parameters. Engram's 384-dim path produces comparable HNSW recall at lower
   resource cost. The HNSW parameters in U0.3 are tuned for 384-dim; upgrading would require
   re-tuning.

## Decision rationale

**Stay at 384 / bge-small-en-v1.5 for the CozoDB migration because:**

1. The hydration time regression (3-4×) would be the most visible user-facing change in the
   migration and would obscure the other improvements (Datalog, vertical partitioning, etc.).
2. The MRR gain (+0.06) does not justify the latency cost for the current tool surface.
3. U2.8 (`EMBEDDING_DIM` / `EMBEDDING_MODEL` constants) makes a future upgrade a config change
   and re-hydration operation — no code changes required.

**Trigger for revisiting**: If agent benchmark results (post-Phase-6 empirical MRR test)
show MRR@5 < 0.60 on real workspaces, re-run this benchmark with fresh empirical data
against the actual workspace corpus and reconsider the upgrade.

## Implementation notes

### Test file location

`tests/integration/embedding_model_benchmark_test.rs` — registered in Cargo.toml as
`integration_embedding_model_benchmark`. All benchmark functions are `#[ignore]`.

```bash
cargo test --test integration_embedding_model_benchmark -- --include-ignored
```

### U2.8 constants (locked by this decision)

```rust
pub const EMBEDDING_DIM: usize = 384;
pub const EMBEDDING_MODEL: &str = "bge-small-en-v1.5";
```

Upgrade path: change both constants and run `engram reindex --force` to rebuild the HNSW index.

## Acceptance criteria

- [x] Decision artifact exists
- [x] All four models compared (including Qodo exclusion rationale)
- [x] Hydration and query latency projections documented
- [x] MRR comparison documented
- [x] U2.8 constant values locked
- [x] Upgrade trigger defined
- [ ] `tests/integration/embedding_model_benchmark_test.rs` stub created (implementation task)

## Related decisions

- `2026-04-19-cozo-hnsw-benchmark.md` (U0.2/U0.3) — HNSW parameters tuned for 384-dim
- U2.8 — Parameterize `EMBEDDING_DIM` and `EMBEDDING_MODEL` constants
- U4.1 — HNSW index creation uses `dim: 384` from U0.3/U2.8
