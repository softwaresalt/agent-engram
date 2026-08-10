---
title: "111-S index coordinator and observability reliability post-merge closure"
doc_type: closure
source: docs/exec-plans/2026-08-07-index-coordinator-observability-reliability-plan.md
shipment_id: "111-S"
feature_id: "117-F"
mode: post-merge
date: 2026-08-10
author: ship
pr: 329
approved_head: "642a820f8061657c235848a06f93496ee034764a"
merge_commit: "fd7d02e01566211f8a0a060d1cb8c4d7a2a60396"
merged_at: "2026-08-10T01:34:46Z"
releasability: READY
closure_status: READY
compaction_status: done
---

## Readiness

**READY.** PR #329 merged by merge commit
`fd7d02e01566211f8a0a060d1cb8c4d7a2a60396`. The commit is reachable from
`origin/main`, has exactly two parents
(`bb22f18320ea4da64650005c2dd8b30add943ca1` and approved HEAD
`642a820f8061657c235848a06f93496ee034764a`), and therefore includes the exact
reviewed release head.

Backlogit archived exactly shipment `111-S`, feature `117-F`, tasks
`117.001-T` through `117.004-T`, and plan review `117.001-R` with the merge
SHA. It returned no items. Later ordered-batch shipments `112-S`, `113-S`, and
`114-S` remain queued and unchanged.

## Quality, Audit, and Review Evidence

| Gate | Result |
|---|---|
| Formatting | PASS — `cargo fmt --all -- --check` |
| Rust lint | PASS — `cargo clippy --all-targets -- -D warnings -W clippy::pedantic` |
| Repository tests | PASS — `cargo test --all-targets` |
| Hosted feature matrix | PASS — no-default-features Cozo/embeddings all-target tests |
| Dependency audit | ACCEPTED BASELINE — exactly `RUSTSEC-2026-0041` through `cozo 0.7.6 -> swapvec 0.3.0 -> lz4_flex 0.10.0`, the same 13 informational warnings, and no dependency-source changes |
| Structured review | PASS — three bounded cycles ended with no P0/P1 |
| Copilot review | PASS — review `4893142896` on exact approved HEAD |
| Reviewer lifecycle | PASS — Copilot absent from requested reviewers |
| Review threads | PASS — zero unresolved |
| Hosted CI | PASS — `build` |
| Merge state | PASS — `CLEAN` |

The audit exception is the previously accepted upstream-pinned repository
baseline, not a new waiver. Deliberation `017-D` and follow-up `27F691AE`
remain the owners of dependency remediation. No additional vulnerability,
unmaintained-crate delta, or `Cargo.toml`/`Cargo.lock` change was accepted.

## Diagnostic Recovery

The initially hidden all-target failure was
`integration_smoke::s072_workspace_status_reports_code_graph_counts`:
`fixture must index at least two functions, got 0`. Ambient
`ENGRAM_DATA_DIR=C:\Source\GitHub\engram\.engram` redirected the disposable
fixture into the live workspace database. S072 now binds isolated
workspace-local storage.

Complete combined output was temporarily captured under repository `logs/`,
the bounded failing section was inspected, and validation returned to normal
verbosity after the fix. The diagnostic log was removed after resolution.
Institutional guidance is recorded in
`docs/compound/workflow-issues/dynamic-diagnostic-escalation-2026-08-08.md`;
formal workflow wording is follow-up `241B503F`.

## Post-Merge Runtime Verification

All probes used test fixtures or disposable databases. No operator workspace
was reindexed or repaired.

| Surface | Result |
|---|---|
| U1 write coordinator and exact-work recovery | 22/22 PASS |
| U2 metrics lifecycle and bounded controls | 8/8 PASS |
| U2 origin-workspace routing | 1/1 PASS |
| U3 no-prior-snapshot recovery | 1/1 PASS |
| U4 transient/persistent immutable SQLITE_BUSY counts | 2/2 PASS |
| S072 isolated workspace graph counts | 1/1 PASS |
| Total focused runtime scenarios | 35/35 PASS |

The probes cover atomic direct-Sync successor claim, exact busy/queued
responses, cancellation and timeout rollback, bounded terminal recovery,
acknowledged metrics controls, unavailable/stalled writers, private-token
reversal, stale generations, origin-workspace routing, invalid-UTF-8 recovery,
and bounded immutable graph-count retries.

## Operational Monitoring

Ship/operator owns the pre-established seven-day observation window that
started with shipment execution on 2026-08-08 and continues post-merge through
2026-08-15.

Use the deployment's normal tracing sink and a once-daily workspace-status
check. Healthy operation has zero occurrences of:

- `detached branch refresh rollback exhausted retries`;
- `terminal branch rollback could not mark metrics unavailable`;
- `metrics writer is unavailable for branch control`;
- `metrics_event_dropped_workspace_mismatch`; and
- metrics lock/control `timed out` errors.

For each monitored workspace with known graph content, run
`engram workspace-status --workspace PATH --format json`. Function, class, and
interface counts must remain consistent with the indexed project and must not
persist at zero. The
`SQLITE_BUSY retry: retrying immutable run_script` warning may occur
transiently, but five attempts, a returned database error, or the same
workspace reporting zero counts on two consecutive checks is an alert.

Intervention is required for duplicate owners, lost routine work, a stalled
metrics writer, cross-workspace metrics persistence, or persistent zero
counts. Preserve database and bounded diagnostics for investigation.

## Rollback

Rollback is a merge-commit revert through a reviewed pull request. Do not
force-push, rewrite history, automatically reindex an operator workspace, or
repair live data as part of rollback.

## Reconciliation and Knowledge

- Compacted context:
  `docs/closure/2026-08-10-111-s-compact-context.md`
- Session memory:
  `docs/memory/2026-08-10/111-S-ship-session-memory.md`
- Diagnostic checkpoint:
  `docs/memory/2026-08-08/circuit-break-cargo-test-all-targets.md`
- Audit checkpoint:
  `docs/memory/2026-08-08/111-s-audit-gate-blocked.md`
- Compound learning:
  `docs/compound/workflow-issues/dynamic-diagnostic-escalation-2026-08-08.md`
