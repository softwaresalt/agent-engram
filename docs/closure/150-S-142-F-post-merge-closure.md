---
title: "150-S post-merge operational closure"
doc_type: closure
shipment_id: "150-S"
feature_id: "142-F"
mode: post-merge
date: 2026-10-27
author: ship
verdict: "READY (runtime probes unverified, tracked as stash 6B3DD712) - PR #431 merged and 150-S safe-closed (non-cascading; 142-F unchanged); pre-close evidence captured via autoharness shipment cascade-close --classify-only (exit 3, SAFE_CLOSE, engine VERIFIED) before the queue record was removed"
closure_status: "READY"
releasability: "READY_WITH_CONDITIONS"
compaction_status: "done"
close_path: "safe_close"
close_evidence: "docs/closure/evidence/150-S-142-F-close-evidence.json"
pr_number: 431
merge_commit: "b64e3d26d566e802237329e6af12c5ae24edeac8"
head_commit_merged: "319cbd9932f792ab7e7e1b780fad74bcc4c029fb"
closure_pr_number: null
closure_pr_merge_commit: null
runtime_verification_report: null
follow_up_stash:
  - "6B3DD712"
  - "D968583A"
blocking_stash: null
shipment_record_status: "archived (archived_status: done) - targeted manual safe-close; see .backlogit/archive/150-S.md"
pre_reconcile: ".backlogit/reconcile/150-S-pre-20261027T2004Z.md"
post_reconcile: ".backlogit/reconcile/150-S-post-20261027T2007Z.md"
---

## Summary

Shipment `150-S` delivered the single task `142.065-T` (PRE-4b): the pinned statistics read now serves
the opened generation. `get_workspace_statistics_with_context` takes an optional `ReadRequestContext`;
the two-argument `get_workspace_statistics` delegates with `None` and keeps the managed path.
`queries_from_read_context` returns queries over the opened generation database for a generation
context and calls `connect_db` only for a managed context, with no fallback to managed data.
PR #431 merged with a merge commit (P-009) at head `319cbd99` and merge SHA
`b64e3d26d566e802237329e6af12c5ae24edeac8`. Source changes: `src/tools/read.rs`, `src/tools/mod.rs`
(one dispatch arm), the new harness `tests/integration/generation_pinned_statistics_test.rs` with its
`Cargo.toml` target, and a retarget in `tests/integration/registry_service_pin_test.rs`.

## Dark-mode decisions and gates

* `DARK_MODE_ACTIVE`; merge pre-authorized, admin fallback not authorized and not used.
* Harness first (`215402e0`): three scenarios failed RED with the `Worker: 142.065-T` marker; the
  implementation (`2de19201`) turned them GREEN.
* Local gates: `cargo fmt`, `cargo clippy` (pedantic), `cargo build --all-targets` PASS; eight consumer
  regression targets unchanged against the pre-implementation baseline. `cargo dev-test` hit one
  load-dependent timing flake (`backlog_index_100_items_under_5_seconds`) that passed alone and in a full
  `--no-fail-fast` run.
* Local adversarial review: READY_WITH_FOLLOWUPS, no P0/P1. One P3 (scenario 3 compares Debug output of a
  `query_memory` result and could pass vacuously) accepted as residual risk.
* CI caught a regression the local run missed: `integration_registry_service_pin` source-scrapes the body
  of `get_workspace_statistics`, now a one-line wrapper. This is a same-contract-surface completion
  (P-021 C1), fixed in `319cbd99` by retargeting the scrape to `get_workspace_statistics_with_context`
  with no assertion weakened.
* Copilot reviews (two, one per pushed HEAD) reported no open issues and no threads;
  `autoharness gate copilot-review 431` returned `SATISFIED: PASS`; `build` and `start-launcher-windows`
  passed on `319cbd99`.

## Safe-close and archive evidence

* Pre-reconcile report `.backlogit/reconcile/150-S-pre-20261027T2004Z.md` was written while
  `.backlogit/queue/150-S.md` was still `active` in the queue.
* `autoharness shipment cascade-close --classify-only --shipment 150-S --feature 142-F --sha
  b64e3d26d566e802237329e6af12c5ae24edeac8 --json` exited 3: `SAFE_CLOSE`, engine `VERIFIED`, no mutation.
  Evidence `docs/closure/evidence/150-S-142-F-close-evidence.json` was written after the pre-report.
* `.backlogit/archive/142.065-T.md` carries `status: archived`, `archived_status: done`,
  `archived_from: .backlogit/queue/142.065-T.md`.
* `.backlogit/archive/150-S.md` was authored by hand (`status: archived`, `archived_status: done`), the
  queue record removed, and `backlogit sync` run. `backlogit shipment ship` was not invoked.
* Covering feature `142-F` is `status: active` with SHA-256
  `42FCA67C1407641C8F3CF4752B3431D978290B1342614A831DF463B572181770` before and after close.
* Post-reconcile report: `.backlogit/reconcile/150-S-post-20261027T2007Z.md`.

## Destructive action record

* ProposedAction: delete `.backlogit/queue/150-S.md` after authoring `.backlogit/archive/150-S.md`
  (manual safe-close step).
* ActionRisk: destructive (removes a tracked file from the working tree).
* Approval: the Dark Factory activation contract for this run (`DARK_MODE_ACTIVE`, operator AFK)
  explicitly directed this manual safe-close ordering, including removal of the queue record. No
  separate per-action G1 approval was obtained; this gap is stated here rather than treating merge
  pre-authorization as deletion approval. The operator may revoke or revisit it on review of this PR.
* Preservation and rollback: the full queue record content is preserved verbatim in the archive copy
  and in git history (the file was tracked on `main` at `b64e3d26`). Recovery is operator-approved only
  and must keep the shipment unique across backlog record directories: first copy
  `.backlogit/archive/150-S.md` outside the backlog record directories as evidence, restore the original
  active record with `git checkout b64e3d26 -- .backlogit/queue/150-S.md`, remove the archived twin
  `.backlogit/archive/150-S.md`, verify exactly one `150-S` record exists, and only then run
  `backlogit sync`.
* ActionResult: queue record removed, `backlogit sync` succeeded, `backlogit shipment get 150-S` reports
  `status: archived`; no archive file was deleted.

## Evidence hash contract

The 110 observation hashes in `docs/closure/evidence/150-S-142-F-close-evidence.json` were recorded on
Windows against CRLF working-copy bytes and do not reproduce from an LF checkout. The pre-close hashes
were not overwritten. This is a tooling contract gap outside this closure (P-021 C1) and is captured as
deferred stash `D968583A`.

## Runtime verification

The change touches the read-server statistics path. The detached daemon (PID 37676) answered `_health`
on `\\.\pipe\engram-80bc60cb-d104-4289-a657-9e0dab645b3b` with `"status":"starting"`
(uptime 11581 s, build `0.3.0-rc.1+ge043299`), so the `engram` CLI probes and
`scripts/diagnose-engram.ps1` were not run against a Ready daemon. Runtime evidence is unverified and
tracked by the existing stash `6B3DD712`; no duplicate was created. The daemon was not stopped.

## Monitoring and rollback handoff

This slice is not a release: it changes only the pinned statistics read path and is not tagged or
published on its own. The handoff below applies to the next release that carries it.

| Evidence | Value |
|---|---|
| Unmet runtime evidence | An explicit `get_workspace_statistics` call (MCP or direct IPC) against a Ready read-server daemon, showing `code_files` and `functions` from the active generation and `/provenance/generation_id` equal to the `generation_id` field of the active generation manifest. |
| Healthy signal | A read-server daemon reaches `ready`; `get_workspace_statistics` returns `code_files` and `functions` matching the published generation and `provenance.generation_id` equal to the manifest `generation_id`; the daemon's `active_revision` (a numeric publication counter, checked separately) equals the manifest `revision`; no new database files appear under the generation data directory after the call. |
| Failure signal | Statistics reporting `code_files == 0` with an active generation, missing `provenance`, new `.db` files appearing beside the generation, or `_health` stuck in `starting` longer than `activation_deadline` (60 s). |
| Monitoring plan | Observe daemon logs per `docs/log-observation-guide.md`; use `workspace-status` and `daemon-status` only to establish readiness and the active revision (`get_workspace_status` counters use a separate `connect_db` path and do not exercise the pinned statistics handler), then run the explicit `get_workspace_statistics` call above once the daemon is Ready (stash `6B3DD712`). |
| Owner | Release owner of the next tagged release; Stage owns stash `6B3DD712` triage. |
| Validation window | The first successful read-server startup after the next release install, plus one restart and one revision publication. |
| Rollback trigger | Any failure signal above that persists across one daemon restart. |
| Rollback procedure | Reinstall the prior GitHub Release asset and flush regenerated `.engram/` state; revert merge commit `b64e3d26d566e802237329e6af12c5ae24edeac8` if the change is unreleased. |

## Source artifact cleanup

Shipped item `142.065-T` carried no `source_stash_id` or `source_deliberation_id`. No stash or
deliberation was archived; no `linked_deliberation_disposition` applies (manual close).

## Compaction (P-020)

`compact-context` ran with `target: all`: the shipment memory
`docs/memory/2026-10-27-ship-150-s-142-065-t-pre-4b.md` was consolidated into
`docs/memory/compacted/2026-10-27-150-s-compacted.md` and the original moved to
`docs/archive/memory/`. Compaction status: `done`.

## Follow-ups

* Stash `6B3DD712` (existing): re-run the `engram` CLI runtime probes once a daemon is Ready; no
  duplicate was created.
* Stash `D968583A` (new, low): evidence observation hashes are CRLF-based and do not reproduce from an
  LF checkout (Copilot thread on PR #432).
* No other new P-021 deferred-scope-expansion stash entries were created.
* Residual P3: harden the scenario 3 `query_memory` comparison to assert `Ok`; recorded here only.
* `closure_pr_number` and `closure_pr_merge_commit` are `null` in this record because it ships in the
  closure PR itself; they are recorded in the session summary.
