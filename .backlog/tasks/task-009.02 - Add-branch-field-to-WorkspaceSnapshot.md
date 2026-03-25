---
id: TASK-009.02
title: '009-02: Add branch field to WorkspaceSnapshot'
status: To Do
assignee: []
created_date: '2026-03-22 21:52'
labels:
  - feature
  - '009'
  - state
dependencies: []
references:
  - src/server/state.rs
parent_task_id: TASK-009
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Add `branch: String` field to `WorkspaceSnapshot` in `src/server/state.rs` to carry the branch name through the application state. This enables `get_workspace_status` and `get_daemon_status` to report which branch the daemon is tracking.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 WorkspaceSnapshot includes branch: String field
- [ ] #2 Branch field is populated during workspace binding
<!-- AC:END -->
