---
title: "Session Memory - 086-S writer-side workspace+config atomicity closure"
type: session-memory
date: 2026-07-17
agent: ship
shipment: 086-S
feature: 092-F
tasks:
  - 092.001-T
  - 092.002-T
---

## Outcome

Shipment 086-S (feature 092-F, task 092.001-T) is fully shipped and closed. The writer-side
workspace+config atomicity fix (082-S adversarial review F4) is merged to main.

## Task and backlog status updates

* `092.001-T` - Atomic `set_workspace_and_config`; bind flow migrated; non-vacuous A->B/B->A
  atomicity test. Status -> done.
* `092-F` - Covering feature. Status -> archived.
* `086-S` - Shipment. Status -> archived.
* `092.002-T` - Created this closure, remains queued. Deferred reader-side atomicity follow-up.

## Files modified (merged branch feat/086-writer-atomicity)

* `src/server/state.rs` - New `set_workspace_and_config` atomic method (acquires
  active_workspace.write() then workspace_config.write(), capacity check first, no partial
  publish). Plus doc edits: T041 deadlock-audit comment rewritten for the intentional
  paired-lock exception (removed the inaccurate !Send claim); `snapshot_dispatch_context` and
  `DispatchSnapshot` rustdoc now reference the atomic writer.
* `src/tools/lifecycle.rs` - Bind flow migrated from the two-await sequence to the single atomic
  call; `get_workspace_status` note updated to reference the atomic writer.
* `tests/integration/get_workspace_status_atomicity_test.rs` - torn-publish + LimitReached
  no-partial-publish tests (target `integration_get_workspace_status_atomicity`, 3 tests).

## Merge trail

* PR #261, branch `feat/086-writer-atomicity`.
* Commits: `81ba27d` (atomic fix + adversarial-review doc fixes, amended), `4888935` (Copilot-nit
  lifecycle doc fix).
* Merge commit `106be1d` (106be1dcbb811ac9f3504a77e8512d431b807fb5).

## Decisions and rationale

* Both write locks acquired in the reader's order (active_workspace -> workspace_config). Only two
  crate-wide double-lock sites (reader read/read, writer write/write), both same order -> no
  inversion -> deadlock-free. RwLock guards are Send, which is why holding across the second lock's
  await compiles.
* Capacity check (LimitReached) runs first while holding both guards; Err returns with no partial
  publish. Byte-identical to set_workspace's check.
* Adversarial review before Copilot (operator directive): GPT-5.6 Sol @ xhigh (key rust/correctness)
  + Gemini-3.1-pro-preview @ high (concurrency). Both rated the core fix sound. Applied 2 P2 doc
  fixes. 1 Copilot cycle only (stale get_workspace_status note).

## Deferred (092.002-T, queued)

Reader-side atomicity for `background_db_hydration` (~lifecycle.rs L336) and `drain_pending_sync`
(~L397): both read the (workspace, config) pair non-atomically via separate snapshot_workspace()
+ workspace_config() awaits. Unchanged by 092.001-T, out of scope for the writer-side task. Fix
needs a new None-gating atomic reader returning Option<(WorkspaceSnapshot, WorkspaceConfig)> that
preserves skip-if-either-None semantics (a naive snapshot_dispatch_context() swap would default the
config instead of skipping - a semantic change). Opt-in/self-healing, so risk is a single stale-
config pass, not data loss.

## Rejected

Privatize set_workspace / set_workspace_config (Gemini C3, P3): both are pub API used by ~35 test
call sites; privatizing breaks the suite. Tests = spec.

## Known flake (not a regression)

`contract_evaluation::c017_03_agents_have_required_subfields` fails under full `--all-targets`
parallel run (evaluation-telemetry cross-binary contention) but passes 5/5 in isolation.

## Next steps

* Drive the 086-S closure PR through the 4-point Copilot merge gate (docs/backlog-only PR skips the
  CI Rust build via paths-ignore) and merge with `--merge`.
* Re-assess origin/main queue for remaining shipments after 086-S closure. Residual/deferred:
  025-S; 081-S (PR #248 LEFT OPEN - operator's active branch, do not close while AFK); residual
  tasks 087.005/006-T, 090.004-T (blocked), 090.005-T (queued), 091.015/016/017/019/020-T,
  and now 092.002-T (queued).
