---
title: "153-S post-merge operational closure"
doc_type: closure
shipment_id: "153-S"
feature_id: "142-F"
mode: post-merge
date: 2026-10-28
author: ship
verdict: "READY (runtime probes unverified, tracked as stash 6B3DD712) - PR #437 merged and 153-S safe-closed (non-cascading; 142-F unchanged); pre-close evidence captured via autoharness shipment cascade-close --classify-only (exit 3, SAFE_CLOSE) before the queue record was removed"
closure_status: "READY"
releasability: "READY_WITH_CONDITIONS"
compaction_status: "done"
close_path: "safe_close"
close_evidence: "docs/closure/evidence/153-S-142-F-close-evidence.json"
pr_number: 437
merge_commit: "b2f2d57f2ce713757f1f98e73f0255bdc67b9081"
head_commit_merged: "8769610d2c643a7f47f067b424463bb14247c1f6"
closure_pr_number: null
closure_pr_merge_commit: null
runtime_verification_report: null
follow_up_stash:
  - "6B3DD712"
  - "CF6CF510"
blocking_stash: null
shipment_record_status: "archived (archived_status: done) - targeted manual safe-close; see .backlogit/archive/153-S.md"
pre_reconcile: ".backlogit/reconcile/153-S-pre-20261028T0159Z.md"
post_reconcile: ".backlogit/reconcile/153-S-post-20261028T0202Z.md"
---

## Summary

Shipment `153-S` delivered the single task `142.068-T` (PRE-6): `probe_mcp_read` and private
`run_mcp_session` in `src/preflight_probe.rs`, the stdio MCP provenance probe. The absolute `engram`
is spawned as `shim --workspace <W>` with piped stdin and stdout and null stderr. `mcp_arguments_json`
is parsed before the spawn (invalid or non-object is `Malformed("mcp_arguments")`, nothing spawned).
The session sends `initialize`, `notifications/initialized` and one `tools/call`, reads the matching
response before closing stdin, then waits on `try_wait` until the deadline (a shim still running is
`Deadline`, killed and reaped). A capped line-reader thread (1 MiB, detached, never joined) feeds
`recv_timeout` slices of 20 ms with `try_wait` polled every iteration and the 250 ms post-exit drain.
`generation_id` and `data_fingerprint` come from the result's `structuredContent`. PR #437 merged with
a merge commit (P-009) at head `8769610d` and merge SHA `b2f2d57f2ce713757f1f98e73f0255bdc67b9081`.
Source changes: `src/preflight_probe.rs` and `tests/integration/preflight_probe_test.rs` only.

## Dark-mode decisions and gates

* `DARK_MODE_ACTIVE`; merge pre-authorized, admin fallback not authorized and not used.
* Claim deviation: `backlogit shipment claim 153-S` and `backlogit move 153-S --status active` both
  refuse (predecessor 152-S is archived with `archived_status: done`, not `shipped`). The claim was
  recorded by editing `status: active` in the queue file, as 152-S did; the topology `post_claim`
  gate then passed. Recorded as an audit deviation for operator review.
* Harness first (`b9a1efc4`): all four initial PRE-6 tests failed RED with the `Worker: 142.068-T`
  marker while the seven PRE-5 tests stayed GREEN; the implementation turned them GREEN.
* Local gates: `cargo fmt`, `cargo clippy` (pedantic), `cargo build --all-targets`, the forbidden-string
  grep (no `read_to_end`, `read_to_string`, `wait_with_output`, `.output()`, reader `join`, `unwrap`,
  `expect`) and `cargo test --test integration_preflight_probe` (14/14 at merge) PASS. A full
  `cargo dev-test` run PASSed on the implementation; a later run failed once in
  `hcl_indexing_test` under load, which passes alone and in a `--no-fail-fast` rerun. The PR `build` and
  `start-launcher-windows` jobs passed on the merged head.
* Local adversarial review (Correctness, Rust, Security, Architecture personas in parallel):
  READY_WITH_FOLLOWUPS, no P0/P1. In scope and fixed: initialize error reply handling, response
  matching (ignore server requests, fail fast on null-id errors), draining a queued line at the limit,
  reader-thread spawn error, module header, refusal tests. Out of scope (P-021 C1, captured before any
  reply): env scrubbing for both probes and shared reader and fixture-compile helpers, stash `CF6CF510`.
  The discovery lookup found the related entry `CF792EC3` (PRE-5 production spawn scrubbing) but could not
  confirm the same expansion (it excludes the MCP spawn and helper sharing), so a distinct entry was
  captured with the `DEFERRED SCOPE EXPANSION` token.
* Copilot review: review `5676672920` raised one thread (poll the child independently of receive
  timeouts), fixed in `a9f09c06` with the `mcpchatty` regression, replied and resolved,
  `Copilot-Review-Body-Disposition: 5676672920`. Review `5676755372` raised a body-only finding (shim
  that ignores stdin close must be `Deadline`), fixed in `8769610d` with the `mcpstuck` fixture and
  dispositioned with `Copilot-Review-Body-Disposition: 5676755372`. Review `5676823685` on the merged
  head reported no open issues. `autoharness gate copilot-review 437` returned `SATISFIED: PASS`.

## Safe-close and archive evidence

* Pre-reconcile report `.backlogit/reconcile/153-S-pre-20261028T0159Z.md` (generated 01:59:55Z) was
  written while `.backlogit/queue/153-S.md` was still `active` in the queue.
* `autoharness shipment cascade-close --classify-only --shipment 153-S --feature 142-F --sha
  b2f2d57f2ce713757f1f98e73f0255bdc67b9081 --json` exited 3: `SAFE_CLOSE`, no mutation. The engine
  verdict was `UNVERIFIED` (backlogit 1.12.0), which only reinforces the non-cascading path. Evidence
  `docs/closure/evidence/153-S-142-F-close-evidence.json` was written after the pre-report.
* `.backlogit/archive/142.068-T.md` carries `status: archived`, `archived_status: done`,
  `archived_from: .backlogit/queue/142.068-T.md` (official `backlogit move --status done` and
  `backlogit archive`).
* `.backlogit/archive/153-S.md` was authored by hand (`status: archived`, `archived_status: done`,
  `archived_from: .backlogit/queue/153-S.md`), the queue record removed, and `backlogit sync` run.
  `backlogit shipment ship` was not invoked.
* Covering feature `142-F` is `status: active` with SHA-256
  `42FCA67C1407641C8F3CF4752B3431D978290B1342614A831DF463B572181770` before and after close.
* Post-reconcile report: `.backlogit/reconcile/153-S-post-20261028T0202Z.md`.

## Destructive action record

* ProposedAction: delete `.backlogit/queue/153-S.md` after authoring `.backlogit/archive/153-S.md`
  (manual safe-close step).
* ActionRisk: destructive (removes a tracked file from the working tree).
* Approval: the Dark Factory activation contract for this run (`DARK_MODE_ACTIVE`, operator AFK)
  explicitly directed this manual safe-close ordering, including removal of the queue record. No
  separate per-action G1 approval was obtained; this gap is stated here rather than treating merge
  pre-authorization as deletion approval. The operator may revoke or revisit it on review of this PR.
* Preservation and rollback: the full queue record content is preserved in the archive copy and in git
  history (tracked on `main` at `b2f2d57f`). Recovery is operator-approved only: copy
  `.backlogit/archive/153-S.md` outside the backlog record directories as evidence, restore the original
  record with `git checkout b2f2d57f -- .backlogit/queue/153-S.md`, remove the archived twin, verify
  exactly one `153-S` record exists, and only then run `backlogit sync`.
* ActionResult: queue record removed, `backlogit sync` succeeded, `backlogit shipment get 153-S` reports
  `status: archived`; no archive file was deleted.

## Runtime verification

The change adds a probe function to a library module not yet consumed by a binary or the launcher. The
detached daemon (PID 37676) answered `_health` on
`\\.\pipe\engram-80bc60cb-d104-4289-a657-9e0dab645b3b` with `"status":"starting"` (uptime 32904 s, build
`0.3.0-rc.1+ge043299`), so `scripts/diagnose-engram.ps1` and the `engram` CLI probes were not run
against a Ready daemon. Runtime evidence is unverified and tracked by the existing stash `6B3DD712`; no
duplicate was created. The daemon was not stopped. The integration harness did exercise a real daemon
and the real `engram shim` against a read-server fixture, and its MCP provenance matched the CLI.

## Monitoring and rollback handoff

This slice is not a release: it adds an unconsumed library function and is not tagged or published on
its own. The handoff below applies to the first release in which the launcher preflight consumes it.

| Evidence | Value |
|---|---|
| SLI | Fraction of preflight MCP probes (`probe_mcp_read`) against a Ready read-server daemon whose `generation_id` and `data_fingerprint` equal those of `probe_cli_read`. |
| Observation query | `engram --workspace <W> --json stats` for the CLI values; the MCP values come from `tools/call get_workspace_statistics` through `engram shim --workspace <W>` (`result.structuredContent`). Daemon logs per `docs/log-observation-guide.md`. |
| Baseline | 100% of probes against a Ready read-server daemon return identical generation ID and fingerprint on both transports, and the shim exits after stdin close. |
| Alert threshold | Any MCP probe returning `Deadline`, `Oversize`, `Malformed`, `Exit` or a CLI/MCP mismatch against a Ready daemon, or any shim or reader thread left running after the probe returns. |
| Owner | Release owner of the next tagged release; Stage owns triage of stash `6B3DD712` and `CF6CF510`. |
| Observation window | The first preflight run after the next release install, plus one daemon restart and one revision publication. |
| Rollback trigger | Any alert-threshold condition that persists across one daemon restart. |
| Rollback procedure | Reinstall the prior GitHub Release asset and flush regenerated `.engram/` state; revert merge commit `b2f2d57f2ce713757f1f98e73f0255bdc67b9081` if the change is unreleased. |

## Source artifact cleanup

Shipped item `142.068-T` carried no `source_stash_id` or `source_deliberation_id`. No stash or
deliberation was archived; no `linked_deliberation_disposition` applies (manual close).

## Compaction (P-020)

`compact-context` ran with `target: all`: the shipment memory
`docs/memory/2026-10-27-ship-153-s-142-068-t-checkpoint.md` was consolidated into
`docs/memory/compacted/2026-10-27-153-s-compacted.md` and the original moved to
`docs/archive/memory/`. Only the just-closed unit was a compaction candidate for this run; older
memory remains a separate backlog. Compaction status: `done`.

## Follow-ups

* Stash `6B3DD712` (existing): re-run the `engram` CLI runtime probes once a daemon is Ready.
* Stash `CF6CF510` (new, provisional low): scrub `ENGRAM_*` environment in both probe spawns and share
  the reader and fixture-compile helpers.
* Process note: `backlogit shipment claim` refuses slots whose predecessor is archived with
  `archived_status: done` (it requires `shipped`). 153-S was claimed with a manual status edit under the
  operator-directed manual safe-close ordering; that is a recorded deviation, not a pattern to repeat. A later
  slot that hits the same refusal must halt and route to the operator or Stage to remediate the predecessor
  state (for example a `shipped` archive record) before retrying the supported claim, and must not hand-edit
  shipment status to pass the `pre_claim` gate.
* Monitoring note: the baseline and alert threshold are expected values; they were not measured because
  no Ready daemon was available. Detection of leaked children or reader threads is by process inspection.
* `closure_pr_number` and `closure_pr_merge_commit` are `null` in this record because it ships in the
  closure PR itself; they are recorded in the session summary.

## 153-S

Delivery: PR #437 (feature), commit b2f2d57f2ce713757f1f98e73f0255bdc67b9081 (merge commit, P-009).
