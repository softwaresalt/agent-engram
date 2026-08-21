---
title: "Completed shipment memory compaction: 102-S, 103-S, 105-S, 106-S + sync-coordinator planning phases 5A/5C/5E"
type: compacted-memory
date: 2026-08-21
status: complete
sources:
  - docs/archive/memory/2026-08-02/circuit-break-all-target-isolated-data.md
  - docs/archive/memory/2026-08-02/circuit-break-backlogit-doctor-target-validation.md
  - docs/archive/memory/2026-08-02/circuit-break-hermetic-stale-pid-recovery.md
  - docs/archive/memory/2026-08-02/circuit-break-stale-pid-verify-alive.md
  - docs/archive/memory/2026-08-02/phase-5a-sync-coordinator-recovery-memory.md
  - docs/archive/memory/2026-08-02/phase-5c-sync-coordinator-stage-memory.md
  - docs/archive/memory/2026-08-02/phase-5e-cancellation-ownership-restart-memory.md
  - docs/archive/memory/2026-08-02/pr-316-p1-plan-remediation-memory.md
  - docs/archive/memory/2026-08-02/ship-103-S-ordinary-index-fail-closed.md
  - docs/archive/memory/2026-08-02/ship-105-S-windows-pid-identity-stale-recovery.md
  - docs/archive/memory/2026-08-02/stage-windows-pid-identity-prerequisite-memory.md
  - docs/archive/memory/2026-08-02/102-S-qualified-caller-attribution-ship-memory.md
  - docs/archive/memory/2026-08-02/106-S-pr316-ship-closure.md
---

# Completed shipment memory compaction

This cluster documents a single 2026-08-02 session that shipped three
independent release units (102-S, 103-S, 105-S) and drove the planning
groundwork for a fourth (106-S / feature 109-F), while circuit-breaking
twice on Windows-specific test-infrastructure defects. The final
implementation of 109-F (Phase 6, PR #319) is covered separately in
`docs/memory/compacted/2026-08-04-104-s-109-f-compacted.md`; this entry
covers only the planning phases (5A/5C/5E) and the concurrent shipments
that unblocked it.

## 102-S — Qualified Python caller attribution (feature 107-F)

PR #307 (merge `89ce5419`) replaced first-match caller attribution at both
qualified Python provenance producers with the existing typed unique-only
lookup. `backlogit shipment ship` initially refused closure because the
post-merge worktree lacked gate evidence (`.backlogit/logs/` is gitignored)
— resolved by reopening and re-completing both archived tasks on the
merged tree without a force override.

## 103-S — Ordinary-index fail-closed follow-ups (feature 108-F)

PR #312 (merge `5c9d466e`) hardened Python/Rust staged-source post-passes
so fallible context loads happen before graph mutation, with prior
snapshots restored on error. Build was blocked twice by circuit breakers
before this shipped:

- **All-target test isolation**: `cargo test --all-targets` failed
  non-deterministically because unrelated test binaries shared one branch
  database (`contract_evaluation`, `integration_retrieval_eval_thresholds`).
  Resolved by an operator-approved untracked hermetic runner assigning a
  unique `ENGRAM_DATA_DIR` per test process (not per binary) — required
  because tests *within* one binary could still collide.
- **Windows stale-PID recovery**: after hermetic isolation, a real defect
  surfaced — `PidFile::verify_alive` (via `sysinfo::System::refresh_process`)
  reported an already-reaped daemon PID as live for 5+ seconds past its
  observed exit, hitting `ShutdownTimeout { timeout_ms: 2000 }`. This was
  investigated in an authorized `investigate-first`/`freeze-scope` probe
  (two test-owned files only), root-caused as a stale-recovery routing
  defect (not a timing/fixture issue), and spun out as its own prerequisite
  shipment (105-S) rather than fixed inside 103-S's scope.

## 105-S — Windows PID identity stale recovery (feature 110-F, prerequisite for 103-S)

PR #310 (merge `846f3b74`) fixed the root cause found above: `PidFile`
preserves `{pid, start_time_unix}` identity and now performs a second
refresh on the same `sysinfo::System`, checking full identity before and
after, rather than trusting a single potentially-false-positive probe.
Three strictly-ordered TDD units (RED identity contracts → RED exact-child
stale-recovery → GREEN same-System refresh) shipped test-first. A Rust
specialist review flagged a P2 (possible Windows PID reuse); the test was
strengthened to compare full identity, not just numeric PID. Runtime
evidence covered structured PID, exact kill/wait recovery, legacy numeric
upgrade, and malformed-metadata rejection.

## 106-S / 109-F planning phases 5A, 5C, 5E — single-authority sync coordinator

Stage recovered a prior planning state, then ran a spike to resolve an
internal-vs-public API compatibility question for a new sync-coordinator
design (chosen: **Option A**, opaque crate-private permit API with full
internal caller migration — justified because `publish = false` and all
critical callers are internal). The design converged on one
mutex-protected coordinator owning generation floor, sequenced owner
identity, complete pending mask, and hydration/drain handoff, with
non-cloneable permits and no detached receivers/masks/task handles. A PR
#316 Copilot P1 (empty pre-acquisition waiters could block forever on a
rebind with no coordinator wake) was remediated with continuous
cancellation and an `AdmissionGuard`/`DriverTaskGuard` ownership model.
Backlog bookkeeping required care throughout: superseded tasks
(109.001–109.012-T) were retired and replaced (109.014–109.031-T) while
`104-S`/`109-F` stayed deliberately blocked until spike findings landed;
`backlogit shipment ship` on 106-S once incorrectly reopened 30 blocked
dependents to queued, which Ship restored byte-for-byte. Sessions in this
phase explicitly ran the compact-context assessment and correctly declined
to compact anything, since 109-F was still active/blocked at the time.

## Circuit breakers (recorded, not re-triggered)

A `backlogit doctor` target-validation call failed 5x with scope errors
(unsupported target-path convention) and was abandoned without retry —
optional operation, did not block Phase 5A recovery.

## Preserved, not compacted

017-D (cozo 0.8+ / RUSTSEC-2026-0041 dependency decision) remained an open
backlog item throughout this cluster and is tracked there, not in memory.
