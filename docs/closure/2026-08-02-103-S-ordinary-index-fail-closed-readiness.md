---
title: "Operational readiness — 103-S ordinary-index fail-closed follow-ups"
doc_type: closure
source: "103-S / 108-F"
description: >-
  Pre-merge runtime, safety, observability, and rollback record for ordinary
  index topology retry preservation and authoritative empty-file eviction.
topic: "Ordinary-index fail-closed retry and empty-file eviction"
depth: closure
decision_status: "READY — local gates and report-only review passed"
author: ship
date: 2026-08-02
verdict: READY
branch: "feat/108-ordinary-index-fail-closed"
linked_artifacts:
  - "103-S"
  - "108-F"
  - "108.002-T"
  - "108.001-T"
---

## Summary

Shipment 103-S changes only the private ordinary full-index implementation.
A partial per-file error no longer certifies newly discovered canonical
workspace topology: the previous snapshot is restored when one exists, and an
absent prior snapshot remains absent. A later clean run therefore retains its
topology retry obligation and can then publish the current snapshot.

After a successful content read returns zero bytes, ordinary indexing now
retracts only that path's derived graph state through the existing
`handle_deleted_file` primitive. Never-indexed empty files remain no-op skips.
Read failures, metadata observations, parse failures, and non-empty files do
not authorize deletion. The public response remains unchanged:
`files_skipped` increments and `files_reconciled` keeps its forced-index-only
meaning.

## Strict-Safety Action Records

### PA-1 — Fail-closed topology snapshot publication

- **ProposedAction:** publish current canonical workspace topology only after a
  clean index result; restore prior state after a partial failure.
- **Targets:** `src/services/code_graph.rs::index_workspace_impl` and its
  disposable integration fixture.
- **ActionRisk:** high because this changes retry certification on the shared
  full-index runtime surface.
- **Approval:** the operator explicitly approved autonomous implementation,
  build/test, CI, PR, and merge activity for 103-S after all gates pass.
- **Approval required:** satisfied for the branch implementation and an exact
  reviewed-HEAD merge; no gate bypass is authorized.
- **ActionResult:** applied on the feature branch by `ac6d21f7`; merge and
  release observation remain pending.
- **Rollback:** revert the release-unit merge. An absent snapshot causes
  conservative recomputation; no source data or schema rollback is needed.

### PA-2 — Authoritative empty-file derived-state eviction

- **ProposedAction:** after a successful zero-byte read, retract derived graph
  state attached to that exact workspace-relative path.
- **Targets:** the ordinary-index per-file loop and disposable integration
  fixtures.
- **ActionRisk:** high because persisted derived rows are deleted, although the
  operation is path-bounded, source files are untouched, and state can be
  rebuilt from source.
- **Approval:** the reviewed shipment and operator directive explicitly
  approve this implementation and merge after all gates pass.
- **Approval required:** satisfied for disposable-fixture execution and the
  exact reviewed-HEAD merge. Ship remains unauthorized to repair, reindex, or
  mutate an operator workspace.
- **ActionResult:** applied on the feature branch by `63f97d30`; merge and
  release observation remain pending.
- **Rollback:** revert the release-unit merge. If a future released binary
  affects a named workspace, provide a target-specific handoff; only the
  operator decides and executes any workspace rebuild.

## Invariants to Preserve

* A partial index never certifies current canonical workspace topology
* The next clean index retains its topology retry obligation
* Successful zero-byte reads retract only the exact indexed path
* Read failure, metadata, parse failure, and non-empty content never authorize
  deletion
* Unchanged sibling records and live edges remain intact
* `files_reconciled` and every CLI, MCP, schema, and wire contract remain
  unchanged

## Runtime Verification

The registered integration suite uses only disposable temporary workspaces and
exercises the production `index_workspace_impl` path:

- one three-phase scenario proves portable invalid-UTF-8 failure retains prior
  topology, restored identical bytes force the retry, and a subsequent clean
  run converges to hash-skip behavior;
- one scenario empties an indexed edge-bearing file while an unchanged sibling
  hash-skips, then proves exact-path code-file, symbol, staged-call, direct,
  resolved, and raw-row teardown without losing the sibling;
- one control proves a never-indexed empty file creates no graph state;
- adjacent package-topology, sync-empty, and oversized-file suites protect
  producer boundaries and existing teardown behavior.

No public test seam, permission-dependent fixture, timing race, sleep, live
daemon, or operator workspace is used. Direct and daemon/MCP callers share the
same private implementation.

## Pre-Deploy Audit Checklist

* Feature flag or rollout gate: not applicable; the private behavior is always
  active once a future binary is released
* Rollback: branch and merge-revert procedures are documented below
* Migration or schema compatibility: no migration or schema change
* Cross-service dependency: none; the direct and daemon/MCP routes share one
  local implementation
* Monitoring: structured log, result fields, snapshot queries, raw-row checks,
  thresholds, owner, and observation window are defined
* Local gates: formatting, strict Clippy, `cargo dev-test`, and targeted
  runtime suites passed; the hybrid hermetic `cargo test --all-targets` gate
  completed successfully at the reviewed branch head

## Observability

| Signal | Healthy | Rollback trigger |
|---|---|---|
| Partial index result | Non-empty `errors`; prior/absent snapshot retained | Current topology certified after any file error |
| Retry liveness | Restored bytes recompute canonical identity | Matching hash suppresses the required retry |
| Clean convergence | Current snapshot publishes; later unchanged file skips | Permanent reparse loop or publication failure |
| Empty-file teardown | Structured debug event for the exact path; no persisted rows remain | Any stale or dangling row survives |
| Accounting | `files_skipped` increments; `files_reconciled` is unchanged | Public counter semantics change |
| Control preservation | Unchanged sibling and live edges remain | Control record or edge disappears |

The structured successful-eviction event is:
`code graph: ordinary index evicted authoritative empty file`, with `path`.
Per-file read failures remain observable through `IndexResult.errors`.

## Migration and Release Boundary

PR #301 merged after the latest release tag, so no released binary contains
the behavior corrected by this shipment. No migration, backfill, automatic
reindex, schema change, or deployed-workspace mutation is required or
authorized. Merging this PR does not publish a binary.

## Deployment and Rollout Path

This is a merge-only change to a locally installed daemon. There is no hosted
deployment, fleet rollout, feature flag, migration, or automatic workspace
repair. A later tagged binary release follows the repository release workflow.
After release, run only the disposable observations in the monitoring plan.

## Resolved Validation Incident

Earlier shared-data all-target attempts were non-hermetic because unrelated
test binaries reused one branch database. A per-test runner proved isolation
but expanded the suite to 1,453 processes and was operationally unsuitable.
The final untracked session runner uses one unique `ENGRAM_DATA_DIR` per test
binary, serializes each binary, and isolates only `contract_evaluation` and
`integration_retrieval_eval_thresholds` per test.

The two allowlisted binaries passed 5/5 and 4/4 respectively, the ordinary
whole-binary proof passed, and the complete hybrid hermetic
`cargo test --all-targets` gate reached exit code zero after the final review
remediations. Process snapshots retained the same seven operator-owned
`C:\Tools\engram.exe` baseline PIDs with zero new target-built or
hermetic-root process leaks. The repository-root `.engram` directory was not
inspected, modified, repaired, deleted, or reindexed.

## Monitoring Plan

- **Pre-merge owner:** Ship.
- **Pre-merge window:** the targeted partial, retry, clean, empty-file, and
  no-op disposable cycles plus ordered repository gates and current-HEAD CI.
- **Post-release owner:** repository operator.
- **Post-release window:** the first three disposable direct or daemon
  ordinary-index cycles after a binary containing the merge is released, or
  seven days from the first cycle, whichever comes first.
- **Healthy release state:** zero unexpected index errors, zero stale/dangling
  rows after authoritative empty reads, one successful retry after a partial
  topology failure, and preserved control edges.

Silence is not success. Post-release closure requires explicit result,
snapshot, raw-row, and control-edge observations. Until a binary release and
that window occur, post-release observation remains pending.

## Rollback

Before merge, reset or revert only the feature-branch commits. After merge,
revert the merge commit with `git revert -m 1 <merge-sha>`, rebuild, and restart
the daemon. Stop release immediately if a live/read-failed file is evicted, a
wrong edge appears, retry state is erased, or control recall falls.

Ship may rebuild only disposable fixture state. Any named operator workspace
requires a separate target-specific handoff and operator-executed action.

## Readiness

**READY FOR PR.** Targeted runtime verification, formatting, strict Clippy,
504/504 library tests, the hybrid hermetic all-target suite, and report-only
review pass. The accepted audit baseline remains one transitive
`RUSTSEC-2026-0041` vulnerability plus 13 allowed warnings with no dependency
change. Exact-HEAD CI, Copilot review, merge, and post-merge observation remain
pending.
