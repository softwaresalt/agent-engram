# Session Memory: 003-unified-code-graph Phase 2

**Date**: 2026-02-16
**Spec**: 003-unified-code-graph
**Phase**: 2 — Foundational (Blocking Prerequisites)
**Branch**: 003-unified-code-graph

## Task Overview

Phase 2 builds the foundational infrastructure that all user stories in the unified code knowledge graph depend on: core models, SurrealDB schema, CRUD queries, tree-sitter parsing service, indexing state management, and comprehensive property/unit tests.

**Tasks completed**: T019–T030 (12 tasks)
**Tasks scope**: Models (5), Schema (1), Queries (1), Parsing (1), State (1), Tests (3)

## Current State

### Files Modified

| File | Change |
|------|--------|
| `src/db/schema.rs` | Added 5 DEFINE TABLE constants for code_file, function, class, interface, and 5 edge tables (calls, imports, inherits_from, defines, concerns). MTREE indexes on embedding fields (384 COSINE). |
| `src/db/mod.rs` | Updated `ensure_schema()` to execute all 5 new schema queries. |
| `src/db/queries.rs` | Added ~500 lines: `CodeGraphQueries` struct with CRUD for 4 node types + 5 edge creation methods + delete/clear helpers. Row types with `into_*()` conversions. |
| `src/services/parsing.rs` | Created ~480 lines: tree-sitter AST parsing for Rust. Extracts functions, structs (as classes), traits (as interfaces), impl blocks (methods + inherits edges), call expressions, use declarations. SHA-256 body hashing. 12 inline `#[cfg(test)]` tests. |
| `src/server/state.rs` | Added `AtomicBool indexing_in_progress` and `RwLock<Option<DateTime<Utc>>> last_indexed_at` fields. CAS-based `try_start_indexing()`, `finish_indexing()`, `is_indexing()`, `last_indexed_at()` methods. |
| `tests/unit/proptest_models.rs` | Added proptest strategies for CodeFile, Function (nested tuples for 13 fields), Class, Interface, CodeEdge + 7 roundtrip tests. |
| `tests/unit/proptest_serialization.rs` | Added 6 serde roundtrip proptests for code graph models. |
| `tests/unit/parsing_test.rs` | Created 15 comprehensive unit tests for parsing service. |
| `Cargo.toml` | Added `[[test]]` entry for `unit_parsing`. |
| `specs/003-unified-code-graph/tasks.md` | Marked T019–T030 as `[X]`. |

### Test Results

- **Library tests**: 68 pass
- **Parsing tests**: 15 pass
- **Proptest models**: 11 pass
- **Proptest serialization**: 16 pass
- **Contract lifecycle**: 9 pass
- **Contract read**: 16 pass
- **Contract write**: 45 pass
- **Integration**: 2 pass
- **Total**: 182 pass, 0 fail (excluding pre-existing benchmark t098)
- **Clippy pedantic**: Clean (exit 0)
- **Format**: Clean (exit 0)

## Important Discoveries

### SurrealDB `.bind()` Requires Owned Values

**Problem**: Using `&str` references in `.bind()` causes `E0521: borrowed data escapes outside of method`.
**Solution**: Always use `.clone()` or `.to_owned()` for string values bound to SurrealDB queries. This is a consistent pattern across the entire codebase.

### tree-sitter 0.24 API Notes

- `node.start_position().row` returns `usize`, not `u32`. Must cast with `as u32`.
- Grammar is loaded via `tree_sitter_rust::LANGUAGE.into()` for the `Language` type.
- `Parser` is `!Send` — must be created inside `spawn_blocking` if used in async context.
- Node kind strings: `function_item`, `struct_item`, `trait_item`, `impl_item`, `call_expression`, `use_declaration`.

### Rust 2024 Edition Match Ergonomics

`ref` modifiers in certain match patterns are disallowed when the default binding mode is not `move`. Remove `ref` from patterns like `if let (Some(ref t), Some(ref s)) = (&a, &b)`.

### proptest Tuple Size Limit

proptest `Strategy` is implemented for tuples up to 12 elements. For larger structures (Function has 13 fields), nest sub-tuples: `((a,b,c,d,e,f,g), (h,i,j,k,l,m))`.

### Clippy Pedantic Patterns

- `similar_names`: Allow with `#[allow(clippy::similar_names)]` on methods with intentionally similar param names (caller_id/callee_id).
- `cast_lossless`: Replace `u32 as i64` with `i64::from(u32)`.
- `cast_possible_wrap`: Extract `u64 as i64` to a let binding with `#[allow(clippy::cast_possible_wrap)]`.
- `needless_raw_string_hashes`: Add `#![allow(clippy::needless_raw_string_hashes)]` at test module level.
- `single_match`: After removing match arms, a single-arm match must be converted to `if`.

## Next Steps

### Phase 3: User Story 1 — Code Structure Indexing (T031–T035)

1. **T031**: Contract test for `index_workspace` (workspace-not-set error, index-in-progress error)
2. **T032**: Code graph indexing orchestration service — file discovery with `ignore` crate, parallel parsing with `spawn_blocking`, character-based token counting, tiered embedding, SSE progress events, batch edge creation
3. **T033**: `index_workspace` tool handler
4. **T034**: Add `index_workspace` to `dispatch()`
5. **T035**: Integration test for full index round-trip

### Key Implementation Decisions for Phase 3

- File discovery should use the `ignore` crate (already depended) for .gitignore-aware traversal
- `CodeGraphConfig.parse_concurrency` of 0 means auto-detect (use available parallelism)
- Token counting is character-based: `body.len() / 4`
- Tiered embedding: Tier 1 (≤ token_limit) gets full embedding, Tier 2 (> token_limit) gets summary-based embedding
- SSE progress events via FR-120 pattern

## Context to Preserve

- Phase 1 committed as `9a7a01f`, pushed to `003-unified-code-graph` branch
- Models (T019–T023) were created in a prior session, verified complete at Phase 2 start
- Pre-existing benchmark test `t098_hydration_1000_tasks_under_500ms` fails in debug mode (6.3s vs 5s threshold) — unrelated to this work
- `CodeGraphQueries` struct is separate from the existing `Queries` struct — they both wrap `Db` but handle different table families
