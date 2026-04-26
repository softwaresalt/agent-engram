---
title: "backlogit_ship_shipment creates only the shipment archive file, not individual item archive files"
description: "backlogit_ship_shipment (MCP tool) moves the shipment record to archive but does NOT create individual archive files for feature/task manifest items — they must be created manually. NOTE: the CLI backlogit shipment ship DOES create individual archive files (observed in 012-S, 2026-04-26)."
problem_type: "shipment archive drift"
category: "workflow-issues"
component: "backlogit shipment closure"
root_cause: "backlogit_ship_shipment marks manifest items as done in its internal registry and deletes their queue files, but does not write individual .backlogit/archive/{item-id}.md files for features and tasks — only the shipment file itself is created on disk."
resolution_type: "workaround"
severity: "high"
message: "post-mode reconcile: ARCHIVE_MISSING for manifest items after backlogit_ship_shipment — only {shipment-id}.md appears under .backlogit/archive/"
file_path: ".backlogit/archive/"
stale_note: "Potentially partially stale. As of 2026-04-26 (shipment 012-S), the CLI command `backlogit shipment ship` DID create individual archive files (033.002-T.md, 033.003-T.md, 012-S.md) as new untracked files. This entry's behavior was observed via the MCP tool `backlogit_ship_shipment` in 001-S. The MCP vs CLI behavior may differ, or the tool may have been updated. Verify before creating archive files manually when using the CLI."
citations:
  - "docs/closure/2026-04-23-001-S-toctou-fix-closure.md"
  - "docs/compound/workflow-issues/ship-shipment-overscoped-manifest-2026-04-20.md"
  - "shipment 001-S post-merge closure, 2026-04-23"
  - "shipment 012-S post-merge closure, 2026-04-26 (CLI: individual archive files WERE created)"
tags:
  - "backlogit"
  - "shipment"
  - "archive"
  - "workflow"
  - "post-merge"
---

## Problem

After calling `backlogit_ship_shipment("001-S", merge_sha)`:

- `.backlogit/archive/001-S.md` was created (untracked, new) ✅
- `.backlogit/queue/001-S.md`, `024-F.md`, `024.001-T.md`, `024.002-T.md` were deleted ✅
- `.backlogit/archive/024-F.md`, `024.001-T.md`, `024.002-T.md` were NOT created ❌

The post-mode reconcile check (`ARCHIVE_MISSING: 024-F`, etc.) flagged the
gap. The items were marked `done` in backlogit's internal registry, but had
no on-disk representation under `.backlogit/archive/`.

This behavior is distinct from the P-007 "archive deletion quirk" (where
backlogit creates then immediately deletes archive files). In this case, the
individual item archive files were simply never written.

## Root Cause

`backlogit_ship_shipment` writes exactly one archive file per shipment run —
the shipment manifest file (`{shipment-id}.md`). Individual feature and task
files in the manifest are removed from the queue directory but are not
individually written to the archive directory.

This is consistent with the behavior documented in
`docs/compound/workflow-issues/ship-shipment-overscoped-manifest-2026-04-20.md`
("Move each item's markdown file to `.backlogit/archive/` individually" was
listed as a PROPOSED fix to the upstream tool, not current behavior).

## Resolution

After `backlogit_ship_shipment` runs and the post-mode reconcile reports
`ARCHIVE_MISSING` for feature/task items:

1. For each missing item, use `backlogit_get_item(id)` to retrieve the full
   item data from backlogit's internal registry.
2. Create `.backlogit/archive/{id}.md` with proper YAML frontmatter,
   including `status: done`. Use an existing archive file as format reference
   (e.g., `.backlogit/archive/023-F.md`).
3. Re-run the post-mode archive check to confirm all items are present.
4. Stage and commit all archive files together:
   ```bash
   git add .backlogit/
   git commit -m "chore: archive {shipment-id} backlog artifacts post-merge"
   ```

Note: `git add .backlogit/` will detect the queue→archive renames even when
the archive files were created manually (Git's rename detection threshold ~50%
similarity is usually met by frontmatter-only changes).

## Prevention

- Always run the shipment-reconcile skill in `mode: post` after
  `backlogit_ship_shipment` to detect missing archive files immediately.
- Treat `backlogit_ship_shipment` as only archiving the shipment manifest
  file on disk. Any feature/task archive files must be created separately.
- The workaround (`backlogit_get_item` → create file) is reliable as long as
  backlogit's internal registry remains intact after the ship command.
- Do NOT commit the post-merge backlogit state until all manifest items have
  verified archive files on disk.

## Related

- `docs/compound/workflow-issues/ship-shipment-overscoped-manifest-2026-04-20.md`
  — the upstream tool validation gap that causes this behavior
- `docs/compound/workflow-issues/backlogit-shipment-ship-force-releases-covering-feature-2026-04-22.md`
  — related force-release of covering features
- `.github/skills/shipment-reconcile/SKILL.md` — the post-mode check that
  catches this issue before committing
