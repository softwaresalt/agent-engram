---
id: TASK-009.07
title: '009-07: Unit tests for branch detection and hashing'
status: To Do
assignee: []
created_date: '2026-03-22 21:53'
labels:
  - feature
  - '009'
  - testing
dependencies: []
references:
  - tests/unit/
parent_task_id: TASK-009
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Add unit tests to `tests/unit/` for:
- `current_git_branch` with valid ref (e.g., `ref: refs/heads/main`)
- `current_git_branch` with nested branch (e.g., `ref: refs/heads/feature/nested-name`)
- `current_git_branch` with detached HEAD (raw SHA)
- `current_git_branch` with missing `.git/HEAD`
- `workspace_hash` produces different hashes for same path on different branches
- `workspace_hash` produces deterministic hash for detached HEAD
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Tests cover valid branch ref parsing
- [ ] #2 Tests cover nested branch names
- [ ] #3 Tests cover detached HEAD fallback
- [ ] #4 Tests cover missing .git/HEAD
- [ ] #5 Tests verify hash divergence across branches
- [ ] #6 All tests pass via cargo test
<!-- AC:END -->
