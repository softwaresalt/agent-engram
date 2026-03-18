# Session Memory: 006-workspace-content-intelligence Phase 6

**Date**: 2026-03-17
**Spec**: `specs/006-workspace-content-intelligence/`
**Phase**: 6 — User Story 4: Git Commit Graph Tracking (Priority: P4)
**Branch**: `006-workspace-content-intelligence`
**Status**: COMPLETE

---

## Task Overview

Phase 6 implements git commit graph indexing and retrieval — indexing a workspace's git history into SurrealDB and querying it by file path, symbol, or date range with embedded diff snippets.

**Tasks completed**: T035, T036, T037, T038, T039, T040, T041, T042 (8/8)

---

## Current State

### Files Created (new)
- `src/services/git_graph.rs` — Full git2-based implementation: `index_git_history`, `walk_commits`, `extract_changes`, `extract_patch_text`
- `tests/integration/git_graph_test.rs` — 12 integration tests (S045–S051, S056, S058–S059, S061, S063)

### Files Modified
- `Cargo.toml` — Added `git2 = { version = "0.19", optional = true, default-features = false }`, `git-graph` feature, `required-features` on integration test entry
- `src/db/queries.rs` — Added `upsert_commit_node`, `latest_indexed_commit_hash`, `select_commits_by_file_path`, `select_commits_by_date_range`
- `src/services/mod.rs` — Added `#[cfg(feature = "git-graph")] pub mod git_graph;`
- `src/tools/mod.rs` — Registered `query_changes` and `index_git_history` in dispatch
- `src/tools/read.rs` — `query_changes` MCP tool (~line 1502), feature-gated
- `src/tools/write.rs` — `index_git_history` MCP tool (~line 1714), feature-gated
- `tests/contract/content_test.rs` — Added git graph contract tests (S052–S060, S074–S075, model round-trip tests)
- `tests/integration/backlog_test.rs` — Minor adjustments
- `tests/unit/registry_parse_test.rs` — Minor adjustments

### Test Results
- `cargo test --test integration_git_graph --features git-graph`: **12/12 PASS**
- `cargo test --test contract_content --features git-graph`: **21/21 PASS**
- `cargo fmt --all -- --check`: **PASS**
- `cargo clippy --all-targets --features git-graph -- -D warnings -D clippy::pedantic`: **PASS**
- Pre-existing benchmark flakes: `t119_flush_state_under_1s`, `t098_hydration_1000_tasks_under_500ms` — timing-sensitive, confirmed not caused by Phase 6

---

## Important Discoveries

### git2 Feature Gating Pattern
All git2 code is behind `#[cfg(feature = "git-graph")]`:
- Module declaration in `src/services/mod.rs`
- MCP tool functions in `src/tools/read.rs` and `src/tools/write.rs`
- Tool dispatch registration in `src/tools/mod.rs`
- `[[test]]` entry in `Cargo.toml` uses `required-features = ["git-graph"]`

### spawn_blocking for git2
All `git2` operations run inside `tokio::task::spawn_blocking` to avoid blocking the async runtime. `git2::Repository` is not `Send`, so it cannot cross await points.

### Incremental Sync
`latest_indexed_commit_hash` returns the most-recently-stored commit hash. `walk_commits` stops when it encounters this hash, enabling incremental indexing.

### Diff Truncation
Diffs exceeding 500 lines are truncated with `[diff truncated]` appended. Truncation happens per-hunk.

### Symbol Query Cross-Reference
`query_changes` with `symbol` crosses into code graph via `find_symbols_by_name` to resolve file path, then filters commits. Unknown symbol → error code 4002.

---

## Next Steps (Phase 7)

**Phase 7: Agent Hooks and Integration Instructions** (T043–T047)
- T043: Integration test for hook file generation
- T044–T047: Hook templates, marker insertion, CLI flags, port-aware URLs
- Key files: `src/installer/mod.rs`, `src/config/mod.rs`, `tests/integration/installer_test.rs`

---

## Context to Preserve

- Benchmark timing failures (`t119`, `t098`) are pre-existing — do NOT attempt to fix in Phase 7+
- `git-graph` feature disabled by default; test with `--features git-graph`
- `git2` uses `default-features = false` to avoid OpenSSL linking issues
- `CommitNode::id` format: `"commit_node:{hash}"` — DB upserts keyed on hash via `commit_hash_idx` unique index
