---
title: "164-S post-merge operational closure"
doc_type: closure
shipment_id: "164-S"
feature_id: "142-F"
mode: post-merge
date: 2026-10-29
author: ship
verdict: "READY (launcher failure matrix and typed-stage diagnostic merged; CI does not yet run the new target, deferred as 00D2A704) - PR #459 merged and 164-S safe-closed (non-cascading; 142-F unchanged)"
closure_status: "READY"
releasability: "READY_WITH_CONDITIONS"
compaction_status: "done"
close_path: "safe_close"
close_evidence: "docs/closure/evidence/164-S-142-F-close-evidence.json"
pr_number: 459
merge_commit: "d9308c1c5193d31a5af5e7bfc64054711e585721"
head_commit_merged: "6a3523e169706f308dfd9ca811e3f0cc9db224b6"
closure_pr_number: null
closure_pr_merge_commit: null
runtime_verification_report: null
follow_up_stash: ["73D7DF69", "00D2A704"]
blocking_stash: null
shipment_record_status: "archived (archived_status: shipped) via backlogit shipment reconcile-shipped"
pre_reconcile: ".backlogit/reconcile/164-S-pre-20261029T0421Z.md"
post_reconcile: ".backlogit/reconcile/164-S-post-20261029T0450Z.md"
---

## Summary

Shipment `164-S` delivered the single task `142.056-T` (F52, route `(c)`, own placeholder, OD-3 B). The new contract
target `tests/contract/start_launcher_failure_test.rs` (`contract_start_launcher_failure`, 5 tests) covers a failure
injected at each of the seven preflight stages (`Build`, `Seal`, `Publish`, `DaemonVerified`, `HealthVerified`,
`CliProbeVerified`, `McpProbeVerified`) with Copilot never launched, paths containing spaces on the launch and cleanup
paths, and the unowned-descendant case (the launcher kills only its own child). `start.ps1` now prints
`engram preflight failed at stage <S>.` when the preflight verdict carries a typed stage. PR #459 merged with a merge
commit (P-009), merge SHA `d9308c1c5193d31a5af5e7bfc64054711e585721`.

## Dark-mode decisions and gates

* `DARK_MODE_ACTIVE`; merge pre-authorized, admin fallback not used.
* Step 2 harness `a06d7e9f`: 227 insertions (under the `SB-1(c)` 400-line cap). Placeholder marker
  `Worker: 142.056-T` in `start.ps1` made the seven-stage table and the marker-absence test fail first (RED).
  Characterization (passed at Step 2): spaced launch path, spaced cleanup path, unowned descendant.
* Step 4 `5e14926f`: the placeholder was replaced by real typed-stage parsing; no `Worker: 142.056-T` marker remains.
  `contract_start_launcher_failure` 5/5 and `contract_start_launcher` 7/7 (untouched).
* Local gates, all PASS: `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings -D clippy::pedantic`,
  `cargo build --all-targets`, `cargo dev-test`.
* Local multi-persona review (Correctness, Rust, Security, Architecture, in parallel): no P0/P1;
  READY_WITH_FOLLOWUPS. P2/P3 residuals were stashed as `73D7DF69` (hang-test pipe-hold flake and descendant/tempdir
  leak, stage-list drift assertion against `RELAY_STAGES`, exact `tasklist` PID match).
* Copilot review: one thread (CI does not run `contract_start_launcher_failure`, `ci.yml` line 130) was out of scope
  under P-021 C1 (`ci.yml` is outside the authorized file set). It was captured first as DEFERRED SCOPE EXPANSION
  stash `00D2A704`, then replied to citing that ID and resolved; the review body was dispositioned
  (`Copilot-Review-Body-Disposition: 5692725299`). The next Copilot review reported no open issues.
  `build` and `start-launcher-windows` passed on merged HEAD `6a3523e169706f308dfd9ca811e3f0cc9db224b6`;
  `autoharness gate copilot-review 459` returned `SATISFIED: PASS`, re-run immediately before merge.

## Safe-close and archive evidence

* Pre-reconcile report `.backlogit/reconcile/164-S-pre-20261029T0421Z.md` was written while
  `.backlogit/queue/164-S.md` was still `active` in the queue.
* `autoharness shipment cascade-close --classify-only --shipment 164-S --feature 142-F --sha
  d9308c1c5193d31a5af5e7bfc64054711e585721 --json` exited 3: `SAFE_CLOSE`, no mutation, engine verdict `VERIFIED`.
  Evidence `docs/closure/evidence/164-S-142-F-close-evidence.json` was written after the pre-report.
* `.backlogit/archive/142.056-T.md` carries `status: archived`, `archived_status: done`,
  `archived_from: .backlogit/queue/142.056-T.md` (`backlogit move --status done` and `backlogit archive`, run on the
  PR branch before the merge).
* `backlogit archive 164-S` moved the record to `.backlogit/archive/164-S.md`; then
  `backlogit shipment reconcile-shipped 164-S` sets `archived_status: shipped`. `backlogit shipment ship` was not
  invoked and no file was deleted by hand.
* Covering feature `142-F` is `status: active` with SHA-256
  `42FCA67C1407641C8F3CF4752B3431D978290B1342614A831DF463B572181770` before and after close.
* This closure document was reviewed and finalized before the `reconcile-shipped` call, which attests it as committed
  with that call; it is not edited afterwards.
* Post-reconcile report: `.backlogit/reconcile/164-S-post-20261029T0450Z.md`.

## 164-S

Delivery: PR #459 (feature), commit d9308c1c5193d31a5af5e7bfc64054711e585721 (merge commit, P-009).

## Procedure deviation record (operator-directed)

The shipment-reconcile skill's non-cascading sequence moves the shipment to `shipped` before `backlogit archive`. This
close instead followed the sequence the operator prescribed for this dark-mode run (the same as 154-S to 163-S):
`backlogit archive 164-S` (`archived_status: active`) and then the official `backlogit shipment reconcile-shipped` with
`--closure-evidence` and `--confirm`, which records a `shipment_reconciled_shipped` event bound to merge SHA
`d9308c1c5193d31a5af5e7bfc64054711e585721` and ends at `archived_status: shipped`. Approval source: operator
activation contract.

## Runtime verification

The matrix runs `start.ps1` (a copy in a directory whose name contains a space) against a rustc-built fixture
`engram.exe`, locally on Windows (5/5). The hosted `build` job (ubuntu-latest) only compiles the `cfg(windows)` target
to zero tests, and `start-launcher-windows` runs only `contract_start_launcher`, so no hosted CI run has executed the
5 tests (deferred `00D2A704`). A real `engram preflight` run against a live daemon was
not performed in this slice; the contract fixture stands in for it.

## Monitoring and rollback handoff

This slice is not a release and is not tagged or published on its own. `start.sh` still has no preflight step
(`142.057-T`, F53).

| Evidence | Value |
|---|---|
| SLI | For a failure at any of the seven preflight stages, the launcher exits non-zero without starting Copilot and prints the typed-stage line (100% of the matrix cases). |
| Observation query | `cargo test --test contract_start_launcher_failure` (local, Windows), and `cargo test --test contract_start_launcher` plus the `start-launcher-windows` CI job on `main` (`gh run list --workflow ci.yml`). |
| Baseline | 5/5 `contract_start_launcher_failure` and 7/7 `contract_start_launcher` pass locally; the CI `start-launcher-windows` job (7 `contract_start_launcher` tests only) passed on the merged HEAD. |
| Alert threshold | One red run of either target on `main`, a flaky `hang`-mode timing failure that persists after one retry, or one confirmed report (issue or operator message to the release owner) that Copilot launched after a failed preflight. |
| Owner | Repository maintainer acting as release owner of the next tagged release (softwaresalt); Stage owns triage of `00D2A704` and `73D7DF69`. |
| Observation window | From the merge of PR #459 until the first tagged release containing the launcher is published, plus one CI cycle on `main` after it. |
| Rollback trigger | A persistent red or flaky `contract_start_launcher_failure` run on `main` that blocks unrelated work, or any alert-threshold condition that persists after one retry. |
| Rollback procedure | Revert only `tests/contract/start_launcher_failure_test.rs` and the typed-stage lines in `start.ps1` from PR #459 (merge `d9308c1c5193d31a5af5e7bfc64054711e585721`); revert the test file back to its F00 placeholder if only the test is the problem. Do not revert the whole merge: that would also undo the `142.056-T` archival and conflict with the closed `164-S` shipment record. Revert `start.ps1` and
`tests/contract/start_launcher_failure_test.rs` together, because the matrix asserts the typed-stage line; to roll back
only the test, restore the F00 placeholder. Reverting the `start.ps1` lines alone leaves `contract_start_launcher_failure`
red, and the fail-closed behavior from `163-S` is unaffected. Preserve backlog state and closure evidence. |

Conditions for `READY_WITH_CONDITIONS`: the observation window above completes, the new target is added to the Windows
CI job (`00D2A704`), and the launcher is exercised once against a live engram daemon.

## Source artifact cleanup

Shipped item `142.056-T` carried no `source_stash_id` or `source_deliberation_id` custom field. No stash or
deliberation was archived; no `linked_deliberation_disposition` entry applies.

## Compaction (P-020)

Compaction was applied with `target: all`: the shipment memory was consolidated into
`docs/memory/compacted/2026-10-29-164-s-compacted.md` and the original placed in
`docs/archive/memory/2026-10-28-ship-164-s-142-056-t-launcher-failure-matrix.md`. Only the just-closed unit was a
candidate. Compaction status: `done`.

## Follow-ups

* `73D7DF69` (P2/P3 hardening of the matrix tests) and `00D2A704` (DEFERRED SCOPE EXPANSION: add the target to the
  `start-launcher-windows` job in `.github/workflows/ci.yml`) are routed to Stage for triage.
* `142.057-T` (F53, `start.sh` parity) and `142.059-T` remain in 142-F.
* Stage proceeds with the next 142-F slot.
