---
title: "148-S post-merge operational closure"
doc_type: closure
shipment_id: "148-S"
feature_id: "142-F"
mode: post-merge
date: 2026-10-27
author: ship
verdict: "READY (runtime probes unverified, tracked as stash 6B3DD712) - PR #427 merged and 148-S safe-closed (non-cascading; 142-F unchanged); pre-close evidence captured via autoharness shipment cascade-close --classify-only (exit 3, SAFE_CLOSE, engine VERIFIED) before the queue record was removed"
closure_status: "READY"
releasability: "READY_WITH_CONDITIONS"
compaction_status: "done"
close_path: "safe_close"
close_evidence: "docs/closure/evidence/148-S-142-F-close-evidence.json"
pr_number: 427
merge_commit: "952f76d1bf7596e4146ed7fa44ff55cdbf764db9"
head_commit_merged: "d4cacde705e99dc16638f89aedba4d43e34781d1"
closure_pr_number: null
closure_pr_merge_commit: null
runtime_verification_report: null
follow_up_stash:
  - "C8AEEFFA"
  - "6B3DD712"
blocking_stash: null
shipment_record_status: "archived (archived_status: done) - targeted manual safe-close; see .backlogit/archive/148-S.md"
pre_reconcile: ".backlogit/reconcile/148-S-pre-20261027T1637Z.md"
post_reconcile: ".backlogit/reconcile/148-S-post-20261027T1638Z.md"
---

## Summary

Shipment `148-S` delivered the single task `142.077-T` (PRE-3b): the `pub`
`set_read_server_gate` / `read_server_gate` slot in `src/server/state.rs`, the
`install_generation_gate` call (after workspace publication, before
`set_hydration_ready_for_generation`, only when `generations_root` is a directory at startup),
and the spawned `drive_generation_activation` driver in `src/daemon/lifecycle_policy.rs`.
PR #427 merged with a merge commit (P-009) at head `d4cacde7` and merge SHA
`952f76d1bf7596e4146ed7fa44ff55cdbf764db9`. The transient same-revision retry stays with
`142.064-T` (Slot-04c).

## Dark-mode decisions and gates

* `DARK_MODE_ACTIVE`; merge pre-authorized, admin fallback not authorized and not used.
* Harness first (`8203dd24`): the new cases failed RED against the `Worker: 142.077-T`
  placeholders while `142.076-T`'s cases stayed GREEN; implementation `fabca0d2`.
* Local adversarial review of `fabca0d2`: `READY_WITH_FOLLOWUPS`, P0=P1=0. In-scope findings fixed in
  `18ad8be0` (explicit absent-at-startup flag, late-install retry, loud test failure). Later deltas
  `4e0a92cb`, `338f77f8` and `d4cacde7` were reviewed as deltas (`READY_WITH_FOLLOWUPS`).
* Copilot review ran three cycles (`5664173139`, `5669784913`, `5670457327`). All seven findings were
  in scope under P-021 C1 and fixed: Unix symlink cleanup (this was the CI `build` failure), capped
  and offloaded manifest probe, stale-revision tracking, one retained in-flight probe,
  publisher read path left unchanged, FIFO-safe non-blocking open. Every thread got a reply and was
  resolved, and each review body carries a `Copilot-Review-Body-Disposition` comment.
  `autoharness gate copilot-review 427` returned `SATISFIED: PASS` for `d4cacde7`.
* Gates at `338f77f8`: `cargo fmt --check`, pedantic clippy, `cargo build --all-targets`,
  full `cargo dev-test`, wiring target and the 8 consumer targets PASS. `d4cacde7` adds a Unix-only
  `O_NONBLOCK` open and test; fmt, clippy, build, lib and consumer targets passed on Windows and
  Linux was verified by CI (`build` and `start-launcher-windows` green).
* `DARK_MODE_MERGE_AUTHORIZED`. No P-021 deferred scope expansion was captured; the advisory review
  follow-ups are stash `C8AEEFFA`.

## Safe-close and archive evidence

`142-F` is a shared covering feature outside the task-only manifest, so the cascade was not used
and `backlogit shipment ship` was never invoked (P-015). `142.077-T` was moved to `done` and
archived on the feature branch (`status: archived`, `archived_status: done`,
`archived_from: .backlogit/queue/142.077-T.md`). The pre-close evidence record was written by
`autoharness shipment cascade-close --classify-only` while `.backlogit/queue/148-S.md` was still
`status: active` (exit 3, `SAFE_CLOSE`, engine `VERIFIED`, `phase_written: pre_close`).
`close_path: safe_close`.

Observed chronology (UTC): pre-mode report 16:37:11, pre-close evidence 16:37:47, shipment
archive record 16:38:00, post-mode report 16:38. The shipment record was preserved in
`.backlogit/archive/148-S.md` (`status: archived`, `archived_status: done`,
`archived_from: .backlogit/queue/148-S.md`) and `.backlogit/queue/148-S.md` removed.
`backlogit sync` succeeded; `142-F.md` SHA-256 before and after is
`42FCA67C1407641C8F3CF4752B3431D978290B1342614A831DF463B572181770` with `status: active`.

| Field | Value |
|---|---|
| `ProposedAction` | Preserve the full shipment record in the archive, remove the source queue record, sync the index, verify the archive and covering-feature state. |
| `ActionRisk` | `destructive` - the queue record is deleted; its fields and body are preserved in the archive and the Git change is reversible. |
| `ActionResult` | `applied` |
| Approval path | `DARK_MODE_ACTIVE` activation record (operator pre-authorized the shipment's merges and closure). |

## Runtime verification

The change touches read-server startup and activation. After the merge, the 2-second `README.md`
mtime re-latch could not run: daemon PID 2304 was no longer running and the named pipe did not
exist. `scripts/diagnose-engram.ps1` reported "no running daemon", and `engram ... daemon-status`
failed with "Daemon failed to reach Ready state within 30000ms". A `workspace-status` probe was
stopped. No daemon was respawned deliberately. This is a blocked prerequisite, recorded rather than
faked. Behavior was verified by the real-process `integration_read_server_generation_wiring` target
(8 cases), the consumer targets and the full `cargo dev-test`. Releasability: `READY_WITH_CONDITIONS`.

## Monitoring and rollback handoff

This slice is not a release: it changes only internal read-server startup wiring and is not tagged
or published on its own. The handoff below applies to the next release that carries it.

| Evidence | Value |
|---|---|
| Unmet runtime evidence | Live `engram` CLI probes against a Ready daemon carrying this change, in particular `get_workspace_status.generation` non-null after a published revision. |
| Healthy signal | A read-server daemon reaches `ready`; with a published revision, `get_workspace_status.generation` is non-null and `active_revision` matches the published revision within 30 s; `get_daemon_status` answers throughout. |
| Failure signal | `_health` stuck in `starting`, `generation` null with a valid published revision after 30 s, `active_revision` not advancing after a valid new revision, or repeated `warn` activation logs for one revision. |
| Monitoring plan | Observe daemon logs per `docs/log-observation-guide.md`; re-run `engram --workspace <root> workspace-status` and `daemon-status` once the daemon is Ready (stash `6B3DD712`). |
| Owner | Release owner of the next tagged release; Stage owns stash `6B3DD712` and `C8AEEFFA` triage. |
| Validation window | The first successful read-server startup after the next release install, plus one restart and one revision publication. |
| Rollback trigger | Any failure signal above that persists across one daemon restart. |
| Rollback procedure | Reinstall the prior GitHub Release asset and flush regenerated `.engram/` state; revert merge commit `952f76d1bf7596e4146ed7fa44ff55cdbf764db9` if the change is unreleased. |

## Source artifact cleanup

Shipped item `142.077-T` carried no `source_stash_id` or `source_deliberation_id`. No stash or
deliberation was archived; no `linked_deliberation_disposition` applies (manual close).

## Compaction (P-020)

`compact-context` ran with `target: all`: the shipment's memory
`docs/memory/2026-10-27-148-s-slot-04b-checkpoint.md` was consolidated into
`docs/memory/compacted/2026-10-27-148-s-compacted.md` and the original moved to
`docs/archive/memory/`. Compaction status: `done`.

## Follow-ups

* Stash `C8AEEFFA` (new): advisory hardening of the activation driver and its tests from the review.
* Stash `6B3DD712` (existing): re-run the `engram` CLI runtime probes once a daemon is Ready; no
  duplicate was created.
* `closure_pr_number` and `closure_pr_merge_commit` are `null` in this record because it ships
  in the closure PR itself; they are recorded in the session summary.
