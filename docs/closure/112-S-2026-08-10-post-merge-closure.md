---
title: "112-S Spark lineage and parser correctness post-merge closure"
doc_type: closure
source: docs/exec-plans/2026-08-07-spark-lineage-parser-correctness-plan.md
shipment_id: "112-S"
feature_id: "115-F"
mode: post-merge
date: 2026-08-10
author: ship
pr: 331
approved_head: "581ec15a799afe5f590aaef9951f3e1b6283f486"
merge_commit: "5db11650aea6e36f286765e3890723f4bc770cd6"
merged_at: "2026-08-10T05:03:18Z"
releasability: READY
closure_status: READY
compaction_status: done
---

## Readiness

**READY.** PR #331 merged by merge commit
`5db11650aea6e36f286765e3890723f4bc770cd6`. The commit is reachable from
`origin/main` and has exactly two parents:
`1d16fa22c6d3dba5fa9636f920da0884966d985e` and exact approved HEAD
`581ec15a799afe5f590aaef9951f3e1b6283f486`.

Backlogit archived shipment `112-S`, feature `115-F`, tasks `115.001-T`
through `115.004-T`, and plan review `115.001-R` with the merge SHA. It
returned no items. Later ordered-batch shipments `113-S` and `114-S` remain
queued and unchanged.

## Quality, Audit, and Review Evidence

| Gate | Result |
|---|---|
| Formatting | PASS — `cargo fmt --all -- --check` |
| Rust lint | PASS — `cargo clippy --all-targets -- -D warnings -D clippy::pedantic` |
| Repository tests | PASS — `cargo test --all-targets` on final approved code |
| Constitutional test gate | PASS — `cargo dev-test` |
| Dependency audit | ACCEPTED BASELINE — exactly `RUSTSEC-2026-0041` through `cozo 0.7.6 -> swapvec 0.3.0 -> lz4_flex 0.10.0`, the same 13 allowed warnings, and no dependency changes |
| Structured review | PASS — P0 0, P1 0, P2 0, P3 0 |
| Copilot review | PASS — review `4893806023` on exact approved HEAD |
| Reviewer lifecycle | PASS — Copilot absent from requested reviewers |
| Review threads | PASS — zero unresolved |
| Hosted CI | PASS — `build` |
| Merge state | PASS — `CLEAN` |

The first Copilot cycle found two threads. The bare-CR Spark
`SIMPLE_COMMENT` boundary was valid and fixed test-first in `8b2cb796`.
The requested automatic extractor-version invalidation was declined because
the reviewed rollout contract intentionally preserves same-version skipping
and requires an explicit operator checkpoint for any released-workspace
reindex. Both threads were answered and resolved before the second
current-HEAD review.

The audit exception is the existing upstream-pinned repository baseline, not
a new waiver. `Cargo.toml` and `Cargo.lock` did not change.

## TDD and Runtime Verification

Observed RED evidence preceded both parser behavior changes:

- nested and backslash-LF-commented SQL rewrote protected `INSERT` text;
- Python read-write-read-write reuse emitted one of two valid edges, while a
  post-invalidation valid Spark read emitted zero of one;
- the review-added bare-CR control swallowed a genuine `INSERT`.

U3 was behavior-neutral unreachable-guard cleanup with pre/post
characterization. U4 was a test-only current-stamp control and was green on
addition against the existing correct skip behavior.

Post-merge probes used repository fixtures and disposable databases only. No
operator workspace was reindexed.

| Surface | Result |
|---|---|
| Spark SQL exact comment and edge controls | 10/10 PASS |
| Python exact read/write and fail-closed controls | 27/27 PASS |
| Extractor-version rollout control | 1/1 PASS |
| Engram CLI smoke | PASS — `cargo run -- --help` |
| Daemon health | PASS — all checks green |
| Total focused runtime scenarios | 38/38 PASS |

No actionable failure was hidden by tool-output truncation. The all-target
logs exceeded the transport preview limit but completed successfully, so the
dynamic diagnostic escalation protocol correctly remained inactive.

## Operational Monitoring

Ship/operator owns a seven-day observation window through 2026-08-17.
Observe lineage edge deltas and parser-error diagnostics for workspaces that
contain Spark SQL or PySpark notebooks. Healthy operation requires:

- no edge from `INSERT` text inside nested, backslash-LF-continued, CRLF, or
  bare-CR comments;
- both exact edges for read-write-read-write reuse;
- preserved one-read/multi-write fan-out; and
- no edge from unresolved, ambiguous, or nested Python events.

Any new false edge or loss of an established recall control is a rollback
trigger. Preserve the affected notebook, exact edge set, and bounded parser
diagnostics for investigation.

## Operator Reindex Handoff

No automatic workspace reindex was run or introduced. Existing persisted
lineage remains unchanged until the operator explicitly chooses to reprocess
an exposed workspace.

If a released workspace contains affected notebooks and historical correction
is required, the operator may approve a workspace-specific forced pass:

```text
engram sync --full --force --workspace <TARGET_WORKSPACE> --format json
```

Run it only against the named workspace after preserving the pre-pass edge
set. Stop if a false edge appears or an established recall control is lost.

## Rollback and Reconciliation

Rollback is a merge-commit revert through a reviewed pull request. Do not
squash, rebase, force-push, automatically reindex an operator workspace, or
repair live lineage as part of rollback. A corrective reindex remains a
separate operator-directed action.

- Compacted context:
  `docs/closure/2026-08-10-112-s-compact-context.md`
- Session memory:
  `docs/memory/2026-08-10/112-S-ship-session-memory.md`
- Compound learning:
  `docs/compound/best-practices/spark-simple-comment-line-endings-2026-08-10.md`
