---
title: "155-S post-merge operational closure"
doc_type: closure
shipment_id: "155-S"
feature_id: "142-F"
mode: post-merge
date: 2026-10-28
author: ship
verdict: "READY (library stages only; no runtime surface consumes them yet) - PR #441 merged and 155-S safe-closed (non-cascading; 142-F unchanged)"
closure_status: "READY"
releasability: "READY_WITH_CONDITIONS"
compaction_status: "done"
close_path: "safe_close"
close_evidence: "docs/closure/evidence/155-S-142-F-close-evidence.json"
pr_number: 441
merge_commit: "9d1d0678908fd3f795c1f0bf36f3b6744e209151"
head_commit_merged: "3eedb4a9afb38dceccc4ac232a01a139d499b55d"
closure_pr_number: null
closure_pr_merge_commit: null
runtime_verification_report: null
follow_up_stash:
  - "16172B49"
  - "1AB175A8"
  - "3CF808CD"
  - "8ECFE00E"
  - "8D20C81B"
blocking_stash: null
shipment_record_status: "archived (archived_status: shipped) via backlogit shipment reconcile-shipped"
pre_reconcile: ".backlogit/reconcile/155-S-pre-20261028T1403Z.md"
post_reconcile: ".backlogit/reconcile/155-S-post-20261028T1406Z.md"
---

## Summary

Shipment `155-S` delivered the single task `142.079-T` (F50b): `ProductionVerifier` in
`crates/engram-indexer/src/preflight.rs`. Build runs `open_generation_store(&layout, true)`, `GenerationId::new(expected)`
and `build_candidate_generation`; Seal runs `seal_candidate_inventory`; Publish runs
`publish_sealed_candidate(.., "engram-preflight")`. The verifier owns a current-thread runtime and never nests, enforces
the earlier of the machine and constructor deadlines with typed `VerificationErrorKind::DeadlineExpired`, binds Seal and
Publish to the one expected generation, and leaves the published generation untouched on any failure. The four probe
stages stay unimplemented for Slot-09c (`142.054-T`). PR #441 merged with a merge commit (P-009) at head `3eedb4a9`,
merge SHA `9d1d0678908fd3f795c1f0bf36f3b6744e209151`.

## Dark-mode decisions and gates

* `DARK_MODE_ACTIVE`; merge pre-authorized, admin fallback not authorized and not used (`DARK_MODE_MERGE_AUTHORIZED`).
* Claim: predecessor 154-S was already `archived_status: shipped`; `pipeline-topology` pre_claim and post_claim exited 0
  and `backlogit shipment claim 155-S` succeeded natively.
* Harness first (`aa54e550`): 6 new tests failed RED against the stub while all F50a tests stayed GREEN; the
  implementation (`12df89a9`) turned them GREEN.
* Local gates: `cargo test --test integration_preflight_gate` (16/16), `cargo test -p engram-indexer --all-targets
  --no-fail-fast`, both pedantic clippy runs, `cargo fmt --all -- --check`, `cargo build -p engram-indexer` and
  `cargo build --all-targets` PASS. The full `cargo dev-test --no-fail-fast` run was green except
  `hcl_indexing_test`, `integration_daemon_startup_order` and `backlog_index_100_items_under_5_seconds`, which are flaky
  under load and passed when rerun alone. PR `build` and `start-launcher-windows` jobs passed on the merged head.
* Local multi-persona review (Correctness, Rust, Security, Architecture in parallel, plus a Correctness re-review of the
  final code): READY_WITH_FOLLOWUPS, no P0/P1. Fixed in `af2d32d8`: nested-runtime panic in `verify`, runtime drop inside
  async, Seal/Publish binding to the expected generation, `layout()` accessor removed. Out of scope (P-021 C1, captured
  before any reply): stash `16172B49`, `1AB175A8`, `3CF808CD`, `8ECFE00E`. No prior matching deferred entry was found.
* Copilot review `5683702311` raised one thread (database-backed fixtures need `cozo-backend`), fixed in `3eedb4a9`,
  replied, resolved. Review `5683886390` raised one thread (post-rename directory-sync failure reported as an ordinary
  publish failure), out of scope under P-021 C1/C2/C3: captured as stash `8D20C81B` before the reply, which cited it,
  then resolved. `Copilot-Review-Body-Disposition` was posted for `5683702311`, `5683773288` and `5683886390`.
  `autoharness gate copilot-review 441` returned `SATISFIED: PASS`.

## Safe-close and archive evidence

* Pre-reconcile report `.backlogit/reconcile/155-S-pre-20261028T1403Z.md` (generated 14:03:40Z) was written while
  `.backlogit/queue/155-S.md` was still `active` in the queue.
* `autoharness shipment cascade-close --classify-only --shipment 155-S --feature 142-F --sha
  9d1d0678908fd3f795c1f0bf36f3b6744e209151 --json` exited 3: `SAFE_CLOSE`, no mutation, engine verdict `UNVERIFIED`.
  Evidence `docs/closure/evidence/155-S-142-F-close-evidence.json` was written after the pre-report.
* `.backlogit/archive/142.079-T.md` carries `status: archived`, `archived_status: done`,
  `archived_from: .backlogit/queue/142.079-T.md` (`backlogit move --status done` and `backlogit archive`).
* `backlogit archive 155-S` moved the record to `.backlogit/archive/155-S.md`; then
  `backlogit shipment reconcile-shipped 155-S` set `archived_status: shipped`. `backlogit shipment ship` was not
  invoked and no file was deleted by hand.
* Covering feature `142-F` is `status: active` with SHA-256
  `42FCA67C1407641C8F3CF4752B3431D978290B1342614A831DF463B572181770` before and after close.
* Post-reconcile report: `.backlogit/reconcile/155-S-post-20261028T1406Z.md`.

## 155-S

Delivery: PR #441 (feature), commit 9d1d0678908fd3f795c1f0bf36f3b6744e209151 (merge commit, P-009).

## Runtime verification

Not applicable. The change adds library stages that no binary or launcher consumes yet, so no runtime surface changed.
Stash `6B3DD712` still tracks runtime probe evidence for later slots.

## Monitoring and rollback handoff

This slice is not a release and is not tagged or published on its own. The handoff applies to the first release whose
launcher preflight consumes `ProductionVerifier`.

| Evidence | Value |
|---|---|
| SLI | Fraction of preflight runs where Build, Seal and Publish succeed or fail typed within the shared deadline, with the previously published generation still serving after every failure. |
| Observation query | Preflight verdict output of the launcher once wired (142.069-T), and `cargo test --test integration_preflight_gate` in CI. |
| Baseline | 16/16 tests pass; no failed stage changes the active manifest. |
| Alert threshold | Any run where a failed or timed-out stage changes the published generation, or any `integration_preflight_gate` failure on `main`. |
| Owner | Release owner of the next tagged release; Stage owns triage of stash `16172B49`, `1AB175A8`, `3CF808CD`, `8ECFE00E` and `8D20C81B`. |
| Observation window | The first release that consumes the preflight, plus one daemon restart. |
| Rollback trigger | Any alert-threshold condition that persists after one retry. |
| Rollback procedure | Revert merge commit `9d1d0678908fd3f795c1f0bf36f3b6744e209151`; the verifier is unconsumed, so reverting removes only the stages, the `tokio` `time` feature on the indexer and the tests. |

## Source artifact cleanup

Shipped item `142.079-T` carried no `source_stash_id` or `source_deliberation_id`. No stash or deliberation was
archived; no `linked_deliberation_disposition` entry applies to this close.

## Compaction (P-020)

`compact-context` ran with `target: all`: the shipment memory was consolidated into
`docs/memory/compacted/2026-10-28-155-s-compacted.md` and the original moved to `docs/archive/memory/`. Only the
just-closed unit was a candidate. Compaction status: `done`.

## Follow-ups

* Stash `16172B49` (provisional medium): Windows trailing-dot and device-name generation IDs can alias `runtime`.
* Stash `1AB175A8` (provisional medium): `Preflight::verify` discards the `VerificationError` kind and reason.
* Stash `3CF808CD` (provisional medium): facade has no cooperative deadline or partial-candidate cleanup.
* Stash `8ECFE00E` (provisional low): `#[non_exhaustive]`, Cargo description, and F50a test timing margin.
* Stash `8D20C81B` (provisional medium): post-rename directory-sync failure reported as a plain publish failure.
* Next slot: `142.054-T` (Slot-09c), which closes F50.
