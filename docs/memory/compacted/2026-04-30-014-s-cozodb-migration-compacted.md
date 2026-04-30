---
title: "Compacted Memory — Shipment 014-S: CozoDB Migration Phase 3-4"
compacted_from:
  - docs/memory/2026-04-30/014-s-phase3-4-impl-memory.md
  - docs/memory/2026-04-30/014-s-pr-ready-checkpoint.md
  - docs/memory/2026-04-30/014-s-post-merge-closure-memory.md
archived_to: docs/archive/memory/
date: 2026-04-30
shipment: 014-S
pr: 53
merge_sha: 84296ff
status: shipped
---

## Outcome

Shipment 014-S fully shipped. PR #53 merged as merge commit `84296ff` on 2026-04-30.
Post-merge closure complete on branch `post-merge/014-s-cozodb-migration-phase-3-4`.

## What Was Built

**Phase 3 — Edge + Traversal Parity** (`001.004-C` + 6 tasks):
- 6 edge tables in CozoDB schema: `calls_edge`, `imports_edge`, `defines_edge`,
  `inherits_from_edge`, `concerns_edge`, `references_edge`
- Edge CRUD: upsert, delete, list for all 6 edge kinds
- `concerns_edge` specialty: keyed `(task_id, symbol_id)` — not `(from, to)`
- Recursive Datalog BFS traversal via `bfs_impl` with in-traversal `allowed_edge_types` filtering
- Symbol identity lookups, graph neighborhood, impact analysis

**Phase 4 — Vector + Hybrid Parity** (`001.005-C` + 5 tasks):
- HNSW index activation (3 indexes: function, class, interface embeddings; 384-dim)
- Native `vector_search_symbols_native` — pure Datalog KNN query, no MTREE workaround
- `hybrid_graph_vector_search` — single-program Datalog combining BFS + ANN
- GC simplification for corrupted embeddings
- Embedding micro-benchmark harness (`#[ignore]` tests)

**Also**: `001.001.005-T` — entity-with-stable-id helper

## Key Bug Fixes Applied

| Bug | Root Cause | Fix |
|-----|-----------|-----|
| `concerns_edge` `:rm` no-op | Uses `task_id`/`symbol_id` keys, not `from`/`to` | Added special case for `task_id`/`symbol_id` columns |
| `imports_edge`/`references_edge` `:rm` no-op | Composite key requires `(from, to, import_path)` / `(from, to, qualified_name)` | Added 3-field `:rm` with SELECT-then-delete |
| `delete_concerns_by_task_and_symbol_name` wrong count | CozoDB `:rm` response ≠ per-row count | SELECT-count-then-delete pattern |
| `update_symbol_embedding` missing `function:` prefix | Only handled `fn:` prefix | Added `function:` and `interface:` dispatch |
| `gc_corrupted_embeddings` parse error | `length(embedding) = 0` invalid Datalog | Bind to variable: `emb_len = length(embedding), emb_len = 0` |
| `resolve_symbol` `file:` prefix unreachable | `"file"` → match arm checked `"code_file"` | Both `code_file:` and `file:` now map to `"code_file"` |
| `edges_from_table` missing `linked_by` | Datalog projection omitted column | Added `linked_by` to projection + mapping branch |
| `update_content_record_embedding` silent miss | Looked up by `file_path` but callers pass record `id` | Look up by `id`, retrieve `file_path`, use in `:put` |
| Post-hoc BFS edge filter (semantic bug) | Included nodes reachable only via excluded types | Filter during traversal, not after |
| HNSW `index_filter` | `CozoDB [Float] cannot be null` type error | Changed to `length(embedding) == 384` |

## Copilot Review Rounds

| Round | Comments | Fixes | Commit |
|-------|----------|-------|--------|
| Round 1 | 14 + 6 clippy | 20 fixes | `0db84d6`, `2bc8ff1` |
| Round 2 | 2 | Sort fix + doc comment | `81de05a` |
| Round 3 | 7 | Composite keys, count, resolve_symbol, linked_by, BFS, schema | `3c44f15` |
| Round 4 | 1 | `update_content_record_embedding` id lookup | `5204f48` |

## Post-Merge Closure

- Pre-reconcile: PROCEED (14/14 pre-archived)
- `backlogit_ship_shipment("014-S", "84296ff")`: 15 archived IDs
- P-007 archive integrity: clean (no deletions)
- Post-reconcile: PROCEED (15/15 archive files)
- Backlog commit: `4b37075`
- Docs commit: `dabf745`
- Closure status: **READY** (validation window: 2026-04-30 → 2026-05-14)

## Stash Follow-Ups

| ID | Title |
|----|-------|
| B6518EB5 | Phase 5 — CozoDB full parity smoke-test suite |
| B2E64C85 | concerns_edge column naming inconsistency (task_id/symbol_id vs from/to) |
| 7FFC6C35 | CozoDB graph traversal: Datalog-native BFS |
| AB4C6CCE | cozo_vector_test.rs: backend-agnostic vector search tests |

## Architecture Changes

`docs/architecture.md` updated:
- CozoDB schema table: 12 → 20 relations (added all edge tables + aux tables)
- Phase 3+ Roadmap → Implementation Status table (all 4 phases complete)
- `CozoDB Queries` module description updated to reflect full Phase 3-4 implementation

## Compound Library

- `cozo-backend-api-parity-stub-required`: updated to reflect full implementations over stubs
- 4 new learnings identified (composite keys, `:rm` count, BFS filter, `file:` prefix)
