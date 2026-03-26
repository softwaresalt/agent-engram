---
id: TASK-009.04
title: '009-04: Modify workspace_hash() to include branch'
status: To Do
assignee: []
created_date: '2026-03-22 21:52'
updated_date: '2026-03-26 00:10'
labels:
  - feature
  - 009
  - database
dependencies: []
references:
  - src/db/workspace.rs
parent_task_id: TASK-009
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Re-scoped**: The original spec proposed embedding the Git branch into the SHA-256 hash. The actual implementation took a different (and arguably better) approach: `connect_db(data_dir, branch)` creates per-branch directories at `data_dir/db/{branch}/`, achieving true database isolation without modifying the hash.\n\n**Remaining issue**: `workspace_hash()` in `src/db/workspace.rs` still only hashes the workspace path, producing the same `workspace_id` for all branches of the same repo. This means:\n- `WorkspaceBinding.workspace_id` does not distinguish branches\n- Any client-side caching or tracking based on `workspace_id` conflates branches\n- `workspace_id` is semantically misleading when multiple branches are used\n\n**Proposed fix**: Include the branch in `workspace_hash()` so `workspace_id` uniquely identifies `(path, branch)` pairs:\n\n```rust\npub fn workspace_hash(path: &Path, branch: &str) -> String {\n    let mut hasher = Sha256::new();\n    hasher.update(path.to_string_lossy().as_bytes());\n    hasher.update(b\":\");\n    hasher.update(branch.as_bytes());\n    let digest = hasher.finalize();\n    hex::encode(digest)\n}\n```\n\nThis requires updating all callers of `workspace_hash()` to pass the branch. The `workspace_id` in `WorkspaceSnapshot` and `WorkspaceBinding` will then be branch-specific.\n\n**Alternative**: Accept that `workspace_id` is path-only and treat it as a workspace family identifier, not a unique database key. Document this distinction. The DB isolation already works correctly via directory structure."
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 workspace_hash produces different IDs for the same path on different branches
- [ ] #2 All callers of workspace_hash updated to pass branch parameter
- [ ] #3 workspace_id in WorkspaceBinding distinguishes branches
- [ ] #4 Existing tests pass with updated workspace_hash signature
- [ ] #5 No regression in connect_db behavior
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
**Current state**: Per-branch DB isolation already works via `connect_db(data_dir, branch)` directory structure. The hash change is a semantic correctness improvement, not a functional requirement.\n\n**Decision needed**: Whether to (a) update workspace_hash to include branch, or (b) accept workspace_id as path-only and document the distinction. Option (a) is cleaner but requires updating all callers. Option (b) is zero-change but may cause confusion.\n\n**Files to modify**: `src/db/workspace.rs` (workspace_hash signature), `src/tools/lifecycle.rs` (set_workspace caller), plus any tests that call workspace_hash directly."

Harness: `cargo test --test unit_branch_hash` — tests S081 and S084 are RED gates. S081 asserts same path + different branch → different hash digest. S084 asserts detached-HEAD SHA prefix produces distinct digest from a named branch. Implement by including the sanitized branch string in the SHA-256 input inside `workspace_hash` in `src/db/workspace.rs`.
<!-- SECTION:NOTES:END -->
