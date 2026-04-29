---
title: "Stale engram citation produces hallucinated symbol references"
description: "Agents citing engram results for files not yet indexed produce incorrect symbol/structure references"
problem_type: "bug"
type: bug
category: bugs
component: "agent-engram overlay / all research-phase skills"
status: resolved
severity: medium
symptom: "Agent cites a function, section, or field from a file that was recently created or modified, but the engram index has not yet captured it. The cited content does not exist or is outdated."
root_cause: "File-watcher ingestion lag — newly created or modified files are not immediately indexed by the engram daemon. Agents that cite engram results without verifying the file is indexed will reference stale or absent index entries."
reproduction_steps: |
  1. Create a new file in the repository (e.g., a new SKILL.md).
  2. In the same session, call list_symbols or unified_search referencing that file.
  3. Observe empty or stale results.
  4. Agent proceeds to cite the 'indexed' data as authoritative.
resolution: "Added 'Verifying File Indexed' protocol to agent-engram.instructions.md (031.001.001-T). Protocol requires agents to verify file is indexed via list_symbols/query_memory before citing results, with sync_workspace fallback and view fallback for still-absent files."
resolution_type: "config_change"
discovered_in: "031-F deliberation (stash 2B842D59); confirmed in multiple Stage sessions"
references:
  - docs/decisions/2026-04-21-031-F-harness-hardening-deliberation.md
  - .github/instructions/agent-engram.instructions.md
  - docs/decisions/2026-04-29-bug-capture-format-decision.md
created_at: 2026-04-29
---

## Symptom

Agent cites a function, section, or field from a file that was recently created or
modified, but the content referenced does not exist in the file or is outdated.

Common manifestation: the agent says "according to `list_symbols` for `new-file.md`,
the section `X` exists" — but `X` is from a previous version or was never indexed.

## Root Cause

File-watcher ingestion lag. The engram daemon indexes files reactively via a file-system
watcher. Newly created or recently modified files are not guaranteed to be indexed within
the same agent session turn that created them. Agents that call `list_symbols` or
`unified_search` without verifying the result is non-empty (or non-stale) will silently
treat an empty or outdated index entry as authoritative.

## Resolution

Protocol added to `.github/instructions/agent-engram.instructions.md` under
"Verifying File Indexed" (task 031.001.001-T, shipment 008-S):

1. Call `list_symbols` or `query_memory` with the file path before citing results.
2. Empty result → call `sync_workspace`, re-query.
3. Still empty → use `view` tool directly; do not cite engram for that file.

## Prevention

The file-load verification step was added to the research phase of the `deliberate`,
`impl-plan`, and `spike` skills (task 031.001.002-T) to enforce the protocol at
every research entry point.
