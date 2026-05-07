---
title: "SQL Parser Decided Plan — 034-F"
description: "Decided implementation for SQL file indexing via tree-sitter-sequel 0.3"
source_plan: docs/exec-plans/2026-04-26-sql-parser-plan.md
feature: 034-F
shipment: 013-S
status: implemented
merged: aedc3e0
decided_at: 2026-04-27
---

## Decision Summary

Add SQL file indexing (`Language::Sql`) via `tree-sitter-sequel 0.3` (ABI 15).
Follow the swift.rs template exactly. New `ExtractedEdge::References { source, target }`
variant required (not pre-existing).

## Requirements (all implemented)

| Requirement | Implementation | Status |
| --- | --- | --- |
| SQL files indexed during workspace sync | `"sql"` in `language_from_path()` | ✅ |
| `CREATE TABLE`/`VIEW` → Class | `create_table`/`create_view` node kinds | ✅ |
| `CREATE FUNCTION` → Function | `create_function` node kind | ✅ |
| `CREATE PROCEDURE` → graceful degradation | ERROR node in grammar 0.3; 0 symbols, no crash | ✅ |
| `INSERT INTO` / `SELECT FROM` → References | `insert`/`from` node kinds | ✅ |
| Unit tests (4 core + 4 secondary) | 10 tests total in `parsing_test.rs` | ✅ |
| IPC integration test | `t034_005_sql_create_table_indexed_via_ipc` | ✅ |
| No regressions | Full `cargo test` passed | ✅ |

## Implementation Units (executed)

| Unit | Task | Commit |
| --- | --- | --- |
| 1 — dep + enum + References variant | 034.001-T | `819fa8d` |
| 2a — core unit tests (TDD red) | 034.002-T | `819fa8d` |
| 2b — secondary unit tests (TDD red) | 034.003-T | `819fa8d` |
| 3 — sql.rs implementation | 034.004-T | `819fa8d` |
| 4 — language_from_path wiring + IPC test | 034.005-T | `819fa8d` |

## Key Constraints (from plan review)

- `create_function` and `create_procedure` are separate node kinds — NOT combined
- `ExtractedEdge::References` arms must be added to ALL exhaustive match sites in `code_graph.rs`
- SQL tests must follow the existing pattern in `parsing_test.rs` (no raw `unwrap()`)
- ABI 15 grammar requires tree-sitter 0.25 runtime (already present in workspace)

## Deferred Scope (stashed)

- 19D78639 — `CREATE PROCEDURE` grammar support when sequel ≥ 0.4
- F15C561F — Resolve `FROM` references to known `Class` nodes in graph
- 8232DE58 — Multi-schema `schema.table` reference parsing
