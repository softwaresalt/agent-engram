---
title: "160-S post-merge operational closure"
doc_type: closure
shipment_id: "160-S"
feature_id: "142-F"
mode: post-merge
date: 2026-10-28
author: ship
verdict: "READY (root preflight relay library added; not yet wired into the CLI) - PR #451 merged and 160-S safe-closed (non-cascading; 142-F unchanged)"
closure_status: "READY"
releasability: "READY_WITH_CONDITIONS"
compaction_status: "done"
close_path: "safe_close"
close_evidence: "docs/closure/evidence/160-S-142-F-close-evidence.json"
pr_number: 451
merge_commit: "986058c6a57a400015a1498c749cb67e908838c5"
head_commit_merged: "943d7da911541e7ef24529b03e54f78e6bf81a1a"
closure_pr_number: null
closure_pr_merge_commit: null
runtime_verification_report: null
follow_up_stash: ["AE9A0B42"]
blocking_stash: null
shipment_record_status: "archived (archived_status: shipped) via backlogit shipment reconcile-shipped"
pre_reconcile: ".backlogit/reconcile/160-S-pre-20261028T2227Z.md"
post_reconcile: ".backlogit/reconcile/160-S-post-20261028T2240Z.md"
---

## Summary

Shipment `160-S` delivered the single task `142.072-T` (NEW-4): `src/cli/commands/preflight.rs`, the root preflight
relay library. It resolves the sibling `engram-indexer` next to the canonical `current_exe()` (no PATH, environment or
fixed-directory lookup), runs `preflight --workspace <W> --timeout-ms <T> --engram <E>` under bounded child I/O
(`T + 2000 ms` deadline via `checked_add`, 4 KiB output cap, `try_wait` loop keyed on the supervisor's exit, 250 ms
post-exit drain, detached reader that is never joined), normalizes the output to exactly one verdict line, and emits one
`engram-preflight-relay stage=<Stage|none> cause=<cause> elapsed_ms=<n>` stderr line (`cause=missing_sibling` appends the constant `sibling=engram-indexer<EXE_SUFFIX>`). `RELAY_STAGES` is guarded against
the indexer's `stage_name` by `preflight_relay_stages_test`. `run_preflight` is not yet called by the CLI (that is
`142.073-T`). PR #451 merged with a merge commit (P-009), merge SHA `986058c6a57a400015a1498c749cb67e908838c5`.

## Dark-mode decisions and gates

* `DARK_MODE_ACTIVE`; merge pre-authorized, admin fallback not used.
* Resume: the claimed shipment and task were not re-claimed. The Step 2 harness commit (614 insertions, over the
  SB-1(c) budget because of verbose formatting) was compacted to 398 insertions keeping all 7 contract and 2 stage
  tests; both suites failed RED with `Worker: 142.072-T`, then passed GREEN (7/7 and 2/2) after the implementation.
* Local gates: both pedantic clippy runs, `cargo fmt --all -- --check`, `cargo build --all-targets`,
  `cargo test -p engram-indexer --all-targets --no-fail-fast` PASS. `cargo dev-test` passes every target except the
  known load-sensitive `backlog_index_100_items_under_5_seconds` (passes in isolation; existing stash `1346BC60` and
  `41DB6AD0`, cited, not duplicated). PR `build` and `start-launcher-windows` passed on the merged head.
* Local multi-persona review (Correctness, Rust, Security, Architecture in parallel): READY_WITH_FOLLOWUPS, no P0/P1.
  In-scope P2/P3 findings fixed: retry of `ErrorKind::Interrupted` reads and single-line escaped diagnostics. The
  out-of-scope finding (grandchildren survive the supervisor kill) was captured as one P-021 C2 entry, stash
  `AE9A0B42`, before any further action.
* Copilot review: two medium threads on the escaped diagnostics (4 KiB cap after escaping; capped echo on oversize
  output). Both were fixed in scope at `943d7da9` with a regression test, answered with the fixing commit and resolved.
  `autoharness gate copilot-review 451` returned `SATISFIED: PASS` for the merged head.

## Safe-close and archive evidence

* Pre-reconcile report `.backlogit/reconcile/160-S-pre-20261028T2227Z.md` was written while
  `.backlogit/queue/160-S.md` was still `active` in the queue.
* `autoharness shipment cascade-close --classify-only --shipment 160-S --feature 142-F --sha
  986058c6a57a400015a1498c749cb67e908838c5 --json` exited 3: `SAFE_CLOSE`, no mutation, engine verdict `UNVERIFIED`.
  Evidence `docs/closure/evidence/160-S-142-F-close-evidence.json` was written after the pre-report.
* `.backlogit/archive/142.072-T.md` carries `status: archived`, `archived_status: done`,
  `archived_from: .backlogit/queue/142.072-T.md` (`backlogit move --status done` and `backlogit archive`).
* `backlogit archive 160-S` moved the record to `.backlogit/archive/160-S.md`; then
  `backlogit shipment reconcile-shipped 160-S` set `archived_status: shipped`. `backlogit shipment ship` was not
  invoked and no file was deleted by hand.
* Covering feature `142-F` is `status: active` with SHA-256
  `42FCA67C1407641C8F3CF4752B3431D978290B1342614A831DF463B572181770` before and after close.
* The `shipment_reconciled_shipped` event attests this closure document as committed with the `reconcile-shipped`
  call; it was reviewed before that call so the attested hash matches the final content.
* Post-reconcile report: `.backlogit/reconcile/160-S-post-20261028T2240Z.md`.

## 160-S

Delivery: PR #451 (feature), commit 986058c6a57a400015a1498c749cb67e908838c5 (merge commit, P-009).

## Procedure deviation record (operator-directed)

The shipment-reconcile skill's non-cascading sequence moves the shipment to `shipped` and verifies live `status: shipped`
before `backlogit archive`. This close instead followed the sequence the operator prescribed in the 160-S dark-mode
resume instructions (the same sequence used for 154-S to 159-S): `backlogit archive 160-S` (`archived_status: active`)
and then the official `backlogit shipment reconcile-shipped` with `--closure-evidence` and `--confirm`, which records a
`shipment_reconciled_shipped` event bound to merge SHA `986058c6a57a400015a1498c749cb67e908838c5` and ends at
`archived_status: shipped`. Approval source: operator activation contract and resume instructions.

## Runtime verification

Not applicable. The relay library is not yet consumed by the launcher or any release artifact, so no runtime surface is
exercised beyond the root contract tests that run compiled fixture supervisors.

## Monitoring and rollback handoff

This slice is not a release and is not tagged or published on its own. The handoff applies to the first release whose
CLI invokes `run_preflight` (`142.073-T` and later).

| Evidence | Value |
|---|---|
| SLI | Fraction of relay runs that print exactly one verdict line from the closed set, exit 0 only for `Succeeded`, otherwise 1, and emit one `engram-preflight-relay` stderr line whose cause is in the closed set; a hanging or oversize supervisor is killed within `T + 2000 ms`. |
| Observation query | `cargo test --test contract_cli_preflight_relay`, `cargo test -p engram-indexer --test preflight_relay_stages_test`, and the relay stderr line once wired. |
| Baseline | 7/7 contract tests and 2/2 stage tests pass; the module contains no `read_to_end`, `read_to_string`, `wait_with_output`, `.output()` or reader `join()` on the supervisor's stdout. |
| Alert threshold | Any relay test failure on `main`, a stage-set drift between `RELAY_STAGES` and `stage_name`, or a stderr line outside the documented grammar. |
| Owner | Release owner of the next tagged release; Stage owns follow-up triage. |
| Observation window | The first release that invokes the relay, plus one daemon restart. |
| Rollback trigger | Any alert-threshold condition that persists after one retry. |
| Rollback procedure | Before any consumer invokes `run_preflight`, revert only the code from PR #451 (merge `986058c6a57a400015a1498c749cb67e908838c5`): remove `src/cli/commands/preflight.rs`, its `pub mod preflight;` line in `src/cli/commands/mod.rs`, the `[[test]] name = "contract_cli_preflight_relay"` entry in `Cargo.toml`, `tests/contract/cli_preflight_relay_test.rs` and `crates/engram-indexer/tests/preflight_relay_stages_test.rs`. Rebuild with `cargo build --all-targets`. Do not revert the whole merge: that would also undo the `142.072-T` archival and conflict with the closed `160-S` shipment record. Preserve backlog state and closure evidence. Once the CLI consumes `run_preflight`, roll back that consumer together with these files in one coordinated change. |

## Source artifact cleanup

Shipped item `142.072-T` carried no `source_stash_id` or `source_deliberation_id`. No stash or deliberation was
archived; no `linked_deliberation_disposition` entry applies to this close.

## Compaction (P-020)

Compaction was applied with `target: all`: the shipment memory was consolidated into
`docs/memory/compacted/2026-10-28-160-s-compacted.md` and the original placed in
`docs/archive/memory/2026-10-28-ship-160-s-slot-13-memory.md`. Only the just-closed unit was a candidate.
Compaction status: `done`.

## Follow-ups

* Stash `AE9A0B42` (P-021 deferred scope expansion): grandchildren of the supervisor survive a relay kill (process-group or Job Object design); Stage triages.
* Stage proceeds with `142.073-T` (NEW-5, Slot-14), which wires `run_preflight` into the CLI.
