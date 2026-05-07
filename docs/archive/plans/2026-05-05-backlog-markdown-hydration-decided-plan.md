---
type: decided-plan
date: 2026-05-05
feature: 002-F — Backlog Markdown Hydration
shipment: 024-S
status: shipped
merge_sha: a56a8ba
pr: 82
source_plan: docs/archive/plans/2026-05-05-backlog-markdown-hydration-plan.md
---

# Decided Plan — 002-F Backlog Markdown Hydration

## Summary

Add a backlog content source type to the engram daemon indexing pipeline. Parses
YAML-frontmatter markdown files (`.backlogit/` format) into engram's graph and vector
stores so agents can query requirements, task context, and decision history alongside
code intelligence.

**Integration point**: `src/services/ingestion.rs` — `content_type == "backlog"` dispatch
branch (not `hydration.rs`). `serde_yaml` already present in Cargo.toml.

## Implementation Units

| Task | Scope | Key File |
|---|---|---|
| 002.001-T | YAML frontmatter parser | `src/services/parsing/frontmatter.rs` (new) |
| 002.002-T | Domain models | `src/models/backlog_graph.rs` (new) — BacklogNode, BacklogEdge, BacklogEdgeType, BacklogContentRecord, BacklogIndexResult |
| 002.003-T | CozoDB schema + queries | `src/db/cozo_backend/schema.rs` (+3 constants), `cozo_queries.rs` (+8 methods) |
| 002.004-T | Backlog indexer service | `src/services/backlog_indexer.rs` (new, 378 lines) — hash-based incremental |
| 002.005-T | Deletion sweep | `sweep_deleted_backlog_files` — requires absolute paths; callers do `workspace_root.join(path)` |
| 002.006-T | Ingestion integration | `ingestion.rs` — dispatch + Unknown status treated as "try" |
| 002.007-T | Architecture docs | `docs/architecture.md` — Backlog Indexer row, 23 CozoDB relations |

## Key Decisions

1. **Separate `backlog_content_record` relation** (not reusing `content_record`): prevents
   path-key collisions when `.backlogit/` paths overlap other indexed sources.
2. **Hash-based incremental sync** (SHA-256 content hash, not mtime): consistent with
   `code_graph.rs` pattern; reliable across OS/FS boundaries.
3. **Unknown source status treated as "try"**: `ingest_all_sources` only skips Missing/Error;
   tests that skip `validate_sources` leave status as Unknown but should still run.
4. **stdlib `std::fs::read_dir`** for directory walking: `walkdir` not in Cargo.toml.
5. **Labels as comma-separated strings**: CozoDB has no native string-array type.
6. **`BacklogIndexResult` has `total_files: usize`** (not vectors for nodes/edges/records):
   vectors were unused by callers; data written to DB inline.
7. **`max_file_size_bytes: u64`** param on `index_backlog_source`: guards against large files
   before reading; passed from `config.max_file_size_bytes` in `ingestion.rs`.
8. **`required-features = ["cozo-backend"]`** on integration test entry in Cargo.toml.

## Constraints

- `query_graph` traversal of `backlog_edge` deferred: stub returns `GraphQueryError::Invalid`
  (pre-existing). Stash entry A7B3C1D2 tracks when stub is implemented.
- `.backlogit/stash.jsonl` ingestion out of scope: JSONL format requires a separate adapter.
- Content-type filtering UI for `unified_search` out of scope.

## Windows Path Traversal Guard

Workspace containment check in `ingestion.rs` must reject `Component::Prefix(_)` paths
(Windows drive-relative: `C:foo`) in addition to `is_absolute()` + `starts_with()`.
See compound learning: `docs/compound/security/windows-drive-relative-path-traversal-component-prefix-2026-05-06.md`.

## Rollback Procedure

1. Remove `backlog` source from `.engram/registry.yaml` → next `ingest_all_sources` skips backlog.
2. Purge existing data if needed:
   ```cozoscript
   :rm backlog_node { file_path: <path> }
   :rm backlog_edge { source_id: <id>, target_id: <id> }
   :rm backlog_content_record { file_path: <path> }
   ```
3. Schema rollback not needed: empty relations are inert; `:create` is idempotent.

## Follow-up Items

| Stash ID | Item | Priority |
|---|---|---|
| A7B3C1D2 | Expose backlog relationship traversal via `query_graph` when stub is implemented | low |
| B9E4F2A1 | Add `backlog` source to default `engram install` registry scaffold | medium |
