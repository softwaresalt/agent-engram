---
title: "138-S post-merge operational closure"
doc_type: closure
shipment_id: "138-S"
feature_id: "142-F"
mode: post-merge
date: 2026-09-12
author: ship
verdict: "CLOSED — PR #391 merged as a merge commit under explicit operator approval ('OPERATOR MERGE APPROVAL: PR #391 is explicitly approved for merge'), verified reachable from origin/main; shipment 138-S manually safe-closed (targeted, non-cascading) following the 133-S/134-S/135-S/137-S precedent; 142-F verified untouched and remains active for later 142-F-covering shipments."
closure_status: "READY"
releasability: "READY_WITH_CONDITIONS"
compaction_status: "done"
pr_number: 391
merge_commit: "81b19b0d91c79c9c456ce703dca42a4688978dfa"
head_commit_merged: "6b39ee94"
closure_pr_number: null
closure_pr_merge_commit: null
runtime_verification_report: "docs/closure/2026-09-12-138-s-runtime-verification.md"
follow_up_stash:
  - "265F99BE"
  - "5C873386"
  - "3FFEE99B"
  - "9D313653"
  - "58B33C45"
  - "EC3BAF22"
  - "DA0AF326"
blocking_stash: null
shipment_record_status: "archived (archived_status: done) — manual safe-close performed 2026-09-12 on post-merge/138-s-generation-activation-request-context"
---

# 138-S post-merge operational closure

## Summary

`138-S` (feature `142-F`) delivered the generation activation service
(`142.018-T` + 4 subtasks — typed manifest parse/bounds enforcement,
`activate_initial` with deadline/store resolution, single-flight
`maybe_activate_newer`, immutable rejection cache with transient backoff),
the mode-agnostic `ReadRequestContext` constructors (`142.019-T`), startup
readiness gating on initial generation activation (`142.028-T`), request
entry order and background activation (`142.029-T`), the read-server
capability gate and context assertion in dispatch (`142.030-T`), the
descriptor-derived stdio MCP tool catalog (`142.031-T`), the
descriptor-derived CLI workflow surface (`142.032-T`), and the read-input
ownership inventory with fail-on-unclassified guard (`142.033-T` + 2
subtasks) — 14 manifest items total.

PR #391 merged as a merge commit (repo policy: merge-only, squash/rebase
disabled) under explicit, PR-scoped operator approval: **"OPERATOR MERGE
APPROVAL: PR #391 is explicitly approved for merge."** Approval was
contingent on the last-mile gates still passing for the exact reviewed
HEAD (`6b39ee94`), which this session independently re-verified via `gh pr
view` and `git merge-base --is-ancestor` before beginning any closure
work. A local checkpoint commit (`1c3eb3e8`), created during an earlier
operator-directed mid-review pause, was confirmed to have never reached
the PR branch/remote and was preserved instead by cherry-picking it onto
the post-merge closure branch (`ce3b2fba`), per explicit operator
instruction not to advance/re-arm review on the implementation PR.

This session also root-caused, via operator-authorized direct named-pipe
IPC access, a pre-existing workspace-generation readiness-latch defect
(`_health` reports `starting` forever after a branch/workspace switch with
no transferred successor). The defect was assessed against 138-S's own
P-021 C1 same-contract-surface test and correctly deferred as out of
scope — captured to stash `265F99BE`, not implemented.

This closure covers `138-S`'s own 14-item manifest only. It does not
re-open, re-plan, or touch any other `142-F`-covering shipment, and it does
not archive or otherwise mutate `142-F` itself, which remains
`status: active` for those later shipments.

## Merge / PR Evidence

| Gate | Result |
|---|---|
| PR #391 state | `MERGED` at `2026-09-12T03:01:40Z` |
| Merge commit | `81b19b0d91c79c9c456ce703dca42a4688978dfa` |
| Reviewed/merged HEAD | `6b39ee94` (exact HEAD the operator approval named) |
| Ancestry | `git fetch origin main` then `git merge-base --is-ancestor 81b19b0d... origin/main` → exit `0` |
| Merge strategy | `--merge` (merge commit); squash/rebase disabled repo-wide (`allow_merge_commit: true`, `allow_squash_merge: false`, `allow_rebase_merge: false`) |
| Local review readiness at HEAD | `READY` — 27/27 threads resolved, 2 findings explicitly suppressed with documented rationale (stash `5C873386` medium, `3FFEE99B` low), 11 review rounds recorded across the PR lifetime |
| P-018 Copilot review gate | reported `SATISFIED` for the exact reviewed HEAD immediately pre-merge (re-verified at the last-mile gate) |
| CI checks | green (build + launcher checks SUCCESS) |
| Local checkpoint commit handling | `1c3eb3e8` confirmed never pushed to the PR branch (remote HEAD unchanged at `6b39ee94` before and after); preserved via cherry-pick as `ce3b2fba` on the post-merge closure branch instead |
| Worktree topology (P-016) | Single worktree throughout (`git worktree list --porcelain` confirmed one entry before and after merge) |

## Runtime Verification

Verdict: **PASS WITH FOLLOW-UP** (full report:
[`2026-09-12-138-s-runtime-verification.md`](./2026-09-12-138-s-runtime-verification.md)) —
build/fmt/clippy all clean; 87/87 targeted tests across all 14 manifest
tasks pass GREEN; full `cargo test --all-targets --no-fail-fast` 2457/2460
GREEN (3 confirmed pre-existing, confirmed-unrelated flakes — stashed at
`9D313653`, `58B33C45`, and newly `EC3BAF22`). The `FOLLOW-UP` qualifier
reflects the `cli-daemon-status` live probe, which was intentionally not
attempted against the shared-environment daemon per explicit operator
instruction (that daemon is independently known to be in the
readiness-latch state described above, tracked at stash `265F99BE`, not a
regression introduced by 138-S).

## Operational Closure Checklist

* **Invariants to preserve**: generation activation accepts only typed,
  bounds-enforced manifests with a single-flight newer-activation path and
  an immutable rejection cache; `ReadRequestContext` stays mode-agnostic;
  startup readiness gates on initial activation; request entry preserves
  descriptor-resolution-before-activation ordering with bounded background
  reconciliation; the dispatch capability gate rejects `Managed`-mode
  contexts; the MCP tool catalog and CLI workflow surface stay
  descriptor-derived and in parity; every read-mode input stays enumerated
  and classified with fail-closed behavior on the unclassified case.
* **Pre-deploy audits**: confirmed the 14-task commit range is
  additive/corrective to the activation/context/startup-gate/request-entry
  surfaces only; no data-migration or schema change introduced; `Cargo.lock`
  regenerated via `cargo check`/`cargo build`, not hand-edited; confirmed
  the readiness-latch defect pre-dates this shipment's own commit range.
* **Deployment / rollout path**: merge-only to `main` (repo policy:
  squash/rebase disabled, merge commit only).
* **Post-deploy checks**: `engram --version` confirmed against the merged
  binary; direct named-pipe IPC (`_health`, `query_memory`) confirmed
  healthy transport/process identity against the shared daemon
  (independent of the readiness-latch signal itself).
* **Risky action record**: see the full three-action breakdown (PR merge,
  shipment safe-close bookkeeping, checkpoint-commit preservation) in
  [`docs/closure/2026-09-12-138-s-operational-closure.md`](./2026-09-12-138-s-operational-closure.md#risky-action-record).
  Summary: (1) merging PR #391 was `ActionRisk: moderate`, `ActionResult:
  applied`, under explicit PR-scoped operator approval. (2) shipment
  safe-close bookkeeping — deleting `.backlogit/queue/138-S.md` and
  authoring `.backlogit/archive/138-S.md` — is `ActionRisk: destructive`
  per the strict-safety schema's literal inclusion of "deletes," but is
  Ship-role-permitted, non-discretionary bookkeeping; `ActionResult:
  applied`, verified via live re-read and byte-for-byte `142-F`
  invariance. (3) cherry-picking the pause checkpoint commit onto the
  closure branch was `ActionRisk: low`, `ActionResult: applied`, confirmed
  isolated from the implementation PR branch.
* **Healthy signals**: see Runtime Verification above.
* **Failure signals**: a future `cargo dev-test`/`cargo ci` failure on
  `main` touching `src/services/generations/*`, `src/server/state.rs`,
  `src/daemon/startup_activation.rs`, `src/daemon/request_entry.rs`,
  `src/tools/mod.rs`, `src/shim/tools_catalog.rs`, or `src/cli/runner.rs`,
  beyond the three already-documented pre-existing flakes.
* **Monitoring plan**: no live dashboards/alerts apply to this
  locally-run developer tool; the operative monitoring signal is the
  daemon's existing structured JSON logs, `get_health_report`, and
  `get_daemon_status`, plus continued observation of `_health.status` for
  the separately-tracked readiness-latch defect (stash `265F99BE`).
* **Rollback trigger**: a reported regression traced to the generation
  activation, request-context, startup-gate, request-entry, dispatch-gate,
  or catalog-derivation modules introduced by this shipment.
* **Rollback procedure**: fix-forward is preferred; if not viable, revert
  this shipment's commit range on `main` per the rollback procedure in
  [`docs/closure/2026-09-12-138-s-operational-closure.md`](./2026-09-12-138-s-operational-closure.md#rollback-procedure).

## Shipment Safe-Close

`138-S`'s manifest (14 tasks, no feature member) does not fully cover its
covering feature `142-F`'s full roster, so the P-015 fully-covered-root
exception for the cascade `backlogit shipment ship` operation does not
apply — the cascade path is forbidden for this shipment regardless of tool
behavior. Additionally, `backlogit shipment ship` was not attempted at all
given the already-documented non-termination defect against this same
covering feature (see
`docs/compound/workflow-issues/backlogit-shipment-ship-non-terminating-large-covering-feature-2026-09-06.md`).
The generic `backlogit move 138-S --status shipped` fallback was confirmed
rejected outright by the CLI. Manual safe-close was performed:
hand-authored `.backlogit/archive/138-S.md` (see its AUDIT RATIONALE for
full detail), removed `.backlogit/queue/138-S.md`, ran `backlogit sync`.
Covering feature `142-F` verified byte-for-byte unchanged (SHA-256
`59263E8FFB779485E135A7AA41D9DAAC89B4A996B767D128D76A1AD2E70404C3`, 802
bytes, both before and after).

## Reconciliation

* Pre-mode: `.backlogit/reconcile/138-S-pre-20260912-030500.md` —
  `recommendation: PROCEED` (all 14 manifest items `pre-archived`, no
  orphans).
* Post-mode: `.backlogit/reconcile/138-S-post-20260912-031100.md` —
  `recommendation: PROCEED` (all archive files present, no deletions per
  `git status -- ".backlogit/archive/"`, P-007 clean).

## Precedent

This canonical, gate-discoverable evidence file follows the same
frontmatter schema established for `134-S`
(`docs/closure/134-S-2026-09-04-post-merge-closure.md`) and most recently
`137-S` (`docs/closure/137-S-2026-09-08-post-merge-closure.md`), both of
which close under the same shared covering feature `142-F`. It is created
proactively as part of this shipment's own closure so that the
`pipeline-topology` gate's `shipment_readiness` check for later
`142-F`-covering shipments finds this file on first read.
