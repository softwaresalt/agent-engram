---
title: "Pre-merge operational closure — 102-S qualified Python caller attribution"
doc_type: closure
source: "102-S / 107-F / PR #307"
description: >-
  Pre-merge release-readiness record for fail-closed qualified Python caller
  attribution in full indexing and incremental sync.
topic: "Fail-closed qualified Python caller attribution"
depth: closure
decision_status: "READY WITH CONDITIONS — CI and explicit merge approval"
author: ship
date: 2026-08-02
verdict: "READY WITH CONDITIONS"
pr: 307
target_commit: "2c86d147e6b7585a2a7f7b7da3fb2152452ba43d"
branch: "feat/107-qualified-staging-caller-attribution"
linked_artifacts:
  - "102-S"
  - "107-F"
  - "107.001-T"
  - "107.002-T"
---

## Summary

PR #307 replaces first-match caller attribution at both qualified Python
provenance producers with the existing typed unique-only lookup. A unique
caller retains the prior staging and exact-target behavior. An ambiguous caller
increments `same_file_ambiguous_dropped` and stages nothing. A missing caller
remains a no-op. The change does not alter schemas, staged-call keys, target
resolution, extraction versions, CLI contracts, or persistence formats.

## Runtime Verification

**Verdict: PASS.** The disposable integration corpus covers full indexing and
incremental sync with two same-name callers, plus a unique-caller control:

- Full indexing produces no staged provenance row keyed to either duplicate,
  increments `same_file_ambiguous_dropped`, and creates no canonical edge from
  either wrong origin to the exact trusted target.
- Incremental sync produces no staged provenance row keyed to either duplicate
  and increments `same_file_ambiguous_dropped`.
- The unique-caller control preserves the canonical edge to the exact target.
- The complete same-file shadowing acceptance binary passes 7/7.

Local `cargo fmt`, pedantic clippy, and `cargo dev-test` passed. The repository's
exact all-target CI command exhausted the local Windows host's disk while
linking, without a test failure; PR #307's clean GitHub runner is the required
condition for that gate. `cargo audit --no-fetch` reports the existing
`lz4_flex 0.10.0` advisory tracked independently by 017-D.

## Invariants to Preserve

- Duplicate same-name callers never receive an arbitrary qualified provenance
  row or canonical edge.
- Exact caller and exact target identity are checked together.
- Full indexing and incremental sync use identical unique, ambiguous, and
  not-found handling.
- Unique-caller recall remains non-zero and target-correct.
- Storage shape, extraction-version behavior, and public contracts remain
  unchanged.

## Pre-Deploy Audits

- PR #301 merged on 2026-07-30, after the latest GitHub release (`v0.2.0`,
  2026-06-16). No release tag contains its merge commit. Therefore the reviewed
  policy requires **no migration/backfill** and no deployed-workspace mutation.
- Copilot reviewed current implementation HEAD `2c86d147` and generated no new
  comments after its one coverage finding was fixed and resolved.
- Merge remains forbidden until CI is green and the operator separately
  approves this exact PR and HEAD.

## Deployment and Rollout Path

This is a merge-only change to a locally installed daemon. There is no hosted
service, fleet rollout, canary, schema migration, or automatic workspace
repair. A later binary release follows the repository's normal tagged release
workflow.

## Risky Action Record

- **ProposedAction:** require a unique same-file caller before staging qualified
  Python provenance in full indexing and incremental sync.
- **ActionRisk:** high, because the admission decision affects persisted graph
  origin even though storage and public contracts are unchanged.
- **Approval:** implementation, build/test, CI, and PR activity were explicitly
  approved. Merge is not approved.
- **ActionResult:** implementation applied and verified on disposable data;
  merge blocked pending green CI and explicit operator approval.
- **Containment:** Ship does not reindex, mutate, or repair any user or deployed
  workspace.

## Healthy and Failure Signals

| Signal | Healthy | Failure |
|---|---|---|
| Duplicate caller provenance | No staged row keyed to either duplicate | Any staged row owned by a duplicate |
| Canonical graph origin | No wrong-origin edge to the exact trusted target | Any wrong-origin canonical edge |
| Ambiguity observability | `same_file_ambiguous_dropped > 0` for the duplicate fixture | Counter remains zero |
| Unique-caller recall | Exact canonical target still resolves | Unique control loses its edge |
| Producer parity | Index and sync both fail closed | Either producer retains first-match behavior |

## Monitoring Plan

The monitoring plane is the acceptance suite plus the first three disposable
index/sync runs after an updated binary is built. For each run, inspect the
staged provenance and canonical edge queries used by the acceptance harness and
confirm the healthy signals above. Silence is not success: the exact identity
queries and ambiguity counter must be observed.

## Rollback

Rollback triggers are any wrong-origin edge, producer asymmetry, or loss of the
unique-caller control. Before merge, revert production commit `70485480` and
restart the daemon. After merge, revert the PR #307 merge commit with
`git revert -m 1 <PR-307-merge-sha>`, rebuild, and restart the daemon.

If a future released binary exposes the change in a named workspace, Ship may
write a target-specific full-reindex handoff after rollback or correction, but
only the operator may execute and verify it. No automatic or fleet-wide reindex
is authorized.

## Validation Window and Owner

- **Window:** the first three disposable daemon index/sync runs after release,
  or seven days from the first run, whichever comes first.
- **Owner:** repository operator.
- **Closeout:** record healthy, degraded, or rolled-back status after the
  window. Do not claim post-release observation before merge and release.

## Readiness

**READY WITH CONDITIONS.** Runtime verification and current-HEAD Copilot review
are clean. Readiness still requires green PR CI and separate operator approval
to merge PR #307 at the exact reviewed HEAD. Shipment 102-S remains active until
positive merged/shipped evidence exists; 103-S must remain queued.
