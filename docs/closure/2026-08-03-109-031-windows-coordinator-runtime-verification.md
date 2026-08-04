---
title: "109.031-T Windows coordinator runtime verification"
date: 2026-08-03
shipment: 104-S
feature: 109-F
task: 109.031-T
branch: feat/109-single-authority-coordinator
commit: 903247b5cd2a3a8c0bb0b1e37b7702fb0767236a
surface: background-job
mode: manual
verdict: blocked
---

# 109.031-T Windows Coordinator Runtime Verification

## Verdict

**BLOCKED.** The required Windows CI-equivalent all-target gate did not produce
a clean run. The 15-minute named-pipe observation and full-release-unit
revert/restart were therefore not started; they remain mandatory before this
task can pass.

## Environment Prechecks

- Windows branch and HEAD:
  `feat/109-single-authority-coordinator` at
  `903247b5cd2a3a8c0bb0b1e37b7702fb0767236a`.
- Worktree was clean before validation.
- `104-S` was active, `109.031-T` was blocked, and completed dependency
  `109.030-T` was confirmed.
- Backlogit MCP was unavailable in the session tool surface; the
  registry-declared CLI fallback was used and the index synchronized.
- Engram daemon status was reachable and bound to
  `C:\Source\GitHub\engram`.
- Test state remained under the repository. The successful isolation root was
  `C:\Source\GitHub\engram\tmp\109031-tests`.
- `TEMP`, `TMP`, and `GIT_CEILING_DIRECTORIES` all pointed to that root.
  A nested `git rev-parse --show-toplevel` exited 128, proving temporary
  workspaces did not inherit this repository.

## Scenarios and Evidence

### Scenario 1 — Test-containment isolation

The original failing assertion passed without changing or weakening the test:

```powershell
$root = (Resolve-Path -LiteralPath 'tmp\109031-tests').Path
$env:TEMP = $root
$env:TMP = $root
$env:GIT_CEILING_DIRECTORIES = $root
cargo test --no-default-features --features cozo-backend,embeddings --test integration_backlog no_git_produces_null_url -- --exact
```

Result: `1 passed; 0 failed`. The startup lifecycle prerequisite also passed
from the same root in 22.22 seconds.

### Scenario 2 — Exact Windows all-target gate

The exact CI command was run with only repository-contained environment
isolation:

```powershell
cargo test --no-default-features --features cozo-backend,embeddings --all-targets
```

Three aggregate attempts did not produce a clean result:

1. With `target\validation-temp`, `integration_daemon_startup_order` exceeded
   its own 30-second guard. An isolated repeat reproduced the timeout; moving
   TEMP away from the busy Cargo target made the unchanged test pass.
2. With `tmp\109031-tests`, `contract_retrieval_eval_status` failed on the
   documented transient Cozo `database is locked (code 5)` condition. Its
   isolated repeat passed.
3. With the same isolated root,
   `integration_retrieval_eval_thresholds` exposed process-global test
   cross-talk: one row hit a Cozo lock and the nominally empty row observed
   another row's evaluation data (`sample_size: 7`). A serial diagnostic still
   reproduced contamination (`sample_size: 2`), so scheduling is not a valid
   remedy.

This is a reproducible Windows test-containment blocker. No timeout was
increased, bypassed, or forced; no assertion was skipped.

### Scenario 3 — Runtime, restart, and rollback

Not executed because Scenario 2 is a fail-closed prerequisite. Existing
earlier 15-minute evidence predates commits `33f9693c`, `62de842f`, and
`903247b5` and cannot validate the current HEAD.

## Ownership and Driver Invariants

The all-target runs reached and passed the changed deterministic unit and
contract fixtures before the unrelated integration blocker. Those fixtures
cover admission registration, guard-to-permit transfer, full-mask recovery,
release notification, retirement acknowledgment, stale terminals,
child-before-ack ordering, and single-driver behavior. They are necessary but
do not replace the missing clean aggregate result or current-HEAD runtime
observation.

## Risky Action Record

- **ProposedAction:** isolate temporary test workspaces inside the repository.
  **ActionRisk:** low. **Approval required:** no. **ActionResult:** applied.
- **ProposedAction:** bypass or extend test timeouts. **ActionRisk:** high.
  **Approval required:** yes. **ActionResult:** abandoned; prohibited by the
  task.
- **ProposedAction:** terminate and restart only a tracked disposable daemon
  PID for rollback proof. **ActionRisk:** moderate. **Approval required:** yes,
  supplied by the operator request. **ActionResult:** not started because the
  prerequisite gate failed.

## Required Follow-up

1. Repair or isolate the process-global retrieval-evaluation test state and
   Windows Cozo reopen transient in a separately authorized test-infrastructure
   task.
2. Re-run the exact all-target command with contained TEMP and Git discovery.
3. On a clean result, perform the current-HEAD 15-minute named-pipe observation,
   restart/reconciliation drill, monitoring checks, and full-unit
   revert/restart.
