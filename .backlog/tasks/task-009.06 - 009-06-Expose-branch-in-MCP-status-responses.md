---
id: TASK-009.06
title: '009-06: Expose branch in MCP status responses'
status: Done
type: task
assignee: []
created_date: '2026-03-22 21:52'
updated_date: '2026-03-25 22:40'
labels:
  - feature
  - 009
  - mcp
dependencies: []
references:
  - src/tools/read.rs
parent_task_id: TASK-009
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Include the `branch` field in the JSON output of `get_workspace_status` and `get_health_report` in `src/tools/read.rs`, giving agents visibility into which branch's code intelligence they are querying.

Expected response shape:
```json
{
  "workspace_id": "a3f8...",
  "path": "/home/user/project",
  "branch": "feat-001",
  "code_graph": { "files": 42, "functions": 312, "edges": 1847 }
}
```
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 get_workspace_status response includes branch field
- [x] #2 get_health_report response includes branch field
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Already implemented. `get_workspace_status` in `src/tools/read.rs` includes `branch` in its `WorkspaceStatus` response struct (line 191). `connect_db` is called with `&snapshot.branch` throughout `read.rs` (11 calls). The `WorkspaceStatus` struct at line 37-47 includes `pub branch: String` with doc comment "Active git branch name (used as the DB storage subdirectory)."
<!-- SECTION:FINAL_SUMMARY:END -->
