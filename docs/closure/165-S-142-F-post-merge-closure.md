---
title: "165-S post-merge operational closure"
doc_type: closure
shipment_id: "165-S"
feature_id: "142-F"
mode: post-merge
date: 2026-10-29
author: ship
verdict: "READY (start.sh fail-closed preflight and exact-child cleanup merged) - PR #461 merged and 165-S safe-closed (non-cascading; 142-F unchanged)"
closure_status: "READY"
releasability: "READY_WITH_CONDITIONS"
compaction_status: "done"
close_path: "safe_close"
close_evidence: "docs/closure/evidence/165-S-142-F-close-evidence.json"
pr_number: 461
merge_commit: "50bd6b5c0a565bc1ca65ab99123f52a64b093335"
head_commit_merged: "541e91d00c0e226ae78e30f197353550d00d4bd1"
closure_pr_number: null
closure_pr_merge_commit: null
runtime_verification_report: null
follow_up_stash: ["69C43666"]
blocking_stash: null
shipment_record_status: "archived (archived_status: shipped) via backlogit shipment reconcile-shipped"
pre_reconcile: ".backlogit/reconcile/165-S-pre-20261029T0557Z.md"
post_reconcile: ".backlogit/reconcile/165-S-post-20261029T0620Z.md"
---

## Summary

Shipment `165-S` delivered the single task `142.057-T` (F53, route `(a-1)`). `start.sh` now runs
`engram preflight "$WORKSPACE" --timeout-ms <budget-3000>` (default budget 15000, `ENGRAM_PREWARM_TIMEOUT_MS`
3001..30000), launches the agent only when the exit status is 0 and a `Succeeded` verdict line is present with no
other state, prints `engram preflight failed at stage <S>.` with stdout and stderr and does not launch on any failure,
and on timeout or interrupt terminates only its own child (TERM, bounded grace, KILL, reap). The contract target
`tests/contract/start_sh_launcher_test.rs` (`contract_start_sh_launcher`, 10 tests) covers success, typed failure,
exit 0 without a verdict, non-zero exit with a `Succeeded` line, invalid budgets, a missing executable, timeout
cleanup, TERM-resistant interrupt cleanup and the workspace working directory. PR #461 merged with a merge commit
(P-009), merge SHA `50bd6b5c0a565bc1ca65ab99123f52a64b093335`.

## Dark-mode decisions and gates

* `DARK_MODE_ACTIVE`; merge pre-authorized, admin fallback not used.
* Step 2 harness `4c81f5c7`: 291 insertions (under the `SB-1(c)` 400-line cap), RED against the `exec`-only
  `start.sh` on `main`. The parked `e24f5ae2` (738 lines) was not reused because it exceeds the cap.
* Step 4 `e3e0b437`, then review fixes `917af107`, `a62abc3d` and `541e91d0` (merged HEAD).
* Local gates, all PASS: `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings -D clippy::pedantic`,
  `cargo build --all-targets`, `cargo test --all-targets` (full run before the last two small commits; the final commits
  were gated by fmt, clippy and `contract_start_sh_launcher` 10/10, and hosted CI ran the full suite on the merged HEAD).
  One timing test, `integration_backlog_hydration::backlog_index_100_items_under_5_seconds`, exceeded its 5 s limit
  twice on the loaded local host (5.9 s and 6.5 s) and passed on the full run and in CI; it is unrelated to this change.
* Local multi-persona review (Correctness, Rust, Security, Architecture, in parallel; Correctness again on the fix
  delta): no P0. Fixed in scope: test process leak, Windows bash discovery, wall-clock deadline, trap cleanup, guarded
  `kill -9`, budget digit-length check, working directory and relative agent path, extra tests. READY_WITH_FOLLOWUPS.
* Out-of-scope finding: `start.sh` environment parity with `start.ps1` (`.env.local`, `ENGRAM_DATA_DIR`, `backlogit
  sync`) was captured first as DEFERRED SCOPE EXPANSION stash `69C43666` (pre-PR, threadless path; PR number and thread
  recorded `N/A` at capture), never expanded into.
* Copilot review: one in-scope thread (interrupt cleanup must escalate TERM to KILL and reap the child) fixed in
  `541e91d0` with a regression test, replied to, resolved, and the review body dispositioned
  (`Copilot-Review-Body-Disposition: 5693348475`). `build` and `start-launcher-windows` passed on `541e91d0`;
  `autoharness gate copilot-review 461` returned `SATISFIED: PASS`.

## Safe-close and archive evidence

* Pre-reconcile report `.backlogit/reconcile/165-S-pre-20261029T0557Z.md` was written while
  `.backlogit/queue/165-S.md` was still `active` in the queue.
* `autoharness shipment cascade-close --classify-only --shipment 165-S --feature 142-F --sha
  50bd6b5c0a565bc1ca65ab99123f52a64b093335 --json` exited 3: `SAFE_CLOSE`, no mutation, engine verdict `VERIFIED`.
  Evidence `docs/closure/evidence/165-S-142-F-close-evidence.json` was written after the pre-report.
* `.backlogit/archive/142.057-T.md` carries `status: archived`, `archived_status: done`,
  `archived_from: .backlogit/queue/142.057-T.md` (`backlogit move --status done` and `backlogit archive`, run on the PR
  branch before the merge).
* `backlogit archive 165-S` moved the record to `.backlogit/archive/165-S.md`; then
  `backlogit shipment reconcile-shipped 165-S` sets `archived_status: shipped`. `backlogit shipment ship` was not
  invoked and no file was deleted by hand.
* Covering feature `142-F` is `status: active` with SHA-256
  `42FCA67C1407641C8F3CF4752B3431D978290B1342614A831DF463B572181770` before and after close.
* This closure document was reviewed and finalized before the `reconcile-shipped` call, which attests it as committed
  with that call; it is not edited afterwards.
* Post-reconcile report: `.backlogit/reconcile/165-S-post-20261029T0620Z.md`.

## 165-S

Delivery: PR #461 (feature), commit 50bd6b5c0a565bc1ca65ab99123f52a64b093335 (merge commit, P-009).

## Procedure deviation record (operator-directed)

The shipment-reconcile skill's non-cascading sequence moves the shipment to `shipped` before `backlogit archive`. This
close instead followed the sequence the operator prescribed for this dark-mode run (the same as 154-S to 164-S):
`backlogit archive 165-S` (`archived_status: active`) and then the official `backlogit shipment reconcile-shipped` with
`--closure-evidence` and `--confirm`, which records a `shipment_reconciled_shipped` event bound to merge SHA
`50bd6b5c0a565bc1ca65ab99123f52a64b093335` and ends at `archived_status: shipped`. Approval source: operator
activation contract.

## Runtime verification

`contract_start_sh_launcher` runs a copy of `start.sh` (in a directory whose name contains spaces) against controlled
Bash `engram` and `copilot` fixtures, locally under Git Bash on Windows (10/10); it also runs on the Linux `build` CI job
through `cargo test --all-targets`. `start-launcher-windows` runs only `contract_start_launcher`, so the Windows job does
not execute the Bash target. A real `engram preflight` run against a live daemon was not performed in this slice; the
fixtures stand in for it.

## Monitoring and rollback handoff

This slice is not a release and is not tagged or published on its own.

| Evidence | Value |
|---|---|
| SLI | After any failed preflight (non-zero exit, timeout, missing executable, or no `Succeeded` verdict) `start.sh` exits non-zero without starting Copilot and prints the typed-stage line when one is reported (100% of the matrix cases). |
| Observation query | `cargo test --test contract_start_sh_launcher` locally, and the `build` job on `main` (`gh run list --workflow ci.yml --branch main`, then search that run's `cargo test` step log for `contract_start_sh_launcher`). |
| Baseline | 10/10 `contract_start_sh_launcher` pass locally; the `build` job (full `cargo test --all-targets` suite, which includes the target) passed on the merged HEAD `541e91d0` (run 40825406910). |
| Alert threshold | One red run of `contract_start_sh_launcher` on `main`, a `timeout_cleanup` or `interrupt` timing failure that persists after one retry, or one confirmed report (issue or operator message to the release owner) that Copilot launched after a failed preflight. |
| Owner | Repository maintainer acting as release owner of the next tagged release (softwaresalt); Stage owns triage of `69C43666`. |
| Observation window | From the merge of PR #461 until the first tagged release containing the launcher is published, plus one CI cycle on `main` after it. |
| Rollback trigger | A persistent red or flaky `contract_start_sh_launcher` run on `main` that blocks unrelated work, a launcher that blocks a healthy workspace, or any alert-threshold condition that persists after one retry. |
| Rollback procedure | Revert only `start.sh` and `tests/contract/start_sh_launcher_test.rs` from PR #461 (merge `50bd6b5c0a565bc1ca65ab99123f52a64b093335`), for example `git checkout 9cb210eb -- start.sh tests/contract/start_sh_launcher_test.rs` on a revert branch (`9cb210eb` is the `main` commit PR #461 branched from and the merge's first parent; the test file exists there as the F00 placeholder, so the `[[test]]` entry in `Cargo.toml` stays valid), then verify with `cargo test --test contract_start_sh_launcher` and open a PR. Do not revert the whole merge: that would also undo the `142.057-T` archival and conflict with the closed `165-S` shipment record. Reverting `start.sh` alone leaves the contract target red, so revert both files together. `start.ps1` is unaffected. Preserve backlog state and closure evidence. |

Conditions for `READY_WITH_CONDITIONS`: the observation window above completes, and the launcher is exercised once
against a live engram daemon on a Unix host.

## Source artifact cleanup

Shipped item `142.057-T` carried no `source_stash_id` or `source_deliberation_id` custom field. No stash or
deliberation was archived; no `linked_deliberation_disposition` entry applies.

## Compaction (P-020)

Compaction was applied with `target: all`: the shipment memory was consolidated into
`docs/memory/compacted/2026-10-29-165-s-compacted.md` and the original placed in
`docs/archive/memory/2026-10-29-ship-165-s-142-057-t-launcher.md`. Only the just-closed unit was a candidate.
Compaction status: `done`.

## Follow-ups

* `69C43666` (DEFERRED SCOPE EXPANSION: `start.sh` environment parity with `start.ps1`) is routed to Stage for triage.
* `142.059-T` (F55) remains in 142-F.
* Stage proceeds with the next 142-F slot.
