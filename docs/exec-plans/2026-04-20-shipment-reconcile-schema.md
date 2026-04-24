---
title: Reconciliation Report Schema — shipment-reconcile skill
date: 2026-04-20
type: schema-spec
related_skill: .github/skills/shipment-reconcile/SKILL.md
related_plan: docs/exec-plans/2026-04-20-shipment-integrity-plan.md
---

# Reconciliation Report Schema

This document defines the exact Markdown structure of the reconciliation report
produced by the `shipment-reconcile` skill. Reports are stored at:

```
.backlogit/reconcile/{shipment_id}-{mode}-{YYYYMMDD-HHMMSS}.md
```

## YAML Frontmatter

```yaml
---
shipment_id: "004-S"
mode: pre                          # pre | post
timestamp: "2026-04-20T14:32:00Z" # ISO 8601
expected_status: done              # pre-mode only; omit for post-mode; queued | active | done
merge_commit_sha: ""               # post-mode only; git SHA of the merge commit
summary:
  total: 13
  matched: 13
  pre_archived: 0
  missing: 0
  status_mismatch: 0
  orphan: 0
recommendation: PROCEED            # PROCEED | HALT — operator reconcile required | HALT — restore archives
---
```

### Field Definitions

| Field | Type | Required | Description |
|---|---|---|---|
| `shipment_id` | string | always | The shipment being reconciled (e.g. `004-S`) |
| `mode` | enum | always | `pre` or `post` |
| `timestamp` | ISO 8601 | always | When the reconciliation ran |
| `expected_status` | enum | pre-mode only | `queued` (fresh intake), `active` (intake when shipment already claimed), or `done` (pre-ship check) |
| `merge_commit_sha` | git SHA | post-mode only | SHA of the merge commit that closed the PR |
| `summary.total` | int | always | Total items in the manifest |
| `summary.matched` | int | always | Items that passed both presence and status checks |
| `summary.pre_archived` | int | always | Items with no queue file but an existing archive file; treated as valid |
| `summary.missing` | int | always | Items with no queue or archive file |
| `summary.status_mismatch` | int | always | Items present but with wrong status |
| `summary.orphan` | int | always | Queue items declaring this shipment_id but absent from manifest |
| `recommendation` | enum | always | `PROCEED`, `HALT — operator reconcile required`, or `HALT — restore archives` |

## Body: Items Table

```markdown
## Items

| ID | Classification | Queue Path | Archive Path | Declared Status | Expected Status |
|---|---|---|---|---|---|
| 002-C        | matched        | .backlogit/queue/002-C.md        | —            | done  | done |
| 002.001-T    | matched        | .backlogit/queue/002.001-T.md    | —            | done  | done |
| 002.002-T    | missing        | —                                 | —            | —     | done |
| 002.003-T    | status-mismatch| .backlogit/queue/002.003-T.md    | —            | queued| done |
| 002.004-T    | pre-archived   | —                                 | .backlogit/archive/002.004-T.md | done  | done |
| 002.010-T    | orphan         | .backlogit/queue/002.010-T.md    | —            | done  | (not in manifest) |
```

### Item Field Definitions

| Column | Description |
|---|---|
| `ID` | Backlog item ID |
| `Classification` | `matched`, `pre-archived`, `missing`, `status-mismatch`, or `orphan` |
| `Queue Path` | Path to the queue file, or `—` if not found |
| `Archive Path` | Path to the archive file (post-mode), or `—` if not yet archived |
| `Declared Status` | The `status:` field read from the item's frontmatter (or `—` if file missing) |
| `Expected Status` | The `expected_status` parameter passed to this reconciliation invocation |

## Body: Recommendation Section

```markdown
## Recommendation

**PROCEED** — all 13 manifest items are matched. No operator action required.
Proceed to `backlogit_ship_shipment`.
```

or on failure:

```markdown
## Recommendation

**HALT — operator reconcile required**

| Issue | Items | Action |
|---|---|---|
| missing (no file) | 002.002-T | Restore from pre-ship commit or remove from manifest |
| status-mismatch | 002.003-T | Mark done or remove from manifest via `backlogit_update_item` |
| orphan (in queue, not in manifest) | 002.010-T | Add to manifest via `backlogit_add_to_shipment` or remove shipment_id from item frontmatter |

Re-invoke Ship Step 6 after manually reconciling the above items.
Do NOT call `backlogit_ship_shipment` until pre-mode returns PROCEED.
```

## Post-Mode Archive Section

Post-mode reports include an additional section:

```markdown
## Archive Verification

| Archive File | Status |
|---|---|
| .backlogit/archive/004-S.md | present |

## git status Check

No deletions detected in .backlogit/archive/.
```

or when deletions detected:

````markdown
## git status Check

**WARNING**: The following archive files were deleted by `backlogit_ship_shipment` and must be restored:

```
D  .backlogit/archive/002-C.md
D  .backlogit/archive/002.001-T.md
```

Run before committing:
```powershell
git restore .backlogit/archive/
```
````

## Item Classification Reference

| Classification | Detection Condition |
|---|---|
| `matched` | File exists at `.backlogit/queue/{id}.*` AND `status` in frontmatter equals `expected_status` |
| `pre-archived` | No file at `.backlogit/queue/{id}.*` but file exists at `.backlogit/archive/{id}.*`; treated as valid |
| `missing` | No file found at `.backlogit/queue/{id}.*` or `.backlogit/archive/{id}.*` |
| `status-mismatch` | File exists at `.backlogit/queue/{id}.*` but `status` does not equal `expected_status` |
| `orphan` | File exists at `.backlogit/queue/{any}.*` with `shipment_id: {this_shipment_id}` in frontmatter but ID is NOT in manifest `items` list |

> **Note**: In `post-mode`, `matched` means file exists at `.backlogit/archive/{id}.*`.
> Queue presence is not checked post-mode since items should have been moved out of queue.

## Example: Intake Check (Step 0.5)

At intake, items are `queued` not `done`. The intake check uses `expected_status: queued`:

```yaml
expected_status: queued
summary:
  matched: 13      # all 13 items have status: queued  ← expected at intake
  missing: 0
  status_mismatch: 0
  orphan: 0
recommendation: PROCEED
```

A `status-mismatch` at intake means an item was created with the wrong status or an
item from a prior shipment was swept in with `status: done`.

## Example: Pre-Ship Check (Step 6)

Before calling `backlogit_ship_shipment`, all items should be `done`:

```yaml
expected_status: done
summary:
  matched: 13      # all 13 items have status: done ← expected at ship time
  missing: 0
  status_mismatch: 0
  orphan: 0
recommendation: PROCEED
```
