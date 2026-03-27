---
id: TASK-010.07
title: '010.7: Baseline Analysis Script (Copilot CLI)'
status: To Do
assignee: []
created_date: '2026-03-27 05:51'
labels:
  - task
dependencies:
  - TASK-010.01
parent_task_id: TASK-010
priority: low
ordinal: 6000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Create a PowerShell script that reads the Copilot CLI session store SQLite database and extracts baseline search usage measurements for non-engram workspaces.

**Script:** `scripts/metrics/Extract-SearchBaseline.ps1` (new)

**Parameters:**
- `-Repository <string>` — filter by repository (e.g., "softwaresalt/agent-engram")
- `-Since <datetime>` — only include sessions after this date
- `-OutputPath <string>` — file path for JSONL output (default: stdout)
- `-Validate` — self-validation mode: runs against fixture data, asserts output schema

**Processing pipeline:**
1. Open session_store.db (read-only)
2. Query sessions filtered by repository and date
3. For each session, query turns
4. Parse assistant responses to identify search tool calls (grep, glob, view invocations)
5. Group contiguous search calls into sequences
6. Classify each sequence by purpose using heuristics:
   - Symbol name patterns → `point_lookup`
   - Multiple grep calls with related terms → `broad_exploration`
   - Grep for callers/references → `impact_trace`
   - View with specific line ranges → `neighborhood`
   - Interleaved with edits → `implementation`
7. Collect codebase metadata (language, LOC, file count, size bracket)
8. Output each sequence as JSONL matching unified schema (engram-specific fields null)

**Unified JSONL schema:** Same common fields as engram's `usage.jsonl` so both can feed into the same analysis pipeline.

**Branch sanitization:** Scripts must replicate `/` → `__` replacement from `sanitize_branch_for_path()`.

**Files to create:** `scripts/metrics/Extract-SearchBaseline.ps1`, `scripts/metrics/README.md`
**Test file:** `scripts/metrics/Extract-SearchBaseline.Tests.ps1` (Pester) or `-Validate` switch

**This task depends only on the JSONL schema from 010.1 and can proceed in parallel with the Rust implementation.**
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Script runs against Copilot CLI session store without errors
- [ ] #2 Output JSONL is valid (each line parses as JSON)
- [ ] #3 --repository filter correctly restricts to specified repo
- [ ] #4 Records contain reasonable token estimates
- [ ] #5 Automated validation via -Validate switch or Pester test
- [ ] #6 Scripts replicate / to __ branch sanitization convention
<!-- AC:END -->
