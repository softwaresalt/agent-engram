---
title: "Post-Merge Closure — 039-F Daemon Reliability Phase 3"
date: 2026-05-04
shipment: 022-S
feature: 039-F
pr: "76"
merge_commit: b1b9bb5
branch: feat/039-F-daemon-reliability-phase3
status: READY
---

## Summary

Daemon Reliability Phase 3 (shipment 022-S, feature 039-F) merged to `main` at
`b1b9bb5` via PR #76. Three mechanical reliability improvements were implemented,
plus three pre-existing bugs unmasked by removing `continue-on-error: true`.

### Shipped items

| ID | Title | Status |
|----|-------|--------|
| 039.001-T | Annotate daemon subprocess tests with `#[cfg_attr]` ignore | Archived |
| 039.002-T | Add `tracing::warn!` to SQLITE_BUSY retry helper | Archived |
| 039.003-T | Remove `continue-on-error` from CI test step | Archived |

### Prerequisite fixes (unmasked by removing `continue-on-error`)

| Fix | File | Test that was failing |
|-----|------|-----------------------|
| `find_symbols_by_name` timing instrumentation | `src/db/cozo_queries.rs` | `symbol_lookup_query_records_timing_stat` |
| `connect_db` fd-lock timeout 5 s → 30 s | `src/db/cozo_backend/mod.rs` | `concurrent_connect_db_*` |
| `record_query_metrics` WARN path for slow queries | `src/db/cozo_queries.rs` | `record_query_metrics_emits_warn_for_slow_query` |

## CI / Review Status

| Gate | Result |
|------|--------|
| CI run 5 | ✅ green (2m 29s) |
| Copilot review rounds | 4 rounds completed, all threads resolved |
| `cargo fmt` | ✅ |
| `cargo clippy -- -D warnings -D clippy::pedantic` | ✅ |

## Invariants to Preserve

1. `run_script_busy_retry_mutable` retries SQLITE_BUSY — the WARN emission
   must not affect retry logic or delay timing.
2. `record_query_metrics` must emit INFO for fast queries and WARN for slow ones
   (`> SLOW_QUERY_THRESHOLD_MS = 100 ms`) — both paths covered by contract tests.
3. `connect_db` fd-lock must serialize concurrent openers without deadlock —
   30 s deadline covers CI burst scenarios without indefinite blocking.
4. CI must fail on unexpected test failures — `continue-on-error: true` must
   not reappear on the test step.

## Pre-Deploy Checks

This is a `main`-branch merge-only release with no deployment gate. The following
confirmations are recorded:

- [x] No database schema changes — no migration needed
- [x] No IPC protocol changes — no daemon restart required
- [x] `#[cfg_attr(target_os = "windows", ignore)]` gates are additive — no Linux
  coverage removed
- [x] fd-lock timeout change is conservative — widens window, no happy-path impact
- [x] `tracing::warn!` paths are in exception/slow branches — no performance impact

## Deployment / Rollout Path

Merge-only. The daemon binary is distributed by users building from source.
No managed deployment surface — changes take effect at next user build.

## Post-Deploy Checks

1. Confirm CI stays green on subsequent PRs — absence of unexpected test failures
   indicates `continue-on-error` removal is stable.
2. Monitor `tracing::warn!` rate in developer logs for SQLITE_BUSY retries —
   unexpected surge indicates upstream CozoDB regression.
3. Monitor `record_query_metrics` WARN events — unexpected surge indicates slow
   query regression.

## Risky Action Record

| Action | Risk | Result |
|--------|------|--------|
| Remove `continue-on-error: true` | moderate — could fail CI on pre-existing flakiness | applied — unmasked 3 fixable bugs; CI now green |
| `#[cfg_attr(target_os = "windows", ignore)]` on rehydration test | low — Copilot initially disputed, corrected via review cycle | applied — daemons are sequential; Linux coverage retained |
| fd-lock timeout 5 s → 30 s | low — widens deadline only | applied — fixed `concurrent_connect_db_*` flakiness |

## Healthy Signals

- CI runs on subsequent PRs pass in ≤ 5 minutes
- No SQLITE_BUSY WARN events in normal single-daemon operation
- `slow query detected` WARN events absent for routine indexing queries

## Failure Signals

- CI test failures on unrelated PRs (would indicate flakiness regression)
- SQLITE_BUSY WARN rate > 0 in normal single-daemon operation (upstream CozoDB issue)
- `concurrent_connect_db_*` tests timing out (fd-lock regressed below 30 s)

## Monitoring Plan

No managed observability surface. Developer machines and CI logs are the
monitoring surface. The `tracing::warn!` events added in this feature provide
structured log entries at WARN level that any log sink can alert on.

## Rollback Trigger

Unexpected CI failures on PRs that do not touch test infrastructure.

## Rollback Procedure

1. `git revert b1b9bb5` to revert the merge commit on main.
2. Restore `continue-on-error: true` to `.github/workflows/ci.yml` test step.
3. Open a new PR with the revert.

## Validation Window

48 hours post-merge. Owner: softwaresalt.

## Source Artifact Cleanup

### Stash references (from 039-F `custom_fields.source_stash_ids`)

| Stash ID | Status |
|----------|--------|
| `100EACD8` | Shipped — annotate subprocess tests (039.001-T). Retire manually. |
| `1BA885AF` | Shipped — SQLITE_BUSY retry WARN (039.002-T). Retire manually. |

No automated stash-retirement operation available in the installed registry.
Both stash IDs recorded here for manual retirement via backlogit UI or CLI.

### Deliberation / plan references

| Artifact | Path |
|----------|------|
| Deliberation | `docs/decisions/2026-05-03-daemon-reliability-phase3-deliberation.md` |
| Plan | `docs/exec-plans/2026-05-03-daemon-reliability-phase3-plan.md` |

## Follow-Up Items for Stage

| # | Item | Priority | Notes |
|---|------|----------|-------|
| 1 | Upgrade CozoDB to >= 0.8 and remove `#[cfg_attr(windows, ignore)]` gates | medium | Tracked stash `100EACD8`; unblocks subprocess test coverage on Windows |
| 2 | Add structured alert on SQLITE_BUSY WARN rate for production daemon deployments | low | Currently relies on log scraping only |

## Readiness Status

**READY** — merge complete, CI green, all review comments resolved.
Monitoring is passive (log-based). No rollout or migration risk.
