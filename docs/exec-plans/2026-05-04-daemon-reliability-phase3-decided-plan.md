---
title: "Daemon Reliability Phase 3 — Decided Plan"
description: "Actionable decisions and rationale for 039-F; verbose deliberation archived"
feature: 039-F
shipment: 022-S
merge_commit: b1b9bb5
merged_at: 2026-05-04
pr: "76"
source_plan: "docs/archive/plans/2026-05-03-daemon-reliability-phase3-plan.md"
source_deliberation: "docs/decisions/2026-05-03-daemon-reliability-phase3-deliberation.md"
date: 2026-05-04
---

## Final Decisions

### 039.001-T: Subprocess test annotation

**Decision**: Use `#[cfg_attr(any(target_os = "windows", target_os = "linux"), ignore = "reason")]`
(not unconditional `#[ignore]`) for `daemon_rehydrates_graph_and_vector_state_after_db_directory_is_deleted`;
`smoke_full_tool_chain_over_ipc` retains its Windows-only gate.

**Rationale**: The CozoDB 0.7.6 `SQLITE_BUSY` panic manifests on both Windows (mandatory file-lock
persistence after `child.wait()`) and Linux (WAL-mode setup race when a new `connect_db` call races
with a prior daemon's teardown handles). The closure-phase CI runs confirmed the Linux failure is
real and not transient flakiness. The `reason` string is required by `clippy::pedantic`.

**Note**: The original 039-F design specified Windows-only. The Linux broadening was discovered during
the post-merge closure PR CI remediation (PR #77) and applied as a fix commit `2d2b500`.

**Tracking**: stash `100EACD8`. Unblock: cozo >= 0.8 for both Windows and Linux.

**Tracking**: stash `100EACD8`. Unblock: cozo >= 0.8.

### 039.002-T: SQLITE_BUSY retry observability

**Decision**: Add `tracing::warn!` inside the retry branch of `run_script_busy_retry_mutable`.
Fields: `attempt`, `max_attempts`, `delay_ms`, `error`.

**Rationale**: Silent retries are unobservable without DEBUG tracing. WARN level is appropriate
as SQLITE_BUSY is exceptional behavior.

**Bonus fixes** (pre-existing bugs unmasked by removing `continue-on-error`):
- `record_query_metrics`: added WARN emission for slow queries (>100ms)
- `find_symbols_by_name`: added `record_timing("symbol_lookup", ...)` timing instrumentation
- `connect_db`: widened fd-lock timeout 5 s → 30 s for CI contention scenarios

### 039.003-T: CI continue-on-error removal

**Decision**: Remove `continue-on-error: true` from the test step. Retain on audit step (advisory).

**Rationale**: The workaround was masking pre-existing failures. With subprocess tests
properly annotated, the gate should enforce correctness. Three pre-existing failures were
discovered and fixed when the flag was removed.

## Rejected Alternatives

| Alternative | Reason Rejected |
|-------------|-----------------|
| Unconditional `#[ignore]` on rehydration test | Removes Linux CI coverage; daemons are sequential not concurrent |
| `ignore` without `reason` string | Rejected by `clippy::pedantic` |
| Keep `continue-on-error: true` | Defeats purpose of a quality gate; masks bugs |
| fd-lock timeout < 30 s | Too tight for CI burst load; caused `concurrent_connect_db_*` flakiness |

## Constraints

- `#![forbid(unsafe_code)]` — no unsafe
- `clippy::pedantic` — all `#[ignore]` must have `reason` strings
- CI must remain green without `continue-on-error` fallback
