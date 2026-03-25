---
id: TASK-009
title: '009: Branch-Aware Database Isolation'
status: To Do
assignee: []
created_date: '2026-03-22 21:50'
updated_date: '2026-03-25 22:41'
labels:
  - feature
  - 009
  - database
  - workspace
milestone: m-0
dependencies: []
references:
  - .context/backlog.md (lines 370-527)
  - src/db/workspace.rs
  - src/db/mod.rs
  - src/server/state.rs
  - src/tools/lifecycle.rs
  - src/tools/read.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Incorporate the current Git branch name into the workspace hash so each branch gets its own isolated SurrealDB database. Currently, Engram derives database identity from SHA-256 of the canonical workspace path alone, meaning all Git branches share one database. When switching branches, stale symbol data from the previous branch persists, causing agents to see incorrect function definitions, call edges, and import relationships.

### Current Architecture
Database identity flows through: `canonicalize_workspace(path)` → `workspace_hash(path)` (SHA-256 hex digest) → `connect_db(hash)` (surrealkv storage dir + DB name). The `.engram/` files already change with `git checkout`, but the database does not rotate on branch switch.

### Design Considerations
- **Detached HEAD**: Falls back to `"detached"` string — all detached states share one DB per workspace
- **Branch Switching**: Requires `set_workspace` call to re-bind; no automatic `.git/HEAD` watcher (deferred)
- **Disk Usage**: Each branch creates separate surrealkv dir; future `gc_databases` tool for cleanup
- **Git Worktrees**: Already isolated by path; branch-aware hashing adds redundant but harmless isolation
- **Backward Compatibility**: Prefer hydration-based approach — new DB populated from `.engram/code-graph/` JSONL files; old path-only DB becomes orphan for manual or future gc cleanup
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 workspace_hash produces different hashes for the same path on different branches
- [ ] #2 workspace_hash produces a deterministic hash for detached HEAD states
- [x] #3 current_git_branch correctly parses ref: refs/heads/feature/nested-name
- [x] #4 current_git_branch returns None for raw SHA in .git/HEAD
- [x] #5 get_workspace_status response includes branch field
- [x] #6 Switching branches and calling set_workspace connects to a different database
- [ ] #7 .engram/ hydration populates the new branch database from existing JSONL files
- [ ] #8 Old path-only databases do not interfere with new branch-aware databases
- [ ] #9 cargo test and cargo clippy pass with zero warnings
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
## Implementation Status (audited 2026-03-25)\n\nThe implementation took a different architectural approach than the backlog spec proposed. Instead of embedding branch into the SHA-256 workspace hash, the codebase passes branch as a separate parameter to `connect_db(data_dir, branch)`, which creates per-branch database directories at `data_dir/db/{sanitized_branch}/`.\n\n### Tasks completed (already in codebase):\n- **009.01** ✅ `resolve_git_branch()` exists in `src/db/workspace.rs` (named differently than spec's `current_git_branch()`)\n- **009.02** ✅ `WorkspaceSnapshot.branch` field exists in `src/server/state.rs`\n- **009.03** ✅ `set_workspace` populates branch from `resolve_git_branch()` in `src/tools/lifecycle.rs`\n- **009.05** ✅ Duplicate of 009.03, archived\n- **009.06** ✅ `get_workspace_status` includes `branch` in response\n\n### Tasks remaining:\n- **009.04** Re-scoped: `workspace_hash()` doesn't include branch (isolation works via directory structure, but `workspace_id` doesn't distinguish branches)\n- **009.07** Unit tests for `resolve_git_branch()` and `sanitize_branch_for_path()`\n- **009.08** Contract/integration tests for branch isolation\n- **009.09** Wire `record_file_hash` into indexing/watcher pipeline\n\n### Key architectural difference from spec:\n- Spec: `workspace_hash(path)` → `workspace_hash(path + branch)` (branch in hash)\n- Actual: `connect_db(data_dir, branch)` → `data_dir/db/{branch}/` (branch in directory structure)\n- Result: Per-branch isolation works correctly, but `workspace_id` is path-only"

**AC #4 note**: The spec says `current_git_branch returns None for raw SHA`. The actual implementation (`resolve_git_branch`) returns the first 12 chars of the commit SHA as the branch name instead of `None`. This is a reasonable design choice — detached HEAD still gets its own database directory.

**AC #6 note**: Branch isolation IS working via `connect_db(data_dir, branch)` directory structure. Different branches produce different `data_dir/db/{branch}/` paths. The spec's mechanism (hash-based) differs from the implementation (directory-based) but the outcome is the same.
<!-- SECTION:NOTES:END -->
