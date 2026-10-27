---
title: "149-S post-merge operational closure"
doc_type: closure
shipment_id: "149-S"
feature_id: "142-F"
mode: post-merge
date: 2026-10-27
author: ship
verdict: "READY (runtime probes unverified, tracked as stash 6B3DD712) - PR #429 merged and 149-S safe-closed (non-cascading; 142-F unchanged); pre-close evidence captured via autoharness shipment cascade-close --classify-only (exit 3, SAFE_CLOSE, engine VERIFIED) before the queue record was removed"
closure_status: "READY"
releasability: "READY_WITH_CONDITIONS"
compaction_status: "done"
close_path: "safe_close"
close_evidence: "docs/closure/evidence/149-S-142-F-close-evidence.json"
pr_number: 429
merge_commit: "bd40390445fd083dc64913c0c646008a82c88d1e"
head_commit_merged: "ec572e54d57eb8b27959a73f818d5aba8a7ad560"
closure_pr_number: null
closure_pr_merge_commit: null
runtime_verification_report: null
follow_up_stash:
  - "80A098CE"
  - "6B3DD712"
blocking_stash: null
shipment_record_status: "archived (archived_status: done) - targeted manual safe-close; see .backlogit/archive/149-S.md"
pre_reconcile: ".backlogit/reconcile/149-S-pre-20261027T1825Z.md"
post_reconcile: ".backlogit/reconcile/149-S-post-20261027T1827Z.md"
---

## Summary

Shipment `149-S` delivered the single task `142.064-T` (PRE-3c), the last slot of the PRE-3 family:
`TransientRetry` inside `drive_generation_activation` (`src/daemon/lifecycle_policy.rs`) re-attempts the
same published revision only after a `RejectionClass::Transient` failure (backoff 100 ms doubling, 2 s
cap; window of `layout.activation_deadline` from the revision's first transient failure). A permanent
rejection is never re-attempted, and an exhausted window waits for a newer revision. PRE-3 is closed.
PR #429 merged with a merge commit (P-009) at head `ec572e54` and merge SHA
`bd40390445fd083dc64913c0c646008a82c88d1e`. Only `src/daemon/lifecycle_policy.rs` and
`tests/integration/read_server_generation_wiring_test.rs` changed in source.

## Dark-mode decisions and gates

* `DARK_MODE_ACTIVE`; merge pre-authorized, admin fallback not authorized and not used.
* Harness first (`6a761628`): `read_server_retries_a_transient_failure_of_the_same_revision` failed RED
  against the `Worker: 142.064-T` marker while `an_absent_inventory_file_is_classified_transient`
  (absent inventory file is `Transient` by `classify`) and every `142.076-T` / `142.077-T` case stayed
  GREEN; `cargo check --all-targets` passed. Implementation `9d844084`.
* Local adversarial review: round 1 `READY_WITH_FOLLOWUPS` (P2: the loop's own capped tick delay hid the
  100 ms backoff; fixed in `4970b9b9` with a pure unit test); later delta reviews `READY`.
* Copilot review: round 1 (`5672140026`, medium) found the retry window started after the awaited manifest
  recheck; in scope under P-021 C1, fixed in `ec572e54`, thread replied and resolved. Round 2
  (`5672228369`) recommended approval with no open issues. Both bodies carry
  `Copilot-Review-Body-Disposition` comments. `autoharness gate copilot-review 429` returned
  `SATISFIED: PASS` for `ec572e54`; CI `build` and `start-launcher-windows` green.
* PRE-3 close evidence: three consecutive full-output runs of
  `cargo test --test integration_read_server_generation_wiring -- --nocapture` after the final build,
  10 passed each with identical per-case maps; all 8 consumer targets match `142.076-T`'s RED-phase
  baseline per case (7, 3, 1, 3, 2, 3, 3, 3 passed). fmt, pedantic clippy (no `unused_imports` allow)
  and `cargo build --all-targets` PASS. `cargo dev-test` passed in full on the first run; after the
  review fix the only local failure was the wall-clock test
  `integration_backlog_hydration::backlog_index_100_items_under_5_seconds` (5.0-8.9 s on a loaded host,
  unrelated to this diff), and CI is green.
* `DARK_MODE_MERGE_AUTHORIZED`. No P-021 deferred scope expansion was captured; advisory P3 follow-ups
  are stash `80A098CE`.

## Safe-close and archive evidence

`142-F` is a shared covering feature outside the task-only manifest, so the cascade was not used and
`backlogit shipment ship` was never invoked (P-015). `142.064-T` was moved to `done` on the feature branch
and its archive record carries `status: archived`, `archived_status: done`,
`archived_from: .backlogit/queue/142.064-T.md`. Subtasks `142.064.001-ST` to `.003-ST` were not touched
(Stage closes them per plan 10.1). The pre-close evidence record was written by
`autoharness shipment cascade-close --classify-only` while `.backlogit/queue/149-S.md` was still
`status: active` (exit 3, `SAFE_CLOSE`, engine `VERIFIED`, `phase_written: pre_close`).
`close_path: safe_close`.

Observed chronology (UTC): pre-mode report 18:25, pre-close evidence 18:26:13, shipment archive record
18:26:20, post-mode report 18:27. The shipment record was preserved in `.backlogit/archive/149-S.md`
(`status: archived`, `archived_status: done`, `archived_from: .backlogit/queue/149-S.md`) and
`.backlogit/queue/149-S.md` removed. `backlogit sync` succeeded; `142-F.md` SHA-256 before and after is
`42FCA67C1407641C8F3CF4752B3431D978290B1342614A831DF463B572181770` with `status: active`.

| Field | Value |
|---|---|
| `ProposedAction` | Preserve the full shipment record in the archive, remove the source queue record, sync the index, verify the archive and covering-feature state. |
| `ActionRisk` | `destructive` - the queue record is deleted; its fields and body are preserved in the archive and the Git change is reversible. |
| `ActionResult` | `applied` |
| Approval path | `DARK_MODE_ACTIVE` activation record (operator pre-authorized the shipment's merges and closure). |

## Runtime verification

The change touches read-server startup activation. The detached daemon (PID 37676) answered `_health`
with `"status":"starting"` after the `set_workspace` re-bind and a 180 s poll and never reached `ready`;
`scripts/diagnose-engram.ps1` reported the daemon live but `daemon-status` and `workspace-status` failed
with "Daemon failed to reach Ready state within 30000ms". The daemon was not stopped or restarted. This is a
blocked prerequisite, recorded rather than faked. Behavior was verified by the real-process
`integration_read_server_generation_wiring` target (10 cases, three identical runs), the consumer targets
and CI. Releasability: `READY_WITH_CONDITIONS`.

## Monitoring and rollback handoff

This slice is not a release: it changes only the internal read-server activation driver and is not tagged
or published on its own. The handoff below applies to the next release that carries it.

| Evidence | Value |
|---|---|
| Unmet runtime evidence | Live `engram` CLI probes against a Ready daemon carrying this change, in particular a transient activation failure that recovers to `active_revision` equal to the published revision. |
| Healthy signal | A read-server daemon reaches `ready`; with a published revision whose inventory file is briefly unreadable, `get_workspace_status.generation.active_revision` reaches the published revision within 10 s of the file becoming readable with no new revision published; `get_daemon_status` answers throughout. |
| Failure signal | `_health` stuck in `starting`, `last_failed_revision` set with `active_revision` null for longer than `activation_deadline` (60 s) after the file is readable, repeated `warn` activation logs for one revision, or a permanently rejected revision being re-attempted. |
| Monitoring plan | Observe daemon logs per `docs/log-observation-guide.md`; re-run `engram --workspace <root> workspace-status` and `daemon-status` once the daemon is Ready (stash `6B3DD712`). |
| Owner | Release owner of the next tagged release; Stage owns stash `6B3DD712` and `80A098CE` triage. |
| Validation window | The first successful read-server startup after the next release install, plus one restart and one revision publication. |
| Rollback trigger | Any failure signal above that persists across one daemon restart. |
| Rollback procedure | Reinstall the prior GitHub Release asset and flush regenerated `.engram/` state; revert merge commit `bd40390445fd083dc64913c0c646008a82c88d1e` if the change is unreleased. |

## Source artifact cleanup

Shipped item `142.064-T` carried no `source_stash_id` or `source_deliberation_id`. No stash or
deliberation was archived; no `linked_deliberation_disposition` applies (manual close).

## Compaction (P-020)

`compact-context` ran with `target: all`: the shipment's memory
`docs/memory/2026-10-27-149-s-slot-04c-checkpoint.md` was consolidated into
`docs/memory/compacted/2026-10-27-149-s-compacted.md` and the original moved to
`docs/archive/memory/`. Compaction status: `done`.

## Follow-ups

* Stash `80A098CE` (new, low): advisory driver coverage and tick-delay refinements from the reviews.
* Stash `6B3DD712` (existing): re-run the `engram` CLI runtime probes once a daemon is Ready; no
  duplicate was created.
* Stage closes `142.064.001-ST` to `.003-ST` through plan 10.1.
* `closure_pr_number` and `closure_pr_merge_commit` are `null` in this record because it ships in the
  closure PR itself; they are recorded in the session summary.
