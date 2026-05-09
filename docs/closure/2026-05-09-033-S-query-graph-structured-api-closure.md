---
title: "033-S query_graph Structured API — Closure"
type: closure
date: 2026-05-09
feature: 048-F
shipment: 033-S
pr: 123
merge_sha: 5cae4a3
status: closed
---

## Summary

Replaced the always-erroring `query_graph` stub with a real BFS-based graph traversal engine.
All 6 backlog items (048-F + 048.001–005-T) delivered via PR #123, merged 2026-05-09.

## Delivered Items

| ID | Title | Status |
|---|---|---|
| 048-F | Query Graph Structured API | archived |
| 048.001-T | Graph Query Model and Parsing | archived |
| 048.002-T | Neighborhood and Transitive Closure | archived |
| 048.003-T | Find Path | archived |
| 048.004-T | MCP Schema and CLI | archived |
| 048.005-T | Backlog Edge Traversal | archived (subsumed by 048.002-T) |

## Files Changed

- `src/models/graph_query.rs` — new `TraversalDirection` enum
- `src/db/cozo_queries.rs` — `resolve_backlog_node` helper, `bfs_directed_impl`, `query_graph_neighborhood`, `transitive_closure`, `find_path`
- `src/tools/read.rs` — full `query_graph` MCP tool handler replacing stub
- `src/shim/tools_catalog.rs` — structured operation schema (replaces Datalog description)
- `src/bin/engram.rs` — structured CLI args for `QueryGraph` subcommand
- `src/cli/commands/search.rs` — multi-arg `run_query_graph` with edge_types filter fix
- `src/services/gate.rs` — doc comment update
- `tests/contract/query_test.rs` — 3 new catalog contract tests
- `tests/unit/graph_query_model_test.rs` — 4 new unit tests (registered in Cargo.toml)
- `Cargo.toml` — registered `unit_graph_query_model` test suite

## Quality Gate Results

- fmt: pass
- clippy: pass
- lib unit tests: 134/134 pass
- contract tests: 17/17 pass (including 3 new)
- unit tests: 4/4 new pass
- CI: green (2 runs, both clean)

## Copilot Review

5 comments received and addressed:
1. `concerns_edge` incoming traversal silently dropped backlog task_id nodes — fixed with `resolve_backlog_node`
2. Backlog edge outgoing nodes used bare ID instead of metadata — fixed
3. Backlog edge incoming nodes used bare ID instead of metadata — fixed
4. `edge_types` CSV parsing included empty strings — fixed with `.filter(|s| !s.is_empty())`
5. BFS behavior not covered by round-trip tests (informational) — acknowledged, tracked as follow-up

All 5 threads resolved via `gh api graphql resolveReviewThread`.

## Key Decisions

- `bfs_directed_impl` kept separate from `bfs_impl` (used by `map_code`/`impact_analysis`) to avoid regressions
- Backlog edges routed through `backlog_edge` table; `concerns_edge` treated as code edge with special column mapping
- Hard cap of 500 nodes applied in MCP handler before DB call
- Default max_depth = 3, default max_nodes = 50
- `backlog_references` API label maps to `edge_type = 'references'` in backlog_edge table

## Rollback

```bash
git revert --no-edit -m 1 5cae4a3
```

## Post-Merge Follow-up

- Integration tests for actual BFS traversal (stashed) — tracked separately
