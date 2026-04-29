---
title: "Decided Plan — 033-F SQL Parser Enhancements"
date: 2026-04-28
feature: 033-F
shipment: 013-S
status: "shipped"
source: "docs/archive/plans/2026-04-28-033-F-sql-parser-enhancements-plan.md"
review_result: "PASS (4 P2 advisories)"
---

## Decided Implementation — 033-F

### Problem

SQL parser shipped in 034-F emits `References` edges with raw identifier strings, but:
1. Code graph service ignores them (`code_graph.rs` had no-op `References` arms)
2. Schema-qualified names (e.g., `FROM public.users`) were truncated to just `public`
3. JOIN-referenced tables were not extracted at all

### Final Decisions

#### Unit 1: `references` DB Table (033.004-T)

- **Table**: `DEFINE TABLE IF NOT EXISTS \`references\` SCHEMAFULL` (NOT `TYPE RELATION`)
  - Rationale: `TYPE RELATION` drops edges silently when target record doesn't exist in SurrealDB v2.6
  - Fields: `source`, `target`, `qualified_name`, `created_at`, `INDEX idx_references_source ON source`
- **Operations**: `create_references_edge(source_id, target_id, qualified_name)` + `delete_edges_from_file("references", file_id)`
- **Allowlist**: `ALLOWED_EDGE_TABLES` const prevents SQL injection via table name parameter
- **CozoDB**: Mirror stub methods in `cozo_queries.rs` returning `Err(backend_err())`

#### Unit 2: Code Graph Wiring (033.001-T)

- **Resolution**: In both `index_workspace` and `sync_workspace`:
  - `delete_edges_from_file("references", &file_id)` cleanup added alongside `defines` cleanup
  - `ExtractedEdge::References` arm replaced: call `get_class_by_name(&target)`
  - If resolved: create edge to class node
  - If unresolved: create self-loop edge with `qualified_name = raw_identifier`
- **Qualified-name fallback**: For `"public.users"`, try full name first, then `"users"` fallback (deliberation Finding 2)
- **Post-pass**: `reresolve_references_edges()` after all files — global scope, re-resolves ALL self-loops
  - N+1 round-trips accepted as correctness-first; optimization deferred (stash 8C651D9F)
- **Counter**: `result.edges_created` incremented

#### Unit 3: Parser Enhancement (033.002-T)

- **Schema-qualified names**: Collect ALL `identifier` children of `object_reference` and join with `.`
  - Applied to `extract_from_references`, `extract_insert_references`, and `extract_sql_name` (P2-1 advisory: accepted for consistency)
- **JOIN tables**: Descend into `join`/`cross_join`/`lateral_join`/`lateral_cross_join` child nodes of `from`; extract `relation` child from each (tree-sitter-sequel 0.3 grammar structure)
- **Dependency**: Independent of Unit 2 (deliberation Finding 1 — false dependency removed)

### Rejected / Deferred

- `TYPE RELATION` schema — silently drops edges; rejected
- `SELECT *` in queries — breaks serde_json on non-empty SurrealDB tables; rejected
- 033.003-T (CREATE PROCEDURE) — blocked on upstream tree-sitter-sequel grammar; deferred
- Batch-UPDATE for reresolve — deferred to stash 8C651D9F
- `INDEX ON target` — deferred to stash E145945C
- DRY refactor of resolution logic — deferred to stash DA9D4948
- Full Class node resolution — deferred to stash B0903A71

### Constraints Preserved

1. `references` is a SurrealQL reserved word — always backtick-escape (P2-3 advisory, critical)
2. `ALLOWED_EDGE_TABLES` allowlist must be maintained
3. `cozo_queries.rs` must mirror every `queries.rs` method signature
4. Never use `SELECT *` in any SurrealDB query (serde_json incompatibility)
