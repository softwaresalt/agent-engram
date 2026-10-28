---
title: "152-S post-merge operational closure"
doc_type: closure
shipment_id: "152-S"
feature_id: "142-F"
mode: post-merge
date: 2026-10-28
author: ship
verdict: "READY (runtime probes unverified, tracked as stash 6B3DD712) - PR #435 merged and 152-S safe-closed (non-cascading; 142-F unchanged); pre-close evidence captured via autoharness shipment cascade-close --classify-only (exit 3, SAFE_CLOSE) before the queue record was removed"
closure_status: "READY"
releasability: "READY_WITH_CONDITIONS"
compaction_status: "done"
close_path: "safe_close"
close_evidence: "docs/closure/evidence/152-S-142-F-close-evidence.json"
pr_number: 435
merge_commit: "18229e212584e625f02c35a6612b40b8a21f9a26"
head_commit_merged: "2174f104966f059ddc56c3673cab16c21527998d"
closure_pr_number: null
closure_pr_merge_commit: null
runtime_verification_report: null
follow_up_stash:
  - "6B3DD712"
  - "CF792EC3"
  - "C799846B"
blocking_stash: null
shipment_record_status: "archived (archived_status: done) - targeted manual safe-close; see .backlogit/archive/152-S.md"
pre_reconcile: ".backlogit/reconcile/152-S-pre-20261028T0025Z.md"
post_reconcile: ".backlogit/reconcile/152-S-post-20261028T0027Z.md"
---

## Summary

Shipment `152-S` delivered the single task `142.067-T` (PRE-5): the synchronous CLI provenance probe
facade `engram::preflight_probe` (`probe_cli_read`, `wait_for_generation`, `ProbeRead`, `PREFLIGHT_READ`,
`HEALTH_READ`, `ProbeReport`, `ProbeError`). It spawns only the canonical `engram` as
`--workspace <W> --json <cli_args...>` with stdin and stderr null, bounded child I/O (1 MiB cap, capped
reader thread, 20 ms owner loop, 250 ms post-exit drain, detached reader, exact-child kill and reap), and
absolute `Instant` deadlines; `wait_for_generation` returns the matching `ProbeReport`. `probe_mcp_read`
(142.068-T) is not implemented. PR #435 merged with a merge commit (P-009) at head `2174f104` and merge
SHA `18229e212584e625f02c35a6612b40b8a21f9a26`. Source changes: `src/preflight_probe.rs`, one line in
`src/lib.rs`, `tests/integration/preflight_probe_test.rs` with its `[[test]]` stanza, and one
`[[surface]]` entry in `.cargo/test-coverage-manifest.toml`.

## Dark-mode decisions and gates

* `DARK_MODE_ACTIVE`; merge pre-authorized, admin fallback not authorized and not used.
* Harness first (`b2899f97`): 7 tests failed RED with the `Worker: 142.067-T` marker; the implementation
  (`498f1ee0`) turned them GREEN. The harness diff was exactly 400 lines (SB-1 limit not exceeded).
* Scope note: the coverage-oracle completeness gate in `cargo dev-test` requires every top-level `src/`
  module to be mapped, so one `src/preflight_probe.rs` surface was added to
  `.cargo/test-coverage-manifest.toml`. It is a file outside the task's authorized list, treated as a
  same-contract completion under P-021 C1 and endorsed by the Architecture persona; the operator should
  confirm on review.
* Local gates: `cargo fmt`, `cargo clippy` (pedantic), `cargo build --all-targets`, the forbidden-string
  grep and `cargo test --test integration_preflight_probe` (7/7) PASS. `cargo dev-test` PASSed at
  `88cd7a98`; later reruns stopped on unrelated timing-sensitive tests
  (`integration_backlog_hydration` 5 s budget, `contract_shim_lifecycle` handshake) under heavy Defender
  load, and each passes alone. The PR `build` job (`cargo test --all-targets`) passed on `2174f104`.
* Local adversarial review (Correctness, Rust, Security, Architecture personas in parallel, plus two
  delta re-reviews): READY_WITH_FOLLOWUPS, no P0/P1. In scope: Interrupted retry and docs (`fffcb4bf`).
  Out of scope (P-021 C1, captured before any reply): production env scrubbing and absolute-path check
  (`CF792EC3`, requires deliberation) and harness coverage gaps and startup-race hardening
  (`C799846B`). Discovery found no matching existing entry.
* Copilot review: first pass raised two valid threads (plan signatures used `Instant` and
  `Result<ProbeReport, _>`; the harness brief had allowed `Duration` and `()`; unbounded fixture
  lifetime). Fixed in `42b2775d`, replied and resolved. A later review body finding (non-object `result`
  accepted) was fixed in `2174f104` and dispositioned with
  `Copilot-Review-Body-Disposition: 5675918686`. `autoharness gate copilot-review 435` returned
  `SATISFIED: PASS`; `build` and `start-launcher-windows` passed on `2174f104`.

## Safe-close and archive evidence

* Pre-reconcile report `.backlogit/reconcile/152-S-pre-20261028T0025Z.md` was written while
  `.backlogit/queue/152-S.md` was still `active` in the queue.
* `autoharness shipment cascade-close --classify-only --shipment 152-S --feature 142-F --sha
  18229e212584e625f02c35a6612b40b8a21f9a26 --json` exited 3: `SAFE_CLOSE`, no mutation. The engine
  verdict was `UNVERIFIED` (backlogit 1.12.0, minor line 1.12 not verified), which only reinforces the
  non-cascading path. Evidence `docs/closure/evidence/152-S-142-F-close-evidence.json` was written after
  the pre-report.
* `.backlogit/archive/142.067-T.md` carries `status: archived`, `archived_status: done`,
  `archived_from: .backlogit/queue/142.067-T.md` (official `backlogit move --status done` and
  `backlogit archive`).
* `.backlogit/archive/152-S.md` was authored by hand (`status: archived`, `archived_status: done`), the
  queue record removed, and `backlogit sync` run. `backlogit shipment ship` was not invoked.
* Covering feature `142-F` is `status: active` with SHA-256
  `42FCA67C1407641C8F3CF4752B3431D978290B1342614A831DF463B572181770` before and after close.
* Post-reconcile report: `.backlogit/reconcile/152-S-post-20261028T0027Z.md`.

## Destructive action record

* ProposedAction: delete `.backlogit/queue/152-S.md` after authoring `.backlogit/archive/152-S.md`
  (manual safe-close step).
* ActionRisk: destructive (removes a tracked file from the working tree).
* Approval: the Dark Factory activation contract for this run (`DARK_MODE_ACTIVE`, operator AFK)
  explicitly directed this manual safe-close ordering, including removal of the queue record. No
  separate per-action G1 approval was obtained; this gap is stated here rather than treating merge
  pre-authorization as deletion approval. The operator may revoke or revisit it on review of this PR.
* Preservation and rollback: the full queue record content is preserved in the archive copy and in git
  history (tracked on `main` at `18229e21`). Recovery is operator-approved only: copy
  `.backlogit/archive/152-S.md` outside the backlog record directories as evidence, restore the original
  record with `git checkout 18229e21 -- .backlogit/queue/152-S.md`, remove the archived twin, verify
  exactly one `152-S` record exists, and only then run `backlogit sync`.
* ActionResult: queue record removed, `backlogit sync` succeeded, `backlogit shipment get 152-S` reports
  `status: archived`; no archive file was deleted.

## Runtime verification

The change adds a new library module (not yet consumed by a binary or the launcher). The detached daemon
(PID 37676) answered `_health` on `\\.\pipe\engram-80bc60cb-d104-4289-a657-9e0dab645b3b` with
`"status":"starting"` (uptime 27266 s, build `0.3.0-rc.1+ge043299`), so `scripts/diagnose-engram.ps1`
and the `engram` CLI probes were not run against a Ready daemon. Runtime evidence is unverified and
tracked by the existing stash `6B3DD712`; no duplicate was created. The daemon was not stopped. The
integration harness did exercise real daemon processes and the real CLI `stats` and `health` commands
against a read-server fixture.

## Monitoring and rollback handoff

This slice is not a release: it adds an unconsumed library module and is not tagged or published on its
own. The handoff below applies to the first release in which the launcher preflight consumes it.

| Evidence | Value |
|---|---|
| SLI | Fraction of preflight CLI probes (`probe_cli_read`) against a Ready read-server daemon that return a report whose `generation_id` equals the published manifest `generation_id`. |
| Observation query | `engram --workspace <W> --json stats` and `engram --workspace <W> --json health`; compare `result.provenance.generation_id` with the active generation manifest. Daemon logs per `docs/log-observation-guide.md`. |
| Baseline | 100% of probes against a Ready read-server daemon return the published generation ID and a `code_files=..;functions=..;..` fingerprint; `HEALTH_READ` is `accepted`. |
| Alert threshold | Any probe returning `Deadline`, `Oversize`, `Malformed` or `Mismatch` against a Ready daemon, or any probe child or reader thread left running after the probe returns. |
| Owner | Release owner of the next tagged release; Stage owns triage of stash `6B3DD712`, `CF792EC3` and `C799846B`. |
| Observation window | The first preflight run after the next release install, plus one daemon restart and one revision publication. |
| Rollback trigger | Any alert-threshold condition that persists across one daemon restart. |
| Rollback procedure | Reinstall the prior GitHub Release asset and flush regenerated `.engram/` state; revert merge commit `18229e212584e625f02c35a6612b40b8a21f9a26` if the change is unreleased. |

## Source artifact cleanup

Shipped item `142.067-T` carried no `source_stash_id` or `source_deliberation_id`. No stash or
deliberation was archived; no `linked_deliberation_disposition` applies (manual close).

## Compaction (P-020)

`compact-context` ran with `target: all`: the shipment memory
`docs/memory/2026-10-27-ship-152-s-142-067-t-checkpoint.md` was consolidated into
`docs/memory/compacted/2026-10-27-152-s-compacted.md` and the original moved to
`docs/archive/memory/`. The threshold scan found `docs/memory/` over its file and size thresholds, but
only the just-closed unit qualified for this run; older memory remains a separate backlog. Compaction
status: `done`.

## Follow-ups

* Stash `6B3DD712` (existing): re-run the `engram` CLI runtime probes once a daemon is Ready; no
  duplicate was created.
* Stash `CF792EC3` (new, provisional medium, requires deliberation): scrub `ENGRAM_DIRECT`,
  `ENGRAM_DATA_DIR` and `ENGRAM_WORKSPACE` in the production probe spawn and validate that `engram` is an
  absolute path.
* Stash `C799846B` (new, provisional low): harness gaps (typed refusal, `Exit`, `Malformed`,
  `ReadError` precedence, `wait_for_generation` re-probe and early-return cases) and startup-race
  hardening.
* Process lesson: quote the plan's function signatures verbatim in the harness brief; the first
  harness used `Duration` and `()` where the plan specifies `Instant` and `ProbeReport`.
* `closure_pr_number` and `closure_pr_merge_commit` are `null` in this record because it ships in the
  closure PR itself; they are recorded in the session summary.
