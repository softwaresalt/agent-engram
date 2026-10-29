---
title: "151-S post-merge operational closure"
doc_type: closure
shipment_id: "151-S"
feature_id: "142-F"
mode: post-merge
date: 2026-10-27
author: ship
verdict: "READY (runtime probes unverified, tracked as stash 6B3DD712) - PR #433 merged and 151-S safe-closed (non-cascading; 142-F unchanged); pre-close evidence captured via autoharness shipment cascade-close --classify-only (exit 3, SAFE_CLOSE, engine VERIFIED) before the queue record was removed"
closure_status: "READY"
releasability: "READY_WITH_CONDITIONS"
compaction_status: "done"
close_path: "safe_close"
close_evidence: "docs/closure/evidence/151-S-142-F-close-evidence.json"
pr_number: 433
merge_commit: "d55ea4146f72e26591bcc09a8f25b68986c677d5"
head_commit_merged: "543d241507c7d814c2577c5fa04c30538f0c46cb"
closure_pr_number: null
closure_pr_merge_commit: null
runtime_verification_report: null
follow_up_stash:
  - "6B3DD712"
  - "65EF8399"
blocking_stash: null
shipment_record_status: "archived (archived_status: done) - targeted manual safe-close; see .backlogit/archive/151-S.md"
pre_reconcile: ".backlogit/reconcile/151-S-pre-20261027T2105Z.md"
post_reconcile: ".backlogit/reconcile/151-S-post-20261027T2106Z.md"
---

## Summary

Shipment `151-S` delivered the single task `142.066-T` (PRE-4): in ReadServer mode,
`get_workspace_statistics` is now admitted at request entry. `GENERATION_PINNED_READS` in
`src/daemon/request_entry.rs` lists the method; `process_request` routes it through `admit_read` and
then `tools::dispatch_with_read_context` with the admitted context. With no gate installed it returns
the typed `GenerationNotYetActivated` refusal. All other requests, `_health` and managed mode are
unchanged, and no generation-control endpoint exists. PR #433 merged with a merge commit (P-009) at
head `543d2415` and merge SHA `d55ea4146f72e26591bcc09a8f25b68986c677d5`. Source changes:
`src/daemon/request_entry.rs` and `tests/integration/read_server_generation_wiring_test.rs` only.

## Dark-mode decisions and gates

* `DARK_MODE_ACTIVE`; merge pre-authorized, admin fallback not authorized and not used.
* Harness first (`dab4cdcf`): three PRE-4 scenarios failed RED with the
  `Worker: 142.066-T GENERATION_PINNED_READS` marker while the 10 PRE-3 cases stayed GREEN; the
  implementation (`8c73058d`) turned them GREEN (13/13).
* Plan R7.8: three consecutive careful-mode runs of `integration_read_server_generation_wiring`,
  `integration_request_entry_activation` and the 8 PRE-3 consumer targets gave identical per-case maps.
* Local gates: `cargo fmt`, `cargo clippy` (pedantic), `cargo build --all-targets` and `cargo dev-test` PASS.
* Local adversarial review (Correctness, Rust, Security, Architecture personas, in parallel):
  READY_WITH_FOLLOWUPS. The Security persona's P1 (the read-server fall-through is not fail-closed for
  non-pinned methods) is out of scope under P-021 C1, because PRE-4 requires every other request to be
  byte-identical. It was captured before any reply as deferred stash `65EF8399` (threadless path,
  `DISCOVERY-STATUS: AMBIGUOUS` against candidate `86F93068`).
* Copilot review of the PR head reported no open issues and no threads;
  `autoharness gate copilot-review 433` returned `SATISFIED: PASS`; `build` and `start-launcher-windows`
  passed on `543d2415`.

## Safe-close and archive evidence

* Pre-reconcile report `.backlogit/reconcile/151-S-pre-20261027T2105Z.md` was written while
  `.backlogit/queue/151-S.md` was still `active` in the queue.
* `autoharness shipment cascade-close --classify-only --shipment 151-S --feature 142-F --sha
  d55ea4146f72e26591bcc09a8f25b68986c677d5 --json` exited 3: `SAFE_CLOSE`, engine `VERIFIED`, no mutation.
  Evidence `docs/closure/evidence/151-S-142-F-close-evidence.json` was written after the pre-report.
* `.backlogit/archive/142.066-T.md` carries `status: archived`, `archived_status: done`,
  `archived_from: .backlogit/queue/142.066-T.md`.
* `.backlogit/archive/151-S.md` was authored by hand (`status: archived`, `archived_status: done`), the
  queue record removed, and `backlogit sync` run. `backlogit shipment ship` was not invoked.
* Covering feature `142-F` is `status: active` with SHA-256
  `42FCA67C1407641C8F3CF4752B3431D978290B1342614A831DF463B572181770` before and after close.
* Post-reconcile report: `.backlogit/reconcile/151-S-post-20261027T2106Z.md`.

## Destructive action record

* ProposedAction: delete `.backlogit/queue/151-S.md` after authoring `.backlogit/archive/151-S.md`
  (manual safe-close step).
* ActionRisk: destructive (removes a tracked file from the working tree).
* Approval: the Dark Factory activation contract for this run (`DARK_MODE_ACTIVE`, operator AFK)
  explicitly directed this manual safe-close ordering, including removal of the queue record. No
  separate per-action G1 approval was obtained; this gap is stated here rather than treating merge
  pre-authorization as deletion approval. The operator may revoke or revisit it on review of this PR.
* Preservation and rollback: the full queue record content is preserved in the archive copy and in git
  history (tracked on `main` at `d55ea414`). Recovery is operator-approved only: copy
  `.backlogit/archive/151-S.md` outside the backlog record directories as evidence, restore the original
  record with `git checkout d55ea414 -- .backlogit/queue/151-S.md`, remove the archived twin, verify
  exactly one `151-S` record exists, and only then run `backlogit sync`.
* ActionResult: queue record removed, `backlogit sync` succeeded, `backlogit shipment get 151-S` reports
  `status: archived`; no archive file was deleted.

## Runtime verification

The change touches the read-server statistics request path. The detached daemon (PID 37676) answered
`_health` on `\\.\pipe\engram-80bc60cb-d104-4289-a657-9e0dab645b3b` with `"status":"starting"`
(uptime 15259 s, build `0.3.0-rc.1+ge043299`), so `scripts/diagnose-engram.ps1` and the `engram` CLI
probes were not run against a Ready daemon. Runtime evidence is unverified and tracked by the existing
stash `6B3DD712`; no duplicate was created. The daemon was not stopped. The integration harness did
exercise real daemon processes and the real CLI `stats` command against a read-server fixture.

## Monitoring and rollback handoff

This slice is not a release: it changes only request-entry admission for the pinned statistics read and
is not tagged or published on its own. The handoff below applies to the next release that carries it.

| Evidence | Value |
|---|---|
| SLI | Fraction of `engram stats` calls against a Ready read-server daemon that return `/provenance/generation_id` equal to the manifest `generation_id` of the active generation. |
| Observation query | `engram --workspace <W> --json stats` and `workspace-status`; compare `result.provenance.generation_id` with the active generation manifest. Daemon logs per `docs/log-observation-guide.md`. |
| Baseline | 100% of stats calls against a Ready read-server daemon carry provenance; a read-server daemon with no generations root refuses `stats` with code 17001 (`GenerationNotYetActivated`). |
| Alert threshold | Any stats call on a Ready read-server daemon with no provenance, `code_files` that disagrees with the active generation, or `_health` stuck in `starting` longer than the 60 s activation deadline. |
| Owner | Release owner of the next tagged release; Stage owns triage of stash `6B3DD712` and `65EF8399`. |
| Observation window | The first successful read-server startup after the next release install, plus one restart and one revision publication. |
| Rollback trigger | Any alert-threshold condition that persists across one daemon restart. |
| Rollback procedure | Reinstall the prior GitHub Release asset and flush regenerated `.engram/` state; revert merge commit `d55ea4146f72e26591bcc09a8f25b68986c677d5` if the change is unreleased. |

## Source artifact cleanup

Shipped item `142.066-T` carried no `source_stash_id` or `source_deliberation_id`. No stash or
deliberation was archived; no `linked_deliberation_disposition` applies (manual close).

## Compaction (P-020)

`compact-context` ran with `target: all`: the shipment memory
`docs/memory/2026-10-27-151-s-ship-checkpoint.md` was consolidated into
`docs/memory/compacted/2026-10-27-151-s-compacted.md` and the original moved to
`docs/archive/memory/`. Compaction status: `done`.

## Follow-ups

* Stash `6B3DD712` (existing): re-run the `engram` CLI runtime probes once a daemon is Ready; no
  duplicate was created.
* Stash `65EF8399` (new, provisional high, requires deliberation): make the read-server request-entry
  path fail-closed for every non-pinned method (`enforce_read_server_dispatch` has no production caller).
* `closure_pr_number` and `closure_pr_merge_commit` are `null` in this record because it ships in the
  closure PR itself; they are recorded in the session summary.

## 151-S

PR #433 (feature), commit d55ea4146f72e26591bcc09a8f25b68986c677d5
