---
title: "159-S post-merge operational closure"
doc_type: closure
shipment_id: "159-S"
feature_id: "142-F"
mode: post-merge
date: 2026-10-28
author: ship
verdict: "READY (preflight entrypoint added to the engram-indexer binary; no launcher consumes it yet) - PR #449 merged and 159-S safe-closed (non-cascading; 142-F unchanged)"
closure_status: "READY"
releasability: "READY_WITH_CONDITIONS"
compaction_status: "done"
close_path: "safe_close"
close_evidence: "docs/closure/evidence/159-S-142-F-close-evidence.json"
pr_number: 449
merge_commit: "ef1df553dc293220052faf343ae5c9a285bf2672"
head_commit_merged: "778ebf0d24527440040eef7f4e16eb8e1af8684e"
closure_pr_number: null
closure_pr_merge_commit: null
runtime_verification_report: null
follow_up_stash: ["9933B77D"]
blocking_stash: null
shipment_record_status: "archived (archived_status: shipped) via backlogit shipment reconcile-shipped"
pre_reconcile: ".backlogit/reconcile/159-S-pre-20261028T2001Z.md"
post_reconcile: ".backlogit/reconcile/159-S-post-20261028T2003Z.md"
---

## Summary

Shipment `159-S` delivered the single task `142.071-T` (NEW-3): `crates/engram-indexer/src/main.rs` is now a sync
`fn main() -> ExitCode`. When the first argument is `preflight`, it calls `preflight_invocation::main_with` with a
`production_factory` (read-only `read_server_layout`, `mint_generation_id`, `ProductionVerifier::new`), prints the
verdict plus a newline, writes the single stderr line and returns the exit code, all before any tokio runtime exists.
Every other invocation builds a current-thread runtime explicitly and runs the unchanged legacy env path. PR #449
merged with a merge commit (P-009) at head `778ebf0d`, merge SHA `ef1df553dc293220052faf343ae5c9a285bf2672`.

## Dark-mode decisions and gates

* `DARK_MODE_ACTIVE`; merge pre-authorized, admin fallback not authorized and not used (`DARK_MODE_MERGE_AUTHORIZED`).
* Claim: predecessors `158-S` and `156-S` were archived `shipped`; `pipeline-topology` pre_claim and post_claim exited 0
  and `backlogit shipment claim 159-S` succeeded natively.
* Harness first: three crate-local tests; scenarios 1 and 2 failed RED against the `Worker: 142.071-T` placeholder and
  scenario 3 passed as characterization. The implementation turned all 3 GREEN; a review fix made output writes
  tolerant of a closed pipe. Task archived in `778ebf0d`.
* Local gates: `cargo test -p engram-indexer --test preflight_entrypoint_test` (3/3), `cargo test -p engram-indexer
  --all-targets --no-fail-fast`, both pedantic clippy runs, `cargo fmt --all -- --check`, `cargo build -p engram-indexer`,
  `cargo build --all-targets` PASS. `cargo dev-test` passes every target except the known load-sensitive
  `backlog_index_100_items_under_5_seconds` (passes in isolation; existing stash `1346BC60` and `41DB6AD0`, cited, not
  duplicated; the root crate is untouched by this shipment). PR `build` and `start-launcher-windows` passed on the merged head.
* Local multi-persona review (Correctness, Rust, Security, Architecture in parallel): READY_WITH_FOLLOWUPS, no P0/P1.
  The P2 (`println!`/`eprint!` panic on a closed pipe) was fixed in scope. Out-of-scope P3 findings in
  `preflight_invocation.rs` (a distinct `SetupError` variant, canonical or rejected relative workspace) were captured as
  one P-021 C2 entry, stash `9933B77D`, before any further action.
* Copilot review `5688834779` ("no open issues"): no threads and no review-body findings.
  `autoharness gate copilot-review 449` returned `SATISFIED: PASS` before and at merge.

## Safe-close and archive evidence

* Pre-reconcile report `.backlogit/reconcile/159-S-pre-20261028T2001Z.md` was written while
  `.backlogit/queue/159-S.md` was still `active` in the queue.
* `autoharness shipment cascade-close --classify-only --shipment 159-S --feature 142-F --sha
  ef1df553dc293220052faf343ae5c9a285bf2672 --json` exited 3: `SAFE_CLOSE`, no mutation, engine verdict `UNVERIFIED`.
  Evidence `docs/closure/evidence/159-S-142-F-close-evidence.json` was written after the pre-report.
* `.backlogit/archive/142.071-T.md` carries `status: archived`, `archived_status: done`,
  `archived_from: .backlogit/queue/142.071-T.md` (`backlogit move --status done` and `backlogit archive`).
* `backlogit archive 159-S` moved the record to `.backlogit/archive/159-S.md`; then
  `backlogit shipment reconcile-shipped 159-S` set `archived_status: shipped`. `backlogit shipment ship` was not
  invoked and no file was deleted by hand.
* Covering feature `142-F` is `status: active` with SHA-256
  `42FCA67C1407641C8F3CF4752B3431D978290B1342614A831DF463B572181770` before and after close.
* Post-reconcile report: `.backlogit/reconcile/159-S-post-20261028T2003Z.md`.

## 159-S

Delivery: PR #449 (feature), commit ef1df553dc293220052faf343ae5c9a285bf2672 (merge commit, P-009).

## Procedure deviation record (operator-directed)

The shipment-reconcile skill's non-cascading sequence moves the shipment to `shipped` and verifies live `status: shipped` before `backlogit archive`. This close instead followed the sequence the operator prescribed in the 159-S dark-mode activation contract (the same sequence approved and merged for 154-S to 158-S): `backlogit archive 159-S` (`archived_status: active`) and then the official `backlogit shipment reconcile-shipped` with `--closure-evidence` and `--confirm`, which records a `shipment_reconciled_shipped` event bound to merge SHA `ef1df553dc293220052faf343ae5c9a285bf2672` and ends at `archived_status: shipped`. Approval source: operator activation contract (`merge_approval_pre_authorized: true`, explicit closure procedure).

## Runtime verification

Not applicable. The new entrypoint is not yet consumed by the launcher or any release artifact, so no runtime surface is exercised beyond the crate-local binary tests.

## Monitoring and rollback handoff

This slice is not a release and is not tagged or published on its own. The handoff applies to the first release whose
launcher invokes `engram-indexer preflight` (142.072-T and later).

| Evidence | Value |
|---|---|
| SLI | Fraction of `engram-indexer preflight` invocations whose stdout, exit code and single stderr line match the contract: usage errors exit 2 with empty stdout, setup errors and deadline failures print `{"state":"Failed","stage":"Build"}` and exit 1, and the stderr line never contains the workspace path or error text. |
| Observation query | `cargo test -p engram-indexer --all-targets --no-fail-fast` (the crate-local tests are not run by `cargo dev-test`), and the launcher's preflight stderr line once wired. |
| Baseline | 3/3 `preflight_entrypoint_test` cases pass; no-argument invocation still fails with `missing required environment variable`; the preflight path writes nothing to the workspace before the verifier runs. |
| Alert threshold | Any `preflight_entrypoint_test` failure on `main`, a legacy-path behavior change, or a stderr line outside the documented grammar. |
| Owner | Release owner of the next tagged release; Stage owns follow-up triage. |
| Observation window | The first release that invokes the preflight entrypoint, plus one daemon restart. |
| Rollback trigger | Any alert-threshold condition that persists after one retry. |
| Rollback procedure | Revert only the code from PR #449 (merge `ef1df553dc293220052faf343ae5c9a285bf2672`): restore `crates/engram-indexer/src/main.rs` to its prior async `#[tokio::main]` form and remove `crates/engram-indexer/tests/preflight_entrypoint_test.rs`. Do not revert the whole merge: that would also undo the `142.071-T` archival and conflict with the closed `159-S` shipment record. Preserve backlog state and closure evidence. Rebuild with `cargo build -p engram-indexer`. |

## Source artifact cleanup

Shipped item `142.071-T` carried no `source_stash_id` or `source_deliberation_id`. No stash or deliberation was
archived; no `linked_deliberation_disposition` entry applies to this close.

## Compaction (P-020)

Compaction was applied with `target: all` following the compact-context skill's output layout: the shipment memory was
consolidated into `docs/memory/compacted/2026-10-28-159-s-compacted.md` and the original placed in
`docs/archive/memory/`. Only the just-closed unit was a candidate. Compaction status: `done`.

## Follow-ups

* Stash `9933B77D` (P-021 deferred scope expansion): distinct `SetupError` variant and relative-workspace handling in `preflight_invocation.rs`; Stage triages.
* Stage proceeds with `142.072-T` (NEW-4) and `142.073-T` (NEW-5, Slot-14).
