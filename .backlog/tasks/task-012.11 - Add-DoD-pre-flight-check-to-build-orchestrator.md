---
id: TASK-012.11
title: Add DoD pre-flight check to build-orchestrator
status: Done
assignee: []
created_date: '2026-03-29 07:17'
labels:
  - harness
  - dynamic-injection
dependencies: []
parent_task_id: TASK-012
priority: high
ordinal: 11000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Add Step 4c (DoD Pre-Flight Check) to build-orchestrator before commit. Reads task's acceptance criteria and DoD items via `backlog-task_view`, verifies all are satisfied. Blocking — must pass before commit. Broadcasts with `[DOD]` prefix. Research Primitive 5.
<!-- SECTION:DESCRIPTION:END -->
