---
title: "110-S fail-closed source reconciliation post-merge closure"
doc_type: closure
source: docs/exec-plans/2026-08-07-fail-closed-source-reconciliation-plan.md
shipment_id: "110-S"
feature_id: "116-F"
mode: post-merge
date: 2026-08-08
author: ship
pr: 327
approved_head: "fb0fc89b0e5a3e1d28c7b8f3c0b2f1a9cc435319"
merge_commit: "b7c9fa1b5ba4fc2d3dca36e2069b6ef669969793"
merged_at: "2026-08-08T06:36:01Z"
releasability: READY
closure_status: READY
compaction_status: done
---

## Readiness

**READY.** PR #327 merged by merge commit
`b7c9fa1b5ba4fc2d3dca36e2069b6ef669969793`. The commit is reachable from
`origin/main`, has exactly two parents, and includes approved HEAD
`fb0fc89b0e5a3e1d28c7b8f3c0b2f1a9cc435319` as its second parent.

Shipment `110-S`, feature `116-F`, and tasks `116.001-T` through `116.005-T`
are archived with the merge SHA. Shipment closure returned no items. Later
same-batch shipments `111-S` through `114-S` remain queued and unchanged.

## Quality and Review Evidence

| Gate | Result |
|---|---|
| Formatting | PASS — `cargo fmt --all -- --check` |
| Rust lint | PASS — pedantic Clippy with warnings denied |
| Repository tests | PASS — isolated `cargo test --all-targets` |
| Dependency audit | PASS with explicit `RUSTSEC-2026-0041` ignore; `cozo 0.7.6 -> swapvec 0.3.0` prevents a compatible `lz4_flex` upgrade |
| Structured review | PASS after three cycles; final Rust and scope reviews reported no P0/P1 |
| Copilot review | PASS on exact approved HEAD |
| Copilot reviewer lifecycle | PASS — requested reviewer absent |
| Review threads | PASS — two resolved, zero unresolved |
| Hosted CI | PASS — `build` |
| Merge state before merge | PASS — `clean` |

No repository audit-policy exception was added. Existing advisory follow-up
`017-D` remains outside this shipment.

## Post-Merge Runtime Verification

All probes used disposable workspaces or test databases. No operator workspace
was reindexed or repaired.

| Surface | Result |
|---|---|
| Shared checked traversal/reconciler | 12/12 PASS |
| PBIP materialization units | 11/11 PASS |
| PBIP DB sweep controls | 3/3 PASS |
| Backlog materialization units | 2/2 PASS |
| Backlog DB sweep controls | 3/3 PASS |
| Notebook snapshot/sweep units | 9/9 PASS |
| Power BI snapshot/marker/sweep units | 19/19 PASS |
| Total focused runtime scenarios | 59/59 PASS |
| Windows directory-symlink capability | PASS — fixture assertions were supported |

Observed destructive-control outcomes were bounded: unavailable or incomplete
passes removed `0`; physical absence removed `1`; complete alias supersession
removed `1`. Notebook and Power BI carried-snapshot controls removed `0` after
post-collection path changes. Power BI incomplete TMDL/JSON controls retained
both last-known-good content and completion markers. Backlog extraction and
PBIP materialization failures retained last-known-good state.

The fail-closed warning branches are tied to these induced incomplete-pass
controls. No unexpected warning or live-control removal occurred.

## Operational Monitoring

Ship owns a seven-day observation window through 2026-08-15. On normal source
ingestion, monitor:

- per-source removed-count deltas;
- fail-closed collection/materialization warning rate;
- missing Power BI completion markers after successful passes; and
- duplicate alias records that persist after a later authoritative pass.

Healthy behavior is zero removal on unavailable/incomplete passes, bounded
removal only for proven absence or complete alias supersession, and cleanup of
harmless retained duplicates on the next authoritative pass.

Intervention is required for any live-control loss, out-of-workspace probe,
source-wide removal during an incomplete pass, or evidence of a second
traversal within one ingestion operation.

## Rollback

Rollback is a merge-commit revert through a reviewed pull request. Do not
force-push, rewrite history, automatically reindex an operator workspace, or
attempt workspace repair as part of rollback. Preserve the database for
diagnosis and retain last-known-good rows.

## Reconciliation and Knowledge

- Pre-claim continuity:
  `.backlogit/checkpoints/checkpoint-20260808-030834.json`
- Compacted context:
  `docs/closure/2026-08-08-110-s-compact-context.md`
- Session memory:
  `docs/memory/2026-08-08/110-S-ship-session-memory.md`
- Compound learning:
  `docs/compound/destructive-reconciliation-needs-a-materialized-snapshot-2026-08-08.md`
