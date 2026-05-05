---
title: "SQLITE_BUSY Retry Metrics — Implementation Plan"
description: "Add an in-process retry counter and MCP query tool for SQLITE_BUSY observability"
source_document: "docs/decisions/2026-05-04-sqlite-busy-retry-metrics-deliberation.md"
decision_status: "decided-plan"
requires_plan_hardening: false
tags:
  - "observability"
  - "sqlite-busy"
  - "metrics"
  - "daemon-reliability"
source_stash_ids:
  - "51B936CD"
---

## Objective

Add an `AtomicU64` retry counter to the CozoDB mutable-script retry helper that
increments on each SQLITE_BUSY retry in `run_script_busy_retry_mutable`. Expose
the counter via a new MCP tool (`get_mutable_script_retry_metrics`) so operators
can query mutable-script retry activity without log parsing.

**Scope boundary**: This feature instruments ONLY `run_script_busy_retry_mutable`
(src/db/cozo_queries.rs). Other retry sites (schema bootstrap in
src/db/cozo_backend/schema.rs, startup auto-sync in src/daemon/ipc_server.rs)
are explicitly out of scope — they can be instrumented in follow-up work.

## Source

Deliberation: docs/decisions/2026-05-04-sqlite-busy-retry-metrics-deliberation.md

## Constitution Check

| Principle | Compliance |
|-----------|-----------|
| I. Safety-First Rust | No unsafe code; AtomicU64 is safe |
| II. Test-First | Contract test for MCP tool, unit test for counter |
| III. Workspace Isolation | Read-only metric, no filesystem writes |
| VI. Single Responsibility | Minimal new code, no new dependencies |

## Implementation Units

### Unit 1: Add retry counter to CozoDB mutable-script retry helper

**Scope**: src/db/cozo_queries.rs

1. Add a module-level `static MUTABLE_RETRY_COUNT: AtomicU64` initialized to 0.
2. Add a `static MUTABLE_LAST_RETRY_EPOCH_MS: AtomicU64` initialized to 0.
   Sentinel: `0` means "no retry has occurred"; non-zero is epoch milliseconds.
3. In `run_script_busy_retry_mutable`, after the existing `tracing::warn!`,
   increment the counter (`Relaxed` ordering) and store current epoch-ms.
4. Add a `pub(crate) fn mutable_script_retry_metrics() -> RetryMetrics` function
   that reads both atomics and constructs the struct:
   - `retry_count: u64` — monotonic total
   - `last_retry_at: Option<DateTime<Utc>>` — `None` when sentinel is 0,
     `Some(...)` via checked conversion from epoch-ms (map overflow to `None`,
     never unwrap)
5. Add a `#[cfg(test)] pub(crate) fn reset_retry_metrics()` to zero both atomics
   for test isolation.

**Files**: src/db/cozo_queries.rs, src/db/mod.rs (re-export)

**Test strategy**: Unit test in tests/unit/ that:
- Captures baseline via `mutable_script_retry_metrics()` before test body
- Triggers the retry path (mock/simulate SQLITE_BUSY)
- Asserts counter delta ≥ 1 (not absolute value, for parallel safety)
- Uses `reset_retry_metrics()` in test setup when running serially

### Unit 2: Add `get_mutable_script_retry_metrics` MCP tool

**Scope**: src/tools/, src/shim/tools_catalog.rs

1. Add a new tool handler `get_mutable_script_retry_metrics` that calls
   `crate::db::mutable_script_retry_metrics()` and returns the counter value
   plus optional ISO-8601 timestamp.
2. Register the tool in the router dispatch (src/tools/mod.rs).
3. Register in tools catalog (src/shim/tools_catalog.rs): add schema entry,
   increment `TOOL_COUNT`.
4. Tool response schema: `{ retry_count: u64, last_retry_at: Option<String> }`
   where `last_retry_at` is RFC-3339 or null.

**Files**: src/tools/mod.rs (dispatch), src/shim/tools_catalog.rs (catalog +
TOOL_COUNT), new handler or inline in existing module

**Test strategy**:
- Contract test in tests/contract/ verifying tool responds with expected schema
- Update tools_catalog_test.rs assertions for new TOOL_COUNT

## Dependency Order

```text
Unit 1 (counter + fn)
  ↓
Unit 2 (MCP tool — depends on Unit 1's pub fn)
```

## Task Decomposition

| Task | Unit | Scope | Acceptance Criteria |
|------|------|-------|---------------------|
| T1: Add AtomicU64 mutable-script retry counter | 1 | src/db/cozo_queries.rs | Counter increments on retry; delta-based unit test passes; reset hook available for test isolation |
| T2: Add get_mutable_script_retry_metrics MCP tool | 2 | src/tools/, src/shim/tools_catalog.rs | Tool returns correct schema; contract test passes; catalog TOOL_COUNT updated; catalog test passes |

## Effort Estimate

2 tasks × ~2 hours = ~4 hours total.

## Requires plan hardening

No — low risk, additive observability, no behavioral changes to existing code paths.

## Plan Review

**Attempt 1 — Gate: FAIL (3 P1, 1 P2)**

Findings addressed in revision:

1. **P1 (Scope Boundary)**: Expanded Unit 2 to include tools_catalog.rs and TOOL_COUNT — RESOLVED
2. **P1 (Learnings Researcher)**: Renamed feature to `mutable_script_retry_metrics`, added explicit scope boundary noting other retry sites are out of scope — RESOLVED
3. **P1 (Constitution/Rust)**: Added test isolation strategy (delta assertions + `#[cfg(test)]` reset hook), specified red-phase test-first — RESOLVED
4. **P2 (Rust)**: Defined sentinel encoding (0 = no retry), specified checked conversion with no unwrap, `Option<DateTime<Utc>>` in struct — RESOLVED

**Attempt 2 — Gate: PASS**

All P1 findings addressed. Plan is ready for harvest.

<!-- plan-review-attempt: 2 -->
