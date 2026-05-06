---
type: compacted-memory
date: 2026-05-06
feature: 002-F — Backlog Markdown Hydration
shipment: 024-S
merge_sha: a56a8ba
pr: 82
branch: feat/002-F-backlog-hydration
status: shipped
sources:
  - docs/archive/memory/stage-024-025-shipment-assembly-memory.md
  - docs/archive/memory/002-F-backlog-hydration-pre-merge-memory.md
  - docs/archive/memory/second-copilot-review-round-fixes-memory.md
  - docs/archive/memory/024-S-third-review-round-memory.md
  - docs/archive/memory/024-S-post-merge-closure-memory.md
---

# Compacted Memory — 024-S / 002-F Backlog Markdown Hydration

## Feature Outcome

PR #82 merged to `main` as commit `a56a8ba` after 4 Copilot review rounds (30 threads total).
All 7 tasks and the parent feature shipped. Closure PR #83 created and awaiting merge.

## Stage Pipeline (pre-build)

Group C (backlog hydration) went through 3 plan-review cycles before harvest:
- Cycle 1: Wrong integration hook (hydration.rs instead of ingestion.rs), missing scope
- Cycle 2: Missing schema design, wrong model replacement, underspecified ContentRecord lifecycle  
- Cycle 3: All blockers fixed; remaining P2 findings accepted as advisory

Key staging decisions:
- Integration via `src/services/ingestion.rs` (not hydration.rs)
- New `backlog_graph.rs` models (keep existing `backlog.rs` untouched)
- CozoDB relations: `backlog_node`, `backlog_edge`, `backlog_content_record` (separate from `content_record` to avoid path-key collisions)
- Hash-based incremental sync (SHA-256, not mtime)
- `serde_yaml` already in Cargo.toml — no new dependency needed
- Labels stored as comma-separated strings (CozoDB has no native string-array type)

## Implementation (002.001-T through 002.007-T)

| Task | File Created/Modified | Notes |
|---|---|---|
| 002.001-T | `src/services/parsing/frontmatter.rs` | YAML frontmatter parser, 5 unit tests |
| 002.002-T | `src/models/backlog_graph.rs` | BacklogNode, BacklogEdge, BacklogEdgeType, BacklogContentRecord, BacklogIndexResult |
| 002.003-T | `src/db/cozo_queries.rs` (+8 methods), `schema.rs` (+3 constants) | backlog CRUD, `backlog_content_record` relation |
| 002.004-T | `src/services/backlog_indexer.rs` (378 lines) | hash-based incremental indexer |
| 002.005-T | `backlog_indexer.rs::sweep_deleted_backlog_files` | deletion sweep; requires absolute paths from caller |
| 002.006-T | `src/services/ingestion.rs` | backlog dispatch; Unknown status treated as "try" |
| 002.007-T | `docs/architecture.md` | Backlog Indexer row, CozoDB Queries updated to 23 relations |

Test coverage: 5+5+5+7=22 unit/contract tests; 6 integration tests (require `--features cozo-backend`).

## Copilot Review Rounds

| Round | Commit | Threads | Key Fixes |
|---|---|---|---|
| 1 | `98ad124` | 13 | Various safety, doc, and lint fixes |
| fmt fix | `7f56bce` | — | rustfmt cleanup after round 1 |
| 2 | `8f5a5b6` | 10 | `BacklogIndexResult` cleanup, `max_file_size_bytes` param, `query_memory` backlog candidates, duration truncation fix |
| 3 | `346a252` | 4 | Module doc corrections, `required-features = ["cozo-backend"]` in Cargo.toml |
| 4 | `fdd9f1c` | 3 | `Component::Prefix` Windows path traversal guard, `compute_deleted_paths` doc, 024-S manifest statuses |

Round 2 notable: removed unused vectors from `BacklogIndexResult`; added `total_files: usize` instead.
Round 4 notable: Windows drive-relative path (`C:foo`) bypasses `is_absolute()` + `starts_with()` — requires `Component::Prefix(_)` check.

## Post-Merge Closure

- Backlog: 9 items archived (002-F, 002.001-T–002.007-T, 024-S) to `.backlogit/archive/`
- Compound learning added: `docs/compound/security/windows-drive-relative-path-traversal-component-prefix-2026-05-06.md`
- `start.ps1` fixed: `backlogit sync index` → `backlogit sync`
- Closure artifact updated to SHIPPED mode: `docs/closure/2026-05-05-002-F-backlog-hydration-closure.md`
- Pre-existing local failure: `contract_shim_lifecycle` 6 tests fail on Windows (SQLite BUSY + IPC); confirmed pre-existing

## Follow-up Stash Items

| Stash ID | Item | Priority |
|---|---|---|
| A7B3C1D2 | Expose backlog relationship traversal via `query_graph` when stub is implemented | low |
| B9E4F2A1 | Add `backlog` source to default `engram install` registry scaffold | medium |

## Next Candidate

Stash D391F5AF — CLI parity for all MCP tool operations (JSON-RPC 2.0 output, `engram manifest` command).
