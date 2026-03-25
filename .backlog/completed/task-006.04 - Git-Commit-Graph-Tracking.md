---
id: TASK-006.04
title: '006-04: Git Commit Graph Tracking'
status: Done
assignee: []
created_date: '2026-03-15'
labels:
  - feature
  - 006
  - userstory
  - p4
dependencies: []
references:
  - specs/006-workspace-content-intelligence/spec.md
parent_task_id: TASK-006
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
As an AI agent performing code review, I query Engram for the commit history of a specific file or function so that I can understand what changed, when, and why — with actual code and text snippets attached to each change — enabling faster change detection and grounded adversarial review.

**Why this priority**: Git history is a critical dimension of workspace knowledge that Engram currently ignores. For adversarial code reviews, agents need to trace the evolution of code to detect regressions, understand intent, and validate that changes align with specifications. This story is P4 because it depends on the content registry (to know which paths to track) and the ingestion pipeline (to store change records).

**Independent Test**: Configure a registry with `type: code, path: src`. Make 5 commits modifying different files in `src/`. Call a new `query_changes` tool with a file path filter. Verify the response includes commit hashes, timestamps, authors, commit messages, and actual diff snippets (added/removed lines) for that file. Call `query_changes` with a function name and verify it returns only commits that touched that function.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 **Given** a workspace with git history, **When** Engram indexes the commit graph, **Then** the system creates commit nodes in SurrealDB with attributes: hash, author, timestamp, message, and parent commit references
- [x] #2 **Given** a commit that modifies 3 files, **When** the commit is indexed, **Then** each file change is stored as a change record linked to the commit node, containing the file path, change type (add/modify/delete), and a diff snippet showing the actual lines added and removed
- [x] #3 **Given** a change record for a modified file, **When** the diff snippet is generated, **Then** the snippet includes up to 20 lines of context around each changed hunk (configurable), preserving enough surrounding code for an agent to understand the change without reading the full file
- [x] #4 **Given** a `query_changes` call with `file_path: "src/server/router.rs"`, **When** the query executes, **Then** the system returns all commit nodes that include a change record for that file, ordered by timestamp descending
- [x] #5 **Given** a `query_changes` call with `symbol: "build_router"`, **When** the query executes, **Then** the system cross-references the commit graph with the code graph to return only commits where the diff touched lines within the `build_router` function's line range
- [x] #6 **Given** a repository with 10,000 commits, **When** initial git graph indexing runs, **Then** the system processes commits in reverse chronological order and supports a configurable depth limit (default: 500 most recent commits) to bound initial indexing time
- [x] #7 **Given** a new commit is made after initial indexing, **When** incremental sync runs, **Then** only the new commits since the last indexed commit are processed and added to the graph
- [x] #8 **Given** a merge commit with multiple parents, **When** the commit is indexed, **Then** all parent references are preserved in the commit node, enabling branch topology traversal ---
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Phase 6: User Story 4 — Git Commit Graph Tracking (Priority: P4)

**Goal**: Git commits are indexed as graph nodes with change records and diff snippets, queryable by file path, symbol name, or date range.

**Independent Test**: Index git history, query by file path, verify commit details with diff snippets.

### Tests for User Story 4

- [x] T035 [P] [US4] Contract test for git graph MCP tools in tests/contract/content_test.rs — verify S052 (query by file_path), S053 (query by symbol), S054 (query by date range), S055 (limit + truncated), S057 (unknown symbol → error 4002), S060 (no git repo → error 5001), S074-S075 (workspace not set → error 1001)
- [x] T036 [P] [US4] Integration test for git graph indexing in tests/integration/git_graph_test.rs — verify S045 (500 commits default depth), S046 (custom depth), S047 (incremental sync), S048 (force re-index), S049 (commit with 3 change types), S050 (diff context lines), S051 (merge commit parents), S056 (nonexistent file → empty), S058 (shallow clone), S059 (empty repo), S061 (large diff truncation), S063 (concurrent index + query)

### Implementation for User Story 4

- [x] T037 [US4] Implement git repository access in src/services/git_graph.rs — wrap entire module in `#[cfg(feature = "git-graph")]`, open git repo with git2::Repository::open(), use spawn_blocking for all git2 operations, handle GitNotFound error; also add `#[cfg(feature = "git-graph")]` guards to git-related MCP tool registrations and model imports
- [x] T038 [US4] Implement commit walker in src/services/git_graph.rs — use git2::Revwalk to iterate commits in reverse chronological order, respect depth limit (default: 500), track last indexed commit hash for incremental sync, support force flag for full re-index
- [x] T039 [US4] Implement diff extraction in src/services/git_graph.rs — for each commit, compute tree-to-tree diff (git2::Diff), extract per-file ChangeRecords with change_type, generate diff snippets with configurable context lines (default: 20), truncate large diffs (> 500 lines), handle merge commits by diffing against first parent
- [x] T040 [US4] Implement CommitNode persistence in src/db/queries.rs — upsert CommitNode records by hash, store parent_hashes, store embedded ChangeRecords, index by timestamp
- [x] T041 [US4] Implement query_changes MCP tool in src/tools/read.rs — accept file_path, symbol, since, until, limit parameters; query commit_node table with filters; for symbol filter, cross-reference with code graph to get line range then filter ChangeRecords by line overlap; return formatted commit list with changes
- [x] T042 [US4] Implement index_git_history MCP tool in src/tools/write.rs — accept depth and force parameters, call git_graph service, return indexing summary (commits_indexed, new_commits, total_changes, elapsed_ms)

**Checkpoint**: Git history queryable by file, symbol, or date range with actual diff snippets.

---
<!-- SECTION:PLAN:END -->

