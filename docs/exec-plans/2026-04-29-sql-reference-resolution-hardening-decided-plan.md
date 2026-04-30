---
title: "SQL Reference Resolution Hardening — Decided Plan"
feature: 035-F
shipment: 016-S
pr: 49
merge_commit: 0e4e79a
status: shipped
source_plan: docs/archive/plans/2026-04-29-sql-reference-resolution-hardening-plan.md
---

## Problem

Four gaps in the SQL reference-resolution subsystem identified during 013-S closure:

1. **Missing index** — `references` table had no `references_target` index; `WHERE target = source` was a full scan
2. **N+1 round-trips** — `reresolve_references_edges` issued one lookup + one UPDATE per unresolved edge
3. **Code duplication** — resolution logic copy-pasted in `code_graph.rs` (2 sites) and `queries.rs`
4. **Incomplete resolution** — only exact case-sensitive match; quoted and schema-qualified identifiers fell through

## Decisions Made

### Unit 1: Schema index
- Added `DEFINE INDEX IF NOT EXISTS references_target ON TABLE \`references\` COLUMNS target` to `DEFINE_CODE_EDGES` in `src/db/schema.rs`
- `IF NOT EXISTS` makes the DDL idempotent on existing workspaces

### Unit 2: Batch lookup
- Added `ReresolveResult { resolved, lookups }` return struct to expose lookup counts for testing (`resolved`: edges promoted to a resolved class node; `lookups`: batch DB round-trips issued, ≤ 1 for batch path)
- Batch pre-compute collects all 4 name variants per class: raw, last-segment, stripped, stripped-last
- Reduces lookup round-trips from O(N) to O(1) batch + per-edge fallback only for cache misses
- Per-edge UPDATE still N (unavoidable without SurrealDB stored procedures)

### Unit 3: DRY refactor
- Extracted `resolve_reference_target(&self, qualified_name: &str) -> Result<Option<String>, EngramError>` on `CodeGraphQueries`
- Replaced both inline blocks in `src/services/code_graph.rs` (index path + sync path)
- `reresolve_references_edges` uses the helper for per-edge fallback after batch miss

### Unit 4: Resolution heuristics
- Candidates-list pattern: build `Vec` of [raw, last-segment, stripped, stripped-last], dedup in insertion order
- Try exact match then case-insensitive match across all candidates in order
- `get_class_by_name_ci`: Rust-side case-insensitive match after full table scan (SurrealDB `string::lowercase()` in WHERE clauses unreliable in embedded KV)
- `strip_sql_quotes`: strips surrounding `"..."` and `[...]` pairs
- Removed `!contains('.')` guard from per-edge fallback — all batch misses reach the full resolver

## Constraints and Rationale

- **No server-side lowercase filtering**: SurrealDB `string::lowercase()` in WHERE clauses does not filter correctly against indexed fields in the embedded KV store; Rust-side post-fetch filtering required
- **cozo-backend stubs**: `resolve_reference_target` and `get_class_by_name_ci` added as `Ok(None)` stubs in `cozo_queries.rs`; `get_class_by_name_ci` needs `#[allow(dead_code)]` because the cozo stub for `resolve_reference_target` never calls it
- **`--all-targets` clippy**: CI runs `cargo clippy --all-targets`; test-file pedantic violations (doc_markdown, similar_names) only caught with the flag

## Files Modified

- `src/db/schema.rs` — `references_target` index
- `src/db/queries.rs` — `ReresolveResult`, `ClassNameIdRow`, `get_class_by_name_ci`, `resolve_reference_target`, `strip_sql_quotes`, refactored `reresolve_references_edges`
- `src/db/cozo_queries.rs` — API parity stubs
- `src/services/code_graph.rs` — 2 inline blocks replaced with `resolve_reference_target`
- `tests/contract/references_edge_test.rs` — 5 new contract tests

## Rejected Alternatives

- **Server-side `string::lowercase()` WHERE filter**: silently returns zero rows in embedded KV; rejected
- **`!contains('.')` guard on fallback**: prevented `public."Users"` from reaching the resolver; removed
- **Inline candidates resolution without candidates-list dedup**: duplication and order-sensitivity; replaced by ordered IndexSet dedup
