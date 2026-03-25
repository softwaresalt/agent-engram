# Backlog

## Feature 009: Branch-Aware Database Isolation: Per-Branch Engram State

**Priority**: Medium enhances multi-branch workflow fidelity for code intelligence
**Discovered**: 2026-03-20 during analysis of workspace-to-database mapping
**Related**: "Strip Task Management" (reduces per-branch DB size since only code graph state persists); "Optimize SurrealDB Usage" (per-branch DBs benefit equally from native queries)

### Problem Statement

Engram currently derives its SurrealDB database identity from a SHA-256 hash of the canonical workspace path alone. Every Git branch within the same workspace shares a single database. When the user switches branches, the code graph indexed on the previous branch remains in the database and may contain stale or incorrect symbol data that does not reflect the new branch's source tree.

For code intelligence, this creates a concrete problem: an agent on `feat-001` may see function definitions, call edges, and import relationships that only exist on `main`, or vice versa. The `sync_workspace` tool can re-index to fix drift, but the fundamental issue is that divergent branches represent different codebases and should have isolated state.

### Current Architecture

The database identity flows through three functions:

1. `canonicalize_workspace(path)` in `src/db/workspace.rs` resolves the workspace root and verifies `.git` exists
2. `workspace_hash(path)` in `src/db/workspace.rs` computes `SHA-256(path_string)` as a hex digest
3. `connect_db(hash)` in `src/db/mod.rs` uses the hash as:
   * The surrealkv storage directory: `~/.local/share/engram/db/{hash}/`
   * The SurrealDB database name within the `engram` namespace

The `.engram/` files (code graph JSONL, config, registry) live in the Git worktree and already change with `git checkout`, so the hydration layer naturally picks up the correct branch's persisted state. The problem is that the database does not rotate on branch switch.

### Proposed Solution

Incorporate the current Git branch name into the workspace hash so each branch gets its own isolated database.

#### 1. Read the Current Git Branch

Add a function to `src/db/workspace.rs` that reads `.git/HEAD` directly (no `git` CLI dependency):

```rust
/// Read the current Git branch from `.git/HEAD`.
/// Returns `None` for detached HEAD or unreadable HEAD.
fn current_git_branch(workspace_root: &Path) -> Option<String> {
    let head_path = workspace_root.join(".git").join("HEAD");
    let content = std::fs::read_to_string(&head_path).ok()?;
    let trimmed = content.trim();
    trimmed
        .strip_prefix("ref: refs/heads/")
        .map(ToString::to_string)
}
```

#### 2. Modify `workspace_hash` to Include Branch

```rust
/// Compute a stable SHA-256 hash for workspace identity.
/// Incorporates the Git branch so each branch gets an isolated database.
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

The colon separator prevents ambiguity between path and branch components.

#### 3. Carry Branch Name Through State

Add the branch name to `WorkspaceSnapshot` (in `src/server/state.rs`) so `get_workspace_status` and `get_daemon_status` can report which branch the daemon is tracking:

```rust
pub struct WorkspaceSnapshot {
    pub workspace_id: String,
    pub path: String,
    pub branch: String,           // ← new field
    pub last_flush: Option<String>,
    pub stale_files: Vec<String>,
    pub connection_count: usize,
    pub file_mtimes: HashMap<PathBuf, u64>,
}
```

Update `set_workspace` in `src/tools/lifecycle.rs` to populate the branch field.

#### 4. Expose Branch in MCP Responses

`get_workspace_status` should include the active branch in its response, giving agents visibility into which branch's code intelligence they are querying:

```json
{
  "workspace_id": "a3f8...",
  "path": "/home/user/project",
  "branch": "feat-001",
  "code_graph": { "files": 42, "functions": 312, "edges": 1847 }
}
```

### Design Considerations

#### Detached HEAD

When `.git/HEAD` contains a raw commit SHA (detached HEAD), `current_git_branch` returns `None` and the hash falls back to `"detached"`. All detached HEAD states within the same workspace share one database. This is acceptable because detached HEAD is typically transient (bisect, rebase, CI checkout). If finer isolation is needed later, the commit SHA could be used instead.

#### Branch Switching

When the user runs `git checkout main`, the daemon does not automatically detect the change. A `set_workspace` call (or the planned `refresh_workspace`) is required to re-bind to the new branch's database. The old branch's database remains on disk and is reused the next time that branch is checked out.

Alternatively, a file watcher on `.git/HEAD` could trigger automatic re-binding, but this adds complexity and should be deferred to a follow-up feature.

#### Disk Usage and Cleanup

Each branch creates a separate surrealkv directory under `~/.local/share/engram/db/`. For repositories with many branches, this could accumulate significant disk usage. Mitigations:

* Add a `gc_databases` tool or CLI subcommand that lists all database directories, cross-references with `git branch --list`, and deletes databases for branches that no longer exist
* Include database size per branch in `get_daemon_status` output
* Document the storage location so users can manually clean up if needed

#### Git Worktrees

Git worktrees already have separate filesystem paths, so they already get separate databases under the current path-only hashing. Branch-aware hashing adds redundant isolation for worktrees (each worktree path + branch produces a unique hash), which is harmless.

#### Backward Compatibility

Existing databases were created with path-only hashes. After this change, the daemon generates a new branch-aware hash and creates a fresh database. The old path-only database becomes orphaned. Options:

* On first `set_workspace` with the new scheme, check if a path-only database exists and offer a one-time migration (copy data to the new branch-keyed DB)
* Accept the orphan and rely on the `.engram/` hydration files to repopulate the new database (preferred, since hydration already reconstructs full state from JSONL files)
* Add a `migrate_db` tool or CLI subcommand for explicit migration

The hydration-based approach is preferred: the new database is populated from `.engram/code-graph/` JSONL files on first bind, and the orphaned path-only database can be cleaned up manually or by a future `gc_databases` command.

### Files to Modify

| File | Change |
|------|--------|
| `src/db/workspace.rs` | Add `current_git_branch()`; modify `workspace_hash()` to include branch |
| `src/server/state.rs` | Add `branch: String` to `WorkspaceSnapshot` |
| `src/tools/lifecycle.rs` | Populate `branch` in `set_workspace`; include branch in status responses |
| `src/tools/read.rs` | Include `branch` in `get_workspace_status` and `get_health_report` output |
| `tests/unit/` | Add tests for `current_git_branch` (valid ref, detached HEAD, missing HEAD) |
| `tests/contract/lifecycle_test.rs` | Assert `branch` field in workspace binding response |
| `tests/integration/multi_workspace_test.rs` | Add branch isolation assertions |

### Verification Criteria

- [ ] `workspace_hash` produces different hashes for the same path on different branches
- [ ] `workspace_hash` produces a deterministic hash for `detached` HEAD states
- [ ] `current_git_branch` correctly parses `ref: refs/heads/feature/nested-name`
- [ ] `current_git_branch` returns `None` for raw SHA in `.git/HEAD`
- [ ] `get_workspace_status` response includes `branch` field
- [ ] Switching branches and calling `set_workspace` connects to a different database
- [ ] `.engram/` hydration populates the new branch database from existing JSONL files
- [ ] Old path-only databases do not interfere with new branch-aware databases
- [ ] `cargo test` and `cargo clippy` pass with zero warnings

### Dependencies

* No new crate dependencies (reads `.git/HEAD` via `std::fs`, SHA-256 via existing `sha2` crate)
* Should be implemented after "Strip Task Management" to avoid duplicating work across task + code graph state
* Compatible with "Optimize SurrealDB Usage" (per-branch databases use the same query engine)
