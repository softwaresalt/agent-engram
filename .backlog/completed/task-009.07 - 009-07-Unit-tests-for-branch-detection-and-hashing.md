---
id: TASK-009.07
title: '009-07: Unit tests for branch detection and hashing'
status: Done
type: task
assignee: []
created_date: '2026-03-22 21:53'
updated_date: '2026-03-26 00:11'
labels:
  - feature
  - 009
  - testing
dependencies: []
references:
  - tests/unit/
parent_task_id: TASK-009
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Add unit tests to `tests/unit/` for the branch detection and hashing functions that already exist in `src/db/workspace.rs`. The implementation uses `resolve_git_branch()` (not `current_git_branch()` as originally spec'd) and `sanitize_branch_for_path()`.\n\n**Functions to test**:\n- `resolve_git_branch(workspace: &Path) -> Result<String, WorkspaceError>` — reads `.git/HEAD`, strips prefix, sanitizes for path use\n- `sanitize_branch_for_path(branch: &str) -> String` — replaces `/` with `__`\n- `workspace_hash(path: &Path) -> String` — SHA-256 of canonical path\n\n**Test strategy**: Create temp directories with mock `.git/HEAD` files containing various contents (valid ref, nested ref, detached SHA, empty, missing). Test requires a `[[test]]` entry in `Cargo.toml`.\n\n**Note**: These functions are currently `pub(crate)` or `fn` (private). Tests may need the functions to be made `pub(crate)` to be testable from `tests/unit/`, or use integration-style testing through the public `set_workspace` API."
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Tests cover resolve_git_branch() with valid ref (ref: refs/heads/main)
- [ ] #2 Tests cover nested branch names (ref: refs/heads/feature/nested-name) and sanitize_branch_for_path
- [ ] #3 Tests cover detached HEAD (raw SHA → first 12 chars)
- [ ] #4 Tests cover missing .git/HEAD (returns WorkspaceError)
- [ ] #5 Tests verify workspace_hash produces same hash for same input (determinism)
- [ ] #6 All tests pass via cargo test
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
**Actual function names**: `resolve_git_branch()` and `sanitize_branch_for_path()` in `src/db/workspace.rs`. The spec used the name `current_git_branch()` but the implementation differs.\n\n**Visibility consideration**: `sanitize_branch_for_path` is private (`fn`). Either make it `pub(crate)` for direct unit testing, or test it indirectly through `resolve_git_branch` by providing branch-name inputs with slashes.\n\n**Cargo.toml**: Add `[[test]]` block for the new test file."

Harness: `cargo test --test unit_branch_workspace` — all 6 tests (S075–S080) are GREEN gates that must remain passing after any refactor. These cover standard ref parsing, nested branch names, detached HEAD SHA truncation, missing HEAD error, no .git dir error, and hash determinism.
<!-- SECTION:NOTES:END -->
