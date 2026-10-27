---
title: "147-S post-merge operational closure"
doc_type: closure
shipment_id: "147-S"
feature_id: "142-F"
mode: post-merge
date: 2026-10-27
author: ship
verdict: "READY (runtime probes unverified, tracked as stash 6B3DD712) - PR #425 merged and 147-S safe-closed (non-cascading; 142-F unchanged); pre-close evidence captured via autoharness shipment cascade-close --classify-only (exit 3, SAFE_CLOSE, engine VERIFIED) before the queue record was removed"
closure_status: "READY"
releasability: "READY_WITH_CONDITIONS"
compaction_status: "done"
close_path: "safe_close"
close_evidence: "docs/closure/evidence/147-S-142-F-close-evidence.json"
pr_number: 425
merge_commit: "a00a71bed94629eef44f478434083b77075b3070"
head_commit_merged: "8c26a88f5c6a9c3e54a3014c45bd51c00ed60591"
closure_pr_number: null
closure_pr_merge_commit: null
runtime_verification_report: null
follow_up_stash:
  - "6B3DD712"
blocking_stash: null
shipment_record_status: "archived (archived_status: done) - targeted manual safe-close; see .backlogit/archive/147-S.md"
pre_reconcile: ".backlogit/reconcile/147-S-pre-20261027T0630Z.md"
post_reconcile: ".backlogit/reconcile/147-S-post-20261027T0633Z.md"
---

## Summary

Shipment `147-S` delivered the single task `142.076-T` (PRE-3a): `run_read_server_startup`
in `src/daemon/lifecycle_policy.rs` now derives workspace path, branch and `workspace_id`
from exactly one `read_server_layout` call (REPLACE). The new target
`integration_read_server_generation_wiring` covers the identity equality and scenario 3.
PR #425 merged with a merge commit (P-009) at head `8c26a88f` (code reviewed at `9e73cce2`)
and merge SHA `a00a71bed94629eef44f478434083b77075b3070`.

## Dark-mode decisions and gates

* `DARK_MODE_ACTIVE`; merge pre-authorized, admin fallback not authorized and not used.
* RED baseline for the PRE-3 family recorded in the `142.076-T` task comments before any
  production edit: 8 consumer per-case maps, a sender grep with zero matches, and HEAD's
  non-Git refusal (exit 1, empty stdout, stderr `is not a Git repository root`).
* Harness first (`232ef7b7`): scenario 1 failed RED against the `Worker: 142.076-T`
  placeholder and `cargo check --all-targets` passed. The scenario 3 cases are
  characterization guards that pass at HEAD, because the refusal is raised in daemon endpoint
  derivation before `run_read_server_startup`. Implementation `9e73cce2` turned scenario 1 GREEN.
* Local review: adversarial review `READY_WITH_FOLLOWUPS`, P0=P1=0. The one P2 (missing path
  would map to `NotFound`) is unreachable through the daemon (verified manually). P3 notes accepted.
* Gates: `cargo fmt --check`, pedantic clippy, `cargo build --all-targets`, `cargo dev-test`
  PASS; all 8 consumer targets match the RED baseline per case.
* CI `build` and `start-launcher-windows` green. Copilot review `5663035492` returned
  "Approval recommended" with no threads and no findings; `autoharness gate copilot-review 425`
  returned `SATISFIED: PASS`. No P-021 deferred scope expansion was captured.
  `DARK_MODE_MERGE_AUTHORIZED`.

## Safe-close and archive evidence

`142-F` is a shared covering feature outside the task-only manifest, so the cascade was not used
and `backlogit shipment ship` was never invoked (P-015). `142.076-T` was moved to `done` and
archived on the feature branch (`status: archived`, `archived_status: done`,
`archived_from: .backlogit/queue/142.076-T.md`). The pre-close evidence record was written by
`autoharness shipment cascade-close --classify-only` while `.backlogit/queue/147-S.md` was still
`status: active` (exit 3, `SAFE_CLOSE`, engine `VERIFIED`, `phase_written: pre_close`).
`close_path: safe_close`.

Observed chronology (UTC, from file and evidence timestamps): pre-mode report 06:30:33,
machine-written pre-close evidence snapshot 06:32:12 (`captured_at`; the first evidence run at
06:30:24 was regenerated after the pre-mode report so the order matches the contract),
shipment archive record 06:32:22, post-mode report 06:33:13. The shipment record was preserved
in `.backlogit/archive/147-S.md` (`status: archived`, `archived_status: done`,
`archived_from: .backlogit/queue/147-S.md`) and `.backlogit/queue/147-S.md` removed.
`backlogit sync` succeeded; `142-F.md` SHA-256 before and after is
`42FCA67C1407641C8F3CF4752B3431D978290B1342614A831DF463B572181770` with `status: active`.

| Field | Value |
|---|---|
| `ProposedAction` | Preserve the full shipment record in the archive, remove the source queue record, sync the index, verify the archive and covering-feature state. |
| `ActionRisk` | `destructive` - the queue record is deleted; its fields and body are preserved in the archive and the Git change is reversible. |
| `ActionResult` | `applied` |
| Approval path | `DARK_MODE_ACTIVE` activation record (operator pre-authorized the shipment's merges and closure). |

## Runtime verification

The change touches daemon startup identity only. After the branch switch, two 180 s
`set_workspace` re-latch polls (360 s total) left `_health` at `starting`, and the `engram` CLI
probes (`daemon-status`, `workspace-status`) failed with "Daemon failed to reach Ready state
within 30000ms". Daemon PID 2304 was left untouched per operator directive. This is a blocked
prerequisite, recorded rather than faked. Behavior was instead verified by the real-process
`integration_read_server_generation_wiring` and consumer targets. Releasability:
`READY_WITH_CONDITIONS` (live probes unverified).

## Source artifact cleanup

Shipped item `142.076-T` carried no `source_stash_id` or `source_deliberation_id`. No stash or
deliberation archived; no `linked_deliberation_disposition` applicable (manual close).

## Compaction (P-020)

`compact-context` ran with `target: all`: the shipment's memory
`docs/memory/2026-10-26-147-s-slot-04a-checkpoint.md` was consolidated into
`docs/memory/compacted/2026-10-27-147-s-compacted.md` and the original moved to
`docs/archive/memory/`. Compaction status: `done`.

## Follow-ups

* Stash `6B3DD712` (existing, from 146-S) already tracks re-running the `engram` CLI runtime
  probes once the daemon is Ready; no duplicate was created.
* Stash `2085EC60` (safe-close contract versus skill `archived_status`) was carried forward on
  the feature branch and remains with Stage.
* `closure_pr_number` and `closure_pr_merge_commit` are `null` in this record because it ships
  in the closure PR itself; they are recorded in the session summary.
