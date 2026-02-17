# Session Memory: 003-unified-code-graph Phase 3

**Date**: 2026-02-16
**Phase**: 3 — US1: Code Structure Indexing
**Spec**: `specs/003-unified-code-graph/`
**Branch**: `003-unified-code-graph`

## Task Overview

Phase 3 implements the core code graph indexing pipeline: file discovery, AST parsing via tree-sitter, symbol extraction, tiered embedding, edge creation, and DB persistence. Five tasks (T031–T035) covering contract tests, service implementation, tool handler, dispatch wiring, and integration tests.

## Completed Tasks

| Task | Description | Status |
|------|-------------|--------|
| T031 | Contract tests for `index_workspace` (1003, 7003) in write_test.rs | ✅ Complete |
| T032 | Code graph indexing orchestration service in code_graph.rs (~520 lines) | ✅ Complete |
| T033 | `index_workspace` tool handler in write.rs | ✅ Complete |
| T034 | Dispatch arm in mod.rs | ✅ Complete |
| T035 | Integration tests (9 tests) in code_graph_test.rs | ✅ Complete |

## Files Modified

- `src/services/code_graph.rs` — NEW: Full indexing pipeline (~520 lines)
- `src/services/mod.rs` — Added `pub mod code_graph;`
- `src/tools/write.rs` — Added `index_workspace` handler + `IndexWorkspaceParams`
- `src/tools/mod.rs` — Added `"index_workspace"` dispatch arm
- `src/db/schema.rs` — Backtick-escaped `function` table name (reserved keyword fix)
- `src/db/queries.rs` — Backtick-escaped `function` in SELECT/DELETE queries
- `tests/contract/write_test.rs` — 2 new contract tests (47 total pass)
- `tests/integration/code_graph_test.rs` — NEW: 9 integration tests
- `Cargo.toml` — Added `[[test]]` entry for `integration_code_graph`
- `specs/003-unified-code-graph/tasks.md` — Marked T031–T035 complete
- `docs/adrs/0007-surrealdb-function-reserved-keyword.md` — NEW ADR

## Important Discoveries

### SurrealDB v2 Reserved Keyword: `function`

The table name `function` is a reserved keyword in SurrealDB v2 (used by `DEFINE FUNCTION` for stored procedures). Raw SurrealQL statements like `DELETE FROM function WHERE ...` fail with a parse error. The fix is backtick-escaping: `` `function` ``. Parameterized queries using `Thing::from(("function", id))` are unaffected because the table name is inside the serialized Thing, not parsed as SurrealQL text. See ADR 0007.

### Integration Test Pattern for code_graph

Tests create temp directories with sample Rust files via `write_sample_file()`, compute workspace hash with `sha256_hex()`, then call `code_graph::index_workspace()` directly. Each test gets its own temp dir and unique workspace hash, avoiding DB namespace collisions.

### File Discovery with `ignore` Crate

`ignore::WalkBuilder` respects `.gitignore` automatically. In test temp dirs without `.git`, it still walks all files correctly. The `hidden(true)` flag means "skip hidden files" (files/dirs starting with `.`).

### Clippy Pedantic Catches

- `match_same_arms`: Merged `Imports` and `Defines` arms in edge processing
- `cast_possible_truncation`: u128→u64 for duration_ms requires `#[allow]`
- `unnecessary_wraps`: `discover_files` changed to return `Vec<PathBuf>` directly
- `needless_question_mark`: `Ok(x?)` simplified to `x`
- `format_collect`: `.map(format!).collect()` changed to `.fold()` with `writeln!`
- `needless_raw_string_hashes`: `r#"..."#` simplified to `r"..."` when no hashes needed

## Test Results

| Suite | Count | Status |
|-------|-------|--------|
| lib (unit) | 68 | ✅ Pass |
| contract_error_codes | 8 | ✅ Pass |
| contract_lifecycle | 9 | ✅ Pass |
| contract_read | 16 | ✅ Pass |
| contract_write | 47 | ✅ Pass |
| integration_code_graph | 9 | ✅ Pass |
| integration_connection | 2 | ✅ Pass |
| unit_parsing | 15 | ✅ Pass |
| unit_proptest | 11 | ✅ Pass |
| unit_proptest_serialization | 16 | ✅ Pass |
| integration_benchmark | 4/6 | ⚠️ 2 pre-existing benchmark failures |
| **Total** | **201/203** | ✅ All new code passes |

- `cargo clippy --all-targets -- -D warnings -D clippy::pedantic`: CLEAN ✅
- `cargo fmt --all -- --check`: CLEAN ✅

## Next Steps

- **Phase 4** (US2: Graph-Backed Dependency Walking): Implement `map_code` tool with BFS traversal, vector search fallback, and `list_symbols` for agent discoverability.
- **Phase 5** (US3: Semantic Code Search): Implement embedding-powered `search_code` tool with result ranking and filters.
- Pre-existing benchmark failures (t098, t100) should be investigated separately — they fail in debug mode due to timing thresholds.

## Context to Preserve

- `src/services/code_graph.rs` contains the full indexing pipeline. Key entry point: `index_workspace(ws_path, ws_id, config, force)`.
- `src/db/queries.rs::CodeGraphQueries` has all code graph CRUD operations.
- `src/db/schema.rs` defines 5 code graph tables + 5 edge tables.
- `src/services/parsing.rs` has tree-sitter parsing logic.
- `src/services/embedding.rs` has the embedding API (feature-gated stub).
- `src/models/` has `CodeFile`, `Function`, `Class`, `Interface`, `CodeGraphConfig`.
- The `function` table name MUST always be backtick-escaped in raw SurrealQL.
