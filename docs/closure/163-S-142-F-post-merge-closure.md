---
title: "163-S post-merge operational closure"
doc_type: closure
shipment_id: "163-S"
feature_id: "142-F"
mode: post-merge
date: 2026-10-29
author: ship
verdict: "READY (start.ps1 pre-warm replaced by fail-closed preflight; start.sh parity 142.057-T and docs 142.059-T pending) - PR #457 merged and 163-S safe-closed (non-cascading; 142-F unchanged)"
closure_status: "READY"
releasability: "READY_WITH_CONDITIONS"
compaction_status: "done"
close_path: "safe_close"
close_evidence: "docs/closure/evidence/163-S-142-F-close-evidence.json"
pr_number: 457
merge_commit: "eb9e481406e76a9511a47592f1c5b600a2e607d6"
head_commit_merged: "c581498d962ba63d343b12c47cc296d08640f21c"
closure_pr_number: null
closure_pr_merge_commit: null
runtime_verification_report: null
follow_up_stash: []
blocking_stash: null
shipment_record_status: "archived (archived_status: shipped) via backlogit shipment reconcile-shipped"
pre_reconcile: ".backlogit/reconcile/163-S-pre-20261029T0308Z.md"
post_reconcile: ".backlogit/reconcile/163-S-post-20261029T0330Z.md"
---

## Summary

Shipment `163-S` delivered the single task `142.055-T` (F51, route `(a-1)`). `start.ps1` now runs
`<workspace>\target\debug\engram.exe preflight <WORKSPACE> --timeout-ms <T>` through
`ProcessStartInfo.ArgumentList`, with `T` equal to the outer budget minus 3000 ms (2000 ms relay grace plus 1000 ms
margin). Preflight replaces the legacy `sync`/`bind` pre-warm under one budget. Copilot launches only when the exit is
`0` and a `Succeeded` verdict line is present with no other state. The tested failure paths (non-zero exit, timeout, missing executable, non-`Succeeded` or absent verdict) and any
exception raised while starting or reading the child block Copilot and prints stdout and stderr. Timeout cleanup terminates only the launcher's own child
process. Only `start.ps1` and `tests/contract/start_launcher_test.rs` changed (plus backlog state). PR #457 merged with
a merge commit (P-009), merge SHA `eb9e481406e76a9511a47592f1c5b600a2e607d6`.

## Dark-mode decisions and gates

* `DARK_MODE_ACTIVE`; merge pre-authorized, admin fallback not used.
* Step 2 harness: the parked commits `1dfc1b5b` (594 insertions) would have broken the `SB-1(c)` 400-line cap, so the
  harness was rewritten compactly (172 insertions). The two fail-open tests became fail-closed tests. The fixture
  `engram.exe` is compiled by the test with `rustc`, so the launcher keeps the exact `target\debug\engram.exe` path
  with no override seam. RED was observed against the old `start.ps1` (all 6 harness tests failing), then GREEN; the seventh test (exit `0` without a `Succeeded` verdict) was added after review, giving 7/7.
* Deviations stricter than the contract text: exit `0` also requires a `Succeeded` verdict line (task acceptance
  criterion); `#Requires -Version 7.0` was added because `ArgumentList` needs .NET Core.
* Local gates, all PASS: `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings -D clippy::pedantic`,
  `cargo build --all-targets`, `cargo dev-test`, `cargo test --test contract_start_launcher`. One load-induced timing
  failure in `backlog_index_100_items_under_5_seconds` passed on rerun and in the next full run; it is unrelated to
  this change.
* Local multi-persona review (Correctness, Rust, Security, Architecture, in parallel): no P0/P1. In-scope P2/P3
  findings were fixed (PowerShell 7 guard, strict verdict, shared drain deadline, specific-message and
  single-invocation assertions, wider timing bounds).
* Copilot review, three rounds on the pipe-capture path, all threads fixed and resolved, each review body
  dispositioned (`Copilot-Review-Body-Disposition`): partial diagnostics kept after the drain deadline, a
  direct-child-terminated assertion, and per-character reads. `build` and `start-launcher-windows` passed on the
  merged HEAD `c581498d962ba63d343b12c47cc296d08640f21c`; `autoharness gate copilot-review 457` returned
  `SATISFIED: PASS`, re-run immediately before merge.
* No P-021 deferred scope expansion was needed; no stash entry was created.

## Safe-close and archive evidence

* Pre-reconcile report `.backlogit/reconcile/163-S-pre-20261029T0308Z.md` was written while
  `.backlogit/queue/163-S.md` was still `active` in the queue.
* `autoharness shipment cascade-close --classify-only --shipment 163-S --feature 142-F --sha
  eb9e481406e76a9511a47592f1c5b600a2e607d6 --json` exited 3: `SAFE_CLOSE`, no mutation, engine verdict `VERIFIED`.
  Evidence `docs/closure/evidence/163-S-142-F-close-evidence.json` was written after the pre-report.
* `.backlogit/archive/142.055-T.md` carries `status: archived`, `archived_status: done`,
  `archived_from: .backlogit/queue/142.055-T.md` (`backlogit move --status done` and `backlogit archive`).
* `backlogit archive 163-S` moved the record to `.backlogit/archive/163-S.md`; then
  `backlogit shipment reconcile-shipped 163-S` sets `archived_status: shipped`. `backlogit shipment ship` was not
  invoked and no file was deleted by hand.
* Covering feature `142-F` is `status: active` with SHA-256
  `42FCA67C1407641C8F3CF4752B3431D978290B1342614A831DF463B572181770` before and after close.
* This closure document was reviewed and finalized before the `reconcile-shipped` call, which attests it as committed
  with that call; it is not edited afterwards.
* Post-reconcile report: `.backlogit/reconcile/163-S-post-20261029T0330Z.md`.
* Correction after Copilot review of the closure PR: the follow-up handoff named `142.074-T` (already done) instead of queued `142.059-T`; the four references were corrected. The `closure_content_hash` in the `shipment_reconciled_shipped` event (`7e17af1bd55e907af8457fe541491c425138b96e61f3aed2c39819d8b0d68392`) attests the text before that correction; `backlogit shipment reconcile-shipped` refuses a second attestation (exit 10, conflicting reconciliation), and `autoharness gate closure-evidence` still passes.

## 163-S

Delivery: PR #457 (feature), commit eb9e481406e76a9511a47592f1c5b600a2e607d6 (merge commit, P-009).

## Procedure deviation record (operator-directed)

The shipment-reconcile skill's non-cascading sequence moves the shipment to `shipped` before `backlogit archive`. This
close instead followed the sequence the operator prescribed for this dark-mode run (the same as 154-S to 162-S):
`backlogit archive 163-S` (`archived_status: active`) and then the official `backlogit shipment reconcile-shipped` with
`--closure-evidence` and `--confirm`, which records a `shipment_reconciled_shipped` event bound to merge SHA
`eb9e481406e76a9511a47592f1c5b600a2e607d6` and ends at `archived_status: shipped`. Approval source: operator
activation contract.

## Runtime verification

The launcher is exercised by `contract_start_launcher` (7 tests) on both the local machine and the hosted Windows
runner (`start-launcher-windows`, passing on the merged HEAD). The tests cover a `Succeeded` launch with one argument
per element (workspace path containing a space), typed-stage failure, exit `0` without a `Succeeded` verdict, a
missing executable, and a timeout where an unowned descendant holding the pipes survives while the direct child is
terminated. A real `engram preflight` run against a live daemon was not performed in this slice; the contract
fixture stands in for it.

## Monitoring and rollback handoff

This slice is not a release and is not tagged or published on its own. The Unix launcher `start.sh` still has no
preflight step (`142.057-T`, F53).

| Evidence | Value |
|---|---|
| SLI | Launcher exits non-zero without starting Copilot in 100% of failing-preflight cases (exit non-zero, timeout, missing executable, non-`Succeeded` verdict), printing the typed-stage line. |
| Observation query | `cargo test --test contract_start_launcher`, and the `start-launcher-windows` job of the PR CI workflow (`gh run list --workflow` on `main`). |
| Baseline | 7/7 tests pass locally and on the hosted runner; they assert the SLI cases directly. |
| Alert threshold | One red run of either on `main`, or one confirmed report (issue or operator message to the release owner) that Copilot launched after a failed preflight or that a healthy workspace was blocked. |
| Owner | Repository maintainer acting as release owner of the next tagged release (softwaresalt); Stage owns follow-up triage; the live-daemon check is due before that release is tagged. |
| Observation window | From the merge of PR #457 until the first tagged release containing the launcher is published, plus one CI cycle on `main` after it. |
| Rollback trigger | Any alert-threshold condition that persists after one retry. |
| Rollback procedure | Revert only the changes from PR #457 (merge `eb9e481406e76a9511a47592f1c5b600a2e607d6`) in `start.ps1` and `tests/contract/start_launcher_test.rs`; revert both together because the contract test asserts the new launcher behavior. Do not revert the whole merge: that would also undo the `142.055-T` archival and conflict with the closed `163-S` shipment record. Reverting `start.ps1` restores the legacy fail-open `sync`/`bind` pre-warm; as the interim mitigation, run `engram preflight <WORKSPACE> --timeout-ms <MS>` by hand before launching Copilot. Preserve backlog state and closure evidence. |

Conditions for `READY_WITH_CONDITIONS`: the observation window above completes and the launcher is exercised once
against a live engram daemon.

## Source artifact cleanup

Shipped item `142.055-T` carried no `source_stash_id` or `source_deliberation_id` custom field. No stash or
deliberation was archived; no `linked_deliberation_disposition` entry applies. Existing stash entries `03AA00A8` and
`E53CB236` (launcher-related deferred scope from earlier slots) were cited and not duplicated; Stage owns their triage.

## Compaction (P-020)

Compaction was applied with `target: all`: the shipment memory was consolidated into
`docs/memory/compacted/2026-10-29-163-s-compacted.md` and the original placed in
`docs/archive/memory/2026-10-29-ship-163-s-slot-16-memory.md`. Only the just-closed unit was a candidate.
Compaction status: `done`.

## Follow-ups

* No new stash entries.
* The compound note that still says to treat fail-open as a requirement is covered by queued `142.059-T` (F55, Superseded by 142-F note). The README and `docs/configuration.md` descriptions of the `engram sync --direct` pre-warm are not owned by any queued task and are routed to Stage for ownership. `start.sh` parity is `142.057-T`.
* Stage proceeds with the next 142-F slot.
