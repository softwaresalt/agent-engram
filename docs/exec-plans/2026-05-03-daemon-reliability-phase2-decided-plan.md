---
title: "Daemon Reliability Phase 2 — Decided Plan"
feature: 038-F
shipment: 021-S
date: 2026-05-03
status: decided
archived_from: docs/exec-plans/2026-05-03-daemon-reliability-phase2-plan.md
---

## Problem

Four daemon data-plane reliability gaps after 037-F (CozoDB Concurrency Hardening Phase 1):
1. `flush_state` does not write `nodes.jsonl` to `{workspace}/.engram/code-graph/{branch}/` — causes empty dehydration
2. Rehydration integration test `#[ignore]`d due to gap 1
3. `record_timing` has zero callers — `query_perf_observability` tests not exercised in integration context
4. Write transactions during concurrent `index_workspace` collide with SQLITE_BUSY after DB open (fd-lock covers open only)

## Decisions

| Unit | Decision |
|------|----------|
| 1 — flush_state fallback | `dehydrate_code_graph` compares `count_functions()` vs `all_functions()` INNER JOIN; fills partial rows from `all_function_metas()` fallback before writing JSONL files |
| 2 — rehydration test | Remove `#[ignore]`; fix DB delete path to `{workspace}/.engram/cozo/{branch_safe}/` |
| 3 — query timing | Add `record_timing` calls inside `graph_neighborhood` + `hybrid_graph_vector_search` in `cozo_queries.rs`; cross-layer dependency accepted as pragmatic (tests call `CodeGraphQueries` directly) |
| 4 — SQLITE_BUSY retry | Per-statement retry via `run_script_busy_retry_mutable` private method (5 attempts, 50–500 ms exp back-off) on all 9 mutable `run_script` calls in `upsert_function/class/interface`; top-level wrapper unsafe and removed |

## Rejected Alternatives

- **Unit 4 top-level retry**: `run_with_busy_retry` wrapping `index_workspace_impl` — rejected because `content_hash` is committed before symbol upserts, so retry skips files that were only partially indexed.
- **Unit 3 service-layer instrumentation only**: wrapper at handler/service level — rejected because integration tests call `CodeGraphQueries` directly and would miss instrumentation.

## Files Modified

- `src/services/dehydration.rs` — Unit 1 fallback
- `tests/integration/graph_vector_rehydration_test.rs` — Unit 2 un-ignore + path fix
- `src/db/cozo_queries.rs` — Units 3 + 4
- `src/services/code_graph.rs` — Unit 4 (top-level wrapper removed)

## Outcomes

All four units shipped in PR #74, merged `ed62e22`. Pre-existing Windows subprocess spawn timeout deferred to stash `100EACD8`.
