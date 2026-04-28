---
title: "Closure Summary — 034-F SQL File Indexing via tree-sitter-sequel"
date: 2026-04-26
mode: post-merge
feature: 034-F
shipment: 013-S
pr_feature: 35
pr_stage: 34
merge_sha_feature: 305b28f
merge_sha_stage: aedc3e0
source_files:
  - docs/closure/2026-04-26-034-F-sql-parser-closure.md
  - docs/closure/2026-04-26-034-F-sql-parser-post-merge-closure.md
---

## Change Summary

SQL file indexing added to the engram code graph via `tree-sitter-sequel 0.3`.
Extracts `Class` symbols for `CREATE TABLE`/`CREATE VIEW`, `Function` for `CREATE FUNCTION`,
and `References` edges for `FROM` and `INSERT INTO`. Graceful degradation on unsupported syntax.

**Files**: `Cargo.toml`, `src/services/parsing.rs`, `src/services/parsing/sql.rs` (new),
`src/services/code_graph.rs`, `tests/unit/parsing_test.rs` (+10 tests),
`tests/integration/lang_ipc_indexing_test.rs` (+1 test).

## CI Status

✅ cozo-backend (1m18s), surreal-backend (8m14s). One CI fix commit (`d243dd2`) for
`clippy::items_after_statements` in test helpers.

## Shipment Closure

013-S archived: 034-F + 5 tasks. Pre/post reconciliation: PROCEED.
Stash entry `8AC6828D` marked `harvested`.

## Knowledge Graduated

- `docs/architecture.md`: Language enum updated, Multi-Language Parsing section updated
- `docs/decisions/2026-04-24-sql-grammar-spike.md`: SQL grammar spike findings
- `docs/decisions/2026-04-26-sql-parser-deliberation.md`: decision record

## Healthy Signals

- `.sql` files appear in `list_symbols` output after `sync_workspace`
- `CREATE TABLE t` → one `Class` symbol; `CREATE FUNCTION f` → one `Function` symbol
- `SELECT ... FROM t` → `References` edge
- No daemon panics on unsupported SQL syntax

## Rollback Trigger

Revert `305b28f` if daemon panics or returns `EngramError` for any `.sql` file.

## Follow-Up Items (stashed)

IDs `19D78639`, `F15C561F`, `8232DE58`:
1. `CREATE PROCEDURE` grammar support (upstream 0.3 limitation)
2. SELECT reference resolution to known Class nodes
3. Multi-schema `schema.table` dotted references

## Post-Merge Status

CLOSED. All items archived. Validation window: 72h after next binary rebuild.
