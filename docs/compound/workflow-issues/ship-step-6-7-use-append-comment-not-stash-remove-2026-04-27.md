---
title: "Ship Step 6.7: Use backlogit_append_comment for source artifact traceability"
category: workflow-issues
date: 2026-04-27
confidence: high
evidence:
  - pr: 33
  - commit: 4086cb0
  - copilot_review: comment 3144379801
tags: [backlogit, ship-agent, source-artifact-cleanup, registry]
---

## Finding

The `ship.agent.md` template at step 6.7 (Source Artifact Cleanup) originally referenced
`backlogit_stash_remove` and `backlogit_archive_item`. Neither operation exists in the installed
backlog registry (backlogit v1.x). Using them would result in tool-not-found errors at runtime.

## Correct Approach

Use `backlogit_append_comment` to record source stash IDs and deliberation references as
traceability notes on the shipped item. This allows the registry's supported workflow to handle
retirement, without requiring non-existent mutation operations.

Pattern used in ship.agent.md step 6.7 after fix:

```text
For each shipped top-level item:
  - Read custom_fields.source_stash_id
  - If present: record in closure artifact + append comment via backlogit_append_comment
  - Inspect references for deliberation artifacts
  - If present: record in closure artifact + append comment
```

## Registry Check

Before relying on any backlogit operation in agent templates, verify it exists in:
`.autoharness/backlog-registry.yaml` under the `operations` section.

Operations confirmed absent from registry: `backlogit_stash_remove`, `backlogit_archive_item`
Operations confirmed present: `backlogit_append_comment`, `backlogit_update_item`, `backlogit_move_item`

## Impact

- Affects: `ship.agent.md` step 6.7 in all harness versions before TUNE-024
- Fixed in: commit `4086cb0` (PR #33)
