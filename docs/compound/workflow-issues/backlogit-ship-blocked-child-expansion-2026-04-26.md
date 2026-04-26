---
title: "backlogit shipment ship auto-expands children of manifest parent items"
description: "When a parent item is in the shipment manifest, backlogit shipment ship expands to all child tasks of that parent — including children not in the manifest. If any child is blocked, the ship command fails."
problem_type: "shipment archive conflict"
category: "workflow-issues"
component: "backlogit shipment closure"
root_cause: "backlogit shipment ship derives release scope from the parent-child relationship, not solely from custom_fields.items. Any blocked child of a manifest parent causes a status-conflict failure."
resolution_type: "workaround"
severity: "high"
message: "ship shipment: complete release scope: item {id} is blocked and cannot ship: backlogit: shipment status conflict"
file_path: ".backlogit/queue/{shipment-id}.md"
citations:
  - "docs/closure/2026-04-25-012-S-closure.md"
  - ".backlogit/reconcile/012-S-pre-20260426T103030.md"
  - "shipment 012-S post-merge closure, 2026-04-26"
tags:
  - "backlogit"
  - "shipment"
  - "archive"
  - "workflow"
  - "blocked"
  - "post-merge"
---

## Problem

During post-merge closure for shipment `012-S`, the reconciled manifest contained
`["033-C", "033.002-T", "033.003-T"]`. Task `033.001-T` was intentionally excluded
because it is a blocked operator-action item (disable rebase merge in GitHub settings).

Despite being excluded from the manifest, `backlogit shipment ship 012-S` failed:

```
Error: ship shipment: complete release scope: item 033.001-T is blocked and cannot ship:
backlogit: shipment status conflict
```

This happened because `033.001-T` has `parent_id: 033-C`, and `033-C` was in the
manifest. The ship command auto-expanded `033-C`'s children and attempted to ship
`033.001-T` regardless of its absence from `custom_fields.items`.

## Root Cause

`backlogit shipment ship` resolves release scope by combining:
1. The manifest `custom_fields.items` list
2. All child items of any parent item in the manifest

This means pruning a blocked child from the manifest is insufficient if its parent
remains in the manifest. The tool walks the parent→child hierarchy independently of
the manifest items list.

## Resolution (proven — 012-S)

### Option A: Remove the parent from the manifest (recommended)

1. Edit `.backlogit/queue/{shipment-id}.md` — remove the parent ID from `custom_fields.items`
2. Pre-archive the parent separately via:
   ```
   backlogit move {parent-id} --status done
   ```
   This moves the parent to `.backlogit/archive/` directly.
3. Run `backlogit sync` to refresh the index
4. Re-run `backlogit shipment ship` — the tool no longer expands children for the
   removed parent

In 012-S: removed `033-C` from manifest, ran `backlogit move 033-C --status done`
(which auto-archived it), then shipped `012-S` with manifest `["033.002-T", "033.003-T"]`.
Result: ship succeeded, `archived_ids: ["033.002-T", "033.003-T", "012-S"]`.

### Option B: `return-blocked` (only works if item is in manifest)

`backlogit shipment return-blocked --shipment {id} --item {blocked-item-id}` removes
a blocked item from a shipment before shipping. However, this only works if the blocked
item is listed in the manifest. It does **not** work for child-expanded items that are
absent from the manifest — the command returns "cannot return item from shipment".

## Prevention

When a shipment manifest includes a parent item that has blocked or incomplete child tasks:

1. Do NOT include the parent in the manifest if any of its children are excluded.
2. Instead, pre-archive the parent via `backlogit move {id} --status done` before
   calling `backlogit shipment ship`, and remove it from the manifest items list.
3. The shipment-reconcile pre-mode check will classify the parent as `pre-archived`
   (valid — not an error).

## Related

- `docs/compound/workflow-issues/ship-shipment-overscoped-manifest-2026-04-20.md`
  — upstream validation gap; parent-child expansion is a related behavior
- `docs/compound/workflow-issues/backlogit-shipment-ship-force-releases-covering-feature-2026-04-22.md`
  — related: ship force-releases covering features via the same expansion logic
- `.github/skills/shipment-reconcile/SKILL.md` — pre-mode check detects orphan items
