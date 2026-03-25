---
id: TASK-009.08
title: '009-08: Contract and integration tests for branch isolation'
status: To Do
assignee: []
created_date: '2026-03-22 21:53'
labels:
  - feature
  - '009'
  - testing
dependencies: []
references:
  - tests/contract/lifecycle_test.rs
  - tests/integration/multi_workspace_test.rs
parent_task_id: TASK-009
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Add contract and integration tests:
- `tests/contract/lifecycle_test.rs`: Assert `branch` field in workspace binding response
- `tests/integration/multi_workspace_test.rs`: Assert branch isolation — switching branches and calling `set_workspace` connects to a different database; `.engram/` hydration populates the new branch database from existing JSONL files; old path-only databases do not interfere
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Contract test asserts branch field in workspace binding response
- [ ] #2 Integration test verifies branch switching connects to different database
- [ ] #3 Integration test verifies hydration populates new branch DB from JSONL
- [ ] #4 Integration test verifies old path-only DBs don't interfere
- [ ] #5 cargo test and cargo clippy pass with zero warnings
<!-- AC:END -->
