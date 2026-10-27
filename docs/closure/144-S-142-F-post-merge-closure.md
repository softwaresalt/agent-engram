---
title: "144-S post-merge operational closure"
doc_type: closure
shipment_id: "144-S"
feature_id: "142-F"
mode: post-merge
date: 2026-10-27
author: ship
verdict: "READY — PR #418 merged and 144-S safe-closed (non-cascading; 142-F unchanged); pre-close evidence captured via autoharness shipment cascade-close --classify-only (exit 3, SAFE_CLOSE, engine VERIFIED) and passes the closure-evidence gate"
closure_status: "READY"
releasability: "READY_WITH_CONDITIONS"
compaction_status: "done"
close_path: "safe_close"
close_evidence: "docs/closure/evidence/144-S-142-F-close-evidence.json"
pr_number: 418
merge_commit: "1182373757b017398c5fba651a1af2494805b819"
head_commit_merged: "1a5d9d7777a6add10f29c5afaef95776b9795e38"
closure_pr_number: 419
closure_pr_merge_commit: "f2eeb39a9cab4c5b3de0806a25676f45f08f529b"
runtime_verification_report: null
follow_up_stash:
  - "26CD3785"
  - "871A8430"
blocking_stash: null
shipment_record_status: "archived (archived_status: done) — targeted manual safe-close; see .backlogit/archive/144-S.md"
pre_reconcile: ".backlogit/reconcile/144-S-pre-20261027T0140Z.md"
post_reconcile: ".backlogit/reconcile/144-S-post-20261027T0150Z.md"
---

## Summary

Shipment `144-S` delivered the single task `142.075-T` (PRE-1a): the new module
`engram::services::generations::candidate_build` with `ReadServerLayout`
(`#[non_exhaustive]`), `READ_SERVER_ACTIVATION_DEADLINE` (60 s), `expected_identity()`,
`read_server_layout(&str)` and `mint_generation_id`, plus the
`integration_candidate_generation_build` harness (8 tests). PR #418 merged with a merge
commit (P-009) at reviewed HEAD `1a5d9d77` and merge SHA `1182373757b017398c5fba651a1af2494805b819`.
The `142.061-T` (Slot-02b) items were not implemented.

## Dark-mode decisions and gates

* `DARK_MODE_ACTIVE`; merge pre-authorized, admin fallback not authorized and not used.
* Local adversarial review (3 personas) and a correctness re-review of the fix delta:
  `READY_WITH_FOLLOWUPS`, P0=0, P1=0. The P2 (`Path::exists` NotFound pre-check) was fixed.
* Gates at HEAD: `cargo fmt --check`, `cargo clippy -D warnings -D clippy::pedantic`,
  `cargo build --all-targets`, targeted test 8/8, `cargo dev-test` all PASS (one earlier
  daemon-startup TTL test timeout was flaky and passed in isolation and on re-run).
* CI `build` and `start-launcher-windows` green; `autoharness gate copilot-review 418` returned
  `SATISFIED` for the reviewed HEAD with zero unresolved threads. `DARK_MODE_MERGE_AUTHORIZED`.
* Startup checkpoint `checkpoint-20261007-205649.json` (143-S closure PR #416, already merged at
  `006af541`) was resolved and carried forward in the pre-claim commit.

## Safe-close and archive evidence

`142-F` is a shared covering feature outside the task-only manifest, so the cascade was not
used and `backlogit shipment ship` was never invoked (P-015). `close_path: safe_close`; no
`cascade-close --classify-only` evidence record was produced at the time (manual procedure as in 141-S/143-S).
It was captured afterwards by the closure-repair PR: `.backlogit/queue/144-S.md` was temporarily restored from
merge commit `1182373757b017398c5fba651a1af2494805b819`, `autoharness shipment cascade-close --classify-only`
was run (exit 3, `SAFE_CLOSE`, engine `VERIFIED`, `phase_written: pre_close`), and the final safe-closed
`.backlogit/` state was restored. `close_evidence` is
`docs/closure/evidence/144-S-142-F-close-evidence.json` and the closure-evidence gate passes. The earlier
BLOCKED status (stash `871A8430`) is superseded; that stash entry is obsolete and left for Stage to archive.

`142.075-T` was archived with `status: done` by `backlogit move --status done` before the merge
(`.backlogit/archive/142.075-T.md`). The shipment record was preserved in
`.backlogit/archive/144-S.md` (`status: archived`, `archived_status: done`,
`archived_from: .backlogit/queue/144-S.md`) and `.backlogit/queue/144-S.md` removed. `backlogit sync`
succeeded; `142-F.md` SHA-256 before and after is
`42FCA67C1407641C8F3CF4752B3431D978290B1342614A831DF463B572181770` with `status: active`.

| Field | Value |
|---|---|
| `ProposedAction` | Preserve the full shipment record in the archive, remove the source queue record, sync the index, verify the archive and covering-feature state. |
| `ActionRisk` | `destructive` — the queue record is deleted; its fields and body are preserved in the archive and the Git change is reversible. |
| `ActionResult` | `applied` |
| Approval path | `DARK_MODE_ACTIVE` activation record (operator pre-authorized the shipment's merges and closure). |

## Runtime verification

The change adds library code only; no runtime surface changed. Probes against the live workspace
daemon via the `engram` CLI: `daemon-status` overall green (PID 2304 untouched), `workspace-status`
bound to the workspace (430 code files, 4,604 functions, 8,750 edges), and `search read_server_layout`
indexes the new module and its tests. Releasability: `READY_WITH_CONDITIONS` (the module has no
caller until `142.061-T` and later slots land).

## Source artifact cleanup

Shipped item `142.075-T` carried no `source_stash_id` or `source_deliberation_id` for this scope
(checked at closure). No stash or deliberation archived; no `linked_deliberation_disposition`
applicable (manual close).

## Follow-ups

* Stash `871A8430`: obsolete (superseded by the evidence capture above); Stage to archive.
* Stash `26CD3785` (DEFERRED SCOPE EXPANSION): plan cites the wrong `lifecycle_policy.rs` path; plan
  assumes `NotFound` for a missing path (needed an explicit pre-check); `142.061-T` must adapt
  `mint_generation_id`'s `GenerationIdError` to `CandidateBuildError`.
* Advisory P3: let-chain in `read_server_layout` versus declared `rust-version = "1.85"` (toolchain is
  pinned newer).
* Unexplained flakiness: the earlier daemon-startup TTL test timeout (passed in isolation and on re-run).
