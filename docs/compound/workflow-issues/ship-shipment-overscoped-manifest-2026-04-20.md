---
date: 2026-04-20
category: workflow-issues
severity: critical
incident_artifact: shipment 003-S
trigger_commit: d663b77
discovery_session: post-merge audit by operator
---

# `backlogit_ship_shipment` archives unbuilt items, deletes them from disk

## Symptoms

After running `backlogit_ship_shipment("003-S", merge_sha)` for a shipment whose manifest was assembled at harvest time but only partially executed:

1. The archived shipment manifest claims 50 items shipped
2. Only 23 items were actually built (Phases 1-2 of an 8-phase plan)
3. The other 27 markdown files (Phases 4-8 chores + tasks + 1 deferred Phase 1 task) were **deleted from disk** without being moved to `.backlogit/archive/`
4. Discovery only happens during post-hoc audit — the manifest looks legitimate
5. The completed 001.003-C chore parent was left orphaned in queue while its 10 child tasks were correctly archived during incremental work

## Root Cause

`backlogit_ship_shipment` treats the shipment manifest as the source of truth for what to archive. It does not cross-check:

* whether each manifest item's underlying work was actually completed (tasks moved through `done` status)
* whether each manifest item's markdown file exists in `.backlogit/queue/` or `.backlogit/archive/`

The tool's behavior:

1. For each item ID in `custom_fields.items`, look for the file in queue/
2. If found, delete from queue/ (intent: move to archive)
3. Write a single archive entry for the **shipment** itself (`003-S.md`)
4. Does NOT individually move each item's markdown to archive
5. Does NOT verify item status before archiving

Result: queue files vanish from disk; only the shipment manifest survives. Items that were never `done` get silently "shipped".

## Compounding Factor: Stage's Speculative Harvest

The Stage agent harvests the **entire plan** into a shipment at planning time. For multi-phase migrations, this means Phases 4-8 entered the manifest before Phase 1 even started. Ship then executed only the front portion of the plan — the manifest was never re-scoped to match what actually shipped.

## Detection Strategy

Before trusting `backlogit_ship_shipment` results, run a reconciliation check:

```powershell
# For each manifest item, verify it exists in queue OR archive (not nowhere)
$manifest = Get-Content .backlogit/archive/{shipment_id}.md | ConvertFrom-Yaml
foreach ($id in $manifest.custom_fields.items) {
    $queue = Test-Path ".backlogit/queue/$id.md"
    $archive = Test-Path ".backlogit/archive/$id.md"
    if (-not $queue -and -not $archive) {
        Write-Warning "MISSING: $id in shipment $($manifest.id) has no file"
    }
}
```

Even better: cross-check item `status` in the markdown file. If status is not `done`, it should not have been in the shipment.

## Workaround / Recovery (proven)

When discovered post-hoc:

1. Identify the `pre-ship` commit (last commit before `backlogit_ship_shipment` ran)
2. For each missing item, restore from that commit:
   ```powershell
   git checkout {pre-ship-sha} -- ".backlogit/queue/{item-id}.md"
   ```
3. Move parents whose children all completed to archive manually
4. Update the shipment manifest's `items:` list to include only items that genuinely shipped
5. Add a `reconciliation_note:` field to the manifest for traceability

Recovery for shipment 003-S: 27 files restored, 1 parent (001.003-C) moved queue → archive, manifest reduced from 50 → 23 items.

## Prevention (proposed feature work)

The `backlogit_ship_shipment` tool MUST:

1. Before deleting any queue file, verify the item's `status: done` in the file frontmatter
2. For items in the manifest with `status: queued` or `status: blocked`, REMOVE them from the manifest before archiving the shipment (do not delete their files)
3. Move each item's markdown file to `.backlogit/archive/` individually (not just delete from queue)
4. Emit a per-item action log: `[archived|skipped-not-done|missing] {id}`
5. Refuse to archive if any manifest item file is missing from disk (raise integrity error)

The Stage agent's harvest-into-shipment behavior should also be revisited:

* Either harvest only the immediate next phase into the shipment (incremental shipments per phase)
* Or treat over-scoped manifests as the norm and require Ship to prune the manifest at merge time based on actual completion status

## Related Compound Learnings

* `docs/compound/best-practices/pub-visibility-for-external-test-harness-2026-04-20.md`
* The P-007 archive-deletion workaround documented in repo memory (run `git restore .backlogit/archive/` after `backlogit_ship_shipment`) is **still valid** but only addresses the corruption of pre-existing archive files. It does NOT address the new bug where queue files are deleted instead of archived.

## Action Items

* Backlog item created: `fix backlogit_ship_shipment manifest reconciliation` (status validation + per-item archive + integrity check)
* Backlog item created: `add Ship pre-archive reconciliation gate` (verify manifest items match what actually shipped before calling backlogit_ship_shipment)
* Update Ship agent post-merge protocol: add explicit reconciliation step before `backlogit_ship_shipment`
