---
id: TASK-012.02
title: Update build-orchestrator — compaction trigger
status: Done
assignee: []
created_date: '2026-03-29 04:56'
labels:
  - harness
  - context-management
dependencies:
  - TASK-012.01
parent_task_id: TASK-012
priority: high
ordinal: 2000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Add threshold check (>40 files or >500 KB) to Step 1 Pre-Flight in `build-orchestrator.agent.md`. Invoke `compact-context` skill when threshold exceeded. Broadcast with `[COMPACT]` prefix. Addresses Research Primitive 1.
<!-- SECTION:DESCRIPTION:END -->
