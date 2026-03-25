---
id: TASK-009.05
title: '009-05: Populate branch in set_workspace lifecycle'
status: To Do
assignee: []
created_date: '2026-03-22 21:52'
labels:
  - feature
  - '009'
  - lifecycle
dependencies: []
references:
  - src/tools/lifecycle.rs
parent_task_id: TASK-009
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Update `set_workspace` in `src/tools/lifecycle.rs` to populate the `branch` field in `WorkspaceSnapshot` when binding a workspace. The branch value comes from `current_git_branch()` (falling back to `"detached"`).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 set_workspace populates branch field from current_git_branch()
- [ ] #2 Detached HEAD produces branch value of 'detached'
<!-- AC:END -->
