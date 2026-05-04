---
title: "Daemon Reliability — Data-Plane Fixes & Concurrency Hardening Phase 2"
description: "Scope validation for a combined shipment of four daemon reliability tasks: flush_state JSONL writes, rehydration test fix, perf observability instrumentation, and index_workspace SQLITE_BUSY retry"
topic: "Daemon runtime reliability — pre-existing test failures and concurrency gap"
depth: "standard"
decision_status: "decided"
promoted_to: "both"
linked_artifacts:
  - "docs/decisions/2026-05-01-cozodb-concurrency-hardening-deliberation.md"
  - ".backlogit/queue/002-D.md"
tags:
  - "daemon"
  - "reliability"
  - "flush-state"
  - "rehydration"
  - "perf-observability"
  - "SQLITE_BUSY"
  - "concurrency"
stash_entries:
  - "A3B7C1D4"
  - "E5F2A8B9"
  - "44452A7D"
  - "9CFB4DBA"
---

## Problem Frame

Four medium-priority stash entries target daemon runtime reliability issues that
pre-date or follow the 037-F CozoDB concurrency hardening (019-S). Three are
pre-existing integration test failures confirmed on clean main before 037-F; one
is a residual concurrency gap discovered after 037-F shipped. All four share the
daemon data-plane domain and together represent a stability sweep that would
bring CI closer to a fully green bar.

The operator selected these as Group C ("Daemon Reliability & Concurrency
Hardening Phase 2") from a grouping analysis that proposed three options.

**Success criteria:**

- All four `#[ignore]` test annotations removed and tests pass under `cargo test`
- No new `SQLITE_BUSY` panics in concurrent `index_workspace` scenarios
- `flush_state` writes `nodes.jsonl` to `.engram/code-graph/{branch}/`
- Perf-observability stat buckets populated during integration test execution
- No regressions in existing passing tests

**Scope boundaries:**

- OUT: cozo 0.8+ upgrade (deferred — does not exist on crates.io)
- OUT: Kotlin parser activation (blocked upstream — `tree-sitter-kotlin` 0.25)
- OUT: SQL parser CREATE PROCEDURE (blocked upstream — `tree-sitter-sequel` grammar)
- OUT: `002-F` backlog hydration (stale legacy item, needs separate triage)

## Research Findings

### Task A3B7C1D4 — Rehydration test timeout

- **Test**: `graph_vector_rehydration_test::daemon_rehydrates_graph_and_vector_state_after_db_directory_is_deleted`
- **Current state**: `#[ignore]` with annotation "pre-existing: nodes.jsonl flush path broken before 025-F changes"
- **Root cause hypothesis**: The test calls `index_flush_and_seed_embedding`, then deletes the DB directory, then expects rehydration from JSONL. But `flush_state` does not write `nodes.jsonl` (see 44452A7D below), so rehydration finds an empty graph.
- **Dependency**: Blocked on 44452A7D — fixing flush_state is a prerequisite for this test.

### Task E5F2A8B9 — Perf observability stat buckets

- **Tests**: `query_perf_observability_test` (3 tests: `graph_traversal_query_records_timing_stat`, `hybrid_search_query_records_timing_stat`, and likely a third)
- **Module**: `src/services/query_stats.rs` — global singleton with `record_timing()`, `reset_timing()`, `timing_snapshot()`
- **Root cause hypothesis**: The query execution paths (`graph_neighborhood`, `hybrid_graph_vector_search`) may not be calling `record_timing()` in integration test context, or the `OnceLock` singleton initialization timing differs between test and production.
- **Independence**: No dependency on other tasks in this group.

### Task 44452A7D — flush_state nodes.jsonl gap

- **Module**: `src/tools/write.rs` (flush_state handler) → `src/services/dehydration.rs` (dehydrate_code_graph)
- **Current state**: `flush_state` calls `dehydrate_code_graph` which writes `nodes.jsonl` and `edges.jsonl` to `{data_dir}/code-graph/{branch}/`. The dehydration code looks correct — it queries all nodes from the DB and serializes them.
- **Root cause hypothesis**: The gap may be that `flush_state` passes `data_dir` (which resolves to a CozoDB storage path like `.engram/cozo/{branch}/`) but `dehydrate_code_graph` writes to `{data_dir}/code-graph/{branch}/`. The test may expect files at `.engram/code-graph/main/` but the data_dir may not be `.engram/`. Investigation needed during implementation.
- **Blast radius**: Low — dehydration is a serialization path, not a query path.

### Task 9CFB4DBA — SQLITE_BUSY retry in index_workspace

- **Context**: 037-F (019-S) added an fd-lock around `DbInstance::new` + schema bootstrap and a retry wrapper (`run_script_retrying`) for individual schema scripts. But write transactions during `index_workspace` (after the DB is open) can still hit SQLITE_BUSY when two workspaces run concurrently on overlapping DB paths.
- **Test**: `s_cs4` concurrent indexing test is `#[ignore]` because of this gap.
- **Module**: `src/tools/write.rs` (index_workspace handler) → `src/services/code_graph.rs` (index_workspace service)
- **Approach**: Add exponential back-off retry around CozoDB write batches in the indexing pipeline, similar to the `run_script_retrying` pattern already in `schema.rs`.
- **Risk**: Moderate — concurrency retry logic requires careful timeout/backoff design.

### Prior deliberation

The 002-D deliberation (decided 2026-05-01) covered the fd-lock scope extension
(shipped as 037-F). Task 9CFB4DBA is a natural continuation — the next layer of
concurrency hardening for write transactions after DB open.

## Options Evaluated

### Option A: Combined Shipment — All Four Tasks

Ship all four tasks as a single feature covering daemon data-plane reliability.
Tasks execute in dependency order: 44452A7D first (flush_state fix), then
A3B7C1D4 (rehydration test), E5F2A8B9 and 9CFB4DBA in parallel.

**Pros:**

- Single PR, single review cycle, single CI validation
- Coherent release narrative ("daemon reliability sweep")
- Dependencies naturally ordered within one shipment
- Reduces open `#[ignore]` test count by 4+ in one release

**Cons:**

- Moderate risk from concurrency item (9CFB4DBA) could delay the entire shipment
- 8 hours estimated effort — at the upper bound for a single shipment

**Effort**: Medium (4 tasks × 2 hours)
**Fit**: Strong — all tasks share daemon runtime domain

### Option B: Split — Data-Plane Fixes First, Concurrency Separately

Ship 44452A7D + A3B7C1D4 + E5F2A8B9 as one low-risk shipment, then 9CFB4DBA
as a separate follow-up shipment.

**Pros:**

- Isolates concurrency risk from straightforward test fixes
- Faster first shipment (3 tasks, ~6 hours)
- If concurrency work takes longer than expected, the test fixes still ship

**Cons:**

- Two shipments, two PRs, two review cycles
- More overhead for closely related work
- 9CFB4DBA may be deferred indefinitely without the grouping pressure

**Effort**: Low + Medium (split across two shipments)
**Fit**: Adequate but fragmented

## Trade-off Comparison

| Criterion | Option A (Combined) | Option B (Split) |
|---|---|---|
| Coherence | High — single narrative | Medium — related but fragmented |
| Risk | Moderate (concurrency item) | Low first shipment, moderate second |
| Shipping efficiency | 1 PR, 1 review cycle | 2 PRs, 2 review cycles |
| Dependency handling | Natural within shipment | Cross-shipment dependency on 002-D |
| CI impact | All `#[ignore]` removed at once | Incremental removal |

## Decision

**Option A: Combined Shipment** — ship all four tasks as "Daemon Reliability —
Data-Plane Fixes & Concurrency Hardening Phase 2."

**Rationale:** The four tasks share the daemon data-plane domain, have a clear
dependency chain, and together represent a coherent stability milestone. The
concurrency item (9CFB4DBA) has a well-understood pattern to follow
(`run_script_retrying` from 037-F) so the risk is contained. The operator
selected this grouping explicitly.

**Covering feature title:** "Daemon Reliability — Data-Plane Fixes & Concurrency Hardening Phase 2"

**Task scope confirmed:**

| Stash ID | Task summary | Dependencies |
|---|---|---|
| `44452A7D` | Fix `flush_state` to write `nodes.jsonl` | None (execute first) |
| `A3B7C1D4` | Fix rehydration test timeout | Depends on `44452A7D` |
| `E5F2A8B9` | Fix perf observability stat buckets | None (independent) |
| `9CFB4DBA` | Add SQLITE_BUSY retry in `index_workspace` | None (independent) |

**Execution order:** 44452A7D → (A3B7C1D4 | E5F2A8B9 | 9CFB4DBA in parallel)

## Rejected Alternatives

**Option B (Split):** Rejected because the overhead of two shipments outweighs
the marginal risk reduction. The concurrency retry pattern is well-understood
from 037-F prior art.

## Unresolved Questions

1. **44452A7D root cause**: Is the `data_dir` path mismatch the actual reason
   `nodes.jsonl` isn't written, or is there a different issue? Ship agent should
   investigate with a characterization test first.
2. **E5F2A8B9 singleton timing**: Does the `OnceLock` singleton behave
   differently in integration test context? May need instrumentation to confirm.
3. **9CFB4DBA retry scope**: Should the retry wrap individual CozoDB write
   batches or the entire `index_workspace` operation? Prefer per-batch retry
   (consistent with `run_script_retrying` granularity).

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Concurrency retry introduces deadlock or livelock | Low | High | Follow proven `run_script_retrying` pattern; cap retries at 20 attempts with exponential backoff |
| flush_state fix causes regression in existing flush paths | Low | Medium | Characterization test before modifying; existing flush tests provide coverage |
| Perf observability fix masks a deeper instrumentation gap | Low | Low | Verify `record_timing` call sites exist in query paths before fixing test |
| Combined shipment blocked by one task | Medium | Medium | Tasks are independently testable; Ship can unblock by reverting one task if needed |
