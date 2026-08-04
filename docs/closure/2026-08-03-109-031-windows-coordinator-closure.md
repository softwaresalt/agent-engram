---
title: "109.031-T Windows coordinator operational closure"
date: 2026-08-03
mode: pre-merge
shipment: 104-S
feature: 109-F
task: 109.031-T
verification_report: "docs/closure/2026-08-03-109-031-windows-coordinator-runtime-verification.md"
status: ready
---

# 109.031-T Windows Coordinator Operational Closure

## Readiness

**READY.** The operator authorized the production lint repairs and required
review remediation. All local quality gates now pass except `cargo audit`,
whose non-zero result is the documented `RUSTSEC-2026-0041` baseline. The PR
may be opened and reviewed; merge still requires explicit operator approval.

## Invariants to Preserve

- One continuous `AdmissionGuard -> OwnerPermit -> transferred OwnerPermit`
  authority chain; no unarmed owner or detached receiver.
- Full masks move exactly once; same-binding retirement owns `0b111` and
  distinct binding carries no old work.
- Mutation-capable children end before acknowledgment; no successor starts
  before acknowledgment and active DB drivers never exceed one.
- Released waiters progress by one post-unlock notification without spin.
- Stale terminals mutate nothing; no pre-permit I/O, optional cleanup,
  process-abort RAII claim, schema action, or partial rollback.

## Pre-deploy Audit

| Check | Result |
|---|---|
| Production/remediation files changed | PASS — `ipc_server.rs`, `write.rs`, `state.rs` |
| Review-blocking correctness remediation | PASS — minimal `lifecycle.rs` partial-error fix |
| TEMP and Git discovery confined to repository | PASS |
| Retrieval-eval RED then test-only GREEN | PASS |
| Exact Windows all-target run | PASS — one post-fix run, exit 0 |
| Current-HEAD 15-minute named-pipe observation | PASS — 16/16 probes |
| Restart/reconciliation evidence | PASS — PID 26388 → PID 41812 |
| Full-release-unit revert/restart | PASS — baseline `df2803e1`, PID 35352 |
| `cargo fmt --all -- --check` | PASS |
| Exact CI clippy | PASS |
| Repository all-target clippy | PASS |
| `cargo dev-test` | PASS — 533 tests after PR remediation |
| Exact CI all-target suite | PASS — exit 0 after PR remediation |
| Standard review | PASS — zero applicable P0/P1 after two cycles |
| Dependency audit | PASS WITH KNOWN ADVISORY — `RUSTSEC-2026-0041` |
| Schema or data rollback | Not applicable |

The lint repairs were behavior-neutral. Review found one real correctness gap:
transferred lifecycle and daemon syncs completed after non-fatal file errors
even when heavy work could not be certified. Two Windows real-database tests
failed RED (`pending=0`, expected `0b111`) and passed GREEN after the shared
fail-closed predicate was applied.

PR review found two additional correctness gaps. Plain full indexes had
incorrectly claimed heavy migration work and therefore forced unchanged-file
re-extraction. Scan-progress child cancellation also aborted without joining,
while parent progress writes were not owner-fenced. The final implementation
uses routine-only ownership for plain indexes, complete ownership for forced
indexes, validates parameters before admission, joins children on structured
exit, and rejects every stale parent or child progress publication.

## Deployment or Rollout Path

This is a merge-only handoff. Keep the PR open on
`feat/109-single-authority-coordinator`; do not merge until the operator gives
explicit approval. No migration, feature flag, schema action, or staged data
rollout is required.

## Post-deploy Checks

1. Confirm named-pipe health and workspace identity after the release daemon
   starts.
2. Run one no-op sync and confirm the coordinator returns to idle.
3. Confirm no duplicate-daemon event and no active DB-driver overlap.
4. If a heavy sync reports per-file errors, confirm the heavy mask remains
   pending for a later retry.

## Monitoring Plan

No external metric sink exists for this pre-merge disposable run. The required
manual checks remain:

| Signal | Healthy baseline | Intervention threshold | Owner |
|---|---|---|---|
| Named-pipe reachability | Every scheduled probe succeeds | Any failed probe | Ship/operator |
| Workspace identity | Constant disposable workspace UUID | Missing or changed identity | Ship/operator |
| Driver overlap | `max_active_db_drivers == 1` | Any value above one | Ship/operator |
| Retirement barrier | Clears only after exact ack | Stuck or early clear | Ship/operator |
| Old work after ack | Zero | Any old work/progress | Ship/operator |
| Release baton | Finite one-owner progress | Stranded waiter or spin | Ship/operator |

Validation window: completed for 15 minutes on Windows, owned by Ship,
`2026-08-04T02:01:45.8318676Z` through
`2026-08-04T02:16:59.8654522Z`.

Post-merge owner: operator/Ship. Observe the first released daemon session and
one explicit sync; retain the existing 15-minute window if any lifecycle signal
deviates from baseline.

## Healthy and Failure Signals

Observed healthy signals were stable named-pipe health, constant disposable
workspace identity, contained branch DB, completed watcher/sync work, no
duplicate daemon, successful current-version reconciliation, and a healthy
complete-unit baseline restart.

Rollback triggers are missing/duplicate terminal behavior,
successor-before-ack, active drivers above one, work after ack, a stuck
barrier, mask loss or cross-binding carry, pre-permit I/O, response drift,
stranded waiters, or any IPC regression.

## Rollback Procedure

The procedure was exercised successfully: build the complete coordinator
baseline `df2803e1834728681288a2669c314dffea004307` in a detached clean
worktree, stop only the tracked current PID, start the baseline binary against
disposable state, verify bind/status/sync, and stop only the tracked baseline
PID. Partial rollback remains forbidden. No schema or data action is required.

## Risky Action Record

Repository-contained test isolation was applied successfully without process
environment mutation in the fixture. Timeout/assertion bypass and suite-wide
serialization were explicitly abandoned. Operator-authorized runtime and
rollback actions were applied only to PIDs `26388`, `41812`, and `35352`; the
clean disposable rollback worktree was removed afterward.

The final source remediation was executed in investigate-first and
freeze-scope modes. **ProposedAction:** remove strict clippy blockers and fix
the review-confirmed transferred partial-error loss. **ActionRisk:** moderate.
**Approval required:** yes, supplied by the operator. **Rollback:** revert
`be805eec36c4da8aa272e3638f1b059ead633adc` as one unit.
**ActionResult:** applied and fully validated.

## Residual Advisory Risk

`cargo audit` still reports `RUSTSEC-2026-0041` for `lz4_flex 0.10.0` through
`cozo 0.7.6 -> swapvec 0.3.0`. `Cargo.toml` and `Cargo.lock` have no branch
diff, CI treats audit as continue-on-error, and queued low-priority `017-D`
owns the dependency upgrade. Thirteen additional audit entries are allowed
maintenance/unsoundness warnings, not new vulnerability failures.

## Final Gate

Open the PR, require green GitHub checks, resolve all bot threads, and require
a Copilot review whose `commit_id` equals the final PR HEAD. Stop before merge
and wait for explicit approval. Shipment `104-S` remains active until merge.
