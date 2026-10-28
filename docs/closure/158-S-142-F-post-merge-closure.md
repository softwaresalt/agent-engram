---
title: "158-S post-merge operational closure"
doc_type: closure
shipment_id: "158-S"
feature_id: "142-F"
mode: post-merge
date: 2026-10-28
author: ship
verdict: "READY (library module only; no runtime surface consumes it yet) - PR #447 merged and 158-S safe-closed (non-cascading; 142-F unchanged)"
closure_status: "READY"
releasability: "READY_WITH_CONDITIONS"
compaction_status: "done"
close_path: "safe_close"
close_evidence: "docs/closure/evidence/158-S-142-F-close-evidence.json"
pr_number: 447
merge_commit: "0071c2eba5655a087ba27467d34432879dc5599c"
head_commit_merged: "867c9d9aa85c32d54d43eade3e42b005deab4b1a"
closure_pr_number: null
closure_pr_merge_commit: null
runtime_verification_report: null
follow_up_stash: []
blocking_stash: null
shipment_record_status: "archived (archived_status: shipped) via backlogit shipment reconcile-shipped"
pre_reconcile: ".backlogit/reconcile/158-S-pre-20261028T1838Z.md"
post_reconcile: ".backlogit/reconcile/158-S-post-20261028T1850Z.md"
---

## Summary

Shipment `158-S` delivered the single task `142.070-T` (NEW-2): the supervisor invocation contract and dispatch
`engram_indexer::preflight_invocation` in `crates/engram-indexer/src/preflight_invocation.rs`. `parse_invocation` is a
strict parser (`--workspace` kept as given, `--timeout-ms` u64, `--engram` absolute canonical regular file; duplicate,
unknown, positional and flag-as-value inputs rejected), `deadline_from` uses `checked_add` behind a documented
`MAX_TIMEOUT_MS` cap (`u32::MAX` seconds), and `main_with` returns `PreflightOutput { stdout, stderr, exit_code }`
with exactly one `engram-preflight stage=<Stage|none> cause=<ok|usage|setup|deadline|verifier> elapsed_ms=<n>` stderr
line. PR #447 merged with a merge commit (P-009) at head `867c9d9a`, merge SHA `0071c2eba5655a087ba27467d34432879dc5599c`.

## Dark-mode decisions and gates

* `DARK_MODE_ACTIVE`; merge pre-authorized, admin fallback not authorized and not used (`DARK_MODE_MERGE_AUTHORIZED`).
* Claim: predecessor `157-S` was already `archived_status: shipped`; `pipeline-topology` pre_claim and post_claim exited 0
  and `backlogit shipment claim 158-S` succeeded natively.
* Harness first (`7bc9a17f`): the three crate-local tests failed RED with `Worker: 142.070-T`; the implementation
  (`32265c68`) turned them GREEN; review fixes in `372ea2ce`. Task archived in `867c9d9a`.
* Local gates: `cargo test -p engram-indexer --test preflight_invocation_test` (3/3), `cargo test -p engram-indexer
  --all-targets --no-fail-fast`, both pedantic clippy runs, `cargo fmt --all -- --check`, `cargo build -p engram-indexer`,
  `cargo build --all-targets` (one transient rustc 0xc0000409 crash, retry passed) and `cargo dev-test` PASS. PR `build`
  and `start-launcher-windows` jobs passed on the merged head.
* Local multi-persona review (Correctness, Rust, Security, Architecture in parallel): READY_WITH_FOLLOWUPS, no P0/P1.
  In-scope P2/P3 items fixed in `372ea2ce` (`&OsStr` flag check, verifier-never-invoked assertion, `u64::MAX` through
  `main_with`, extra `--engram` cases, cap boundary test, rustdoc contract). Accepted advisories: post-hoc `cause=deadline`
  heuristic and `MAX_TIMEOUT_MS` cap are documented. No new out-of-scope findings, so no P-021 C2 capture and no new stash entry. Existing stash `47DAEF9D` (deferred scope expansion from PR #410: portable timeout bound for the `u64::MAX` case) is addressed in code by the `MAX_TIMEOUT_MS` cap; any acceptance-criteria or plan change and the stash disposition remain Stage-owned, so it is left open for Stage triage (cited, not duplicated).
* Copilot review `5687751845` ("no open issues"): no threads and no review-body findings.
  `autoharness gate copilot-review 447` returned `SATISFIED: PASS` before and at merge.

## Safe-close and archive evidence

* Pre-reconcile report `.backlogit/reconcile/158-S-pre-20261028T1838Z.md` (generated 18:38:43Z) was written while
  `.backlogit/queue/158-S.md` was still `active` in the queue.
* `autoharness shipment cascade-close --classify-only --shipment 158-S --feature 142-F --sha
  0071c2eba5655a087ba27467d34432879dc5599c --json` exited 3: `SAFE_CLOSE`, no mutation, engine verdict `UNVERIFIED`.
  Evidence `docs/closure/evidence/158-S-142-F-close-evidence.json` was written after the pre-report.
* `.backlogit/archive/142.070-T.md` carries `status: archived`, `archived_status: done`,
  `archived_from: .backlogit/queue/142.070-T.md` (`backlogit move --status done` and `backlogit archive`).
* `backlogit archive 158-S` moved the record to `.backlogit/archive/158-S.md`; then
  `backlogit shipment reconcile-shipped 158-S` set `archived_status: shipped`. `backlogit shipment ship` was not
  invoked and no file was deleted by hand.
* Covering feature `142-F` is `status: active` with SHA-256
  `42FCA67C1407641C8F3CF4752B3431D978290B1342614A831DF463B572181770` before and after close.
* Post-reconcile report: `.backlogit/reconcile/158-S-post-20261028T1850Z.md`.

## 158-S

Delivery: PR #447 (feature), commit 0071c2eba5655a087ba27467d34432879dc5599c (merge commit, P-009).

## Procedure deviation record (operator-directed)

The shipment-reconcile skill's non-cascading sequence moves the shipment to `shipped` and verifies live `status: shipped` before `backlogit archive`. This close instead followed the sequence the operator prescribed in the 158-S dark-mode activation contract (the same sequence approved and merged for 154-S to 157-S): `backlogit archive 158-S` (`archived_status: active`) and then the official `backlogit shipment reconcile-shipped` with `--closure-evidence` and `--confirm`, which records a `shipment_reconciled_shipped` event bound to merge SHA `0071c2eba5655a087ba27467d34432879dc5599c` and ends at `archived_status: shipped`. Approval source: operator activation contract (`merge_approval_pre_authorized: true`, explicit closure procedure).

## Runtime verification

Not applicable. The change adds a library module that no binary or launcher consumes yet, so no runtime surface changed.

## Monitoring and rollback handoff

This slice is not a release and is not tagged or published on its own. The handoff applies to the first release whose
launcher consumes `preflight_invocation` (142.071-T and later).

| Evidence | Value |
|---|---|
| SLI | Fraction of `engram-indexer preflight` invocations whose stdout, exit code and single stderr line match the contract: usage errors exit 2 with empty stdout, setup errors and failed verdicts exit 1, success exits 0, and the stderr line matches `engram-preflight stage=<Stage\|none> cause=<ok\|usage\|setup\|deadline\|verifier> elapsed_ms=<n>`. |
| Observation query | `cargo test -p engram-indexer --all-targets --no-fail-fast` (the crate-local test is not run by `cargo dev-test`), and the launcher's preflight stderr line once wired (142.071-T). |
| Baseline | 3/3 `preflight_invocation_test` cases pass; the stderr line carries no paths, env values or payloads; `--timeout-ms 0` never invokes the verifier. |
| Alert threshold | Any `preflight_invocation_test` failure on `main`, or any consumer observing a stderr line outside the documented grammar. |
| Owner | Release owner of the next tagged release; Stage owns follow-up triage. |
| Observation window | The first release that consumes the invocation contract, plus one daemon restart. |
| Rollback trigger | Any alert-threshold condition that persists after one retry. |
| Rollback procedure | The module is unconsumed today, so reverting merge commit `0071c2eba5655a087ba27467d34432879dc5599c` removes only the module, its `pub mod` line and its test. Once 142.071-T consumes it, roll back the consuming release together with this merge and rebuild. |

## Source artifact cleanup

Shipped item `142.070-T` carried no `source_stash_id` or `source_deliberation_id`. No stash or deliberation was
archived; no `linked_deliberation_disposition` entry applies to this close.

## Compaction (P-020)

Compaction was applied with `target: all` following the compact-context skill's output layout: the shipment memory was
consolidated into `docs/memory/compacted/2026-10-28-158-s-compacted.md` and the original placed in
`docs/archive/memory/`. Only the just-closed unit was a candidate. Compaction status: `done`.

## Follow-ups

* No new stash entries. Stage should triage existing stash `47DAEF9D` (the cap is now implemented). Stage proceeds with `142.071-T` (NEW-3, Slot-12).
