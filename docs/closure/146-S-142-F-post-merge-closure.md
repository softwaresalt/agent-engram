---
title: "146-S post-merge operational closure"
doc_type: closure
shipment_id: "146-S"
feature_id: "142-F"
mode: post-merge
date: 2026-10-27
author: ship
verdict: "READY (runtime probes unverified, tracked as stash 6B3DD712) — PR #423 merged and 146-S safe-closed (non-cascading; 142-F unchanged); pre-close evidence captured via autoharness shipment cascade-close --classify-only (exit 3, SAFE_CLOSE, engine VERIFIED) before the queue record was removed"
closure_status: "READY"
releasability: "READY_WITH_CONDITIONS"
compaction_status: "done"
close_path: "safe_close"
close_evidence: "docs/closure/evidence/146-S-142-F-close-evidence.json"
pr_number: 423
merge_commit: "878c5eedb4c15f29e50cf44840aed30e15991c06"
head_commit_merged: "e4f97ba0dedd828d82d1f4ae48e8d63f9167e74b"
closure_pr_number: null
closure_pr_merge_commit: null
runtime_verification_report: null
follow_up_stash:
  - "6B3DD712"
blocking_stash: null
shipment_record_status: "archived (archived_status: done) — targeted manual safe-close; see .backlogit/archive/146-S.md"
pre_reconcile: ".backlogit/reconcile/146-S-pre-20261027T0510Z.md"
post_reconcile: ".backlogit/reconcile/146-S-post-20261027T0511Z.md"
---

## Summary

Shipment `146-S` delivered the single task `142.062-T` (PRE-2): `open_generation_store`,
`current_published_revision`, `publish_sealed_candidate` and the private `within_root`
comparator in `src/services/generations/candidate_build.rs`, with `publish.rs` changed only
in the visibility of `read_current_manifest_revision`, and the new target
`integration_candidate_generation_publish` (3 scenarios). PR #423 merged with a merge commit
(P-009) at reviewed HEAD `e4f97ba0` and merge SHA `878c5eedb4c15f29e50cf44840aed30e15991c06`.

## Dark-mode decisions and gates

* `DARK_MODE_ACTIVE`; merge pre-authorized, admin fallback not authorized and not used.
* Harness first (`42d1eb5f`): all 3 scenarios failed RED with the `Worker: 142.062-T` marker
  and `cargo check --all-targets` passed; implementation `c5e175d4` turned them GREEN.
* Local review: 3-persona adversarial review `READY_WITH_FOLLOWUPS`, P0=P1=P2=0; P3 notes
  accepted. The Copilot-fix delta was re-reviewed directly (44 lines): no new findings.
* Gates: `cargo fmt --check`, pedantic clippy, `cargo build --all-targets`, `cargo dev-test`
  PASS (one load-related `hcl_indexing_test` cold-start flake passed on rerun);
  `integration_generation_publish` 8/8 and `integration_candidate_generation_build` 10/10.
* CI `build` and `start-launcher-windows` green. Copilot review `5662144484` raised one
  high-severity finding (directory created outside the workspace before the containment check).
  Classified in scope under P-021 C1 (completes the authorized containment contract), fixed in
  `e4f97ba0` with a regression, thread replied and resolved, and dispositioned with
  `Copilot-Review-Body-Disposition: 5662144484`. The follow-up Copilot review returned
  "Approval recommended"; `autoharness gate copilot-review 423` returned `SATISFIED: PASS`
  for the reviewed HEAD. No P-021 deferred scope expansion was captured.
  `DARK_MODE_MERGE_AUTHORIZED`.

## Safe-close and archive evidence

`142-F` is a shared covering feature outside the task-only manifest, so the cascade was not used
and `backlogit shipment ship` was never invoked (P-015). The pre-close evidence record was written
by `autoharness shipment cascade-close --classify-only` while `.backlogit/queue/146-S.md` was still
`status: active` (exit 3, `SAFE_CLOSE`, engine `VERIFIED`, `phase_written: pre_close`), before the
manual close. `close_path: safe_close`.

`142.062-T` was moved to `done` on the feature branch, which left it archive-resident with
`status: done` and no `archived_status`. Copilot review of the closure PR caught that this was not a
completed archive; `backlogit archive 142.062-T` was then run on the closure branch and
`.backlogit/archive/142.062-T.md` now has `status: archived`, `archived_status: done`,
`archived_from: .backlogit/queue/142.062-T.md`.

Observed chronology (UTC, from file and evidence timestamps; the first authored timestamps were
placeholders and were corrected from execution evidence): pre-mode report 05:10:59, machine-written
pre-close evidence snapshot 05:11:10 (`captured_at`), shipment archive record 05:11:28, post-mode
report 05:11:55, task archive completion 05:25. The pre-close evidence therefore preceded the
close. The generated evidence snapshot was not altered. The shipment record was preserved in
`.backlogit/archive/146-S.md` (`status: archived`, `archived_status: done`,
`archived_from: .backlogit/queue/146-S.md`) and `.backlogit/queue/146-S.md` removed.
`backlogit sync` succeeded; `142-F.md` SHA-256 before and after is
`42FCA67C1407641C8F3CF4752B3431D978290B1342614A831DF463B572181770` with `status: active`.

| Field | Value |
|---|---|
| `ProposedAction` | Preserve the full shipment record in the archive, remove the source queue record, sync the index, verify the archive and covering-feature state. |
| `ActionRisk` | `destructive` — the queue record is deleted; its fields and body are preserved in the archive and the Git change is reversible. |
| `ActionResult` | `applied` |
| Approval path | `DARK_MODE_ACTIVE` activation record (operator pre-authorized the shipment's merges and closure). |

## Runtime verification

The change adds library code only; no runtime surface changed. After re-latching readiness with
`set_workspace` over the named pipe, `_health` still reported `starting` for 60 s and the
`engram` CLI probes (`daemon-status`, `workspace-status`, `search`) failed with
"Daemon failed to reach Ready state within 30000ms". Daemon PID 2304 was left untouched per
operator directive. This is a blocked prerequisite, recorded rather than faked. Releasability:
`READY_WITH_CONDITIONS` (no caller exists until later PRE slots land; live probes unverified).

## Source artifact cleanup

Shipped item `142.062-T` carried no `source_stash_id` or `source_deliberation_id` (checked at
closure). No stash or deliberation archived; no `linked_deliberation_disposition` applicable
(manual close).

## Compaction (P-020)

`compact-context` ran with `target: all`: the shipment's memory
`docs/memory/2026-10-26-ship-146-s-pre-2-implementation.md` was consolidated into
`docs/memory/compacted/2026-10-27-146-s-compacted.md` and the original moved to
`docs/archive/memory/`. Compaction status: `done`.

## Follow-ups

* Stash `6B3DD712`: re-run the `engram` CLI runtime probes once the daemon is Ready, and confirm
  the junction scenarios execute on Windows CI.
* `closure_pr_number` and `closure_pr_merge_commit` are `null` in this record because it ships
  in the closure PR itself; they are recorded in the session summary.

## 146-S

PR #423 (feature), commit 878c5eedb4c15f29e50cf44840aed30e15991c06
