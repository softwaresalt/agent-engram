---
date: 2026-04-26
shipment: 012-S
scope: workflow-issues
mode: propose+apply
---

# Compound Refresh — 012-S Post-Merge

## Entries Reviewed

| Entry | Classification | Rationale |
|---|---|---|
| ship-shipment-overscoped-manifest-2026-04-20.md | **update** | New variant observed in 012-S: ship auto-expands children of parent items in manifest, blocking ship when a child is locked. Workaround confirmed. |
| ship-shipment-no-item-archive-files-2026-04-23.md | **keep (flag stale)** | Individual archive files WERE created by acklogit shipment ship in 012-S. Behavior may have changed in newer CLI versions. Flagged as potentially stale. |
| acklogit-shipment-ship-force-releases-covering-feature-2026-04-22.md | **keep** | Still accurate. Our workaround (remove parent from manifest) worked correctly in 012-S — 033-C not in archived_ids. |
| All other entries | **keep** | Not in scope of 012-S changes. |

## New Entry

Created: docs/compound/workflow-issues/backlogit-ship-blocked-child-expansion-2026-04-26.md

Finding: acklogit shipment ship auto-expands children of parent items in the manifest. If any child task is locked, the ship command fails even if that child is not in the manifest items list. Workaround: remove the parent from manifest and pre-archive it via acklogit move {id} --status done.

## Files Modified

- Created new compound entry (above)
- ship-shipment-no-item-archive-files-2026-04-23.md: added stale frontmatter note

## Follow-Up

None — all classifications are evidence-backed.
