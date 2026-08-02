---
title: "Operational closure — 105-S Windows PID identity stale recovery"
doc_type: closure
source: "105-S / 110-F / PR #310"
description: >-
  Post-merge closure record for deterministic Windows process-identity
  verification and stale-daemon recovery.
topic: "Windows PID identity and stale-daemon recovery"
depth: closure
decision_status: "SHIPPED — post-release observation pending"
author: ship
date: 2026-08-02
verdict: SHIPPED
pr: 310
merge_commit: "846f3b74bf7292a07634e4fd6e44a388be411666"
target_commit: "846f3b74bf7292a07634e4fd6e44a388be411666"
branch: "feat/110-windows-pid-identity-stale-recovery"
linked_artifacts:
  - "105-S"
  - "110-F"
  - "110.001-T"
  - "110.002-T"
  - "110.003-T"
---

## Summary

PR #310 changes `PidFile::verify_alive` to refresh one sysinfo `System`
twice and require the recorded non-sentinel start fingerprint before and after
the active-state refresh. Reaped and start-mismatched Windows identities are
stale. Lifecycle respawn and shutdown waiting now retain the complete
`PidFile` identity instead of degrading to a numeric PID.

Endpoint shutdown and `engram.lock` remain authoritative. Production code
never terminates a process from PID metadata. Structured JSON and legacy
numeric PID files remain readable, and no schema, protocol, dependency,
public API, or timeout changed.

## TDD Evidence

- **110.001-T RED:** all four identity scenarios compiled; matching,
  mismatch, and legacy controls passed while the retained reaped-child
  assertion failed against the old verifier.
- **110.002-T RED:** the exact daemon child was killed and waited, its handle
  remained owned, and recovery failed against the old verifier with
  `ShutdownTimeout { timeout_ms: 2000 }`.
- **110.003-T GREEN:** the four identity scenarios and exact-child recovery
  passed after the same-System double refresh and lifecycle identity
  propagation.

No timing sleep, public test seam, unsafe code, direct production PID kill, or
timeout increase was introduced.

## Repository Validation

- Targeted stale-PID recovery: 4/4.
- Adjacent lock, shim lifecycle, daemon lifecycle, workspace lifecycle, and
  lifecycle contract suites: PASS.
- `cargo fmt --all -- --check`: PASS.
- Serialized strict clippy with `--all-targets -j 1`: PASS.
- Serialized `cargo dev-test`: 504/504.
- Approved hermetic `cargo test --locked --all-targets -j 1`: PASS with a
  unique `ENGRAM_DATA_DIR` per test process under
  `logs/test-data-hermetic/105-S`.
- `cargo audit --no-fetch`: unchanged accepted baseline
  `RUSTSEC-2026-0041` through `cozo 0.7.6 -> swapvec 0.3.0 ->
  lz4_flex 0.10.0`, plus 13 allowed warnings. `Cargo.toml` and `Cargo.lock`
  are unchanged; the repository records this advisory as accepted pending the
  separately scoped Cozo major-version evaluation.
- Standard review: PASS.
- Rust specialist review: PASS after the recovery assertion was hardened to
  compare the complete PID/start identity rather than assuming the numeric PID
  cannot be reused.
- GitHub CI `build`: PASS on exact head
  `900a815dc40effd75cb04a95c9db5d533a7e4ec1`.
- Copilot reviewed that exact head, produced zero unresolved threads, and was
  removed from requested reviewers before merge.

The optional `--all-features` clippy probe exposed pre-existing OpenTelemetry
API drift in `src/server/observability.rs`. That optional surface is outside
105-S and was not changed; the repository's required non-optional strict
clippy gate is green.

## Windows Runtime Verification

Runtime verification used only the disposable workspace
`logs/test-data-hermetic/105-S/runtime-workspace`.

1. Normal startup wrote structured identity
   `{pid: 7528, start_time_unix: 1785677991}`.
2. `daemon-status` reported green PID liveness, workspace identity, and pipe
   reachability with `duplicate_daemon_detected = 0`.
3. Ship retained the exact process handle, terminated PID 7528, waited for
   exit, and invoked normal recovery without deleting runtime state.
4. Recovery succeeded without `ShutdownTimeout` and wrote new structured
   identity `{pid: 39488, start_time_unix: 1785678019}`.
5. Replacing the disposable PID file with legacy numeric `39488` kept the
   reachable daemon manageable and all health checks green.
6. After exact-child kill/wait, normal recovery upgraded the legacy fixture to
   structured identity `{pid: 3472, start_time_unix: 1785678065}`.
7. Malformed PID metadata did not replace the live daemon: health stayed green,
   PID 3472 remained live, and `duplicate_daemon_detected` stayed zero.
8. The final disposable daemon was explicitly stopped and waited.

No operator workspace, deployed cache, or repository-root `.engram` directory
was read, repaired, reindexed, modified, or deleted.

## Invariants to Preserve

- Structured liveness requires PID, active process state, and matching
  non-sentinel start identity.
- Reaped and mismatched identities are stale.
- Legacy numeric identity remains readable and manageable but never
  authorizes direct termination.
- Malformed metadata cannot authorize PID-directed action.
- Endpoint shutdown and the OS lock remain the final safety authorities.
- The two-second shutdown timeout remains unchanged.

## Monitoring Plan

The owner is Ship for merge verification and the repository operator after the
first released binary containing PR #310 is installed. The observation window
starts at that installation, covers at least 24 hours and the first three
Windows daemon restart/recovery events, and has a hard stop at seven days. If
fewer than three events occur by day seven, close the window as
`healthy-no-event`, `degraded`, or `rolled-back` rather than leaving it open
indefinitely. Record the outcome and observed event count in this closure
artifact through a follow-up documentation PR.

| Signal | Healthy | Rollback trigger |
|---|---|---|
| Stale recovery | No timeout or manual cleanup | Any new `ShutdownTimeout` |
| Replacement safety | One live daemon and lock holder | Any duplicate live daemon or lock bypass |
| Identity health | PID/workspace/pipe checks green | Matching live structured identity classified dead |
| Legacy upgrade | Reachable numeric daemon remains manageable | Any legitimate legacy daemon becomes unreachable or unmanageable, is killed, or is silently replaced |

Existing reliability counters are not emitted on every shim path, so the
observation window uses `daemon-status`, process identity, lock behavior, and
structured logs rather than claiming an unavailable central dashboard.

## Rollback

Rollback on any trigger above by reverting the PR #310 merge commit with
`git revert -m 1 846f3b74bf7292a07634e4fd6e44a388be411666`,
rebuilding, and performing a normal endpoint-driven daemon restart. Do not
delete `.engram`, rewrite operator PID files, raise timeouts, or terminate a
process based only on PID metadata. Keep 103-S and 108-F blocked if rollback is
required.

## Risky Action Record

- **ProposedAction:** change Windows liveness semantics and retain full process
  identity through shutdown waiting.
- **ActionRisk:** high runtime impact without public shape change.
- **Approval:** reviewed Stage plan, shipment claim, validation, exact-head
  review, CI, and merge were explicitly authorized.
- **ActionResult:** applied and merged as
  `846f3b74bf7292a07634e4fd6e44a388be411666`.
- **Containment:** all runtime state was disposable and isolated from operator
  workspaces.

## Readiness

**SHIPPED.** PR #310 merged with a merge commit, backlogit archived 105-S,
110-F, and all three serial tasks with positive merge evidence. The prerequisite
is ready to unblock 103-S and 108-F after this closure PR merges. Post-release
observation remains pending and must not be inferred complete.
