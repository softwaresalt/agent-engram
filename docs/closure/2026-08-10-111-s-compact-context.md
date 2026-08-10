---
title: "111-S compacted closure context"
doc_type: closure
source: docs/closure/111-S-2026-08-10-post-merge-closure.md
date: 2026-08-10
shipment_id: "111-S"
status: done
---

Shipment `111-S` released feature `117-F` and tasks `117.001-T` through
`117.004-T` in PR #329. Exact approved HEAD
`642a820f8061657c235848a06f93496ee034764a` merged as two-parent commit
`fd7d02e01566211f8a0a060d1cb8c4d7a2a60396`.

The release atomically claims direct-Sync successor work, makes metrics
lifecycle controls acknowledged and bounded while preserving droppable
ordinary events, routes events by originating workspace, restores coherent
state after cancelled workspace/branch publication, retries immutable graph
count reads on SQLITE_BUSY, and preserves no-prior-snapshot recovery.

The hidden all-target failure was S072 reporting zero functions because
ambient `ENGRAM_DATA_DIR` contaminated its disposable fixture. Workspace-log
diagnostic escalation exposed the failure; isolated storage fixed it. The
reusable escalation/de-escalation policy is in
`docs/compound/workflow-issues/dynamic-diagnostic-escalation-2026-08-08.md`,
with formal follow-up `241B503F`.

Formatting, pedantic Clippy, all-target tests, hosted feature-matrix tests,
three review cycles, exact-HEAD Copilot review, hosted CI, and 35/35 focused
post-merge scenarios passed. Audit output exactly matched the accepted
`RUSTSEC-2026-0041` upstream-pinned baseline; owners remain `017-D` and
`27F691AE`.

Backlogit archived exactly `111-S`, `117-F`, `117.001-R`, and tasks
`117.001-T` through `117.004-T` with merge evidence. Shipments `112-S` through
`114-S` remain queued. The coordinator/metrics/graph-count observation window
ends 2026-08-15.
