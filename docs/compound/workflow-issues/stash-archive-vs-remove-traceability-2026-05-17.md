---
title: "Stash entries must be archived (not removed) when harvested"
category: backlog-hygiene
observed: 2026-05-17
shipment: 046-S
severity: medium
---

# Stash Entries Must Be Archived When Harvested

## Problem

Stage agents sometimes call `stash_remove` (destructive) instead of
`stash_archive` (traceability-preserving) when harvesting a stash entry into
a feature/task. The stash entry disappears from both active and archive
storage. Artifacts referencing the stash ID (`source_stash_id`,
`linked_stash_id`) then point to a non-existent record.

Copilot review on PR #154 flagged all three artifacts (060-F, 008-D, 046-S)
for this gap. The fix required a manual append to
`.backlogit/archive/stash.jsonl` before the PR could proceed.

## Root Cause

`stash_remove` deletes the entry; `stash_archive` moves it to the archive with
`reason: harvested` and `harvested_artifact_id`. Stage agents must always
prefer `stash_archive` at harvest time.

## Correct Archive Format

```json
{
  "id": "<stash_id>",
  "priority": "high",
  "kind": "unknown",
  "text": "<original stash text>",
  "created_at": "<ISO 8601>",
  "deliberation_id": "<linked deliberation ID if any>",
  "archived_at": "<harvest timestamp ISO 8601>",
  "reason": "harvested",
  "harvested_artifact_id": "<feature or task ID>"
}
```

## Detection Signal

If Copilot review raises "stash ID not present in active stash or
`.backlogit/archive/stash.jsonl`" comments on a PR, the stash entry was
removed rather than archived.

## Fix Protocol

1. Look up stash text from the artifact's `linked_stash_text` field
   (present in deliberation `custom_fields`) or from session memory.
2. Append the archive JSON record manually to `.backlogit/archive/stash.jsonl`.
3. Commit as `chore(docs): archive harvested stash entry <id> for traceability`.
4. Reply to Copilot comments referencing the fix commit.
5. Resolve the review threads via GraphQL.

## Prevention

Stage agent must call `stash_archive` — not `stash_remove` — at harvest time.
The backlogit instructions explicitly state: "Prefer `stash_archive` over
`stash_remove` — archiving preserves traceability; removal is destructive and
deprecated."
