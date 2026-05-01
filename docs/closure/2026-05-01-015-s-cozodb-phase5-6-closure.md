---
title: "015-S CozoDB Phase 5-6 Operational Closure"
shipment: 015-S
branch: chore/015-s-cozodb-phase5-6
phases: "Phase 5 (verification tests), Phase 6 (default backend flip + documentation)"
status: ready-for-review
date: 2026-05-01
---

## Summary

Shipment 015-S delivers Phases 5 and 6 of the CozoDB migration. Phase 5 adds
verification test coverage for cold-restart round-trips, dual-backend parity
contracts, and the vector-search feature gate. Phase 6 flips the default Cargo
feature from `surreal-backend` to `cozo-backend` and updates documentation to
reflect CozoDB as the production default. Phase 7 (SurrealDB removal) is deferred
to a separate PR after a 7-day observation window.

---

## Monitoring Plan

### SLIs and Key Metrics

| SLI | Signal | Alert Threshold |
| --- | --- | --- |
| Daemon startup time | Time from `engram daemon` invocation to first successful tool response | > 5 s |
| First-query latency | Round-trip time for `list_symbols` on a fresh (non-cached) workspace | > 2 s |
| Rehydration success rate | Fraction of `set_workspace` calls that complete without error | < 100 % |
| Cold-restart integrity | Nodes + edges after restart == nodes + edges before dehydration | Any mismatch |

### Dashboard / Query

These are manual observation signals (no external monitoring system configured):

- `RUST_LOG=info engram daemon` — startup log emits `hydration complete` with elapsed time
- `RUST_LOG=debug engram daemon` — cold-restart logs emit node/edge counts before and after
- CI test output — `integration_cozo_cold_restart` and `integration_cozo_dual_backend_sweep` capture parity signals automatically

### Baseline

- Startup time: < 1 s for empty workspace (measured in CI on Linux)
- First-query latency: < 200 ms for empty workspace
- Rehydration success rate: 100 % in all CI runs

### Owner

Ship agent / @softwaresalt — 7-day observation window from merge date.

---

## Pre-Deploy Audit Checklist

- [x] Feature flag rollout gate: `cozo-backend` flip is a compile-time change; no runtime flag needed
- [x] Rollback procedure documented (see below)
- [x] Data migration compatibility: CozoDB SQLite files are not migrated from SurrealDB; existing `.engram/` JSONL dehydration files are reused by both backends — no migration risk
- [x] No cross-service boundaries affected (daemon is a local IPC process)
- [x] Monitoring plan (above) is complete
- [x] `surreal-backend` still compiles: verified with `cargo check --no-default-features --features surreal-backend`
- [x] All quality gates pass: `cargo fmt`, `cargo clippy`, `cargo test`

---

## Post-Deploy Observation Window

| Item | Value |
| --- | --- |
| Window duration | 7 days from PR merge |
| Owner | @softwaresalt |
| Start condition | PR #60 merged to main |
| End condition | 7 days elapsed with no regression reports |
| Active signals | CI green on `main`; no issue reports on daemon startup or query failures |

During the window, watch for:

- Unexpected daemon startup failures in CI
- Any report of `list_symbols`, `map_code`, or `unified_search` returning empty results on non-empty workspaces
- Any SQLite file-lock errors on Windows (known risk mitigated by `CodeGraphQueries` drop-before-delete pattern)

---

## Rollback Trigger

| Trigger | Condition |
| --- | --- |
| Startup failure rate | Any daemon startup failure traceable to CozoDB initialization |
| Query error rate | Any `EngramError::CodeGraph` error on a workspace that indexed successfully |
| Cold-restart mismatch | Node or edge count changes unexpectedly after a dehydrate → hydrate cycle |

---

## Rollback Procedure

**ProposedAction**: Revert Cargo.toml default feature<br>
**ActionRisk**: moderate (reverts production default; surreal-backend is still present and tested)<br>
**ActionResult**: planned

Steps:

1. Revert `Cargo.toml` line 69: `default = ["embeddings", "cozo-backend"]` → `default = ["embeddings", "surreal-backend"]`
2. Revert the feature comment on the `surreal-backend` line to remove "non-default" description
3. Run `cargo check` to confirm surreal-backend builds as default
4. Commit as `fix(build): revert default backend to surreal-backend` and open a PR
5. After merge, update this closure artifact with `ActionResult: rolled-back`

The CozoDB implementation files and tests are **not removed** on rollback — they remain available for investigation and re-enablement.

---

## Phase 7 Deferral

Phase 7 (U7.1: drop surrealdb dependency, U7.2: delete SurrealDB implementation files) is
intentionally deferred. The 7-day observation window must complete without regression before
Phase 7 begins.

**Phase 7 entry criteria**:
- 7-day observation window elapsed with no rollback triggers fired
- All CI checks green on `main` throughout the window
- Operator approves Phase 7 (`ActionRisk: destructive` — irreversible deletion of SurrealDB backend files)

Phase 7 work items are in the backlog under shipment 015-S (U7.1, U7.2).

---

## Healthy Signals

After merge, these signals indicate Phase 5-6 is stable:

- `cargo test` passes on `main` with default features (cozo-backend)
- `cargo test --no-default-features --features surreal-backend` passes (surreal still works)
- Daemon starts cleanly on Linux, macOS, and Windows in CI
- Cold-restart round-trips preserve exact node/edge counts
- `unified_search` returns results for queries against indexed workspaces

## Failure Signals

These signals indicate a problem requiring investigation or rollback:

- Daemon fails to start: `CozoDb` initialization error (SQLite file creation failure)
- `list_symbols` returns empty for a workspace that was indexed without error
- Embedding dimension mismatch errors from `validate_cozo_embedding`
- SQLite file-lock errors on Windows during cold-restart or `flush_state`
