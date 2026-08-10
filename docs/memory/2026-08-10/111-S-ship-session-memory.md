---
title: "111-S Ship session memory"
doc_type: memory
date: 2026-08-10
agent: ship
shipment_id: "111-S"
feature_id: "117-F"
pr: 329
status: closed
---

## Recovery

Ship resumed the retained dirty branch without reclaiming or discarding prior
work. Adaptive workspace logging recovered the output-truncated all-target
failure: S072 expected at least two indexed functions but found zero. Ambient
`ENGRAM_DATA_DIR` had routed its disposable fixture to the live workspace
database. Isolated fixture storage fixed the test, and verbose diagnostic
capture was removed and disabled after resolution.

The policy learning is
`docs/compound/workflow-issues/dynamic-diagnostic-escalation-2026-08-08.md`;
formal instruction follow-up is `241B503F`.

## Delivery

The shipment delivered atomic direct-Sync successor claim, acknowledged and
bounded metrics control, cancellation-safe workspace/branch publication,
origin-workspace metrics routing, no-prior-snapshot recovery, and immutable
SQLITE_BUSY graph-count retries. Three structured review cycles and iterative
Copilot feedback were remediated without widening the reviewed U1-U4 scope.

Formatting, pedantic Clippy, all-target tests, hosted feature-matrix tests,
and hosted CI passed. Audit output exactly matched the accepted
`RUSTSEC-2026-0041` baseline through
`cozo -> swapvec -> lz4_flex 0.10.0`; `017-D` and `27F691AE` remain the
follow-ups.

## Pull Request and Merge

PR #329 received Copilot review `4893142896` on exact approved HEAD
`642a820f8061657c235848a06f93496ee034764a`. Requested reviewers were empty,
all threads were resolved, hosted `build` passed, and merge state was clean.

The PR merged by merge commit only at `2026-08-10T01:34:46Z`. Merge commit
`fd7d02e01566211f8a0a060d1cb8c4d7a2a60396` has parents
`bb22f18320ea4da64650005c2dd8b30add943ca1` and the approved HEAD. No squash,
rebase, bypass, force push, auto-merge, or branch deletion was used.

## Runtime and Closure

Post-merge focused runtime verification passed 35/35 scenarios: 22 write
coordinator, eight metrics lifecycle, one origin-routing, one snapshot
recovery, two immutable SQLITE_BUSY, and one isolated S072 smoke scenario.

Backlogit archived exactly shipment `111-S`, feature `117-F`, plan review
`117.001-R`, and tasks `117.001-T` through `117.004-T` with the merge SHA. It
returned no items. Shipments `112-S`, `113-S`, and `114-S` remain queued.

The pre-established seven-day observation window began with shipment
execution on 2026-08-08 and continues post-merge through 2026-08-15. Rollback
is a reviewed merge-commit revert; no automatic operator-workspace reindex or
repair is authorized.
