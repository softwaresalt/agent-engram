---
title: "Daemon Reliability Phase 3 — Subprocess Stability & Retry Observability"
description: "Implementation plan for annotating daemon subprocess test failures and adding retry tracing"
source_document: "docs/decisions/2026-05-03-daemon-reliability-phase3-deliberation.md"
feature_scope: "Daemon subprocess test annotation, retry observability, CI hardening"
requires_plan_hardening: no
tags:
  - "cozo"
  - "daemon"
  - "subprocess"
  - "observability"
  - "CI"
---

## Objective

Ship three bounded tasks that close the remaining daemon reliability gaps from
038-F follow-ups: annotate failing subprocess tests with root-cause documentation,
add observability to the SQLITE_BUSY retry helper, and tighten CI by removing
`continue-on-error` for tests that are now passing.

## Source

Deliberation: `docs/decisions/2026-05-03-daemon-reliability-phase3-deliberation.md`

Stash entries consumed:

- `100EACD8` — Daemon subprocess spawn timeout (medium)
- `1BA885AF` — SQLITE_BUSY retry tracing::warn (low)

Queue item to archive:

- `002-D` — fd-lock scope extension deliberation (already resolved by 037-F)

## Constitution Check

| Principle | Status |
|---|---|
| I. Safety-First Rust | ✅ No unsafe code; no unwrap/expect |
| II. Test-First Development | ✅ Task 1 writes test annotations (characterization); Task 2 adds observable behavior testable via tracing subscriber |
| III. Workspace Isolation | ✅ All changes within repo |
| IV. CLI Containment | ✅ |
| V. Structured Observability | ✅ Task 2 directly improves observability |
| VI. Single Responsibility | ✅ No new dependencies |

## Implementation Units

### Unit 1: Annotate Daemon Subprocess Tests (100EACD8)

**Files:**

- `tests/integration/smoke_test.rs`
- `tests/integration/graph_vector_rehydration_test.rs`

**Approach:**

Add `#[ignore]` annotations to the two affected tests with structured comments
documenting:

- Root cause: CozoDB 0.7.6 `sqlite.rs:49` `conn.prepare().unwrap()` panics in
  subprocess context on Windows
- Tracking reference: stash `100EACD8`, upstream cozo issue
- Unblock condition: cozo >= 0.8 with graceful SQLITE error handling

The `#[ignore]` annotation ensures these tests are excluded from `cargo test`
by default but remain runnable via `cargo test -- --ignored` for manual
verification when upstream conditions change.

**Acceptance criteria:**

- Both tests carry `#[ignore]` with explanatory doc comment
- `cargo test` no longer fails on these tests
- Comment references upstream root cause and unblock condition

### Unit 2: Add tracing::warn to SQLITE_BUSY Retry (1BA885AF)

**Files:**

- `src/db/cozo_queries.rs` (lines 300-327)

**Approach:**

Inside the retry branch of `run_script_busy_retry_mutable`, add:

```rust
tracing::warn!(
    attempt = attempt + 1,
    max_attempts = MAX_ATTEMPTS,
    delay_ms = delay.as_millis(),
    error = %msg,
    "SQLITE_BUSY retry: retrying mutable run_script"
);
```

This makes each retry visible in structured logs without changing behavior.

**Acceptance criteria:**

- Each retry attempt emits a `tracing::warn` span with attempt count, delay,
  and error context
- `cargo clippy -- -D warnings -D clippy::pedantic` passes
- Existing tests continue to pass (retry behavior unchanged)

### Unit 3: CI continue-on-error Tightening

**Files:**

- `.github/workflows/ci.yml`

**Approach:**

With the daemon subprocess tests annotated `#[ignore]`, and the
`integration_graph_vector_rehydration` and `integration_query_perf_observability`
failures resolved by 038-F (tasks 038.002-T and 038.003-T), evaluate whether
`continue-on-error: true` can be removed from the test step.

If removal causes CI to fail on other tests, add those test names to the
ignore list with documentation. The goal is to make CI a proper gate.

**Acceptance criteria:**

- `continue-on-error: true` removed from the test step (or reduced to audit only)
- CI passes with the `#[ignore]` annotations in place
- Comment updated to explain why continue-on-error was removed
- If other failures surface, they are documented and tracked

## Dependency Order

```text
Unit 1 (annotate subprocess tests)
  ↓
Unit 3 (CI tightening — depends on Unit 1 annotations being in place)

Unit 2 (retry observability — independent, can run in parallel with Unit 1)
```

## Estimated Effort

- Unit 1: ~1 hour (test annotation + documentation)
- Unit 2: ~30 minutes (add tracing::warn call)
- Unit 3: ~1 hour (CI change + verification)
- Total: ~2.5 hours (3 tasks × 2-hour budget = 6 hours available)

## Risks

| Risk | Likelihood | Mitigation |
|---|---|---|
| Other test failures surface when continue-on-error removed | Medium | Track as follow-up stash entries; restore continue-on-error if needed |
| tracing::warn in hot path affects perf | Very Low | Retry path is exceptional; warn level is cheap |
| Ignored tests forgotten indefinitely | Low | Stash `1092D3D6` tracks upstream cozo upgrade; compound learning references the ignore |
