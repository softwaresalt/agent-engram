---
id: TASK-014.03
title: Add granularity check to harness-architect
status: Done
assignee: []
created_date: '2026-03-29 06:55'
labels:
  - harness
  - task-granularity
dependencies: []
parent_task_id: TASK-014
priority: medium
ordinal: 3000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Add advisory granularity check at Step 4.3 in `harness-architect.agent.md`. Flag tasks referencing >3 files, >5 functions, or requiring >4 test scenarios. Broadcast warning; do not block harness generation. Recommend re-running backlog-harvester to split.
<!-- SECTION:DESCRIPTION:END -->
