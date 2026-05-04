---
title: "Daemon Reliability — Data-Plane Fixes & Concurrency Hardening Phase 2"
source: "docs/decisions/2026-05-03-daemon-reliability-phase2-deliberation.md"
status: "draft"
feature_type: "feature"
stash_entries:
  - "A3B7C1D4"
  - "E5F2A8B9"
  - "44452A7D"
  - "9CFB4DBA"
---

## Problem Frame

Four daemon runtime reliability issues remain after the 037-F CozoDB concurrency
hardening shipped. Three are pre-existing integration test failures (`#[ignore]`
on clean main); one is a residual concurrency gap discovered after 037-F. All
four target the daemon data plane: flush/dehydration paths, query instrumentation,
and write-transaction concurrency.

**Code surfaces:**

| Concern | Primary module | Test file |
|---|---|---|
| flush_state → nodes.jsonl | `src/tools/write.rs`, `src/services/dehydration.rs` | `tests/integration/graph_vector_rehydration_test.rs` |
| Graph/vector rehydration | `src/services/hydration.rs` | `tests/integration/graph_vector_rehydration_test.rs` |
| Query perf observability | `src/services/query_stats.rs`, `src/db/cozo_queries.rs` | `tests/integration/query_perf_observability_test.rs` |
| index_workspace SQLITE_BUSY | `src/tools/write.rs`, `src/services/code_graph.rs` | Concurrent indexing test (currently `#[ignore]`) |

## Requirements Trace

| Source requirement | Implementation unit |
|---|---|
| flush_state writes nodes.jsonl to .engram/code-graph/{branch}/ | Unit 1 |
| Rehydration test passes after flush+DB delete+restart | Unit 2 |
| Perf stat buckets populated during integration test execution | Unit 3 |
| No SQLITE_BUSY panics in concurrent index_workspace | Unit 4 |

## Implementation Units

### Unit 1: Fix flush_state nodes.jsonl Write Path (44452A7D)

**Execution posture:** characterization-first

**What:** Investigate and fix why `flush_state` does not produce `nodes.jsonl`
at `{workspace}/.engram/code-graph/{branch}/nodes.jsonl` in the integration
test context.

**Root cause analysis:** The `dehydrate_code_graph` function in
`src/services/dehydration.rs` writes to `{data_dir}/code-graph/{branch}/`.
The `flush_state` handler passes `snapshot.data_dir` as `data_dir`. The test
expects the file at `{workspace}/.engram/code-graph/main/nodes.jsonl`.

Investigation needed:
1. Confirm what `snapshot.data_dir` resolves to inside `DaemonHarness` — if it
   points to a temp CozoDB storage path rather than `{workspace}/.engram/`, the
   JSONL files would be written to the wrong location.
2. Check whether the DB contains any nodes at the time `flush_state` is called
   (the dehydration function skips writing when `total_nodes == 0`).

**Files affected:**

- `src/tools/write.rs` — `flush_state` handler (data_dir resolution)
- `src/services/dehydration.rs` — `dehydrate_code_graph` (path construction)
- Possibly `src/server/state.rs` or `src/config/` — workspace snapshot data_dir field

**Tests:**

- Characterization test: call `flush_state` in an integration context and log the
  actual `data_dir` value and file paths written
- Existing test: `graph_vector_rehydration_test::daemon_rehydrates_graph_and_vector_state_after_db_directory_is_deleted`
  — remove `#[ignore]` and verify it passes

**Acceptance criteria:**

- `flush_state` writes `nodes.jsonl` and `edges.jsonl` to
  `{workspace}/.engram/code-graph/{branch}/` when the graph is non-empty
- The assertion at test line 202–207 (`files_written` is non-empty) passes
- The assertion at test line 209–214 (`nodes.jsonl` exists) passes
- `ENGRAM_DATA_DIR` env override path contract is preserved (add regression test)

### Unit 2: Un-ignore Rehydration Test (A3B7C1D4)

**Execution posture:** test-first (verify the ignore can be removed after Unit 1)

**What:** Remove the `#[ignore]` annotation from
`daemon_rehydrates_graph_and_vector_state_after_db_directory_is_deleted` and
verify the full rehydration lifecycle works: index → flush → delete DB →
restart → rehydrate from JSONL.

**Dependency:** Blocked on Unit 1 (flush_state must write nodes.jsonl first).

**Files affected:**

- `tests/integration/graph_vector_rehydration_test.rs` — remove `#[ignore]`
  annotation and the explanatory comment (lines 283–289)
- Possibly adjust the 30-second timeout at line 153 if the startup auto-index
  timing is still tight

**Tests:**

- The test itself IS the deliverable — it must pass end-to-end

**Acceptance criteria:**

- `#[ignore]` annotation removed
- Test passes under `cargo test --test graph_vector_rehydration_test`
- No timeout at the 30s deadline (or increase timeout if root cause is slow
  startup indexing on the temp workspace)

### Unit 3: Add Query Timing Instrumentation (E5F2A8B9)

**Execution posture:** test-first

**What:** Add `record_timing` calls to the query methods in
`src/db/cozo_queries.rs` so the perf-observability stat buckets are populated
when queries execute.

**Root cause (confirmed):** `record_timing` in `src/services/query_stats.rs`
has **zero callers** in production code. The `graph_neighborhood` and
`hybrid_graph_vector_search` methods in `src/db/cozo_queries.rs` do not
instrument themselves. The integration tests call these methods directly, so
instrumentation must live inside the query methods or in a thin wrapper.

**Approach:** Add timing instrumentation at the entry/exit of each query method
in `cozo_queries.rs`. Use `std::time::Instant::now()` + `elapsed()` and call
`query_stats::record_timing("bucket_name", elapsed_ms)` at the end of each
method. Bucket names should match what the tests expect:

- `graph_neighborhood` → `"graph_traversal"` bucket
- `hybrid_graph_vector_search` → `"hybrid_search"` bucket

**Layer trade-off (plan-review amendment):** Placing instrumentation in `db/`
creates a dependency on the `services::query_stats` singleton. This is accepted
as a pragmatic cross-cutting concern: tests call `CodeGraphQueries` directly, so
instrumentation must be at or below the query method boundary to be exercised by
existing tests. The dependency is limited to a single `record_timing()` call per
method. If more cross-cutting concerns accumulate in `db/`, extract a shared
observability utility module. Verify `record_timing` actual parameter type
(`u64` vs `f64`) before implementation.

**Files affected:**

- `src/db/cozo_queries.rs` — add timing instrumentation to `graph_neighborhood`
  and `hybrid_graph_vector_search`
- Possibly `src/services/query_stats.rs` if additional bucket names are needed

**Tests:**

- Existing tests: `query_perf_observability_test` (3 tests):
  - `graph_traversal_query_records_timing_stat`
  - `hybrid_search_query_records_timing_stat`
  - `reset_timing_clears_all_accumulated_stats` (already passes via direct calls)
  - `slow_query_threshold_increments_slow_count_in_snapshot` (already passes via direct calls)

**Acceptance criteria:**

- `graph_traversal_query_records_timing_stat` passes (graph_traversal bucket populated after `graph_neighborhood`)
- `hybrid_search_query_records_timing_stat` passes (hybrid_search bucket populated after `hybrid_graph_vector_search`)
- All 4 perf observability tests pass under `cargo test --test query_perf_observability_test`
- No regressions in other integration tests

### Unit 4: Add SQLITE_BUSY Retry to index_workspace (9CFB4DBA)

**Execution posture:** test-first

**What:** Add retry logic with exponential backoff around CozoDB write batches
in the `index_workspace` pipeline so concurrent workspaces don't panic on
SQLITE_BUSY during write transactions.

**Context:** The fd-lock in `connect_db` covers DB open + schema bootstrap
(037-F), and `run_script_retrying` covers individual schema scripts. But
write transactions during `index_workspace` (after the DB is open) can still
collide when two workspaces share overlapping DB paths.

**Approach:** Follow the `run_script_retrying` backoff parameters from
`src/db/cozo_backend/schema.rs` but implement a **new async retry wrapper**
using `tokio::time::sleep` (the existing helper is synchronous and must not
be used in async Tokio code paths):
- Implement an async retry function local to the indexing pipeline in
  `src/services/code_graph.rs` (do not extract or share `run_script_retrying`
  from schema.rs — keep retry logic co-located with the operation it protects)
- Wrap CozoDB write operations at a **coarse boundary** (per-file batch or
  top-level indexing operation, not per-individual-write) to avoid exploding
  total indexing time under contention
- Cap at 20 attempts, exponential backoff 25ms → 500ms (consistent with schema retry)
- Only retry on SQLITE_BUSY error code; propagate all other errors immediately

**Files affected:**

- `src/services/code_graph.rs` — `index_workspace` function, write batch calls
  (new async retry wrapper implemented here, co-located with indexing logic)
- `src/db/cozo_queries.rs` — if upsert methods need retry wrappers
- Concurrent indexing test file — remove `#[ignore]` from `s_cs4`

**Tests:**

- Un-ignore the `s_cs4` concurrent indexing test
- Verify concurrent `index_workspace` calls on overlapping DB paths complete
  without SQLITE_BUSY panics

**Acceptance criteria:**

- Concurrent `index_workspace` operations succeed with retry instead of panicking
- `s_cs4` concurrent indexing test passes
- Retry attempts are logged via `tracing::warn!` for observability
- No regressions in sequential indexing paths

## Dependency Graph

```text
Unit 1 (flush_state fix)
  ↓
Unit 2 (rehydration test)    Unit 3 (perf observability)    Unit 4 (SQLITE_BUSY retry)
       [independent]              [independent]                  [independent]
```

Unit 1 must complete first. Units 2, 3, and 4 can execute in parallel after
Unit 1, though Ship will likely execute them sequentially.

**Suggested execution order:** 1 → 3 → 4 → 2

Rationale: Unit 3 (perf observability) is the simplest fix and builds
confidence. Unit 4 (concurrency retry) is the riskiest and benefits from
careful attention. Unit 2 (rehydration test) is verification-only after
Unit 1 and should be last to confirm the full flush→rehydrate lifecycle.

## Decisions and Rationale

| Decision | Rationale |
|---|---|
| Instrument inside `cozo_queries.rs` methods | Tests call `CodeGraphQueries` directly, so timing must be recorded at that layer |
| Reuse `run_script_retrying` pattern for index writes | Proven pattern from 037-F; consistent retry semantics across the codebase |
| Characterization-first for flush_state | Root cause is uncertain (path resolution vs empty DB vs timing); characterization test eliminates guesswork |
| 20-attempt / 25ms→500ms backoff for index retry | Matches existing schema retry parameters; ~7.8s worst-case is acceptable for indexing |

## Risks and Caveats

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| flush_state root cause is deeper than path resolution | Medium | Medium | Characterization test will expose actual behavior before fix attempt |
| Timing instrumentation adds measurable overhead to queries | Low | Low | `Instant::now()` + `elapsed()` is sub-microsecond; negligible vs DB roundtrips |
| Retry logic masks genuine corruption errors | Low | High | Only retry on SQLITE_BUSY error code; propagate all other errors immediately |
| Rehydration test is flaky due to startup timing | Medium | Low | Increase timeout from 30s if needed; consider polling with backoff |

## Plan Hardening Signals

- [x] Public API, schema, or contract change — **absent**: no public API changes; internal instrumentation only
- [x] Security, auth, permission, or compliance-sensitive behavior — **absent**
- [x] Migration, backfill, destructive data/config action, or irreversible step — **absent**
- [x] External integration, operator checkpoint, or external dependency — **absent**
- [x] High runtime, rollout, or rollback risk — **absent**: all changes are internal daemon behavior with existing test coverage

**Requires plan hardening: no**

## Runtime Verification and Closure

### Changed runtime surfaces

| Unit | Runtime surface | Verification |
|---|---|---|
| Unit 1 | `flush_state` MCP tool output | Verify `files_written` includes `nodes.jsonl` after indexing a workspace |
| Unit 3 | `get_health_report` MCP tool (timing section) | Verify timing stats appear in health report after running queries |
| Unit 4 | `index_workspace` MCP tool under concurrency | Verify concurrent indexing completes without errors |

### Closure expectations

- `cargo test` passes with all four previously-ignored tests un-ignored
- CI `continue-on-error` annotation updated if these were the last pre-existing failures guarded by it
- Monitoring: observe daemon logs for any SQLITE_BUSY warnings during normal operation
- Rollback trigger: any SQLITE_BUSY panic in production after deploy → revert and investigate

<!-- plan-review-attempt: 1 -->

## Plan Review

**Gate decision: ADVISORY**

Two P1 findings were identified and addressed by targeted plan amendments
(Units 3 and 4). Remaining P2/P3 findings are advisory for the Ship agent.

### Review Personas

| Persona | Model | Findings |
|---|---|---|
| Constitution Reviewer | claude-opus-4.6 | 6 |
| Rust Reviewer | claude-opus-4.6 | 6 |
| Scope Boundary Auditor | claude-opus-4.6 | 5 |
| Architecture Strategist | claude-sonnet-4.5 (cross-model) | 6 |
| Learnings Researcher | claude-haiku-4.5 | 0 (docs/compound/ empty) |

### P1 Findings (Addressed by Plan Amendments)

**P1-1: Unit 3 — Instrumentation layer creates db/→services/ dependency**
Sources: Architecture Strategist (P0→P1), Rust Reviewer
Amendment applied: Added explicit layer trade-off rationale to Unit 3.
Instrumentation stays in cozo_queries.rs (tests call it directly); dependency
limited to single record_timing() call. Type signature verification noted.

**P1-2: Unit 4 — Sync retry helper incompatible with async context**
Sources: Rust Reviewer, Architecture Strategist, Scope Auditor
Amendment applied: Replaced "reuse run_script_retrying" with new async retry
wrapper using tokio::time::sleep. Retry scoped to coarse per-file boundary.
Removed schema.rs extraction from files list.

### P2 Findings (Advisory for Ship Agent)

**P2-1: Unit 1 scope breadth** — 3–4 files plus investigation may exceed 2-hour
rule. Ship agent should split into characterization (1A) and fix (1B) if
investigation reveals root cause spans multiple modules.
Sources: Scope Auditor (P1→P2), Constitution Reviewer

**P2-2: Unit 4 scope breadth** — Touches code_graph.rs and cozo_queries.rs.
Ship agent should start narrow (code_graph.rs only) and expand only if needed.
Sources: Scope Auditor (P1→P2)

**P2-3: record_timing parameter type** — Plan says f64; actual API may use u64.
Ship agent must verify actual signature before implementation.
Sources: Rust Reviewer

**P2-4: data_dir contract preservation** — Must preserve ENGRAM_DATA_DIR env
override. Added to Unit 1 acceptance criteria.
Sources: Rust Reviewer

**P2-5: Observability gaps** — Add tracing::debug!/warn! for flush path
resolution and retry attempts/exhaustion.
Sources: Constitution Reviewer, Architecture Strategist

**P2-6: Failed query timing** — Decide whether record_timing captures error
paths or only successful queries.
Sources: Rust Reviewer

**P2-7: Test-first enforcement** — Each unit should verify the failing test
exists before writing production code.
Sources: Constitution Reviewer

**P2-8: Retry granularity** — Retry at per-file batch boundary, not per-write.
Addressed in plan amendment; Ship agent should verify boundary choice.
Sources: Rust Reviewer, Architecture Strategist

### P3 Findings (Informational)

**P3-1: Execution order** — Consider 1 → 2 → 3 → 4 instead of 1 → 3 → 4 → 2
to validate the Unit 1 fix immediately via Unit 2.
Sources: Scope Auditor

**P3-2: Circuit breaker integration** — Unit 4 retry exhaustion should surface
warnings aligned with circuit-breaker protocol.
Sources: Architecture Strategist

**P3-3: Unit 1 diagnostic checkpoint** — Discriminate path-resolution vs
empty-DB-state early to avoid broad investigation.
Sources: Constitution Reviewer

### Hardening Assessment

Plan hardening signals: all absent. No hardening required. Confirmed.

### Runtime Verification and Closure Readiness

Present in plan. Covers flush_state output, health report timing, and
concurrent indexing. Adequate for the scope of changes.
