---
title: "137-S post-merge operational closure"
doc_type: closure
shipment_id: "137-S"
feature_id: "142-F"
mode: post-merge
date: 2026-09-08
author: ship
verdict: "CLOSED — PR #388 merged as a merge commit under explicit operator approval ('PR 388: Merge approved'), verified reachable from origin/main; shipment 137-S manually safe-closed (targeted, non-cascading) following the 133-S/134-S/135-S/136-S precedent; 142-F verified untouched and remains active for later 142-F-covering shipments."
closure_status: "READY"
releasability: "READY_WITH_CONDITIONS"
compaction_status: "done"
pr_number: 388
merge_commit: "ef0135bf05aba5d7329a7306eb78bb9b18e02f69"
head_commit_merged: "653e973e201883790a9862e6ff54ea84efbb902d"
closure_pr_number: 389
closure_pr_merge_commit: null
runtime_verification_report: "docs/closure/2026-09-08-137-s-runtime-verification.md"
follow_up_stash:
  - "AF5CE07E"
  - "7D47F30B"
  - "DA0AF326"
  - "F35EA0E6"
blocking_stash: null
shipment_record_status: "archived (archived_status: done) — manual safe-close performed 2026-09-08 on post-merge/137-s-candidate-indexing-direct-sync-boundary-and-supervisor-crate-separation"
---

# 137-S post-merge operational closure

## Summary

`137-S` (feature `142-F`) delivered sealed-`IndexTarget`-only acceptance in
the candidate indexing service (`142.015-T`, F10), the `ReadServer`
direct-sync refusal boundary (`142.016-T`, F11), the real
`engram-indexer` supervisor crate entry points (`142.020-T`, F12), the
supervisor workspace-boundary contract (`142.021-T`, F13), the distinct
supervisor release artifact (`142.022-T`, F14), and the agent-install
supervisor exclusion (`142.027-T`, F15).

PR #388 merged as a merge commit (repo policy: merge-only, squash/rebase
disabled) under explicit, PR-scoped operator approval: **"PR 388: Merge
approved."** Approval was contingent on the last-mile gates still passing
for the exact reviewed HEAD (`653e973e201883790a9862e6ff54ea84efbb902d`),
which this session independently re-verified via `gh pr view` and
`git merge-base --is-ancestor` before beginning any closure work.

This closure covers `137-S`'s own 6-item manifest only. It does not
re-open, re-plan, or touch any other `142-F`-covering shipment (`138-S`
onward), and it does not archive or otherwise mutate `142-F` itself, which
remains `status: active` for those later shipments.

## Merge / PR Evidence

| Gate | Result |
|---|---|
| PR #388 state | `MERGED` at `2026-09-09T06:05:39Z` |
| Merge commit | `ef0135bf05aba5d7329a7306eb78bb9b18e02f69` |
| Reviewed/merged HEAD | `653e973e201883790a9862e6ff54ea84efbb902d` (exact HEAD the operator approval named) |
| Ancestry | `git fetch origin main` then `git merge-base --is-ancestor ef0135bf... origin/main` → exit `0` |
| Merge strategy | `--merge` (merge commit); squash/rebase disabled repo-wide (`allow_merge_commit: true`, `allow_squash_merge: false`, `allow_rebase_merge: false`) |
| Local review readiness at HEAD | `READY_WITH_FOLLOWUPS`, single P2 finding deferred to stash `AF5CE07E` (per the pre-PR session record) |
| P-018 Copilot review gate | reported `SATISFIED` for the exact reviewed HEAD immediately pre-merge (per merge evidence supplied for this session) |
| CI checks | `build`: SUCCESS, `start-launcher-windows`: SUCCESS |
| Pipeline-topology gate (`--phase lifecycle`, pre-safe-close) | `exit_code: 0`, `active_shipment_invariant` passed (`active_shipment_ids: ["137-S"]`), `branch_ownership` passed, `worktree_topology` passed (`WORKTREE_TOPOLOGY_OK`, single implementation worktree), `shipment_readiness` passed (`predecessor_ids: ["135-S", "136-S"]`) |
| Worktree topology (P-016) | Single worktree throughout (`git worktree list --porcelain` confirmed one entry before and after merge) |

## Runtime Verification

Verdict: **PASS WITH FOLLOW-UP** (full report:
[`2026-09-08-137-s-runtime-verification.md`](./2026-09-08-137-s-runtime-verification.md)) —
build/fmt/clippy all clean; 15/15 targeted tests across this shipment's six
manifest tasks pass GREEN; full `cargo dev-test` 688/689 GREEN (the one
exception is a long-documented, pre-existing, confirmed-unrelated Windows
stdout-truncation flake, stashed as `7D47F30B`). The `FOLLOW-UP` qualifier
reflects the `cli-daemon-status` live probe, which was `BLOCKED` on a
bounded 30-second budget — consistent with the previously-documented
per-branch Cozo first-index cold-start cost for a brand-new branch
namespace (135-S precedent), not a code defect.

## Operational Closure Checklist

* **Invariants to preserve**: the indexing entry point accepts only a
  sealed `IndexTarget`; `IndexTarget::Candidate` never writes into the
  active generation; `ReadServer` mode refuses direct sync with the stable,
  non-retryable F38 refusal while `Managed` mode is unchanged; the agent
  package declares no supervisor binary and the supervisor appears in
  neither the CLI nor the MCP tool catalog; the release workflow publishes
  `engram-indexer` as a distinct artifact excluded from the agent archive;
  agent installation installs no supervisor binary.
* **Pre-deploy audits**: `crates/engram-indexer/Cargo.toml` confirmed to
  have gained only dependency-section changes; `Cargo.lock` regenerated
  via `cargo check --all-targets`, not hand-edited.
* **Deployment / rollout path**: merge-only to `main` (repo policy:
  squash/rebase disabled, merge commit only). The next `release.yml` build
  is the next rollout checkpoint for the new `engram-indexer` distinct
  artifact.
* **Post-deploy checks**: `engram --version` confirmed against the merged
  binary; a fresh `engram install` in a scratch workspace should be
  spot-checked to confirm it never places or references an
  `engram-indexer` binary once F14's workflow first executes on a tagged
  release.
* **Risky action record**: see the full two-action breakdown (PR merge,
  then shipment safe-close bookkeeping) in
  [`docs/closure/2026-09-08-137-s-operational-closure.md`](./2026-09-08-137-s-operational-closure.md#risky-action-record).
  Summary: (1) merging PR #388 was `ActionRisk: moderate` (additive-only,
  no wired production caller yet), `ActionResult: applied`, under explicit
  PR-scoped operator approval. (2) the shipment safe-close bookkeeping
  itself — deleting `.backlogit/queue/137-S.md` and authoring
  `.backlogit/archive/137-S.md` — is `ActionRisk: destructive` per the
  strict-safety schema's literal inclusion of "deletes," but is
  Ship-role-permitted, not independently destructive-approval-gated,
  bookkeeping (Role Boundary explicitly allows "close shipments, archive
  completed items"; content fully preserved in the archive file; fully
  git-reversible; lands on a dedicated closure branch/PR requiring its own
  separate operator approval before reaching `main`). `ActionResult:
  applied`, verified via live re-read and byte-for-byte `142-F`
  invariance.
* **Healthy signals**: see Runtime Verification above.
* **Failure signals**: a future `cargo dev-test`/`cargo ci` failure on
  `main` touching `src/services/code_graph.rs`, `src/cli/direct.rs`,
  `src/installer/mod.rs`, `crates/engram-indexer/*`, or
  `.github/workflows/release.yml`, beyond the already-documented
  pre-existing `archive_verifier` flake.
* **Monitoring plan**: no live dashboards/alerts apply to this locally-run
  developer tool; the operative monitoring signal is the next
  scheduled/triggered `release.yml` build (first execution against the new
  `engram-indexer` artifact steps) and the targeted test suite on
  subsequent shipments (F16-F18) that wire cross-process coordination
  between the supervisor and daemon writers (tracked at stash `AF5CE07E`).
* **Rollback trigger**: a `release.yml` build failure, or a reported
  regression traced to the sealed-`IndexTarget`, direct-sync refusal, or
  supervisor-crate modules introduced by this shipment.
* **Rollback procedure**: fix-forward is preferred; if not viable, revert
  this shipment's commit range on `main` per the rollback procedure in
  [`docs/closure/2026-09-08-137-s-operational-closure.md`](./2026-09-08-137-s-operational-closure.md#rollback-procedure).

## Shipment Safe-Close

`137-S`'s manifest (6 tasks, no feature member) does not fully cover its
covering feature `142-F`'s 59-unit roster, so the P-015 fully-covered-root
exception for the cascade `backlogit shipment ship` operation does not
apply — the cascade path is forbidden for this shipment regardless of tool
behavior. Additionally, `backlogit shipment ship` was not attempted at all
given the already-documented non-termination defect against this same
covering feature (see
`docs/compound/workflow-issues/backlogit-shipment-ship-non-terminating-large-covering-feature-2026-09-06.md`,
addendum). The generic `backlogit move 137-S --status shipped` fallback
was confirmed rejected outright by the CLI (exit code 9). Manual safe-close
was performed: hand-authored `.backlogit/archive/137-S.md` (see its AUDIT
RATIONALE for full detail), removed `.backlogit/queue/137-S.md`, ran
`backlogit sync`. Covering feature `142-F` verified byte-for-byte unchanged
(SHA-256 `59263E8FFB779485E135A7AA41D9DAAC89B4A996B767D128D76A1AD2E70404C3`,
802 bytes, both before and after).

## Reconciliation

* Pre-mode: `.backlogit/reconcile/137-S-pre-20260908T230954Z.md` —
  `recommendation: PROCEED` (all 6 manifest items `pre-archived`, no
  orphans).
* Post-mode: `.backlogit/reconcile/137-S-post-20260908T231240Z.md` —
  `recommendation: PROCEED` (all archive files present, no deletions per
  `git status -- ".backlogit/archive/"`, P-007 clean).

## Precedent

This canonical, gate-discoverable evidence file follows the same
frontmatter schema established for `134-S`
(`docs/closure/134-S-2026-09-04-post-merge-closure.md`) and most recently
`136-S` (`docs/closure/136-S-2026-09-08-post-merge-closure.md`), both of
which close under the same shared covering feature `142-F`. It is created
proactively as part of this shipment's own closure (rather than as a
later repair, as was needed for `135-S`) so that the `pipeline-topology`
gate's `shipment_readiness` check for `138-S` and later `142-F`-covering
shipments finds this file on first read.
