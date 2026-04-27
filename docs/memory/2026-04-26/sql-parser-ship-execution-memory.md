---
title: SQL parser Ship execution memory
date: 2026-04-26
session: c0c7e503-873e-4654-a5ab-5914f4585e57
shipment: 013-S
feature: 034-F
branch: feature/034-F-sql-parser
pr: "35"
---

# SQL Parser Ship Execution Memory

## Tasks completed

- 034.001-T: Cargo.toml dep, Language::Sql, ExtractedEdge::References, sql.rs stub, code_graph.rs no-op arms, language_from_path
- 034.002-T: Core unit tests (CREATE TABLE, CREATE FUNCTION, multi-statement, empty)
- 034.003-T: Secondary unit tests (CREATE VIEW, CREATE PROCEDURE graceful degradation, INSERT refs, SELECT refs)
- 034.004-T: sql.rs implementation with correct node kinds from debug inspection
- 034.005-T: language_from_path wiring + SQL IPC integration test

## Files modified

- `Cargo.toml` — added `tree-sitter-sequel = "0.3"`
- `Cargo.lock` — locked to v0.3.11
- `src/services/parsing.rs` — Language::Sql, ExtractedEdge::References, pub parse_sql_source
- `src/services/parsing/sql.rs` — NEW: full SQL parser (~210 lines)
- `src/services/code_graph.rs` — References match arms, "sql" in language_from_path
- `tests/unit/parsing_test.rs` — 10 new SQL tests (8 scenario + 2 debug)
- `tests/integration/lang_ipc_indexing_test.rs` — t034_005_sql_create_table_indexed_via_ipc

## Key decisions

- Node kind names for tree-sitter-sequel 0.3 differ from assumed: `create_table` (not `create_table_statement`), `from` is a sibling inside `statement` (not inside `select`), etc.
- Grammar wraps all statements in `statement` containers — must iterate program > statement > child
- CREATE PROCEDURE parses as ERROR node in this grammar version — tested for graceful degradation (0 symbols, no crash)
- `extract_sql_name` extracts from `object_reference` > `identifier` child (not via field name API)

## Quality gates

All three gates passed on final commit:
- `cargo fmt --all -- --check` ✅
- `cargo clippy -- -D warnings -D clippy::pedantic` ✅
- `cargo test --test unit_parsing` ✅ 46 passed, 0 failed

## Commit

`819fa8d` on `feature/034-F-sql-parser`

## PR

#35 — targeting `stage/034-F-sql-parser`

## Next steps

- PR #35 review and merge
- shipment-reconcile post-mode after merge
- compound-refresh for tree-sitter-sequel grammar node kind patterns
