---
title: "145-S post-merge operational closure"
doc_type: closure
shipment_id: "145-S"
feature_id: "142-F"
mode: post-merge
date: 2026-10-27
author: ship
verdict: "READY — PR #421 merged and 145-S safe-closed (non-cascading; 142-F unchanged); pre-close evidence captured via autoharness shipment cascade-close --classify-only (exit 3, SAFE_CLOSE, engine VERIFIED) before the queue record was removed"
closure_status: "READY"
releasability: "READY_WITH_CONDITIONS"
compaction_status: "done"
close_path: "safe_close"
close_evidence: "docs/closure/evidence/145-S-142-F-close-evidence.json"
pr_number: 421
merge_commit: "8693f6687ed2caec6cabe1ff9e17954fe472c542"
head_commit_merged: "16793ee1fbfe93cbf6672db66f5c9c39d9d00a76"
closure_pr_number: null
closure_pr_merge_commit: null
runtime_verification_report: null
follow_up_stash:
  - "30B6579E"
  - "2E381AFD"
  - "F03AAA77"
  - "420048FC"
blocking_stash: null
shipment_record_status: "archived (archived_status: done) — targeted manual safe-close; see .backlogit/archive/145-S.md"
pre_reconcile: ".backlogit/reconcile/145-S-pre-20261027T0335Z.md"
post_reconcile: ".backlogit/reconcile/145-S-post-20261027T0345Z.md"
---

## Summary

Shipment `145-S` delivered the single task `142.061-T` (PRE-1b, the PRE-1 close):
`build_candidate_generation`, `seal_candidate_inventory`, the closed
`#[non_exhaustive]` `CandidateBuildError`, and private-field `BuiltCandidate` /
`SealedCandidate` in `src/services/generations/candidate_build.rs`, with scenarios
(2) build and (3) seal added to `integration_candidate_generation_build` (10 tests
plus 4 unit tests). PR #421 merged with a merge commit (P-009) at reviewed HEAD
`16793ee1` and merge SHA `8693f6687ed2caec6cabe1ff9e17954fe472c542`. PRE-2 items
(`142.062-T`) were not implemented.

## Dark-mode decisions and gates

* `DARK_MODE_ACTIVE`; merge pre-authorized, admin fallback not authorized and not used.
* Harness first: scenarios 2 and 3 failed RED in this task's own stubs while scenario 1
  stayed GREEN; then implementation turned all 10 tests GREEN.
* Local review: adversarial, Rust and correctness persona reviews plus a delta re-review,
  `READY_WITH_FOLLOWUPS` / `READY`, P0=0, P1=0. In-scope P2 findings (missing `cozo/`
  directory mapping, seal error kind, `Index` display) were fixed in `46fe5d17`.
* Gates: `cargo fmt --check`, `cargo clippy -D warnings -D clippy::pedantic`,
  `cargo build --all-targets` PASS. `cargo dev-test --no-fail-fast` passed except two
  load-sensitive tests (`integration_backlog_hydration` 5 s timing,
  `integration_markdown_indexing` cozo lock panic) that passed on rerun; the timing
  test also passes on baseline `main`.
* Builds used `CARGO_TARGET_DIR=target-145` because the live daemon (PID 2304) locks
  `target\debug\engram.exe`; the daemon was not touched.
* CI `build` and `start-launcher-windows` green. Copilot review `5661782011` raised two
  findings; both were classified out of scope under P-021 C1, captured as stashes
  `F03AAA77` and `420048FC` first, replied to citing those IDs, resolved, and dispositioned
  with `Copilot-Review-Body-Disposition: 5661782011`. `autoharness gate copilot-review 421`
  returned `SATISFIED: PASS` for the reviewed HEAD, run again immediately before merge.
  `DARK_MODE_MERGE_AUTHORIZED`.
* Decision: `mint_generation_id` keeps `GenerationIdError`. The plan's closed variant list
  has no variant for it, so adapting needs a plan revision (stash `30B6579E`).

## Safe-close and archive evidence

`142-F` is a shared covering feature outside the task-only manifest, so the cascade was not
used and `backlogit shipment ship` was never invoked (P-015). The pre-close evidence record
was written by `autoharness shipment cascade-close --classify-only` while
`.backlogit/queue/145-S.md` was still `status: active` (exit 3, `SAFE_CLOSE`, engine
`VERIFIED`, `phase_written: pre_close`), before the manual close. `close_path: safe_close`.

`142.061-T` was archived with `archived_status: done` on the feature branch
(`.backlogit/archive/142.061-T.md`). The shipment record was preserved in
`.backlogit/archive/145-S.md` (`status: archived`, `archived_status: done`,
`archived_from: .backlogit/queue/145-S.md`) and `.backlogit/queue/145-S.md` removed.
`backlogit sync` succeeded; `142-F.md` SHA-256 before and after is
`42FCA67C1407641C8F3CF4752B3431D978290B1342614A831DF463B572181770` with `status: active`.

| Field | Value |
|---|---|
| `ProposedAction` | Preserve the full shipment record in the archive, remove the source queue record, sync the index, verify the archive and covering-feature state. |
| `ActionRisk` | `destructive` — the queue record is deleted; its fields and body are preserved in the archive and the Git change is reversible. |
| `ActionResult` | `applied` |
| Approval path | `DARK_MODE_ACTIVE` activation record (operator pre-authorized the shipment's merges and closure). |

## Runtime verification

The change adds library code only; no runtime surface changed. The probes against the
live workspace daemon via the `engram` CLI (`daemon-status`, `workspace-status`, `search`)
could not run: after the branch switch the daemon (PID 2304, process alive, left untouched
per operator directive) did not reach `Ready` within the 600 s CLI wait. This is a blocked
prerequisite, recorded rather than faked. Releasability: `READY_WITH_CONDITIONS` (no
caller exists until the later PRE slots land, and live probes remain unverified).

## Source artifact cleanup

Shipped item `142.061-T` carried no `source_stash_id` or `source_deliberation_id`
(checked at closure). No stash or deliberation archived; no
`linked_deliberation_disposition` applicable (manual close).

## Compaction (P-020)

`compact-context` ran with `target: all`: the shipment's memory
`docs/memory/2026-10-26-ship-145-s-build-complete.md` was consolidated into
`docs/memory/compacted/2026-10-27-145-s-compacted.md` and the original moved to
`docs/archive/memory/`. Compaction status: `done`.

## Follow-ups

* Stash `30B6579E` (DEFERRED SCOPE EXPANSION): `mint_generation_id` return type needs a plan revision.
* Stash `2E381AFD`: `.engram` exclusion hardening, orphan candidate cleanup, extra branch tests.
* Stash `F03AAA77`: reject or report `IndexResult.errors` during candidate build.
* Stash `420048FC`: stream the SHA-256 for large databases.
* Re-run the `engram` CLI runtime probes once the daemon is Ready.
