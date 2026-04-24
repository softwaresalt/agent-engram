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

## Resolution

This compound learning has been operationalized as a reusable harness skill:

**Skill**: `.github/skills/shipment-reconcile/SKILL.md`

The skill provides a GI/GR double-entry reconciliation check as a compensating
control for the missing `backlogit_ship_shipment` per-item validation:

* `mode: pre` — runs before `backlogit_ship_shipment`; verifies every manifest
  item is present in queue with `status: done`; detects orphan items; halts on
  any discrepancy (`RECONCILE_FAIL`)
* `mode: post` — runs after archive + restore; verifies archive integrity and
  detects the known file-deletion quirk

### Amended Agent Steps

* **Stage Step 5.5 / step 3** (`.github/agents/stage.agent.md`): A new scope
  guard (step 3.0) requires that `backlogit_add_to_shipment` is ONLY called for
  items emitted by the current harvest invocation (`harvest_ids`). Pre-existing
  queue items must be excluded.

* **Ship Step 0.5** (`.github/agents/ship.agent.md`): An intake reconciliation
  check (`mode: pre`, `expected_status: queued`) now runs after shipment claim,
  catching Stage-side over-inclusion before any build work begins.

* **Ship Step 6** (`.github/agents/ship.agent.md`): Pre-archive and post-archive
  reconciliation gates (`mode: pre`/`mode: post`) wrap the
  `backlogit_ship_shipment` call and archive restore step.

### Upstream Escalation

An issue draft for the `backlogit` maintainers is at:
`docs/upstream/backlogit-ship-shipment-validation-2026-04-20.md`

### Delivery

Shipped in shipment `004-S` via branch `chore/004-s-shipment-integrity`.
Implementation plan: `docs/exec-plans/2026-04-20-shipment-integrity-plan.md`

## Action Items

> **004-S delivered** (shipment `004-S`, merge commit `86b468511b92b2ac8f2ad6bbb9fc0f2f7e85b4ec`):

* ✅ `add Ship pre-archive reconciliation gate` — delivered as `shipment-reconcile` skill + Ship Step 6 gates
* ✅ `Update Ship agent post-merge protocol` — Ship Step 6 now has pre/post reconciliation wrapping `backlogit_ship_shipment`
* ✅ Stage scope guard — Stage Step 5.5/step 3.0 prevents over-inclusion at harvest assembly time
* ✅ `fix backlogit_ship_shipment manifest reconciliation` — upstream issue submitted as
  [softwaresalt/backlogit#63](https://github.com/softwaresalt/backlogit/issues/63) in shipment 010-S
* ✅ `pre-archived classification spec gap` — `shipment-reconcile` SKILL.md and schema updated in
  shipment 010-S to add `pre-archived` as a distinct classification (separate from `matched`),
  covering items already archived by a prior shipment; `expected_status: active` added as valid
  enum for intake checks on already-claimed shipments (commit `0897e22`)
