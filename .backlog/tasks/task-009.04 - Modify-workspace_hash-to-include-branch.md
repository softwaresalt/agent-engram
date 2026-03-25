---
id: TASK-009.04
title: '009-04: Modify workspace_hash() to include branch'
status: To Do
assignee: []
created_date: '2026-03-22 21:52'
labels:
  - feature
  - '009'
  - database
dependencies: []
references:
  - src/db/workspace.rs
parent_task_id: TASK-009
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Modify `workspace_hash()` in `src/db/workspace.rs` to incorporate the Git branch name into the SHA-256 hash. Use a colon separator between path and branch to prevent ambiguity. Falls back to `"detached"` when `current_git_branch` returns `None`.

```rust
pub fn workspace_hash(path: &Path) -> String {
    let branch = current_git_branch(path).unwrap_or_else(|| "detached".to_string());
    let mut hasher = Sha256::new();
    hasher.update(path.to_string_lossy().as_bytes());
    hasher.update(b":");
    hasher.update(branch.as_bytes());
    let digest = hasher.finalize();
    hex::encode(digest)
}
```

No new crate dependencies — uses existing `sha2` crate.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 workspace_hash produces different hashes for the same path on different branches
- [ ] #2 workspace_hash produces a deterministic hash for detached HEAD states
- [ ] #3 Colon separator prevents path/branch ambiguity
<!-- AC:END -->
