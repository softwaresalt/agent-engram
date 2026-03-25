---
id: TASK-009.01
title: '009-01: Add current_git_branch() to workspace.rs'
status: To Do
assignee: []
created_date: '2026-03-22 21:51'
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
Add a function to `src/db/workspace.rs` that reads `.git/HEAD` directly (no `git` CLI dependency) and returns the current branch name. Returns `None` for detached HEAD or unreadable HEAD.

```rust
fn current_git_branch(workspace_root: &Path) -> Option<String> {
    let head_path = workspace_root.join(".git").join("HEAD");
    let content = std::fs::read_to_string(&head_path).ok()?;
    let trimmed = content.trim();
    trimmed.strip_prefix("ref: refs/heads/").map(ToString::to_string)
}
```

Must handle: valid branch refs, nested branch names (e.g., `feature/nested-name`), detached HEAD (raw SHA), and missing `.git/HEAD`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 current_git_branch correctly parses ref: refs/heads/feature/nested-name
- [ ] #2 current_git_branch returns None for raw SHA in .git/HEAD
- [ ] #3 current_git_branch returns None when .git/HEAD is missing
<!-- AC:END -->
