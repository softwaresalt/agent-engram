# Session Memory: 003-unified-code-graph Phase 4

**Date**: 2025-02-16
**Spec**: 003-unified-code-graph
**Phase**: 4 — User Story 2: Graph-Backed Dependency Walking
**Branch**: 003-unified-code-graph
**Status**: Complete — all 6 tasks (T036–T041) implemented and verified

## Tasks Completed

| Task | Description | Status |
|------|-------------|--------|
| T036 | Contract test for `map_code` (workspace-not-set 1003, empty graph fallback) | Done |
| T037 | Contract test for `list_symbols` (workspace-not-set 1003, empty graph 7004) | Done |
| T038 | BFS traversal queries + symbol listing/filtering queries in queries.rs | Done |
| T039 | `map_code` tool handler with exact-name lookup, BFS, vector-search fallback | Done |
| T040 | `list_symbols` tool handler with filters and pagination | Done |
| T041 | `map_code` and `list_symbols` dispatch arms in mod.rs | Done |

## Files Modified

| File | Changes |
|------|---------|
| `src/db/queries.rs` | +~600 lines: `SymbolMatch`, `BfsEdge`, `BfsResult`, `SymbolFilter`, `SymbolListEntry`, `SymbolListResult` types; `find_symbols_by_name()`, `bfs_neighborhood()`, `get_outbound_edges()`, `get_inbound_edges()`, `resolve_symbol()`, `list_symbols()`, `count_all_symbols()`, `vector_search_symbols()` on `CodeGraphQueries`; `cosine_similarity()`, `parse_node_id()`, `parse_thing()` helpers. Fixed duplicate `CountRow` (existing field is `count: u64`). |
| `src/tools/read.rs` | +~200 lines: `MapCodeParams`, `ListSymbolsParams` structs; `map_code()`, `list_symbols()`, `symbol_match_to_json()` functions. Three response modes: single_match (root + BFS neighbors), multiple_matches, fallback (vector search). |
| `src/tools/mod.rs` | +2 match arms: `"map_code"` → `read::map_code`, `"list_symbols"` → `read::list_symbols` |
| `tests/contract/read_test.rs` | +4 tests: `contract_map_code_requires_workspace`, `contract_map_code_empty_graph_uses_fallback`, `contract_list_symbols_requires_workspace`, `contract_list_symbols_empty_graph_returns_error` |
| `specs/003-unified-code-graph/tasks.md` | Marked T036–T041 as `[x]` |
| `docs/adrs/0008-bfs-graph-traversal.md` | New ADR: application-level BFS decision |
| `docs/adrs/0009-vector-search-fallback.md` | New ADR: vector search fallback for `map_code` |

## Decisions Made

1. **Application-level BFS** (ADR-0008): BFS traversal at application level with 2 queries per depth level (outbound + inbound edges), rather than SurrealDB recursive graph operators, for full control over truncation and bidirectional edge collection.
2. **Vector search fallback** (ADR-0009): When exact-name lookup fails, `map_code` falls back to cosine-similarity vector search on symbol embeddings. If the embedding model is not loaded (stub mode), returns empty fallback result rather than erroring.
3. **CountRow reuse**: Discovered existing `CountRow { count: u64 }` in queries.rs — reused it for symbol counting instead of adding a duplicate struct.
4. **Clippy fixes**: Applied `let...else` pattern, removed redundant closure, used `.clamp(1, 500)` instead of `.min().max()`.

## Known Issues

- `t098_hydration_1000_tasks_under_500ms` benchmark test continues to fail in debug mode (~6.6s vs 5s threshold). Pre-existing issue, not related to Phase 4.
- `fastembed` TLS issue remains pending — embedding model runs in stub mode during tests.

## Test Results

- 68 library tests: PASSED
- 84 contract tests (8 error_codes + 9 lifecycle + 20 read + 47 write): PASSED
- 5/6 benchmark tests: PASSED (1 pre-existing failure)
- Clippy pedantic: CLEAN
- rustfmt: CLEAN

## Next Steps

- Phase 5 (User Story 3 — Impact Analysis): `impact_analysis` tool using BFS from the changed symbol outward, collecting all dependent symbols and their files.
- Phase 6 (User Story 4 — Change Awareness): `get_stale_contexts` tool combining Git diff parsing with graph traversal to identify tasks whose context references modified code.
