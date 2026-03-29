---
id: TASK-012.01
title: Create compact-context skill
status: Done
assignee: []
created_date: '2026-03-29 02:14'
labels:
  - harness
  - context-management
dependencies: []
parent_task_id: TASK-012
priority: high
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
New `.github/skills/compact-context/SKILL.md` with two-phase Assess+Compact workflow. Scans `.copilot-tracking/` for stale files (>14 days), produces summaries, archives originals to `.copilot-tracking/archive/`. Never deletes. Cross-references active backlog tasks to preserve referenced files.
<!-- SECTION:DESCRIPTION:END -->
