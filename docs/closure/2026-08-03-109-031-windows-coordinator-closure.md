---
title: "109.031-T Windows coordinator operational closure"
date: 2026-08-03
mode: pre-merge
shipment: 104-S
feature: 109-F
task: 109.031-T
verification_report: "docs/closure/2026-08-03-109-031-windows-coordinator-runtime-verification.md"
status: blocked
---

# 109.031-T Windows Coordinator Operational Closure

## Readiness

**BLOCKED.** Do not create a PR from this gate evidence. The exact Windows
all-target prerequisite is not clean, and current-HEAD named-pipe observation
and full-unit rollback evidence are consequently absent.

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
| Production files changed by `109.031-T` | PASS — zero |
| TEMP and Git discovery confined to repository | PASS |
| Deterministic/contract fixtures reached by aggregate run | PASS before unrelated blocker |
| Exact Windows all-target run | BLOCKED |
| Current-HEAD 15-minute named-pipe observation | NOT STARTED |
| Restart/reconciliation evidence | NOT STARTED |
| Full-release-unit revert/restart | NOT STARTED |
| Schema or data rollback | Not applicable |

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

Validation window: 15 minutes after current-HEAD startup, owned by Ship, after
the all-target prerequisite is green.

## Healthy and Failure Signals

Healthy means stable named-pipe health, correct workspace identity, completed
hydration/sync, no duplicate daemon, no driver overlap, and successful
reconciliation after restart.

Rollback triggers are missing/duplicate terminal behavior,
successor-before-ack, active drivers above one, work after ack, a stuck
barrier, mask loss or cross-binding carry, pre-permit I/O, response drift,
stranded waiters, or any IPC regression.

## Rollback Procedure

Revert the complete coordinator release unit to baseline
`df2803e1834728681288a2669c314dffea004307`, then restart the daemon against
disposable state and verify hydration, reconciliation, workspace identity, and
named-pipe health. Partial rollback is forbidden. No schema or data action is
required.

## Risky Action Record

Repository-contained test isolation was applied successfully. Timeout bypass
was explicitly abandoned. Disposable daemon termination/restart was authorized
by the operator but not executed because the aggregate prerequisite failed.

## Next Gate

Resolve the reproducible retrieval-evaluation test-state contamination and
Windows Cozo lock artifact without product behavior changes, then repeat the
all-target, 15-minute runtime, restart, monitoring, and full-unit rollback
evidence. Keep `109.031-T` blocked and keep `104-S` active.
