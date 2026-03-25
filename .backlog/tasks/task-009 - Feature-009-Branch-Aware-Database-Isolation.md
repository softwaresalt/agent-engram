---
id: TASK-009
title: '009: Branch-Aware Database Isolation'
status: To Do
type: feature
assignee: []
created_date: '2026-03-22 21:50'
updated_date: '2026-03-23 20:29'
labels:
  - feature
  - '009'
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
- [ ] #3 current_git_branch correctly parses ref: refs/heads/feature/nested-name
- [ ] #4 current_git_branch returns None for raw SHA in .git/HEAD
- [ ] #5 get_workspace_status response includes branch field
- [ ] #6 Switching branches and calling set_workspace connects to a different database
- [ ] #7 .engram/ hydration populates the new branch database from existing JSONL files
- [ ] #8 Old path-only databases do not interfere with new branch-aware databases
- [ ] #9 cargo test and cargo clippy pass with zero warnings
<!-- AC:END -->
