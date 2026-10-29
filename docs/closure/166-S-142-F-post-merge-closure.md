---
title: "166-S post-merge operational closure"
doc_type: closure
shipment_id: "166-S"
feature_id: "142-F"
mode: post-merge
date: 2026-10-29
author: ship
verdict: "READY (activation-settle barrier and positive witness target merged) - PR #464 merged and 166-S safe-closed (non-cascading; 142-F unchanged)"
closure_status: "READY"
releasability: "READY_WITH_CONDITIONS"
compaction_status: "done"
close_path: "safe_close"
close_evidence: "docs/closure/evidence/166-S-142-F-close-evidence.json"
pr_number: 464
merge_commit: "152651bafd7a4ac0dd316fc9dbcf7f8d062d11ca"
head_commit_merged: "b6a82e5e31f71c1743521bc48a9370ce225dd85d"
closure_pr_number: null
closure_pr_merge_commit: null
runtime_verification_report: null
follow_up_stash: []
blocking_stash: null
shipment_record_status: "archived (archived_status: shipped) via backlogit shipment reconcile-shipped"
pre_reconcile: ".backlogit/reconcile/166-S-pre-20261029T0804Z.md"
post_reconcile: ".backlogit/reconcile/166-S-post-20261029T0830Z.md"
---

## Summary

Shipment `166-S` (Slot-19, the final slot of the 142-F DAG queue) delivered the single task `142.063-T` (PRE-3F, route
`(c)`). It adds the shared support module `tests/helpers/activation_settle.rs` (six pinned public items, included by
`#[path]`, not registered in `tests/helpers/mod.rs`), the new target `integration_read_server_activation_settle`
(`tests/integration/read_server_activation_settle_test.rs`) with its own fixture and the positive test
`published_fixture_generation_activates_at_startup_and_quiesces`, and one `[[test]]` stanza in `Cargo.toml`. The barrier
polls `get_workspace_status` every 25 ms for up to `SETTLE_TIMEOUT` (75 s), returns `NoActivator` on a null generation
block, and returns `Active` only after two observations 250 ms apart with equal revisions, equal filesystem snapshots
and equal binding fingerprints, never returning `Ok` after the timeout. No production file changed and
`tests/contract/read_server_cli_mcp_parity_test.rs` is unchanged from `main`. PR #464 merged with a merge commit
(P-009), merge SHA `152651bafd7a4ac0dd316fc9dbcf7f8d062d11ca`.

## Dark-mode decisions and gates

* `DARK_MODE_ACTIVE`; merge pre-authorized, admin fallback not used (not authorized, not needed).
* Phase A (before the claim): repair PR #463 (merge `72501dadf547d6e3be2ce9f054731061698df712`) added the canonical
  `docs/closure/143-S-142-F-post-merge-closure.md` (the legacy date-named file stays in place), appended per-shipment
  sections to the 143-S, 146-S and 151-S closure files, and moved the three archives from `archived_status: done` to
  `shipped` through `backlogit shipment reconcile-shipped`. The `archived_status: done` to `active` step was a single
  operator-authorized manual frontmatter edit run immediately before each reconcile call (reconcile requires `active`);
  the Copilot thread that asked about it was answered and resolved. After the merge,
  `autoharness gate pipeline-topology --phase pre_claim` exited 0 for 166-S, the claim succeeded and `post_claim` exited 0.
* Harness commit `0679e064` (HC): 3 files, 400 insertions, at the `SB-1(c)` cap. Red phase: the single test failed at
  step (b) with `PRE3F-RED: (b) activation did not settle: activation-settle barrier not implemented (PRE-3F);
  endpoint=...` after step (a) passed. The first HC carried a step (c) assertion that could never hold (the
  `generation` status block has no ID string); it was found while writing the IC, fixed and amended into HC before
  anything was pushed.
* Implementation commit `c953551b` (IC): `git diff 0679e064 c953551b` touches only `tests/helpers/activation_settle.rs`.
  Post-IC test hardening (`5de0b302`, `b6a82e5e`, merged HEAD) is separate and touches only the target file.
* Local gates, all PASS (fmt, clippy, build, oracle and the three target runs on the final tree; the full suite on the mid-review HEAD): `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings -D
  clippy::pedantic`, `cargo build --all-targets`, `pwsh scripts/test-coverage-oracle.ps1 --mode completeness`
  (`STATUS=PASS`), three consecutive target runs GREEN (each with one `F54 settle: Active after ~400-450 ms` line and no
  `PRE3F-BLOCK:` or `WARNING:` line), and `cargo dev-test --no-fail-fast` (0 failures on the mid-review HEAD). Two unrelated
  timing tests failed once under load in full runs and passed in isolation and on rerun
  (`services::metrics::tests::full_channel_branch_switch_is_acknowledged_before_following_event`,
  `hcl_indexing_test::cold_start_lists_and_maps_all_three_hcl_aliases`). Hosted `build` and `start-launcher-windows`
  passed on the merged HEAD (run 40838568830).
* Local multi-persona review (Correctness, Rust, Security, Architecture, in parallel): no P0. Fixed in scope: child
  ownership in `stop_daemon`, snapshot errors re-poll instead of aborting. The Security reviewer could not run git and
  returned an incomplete review; it is recorded as incomplete, not as a pass. READY_WITH_FOLLOWUPS.
* Copilot review: five in-scope threads over two rounds (late `Ok` after timeout, Windows lock errors 32 and 33,
  `None == None` quiescence, step (c) contents assertions, bounded IPC retries). Each was fixed, replied to and resolved;
  `autoharness gate copilot-review 464` returned `SATISFIED: PASS` on `b6a82e5e`.
* Declined with rationale (not a scope expansion, so no P-021 capture): `required-features` on the spec-pinned
  `[[test]]` stanza (matches sibling targets), `spawn_blocking` for snapshots (brief, by design no concurrent IPC), and
  `String` error flattening in test helpers (P3).

## Safe-close and archive evidence

* Pre-reconcile report `.backlogit/reconcile/166-S-pre-20261029T0804Z.md` was written while
  `.backlogit/queue/166-S.md` was still `active` in the queue.
* `autoharness shipment cascade-close --classify-only --shipment 166-S --feature 142-F --sha
  152651bafd7a4ac0dd316fc9dbcf7f8d062d11ca --json` exited 3: `SAFE_CLOSE`, no mutation, engine verdict `VERIFIED`.
  Evidence `docs/closure/evidence/166-S-142-F-close-evidence.json`.
* `.backlogit/archive/142.063-T.md` carries `status: archived`, `archived_status: done`,
  `archived_from: .backlogit/queue/142.063-T.md` (`backlogit move --status done` and `backlogit archive`, run on the PR
  branch before the merge).
* `backlogit archive 166-S` moved the record to `.backlogit/archive/166-S.md`; then
  `backlogit shipment reconcile-shipped 166-S` sets `archived_status: shipped`. `backlogit shipment ship` was not
  invoked and no file was deleted by hand.
* Covering feature `142-F` is `status: active` with SHA-256
  `42FCA67C1407641C8F3CF4752B3431D978290B1342614A831DF463B572181770` before and after close.
* This closure document was reviewed and finalized before the `reconcile-shipped` call, which attests it as committed
  with that call; it is not edited afterwards.
* Post-reconcile report: `.backlogit/reconcile/166-S-post-20261029T0830Z.md`.

## 166-S

Delivery: PR #464 (feature), commit 152651bafd7a4ac0dd316fc9dbcf7f8d062d11ca (merge commit, P-009).

## Procedure deviation record (operator-directed)

The shipment-reconcile skill's non-cascading sequence moves the shipment to `shipped` before `backlogit archive`. This
close instead followed the sequence the operator prescribed for this dark-mode run (the same as 154-S to 165-S):
`backlogit archive 166-S` (`archived_status: active`) and then the official `backlogit shipment reconcile-shipped` with
`--closure-evidence` and `--confirm`, which records a `shipment_reconciled_shipped` event bound to merge SHA
`152651bafd7a4ac0dd316fc9dbcf7f8d062d11ca` and ends at `archived_status: shipped`. Approval source: operator
activation contract.

## Runtime verification

The new target spawns the real `engram` daemon binary against a temporary workspace with a pre-published generation,
and ran GREEN locally on Windows three times and on the hosted `build` job through the full suite. The Linux and macOS
behavior of the barrier is covered only by hosted CI; no live-workspace run was performed.

## Monitoring and rollback handoff

This slice is test-only and is not a release; it is not tagged or published on its own.

| Evidence | Value |
|---|---|
| SLI | `integration_read_server_activation_settle` passes with one `F54 settle: Active` line and no `PRE3F-BLOCK:` or `WARNING:` line. |
| Observation query | `cargo test --test integration_read_server_activation_settle -- --nocapture` locally, and the `build` job on `main` (`gh run list --workflow ci.yml --branch main`). |
| Baseline | Three consecutive local runs GREEN (settle after about 400 to 450 ms, 6 to 10 s wall clock); `build` passed on the merged HEAD `b6a82e5e` (run 40838568830). |
| Alert threshold | One red run of the target on `main` that persists after one retry, or any `SETTLE_TIMEOUT` expiry (75 s) in CI. |
| Owner | Repository maintainer acting as release owner of the next tagged release (softwaresalt); Stage owns any new 142-F slot planning that consumes the barrier. |
| Observation window | From the merge of PR #464 until the first downstream task (`142.058-T` F54 or a PA-5 task) includes the support module, plus one CI cycle on `main` after it. |
| Rollback trigger | A persistent red or flaky target on `main` that blocks unrelated work. |
| Rollback procedure | Revert only the three PR #464 paths (`tests/helpers/activation_settle.rs`, `tests/integration/read_server_activation_settle_test.rs`, the `Cargo.toml` stanza) on a revert branch, then run `cargo check --all-targets` and open a PR. Do not revert the whole merge: that would undo the `142.063-T` archival and conflict with the closed `166-S` shipment record. Preserve backlog state and closure evidence. |

Conditions for `READY_WITH_CONDITIONS`: the observation window above completes, and the downstream consumers of the
support module build without a dead-code lint.

## Source artifact cleanup

Shipped item `142.063-T` carried no `source_stash_id` or `source_deliberation_id` custom field. No stash or
deliberation was archived; no `linked_deliberation_disposition` entry applies.

## Compaction (P-020)

Compaction was applied with `target: all`: the shipment memory was consolidated into
`docs/memory/compacted/2026-10-29-166-s-compacted.md` and the original placed in
`docs/archive/memory/2026-10-29-ship-166-s-142-063-t-pre-3f.md`. Only the just-closed unit was a candidate.
Compaction status: `done`.

## Follow-ups

* No P-021 deferred-scope-expansion stash was created in this shipment.
* This is the last slot of the 23-shipment 142-F DAG queue; `142-F` remains active and byte-identical, and Stage plans
  the PA-5 and `142.058-T` (F54) work that consumes the barrier.
