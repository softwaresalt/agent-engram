# Session Memory: 003-unified-code-graph Phase 1

**Date**: 2025-02-16
**Phase**: 1 — Setup
**Spec**: specs/003-unified-code-graph/

## Task Overview

Phase 1 establishes the foundational infrastructure for the unified code knowledge graph feature: new dependencies, error taxonomy, configuration models, module scaffolding, and embedding model upgrade. Seven tasks (T012–T018) covering dependency management, embedding model switch, error codes, error types, configuration structs, service module declarations, and model module declarations.

## Current State

### Tasks Completed

- **T012**: Added `tree-sitter = "0.24"`, `tree-sitter-rust = "0.23"`, and `ignore = "0.4"` to `Cargo.toml` dependencies
- **T013**: Switched embedding model from `AllMiniLML6V2` to `BGESmallENV15` in `src/services/embedding.rs`, updated `InitOptions` → `TextInitOptions` for fastembed 3.x API
- **T014**: Added 7xxx error code constants to `src/errors/codes.rs`: `PARSE_ERROR` (7001), `UNSUPPORTED_LANGUAGE` (7002), `INDEX_IN_PROGRESS` (7003), `SYMBOL_NOT_FOUND` (7004), `FILE_TOO_LARGE` (7006), `SYNC_CONFLICT` (7007)
- **T015**: Added `CodeGraphError` enum with 6 variants + `#[from]` conversion to `EngramError` in `src/errors/mod.rs`, plus full `to_response()` match arms with JSON details and suggestion fields
- **T016**: Added `CodeGraphConfig` and `EmbeddingConfig` structs to `src/models/config.rs` with serde defaults, added `code_graph` field to `WorkspaceConfig`
- **T017**: Added `pub mod code_graph` and `pub mod parsing` declarations to `src/services/mod.rs`
- **T018**: Added module declarations (`code_edge`, `code_file`, `class`, `function`, `interface`) and re-exports (`CodeEdge`, `Class`, `CodeFile`, `Function`, `Interface`, `CodeGraphConfig`, `EmbeddingConfig`) to `src/models/mod.rs`

### Files Modified

| File | Changes |
|------|---------|
| `Cargo.toml` | Added tree-sitter, tree-sitter-rust, ignore deps |
| `src/errors/codes.rs` | Added 6 constants (7001–7007), updated module docstring |
| `src/errors/mod.rs` | Added `CodeGraphError` enum, `EngramError::CodeGraph` variant, 6 `to_response()` arms |
| `src/models/config.rs` | Added `CodeGraphConfig`, `EmbeddingConfig` structs, `code_graph` field on `WorkspaceConfig` |
| `src/models/mod.rs` | Added 5 module declarations, 7 re-exports, updated docstring |
| `src/services/mod.rs` | Added `pub mod code_graph`, `pub mod parsing`, updated docstring |
| `src/services/embedding.rs` | Model switch `AllMiniLML6V2` → `BGESmallENV15`, `InitOptions` → `TextInitOptions` |
| `tests/unit/proptest_models.rs` | Added `CodeGraphConfig::default()` to `arb_workspace_config()` |

### Files Created

| File | Purpose |
|------|---------|
| `docs/adrs/0006-embedding-model-bge-small.md` | ADR for embedding model switch decision |

### Test Results

- Library tests: 56 passed, 0 failed
- Property tests (proptest): 5 passed, 0 failed
- Contract write tests: 45 passed, 0 failed
- Error codes contract tests: 8 passed, 0 failed
- Clippy: clean (0 warnings with `-D warnings -D clippy::pedantic`)
- Formatting: clean (`cargo fmt --all -- --check` exit 0)

## Important Discoveries

### Decisions Made

1. **Embedding model switch**: Changed from `AllMiniLML6V2` to `BGESmallENV15` based on spec 003 research showing better retrieval accuracy on code benchmarks. Both produce 384-dim vectors, so no schema changes needed. Recorded in ADR 0006.

2. **Error code gap at 7005**: The 7xxx error code range skips 7005, reserving it for future use. Codes are: 7001 (ParseError), 7002 (UnsupportedLanguage), 7003 (IndexInProgress), 7004 (SymbolNotFound), 7006 (FileTooLarge), 7007 (SyncConflict).

3. **CodeGraphConfig defaults**: `max_file_size_bytes` = 1 MB, `parse_concurrency` = 0 (auto-detect CPU cores), `max_traversal_depth` = 5, `max_traversal_nodes` = 50, `supported_languages` = `["rust"]`, `embedding.token_limit` = 512.

### Pre-existing Issues Noted

- `tools/mod.rs` has 3 `unwrap()` calls on `serde_json::to_value()` in production paths (pre-existing, not introduced by Phase 1)
- Pre-existing stub files existed from prior work: `code_file.rs`, `function.rs`, `class.rs`, `interface.rs`, `code_edge.rs`, `parsing.rs`, `code_graph.rs`

### Failed Approaches

- First `multi_replace_string_in_file` call for `src/errors/mod.rs` partially failed, creating a duplicate `CodeGraphError` enum definition. Resolved by removing the duplicate.

## Next Steps

Phase 2 (Foundational) covers tasks T019–T030 (12 tasks):

- T019–T024: Core model structs (`CodeFile`, `Function`, `Class`, `Interface`, `CodeEdge`) with full field definitions, serde attributes, and proptest strategies
- T025–T027: SurrealDB schema definitions (`DEFINE TABLE`, `DEFINE FIELD`) for code graph entities
- T028–T030: Base parsing service, `CodeGraphQueries` struct, and parameter validation

### Key Context for Phase 2

- Stub model files already exist with basic struct definitions — Phase 2 must expand them with full field sets per `data-model.md`
- `CodeGraphConfig` is available via `WorkspaceConfig.code_graph` for parsing service configuration
- Error types `CodeGraphError` and codes are ready for use in parsing and query layers
- The `Queries` struct pattern in `src/db/queries.rs` should be followed for `CodeGraphQueries`

## Context to Preserve

- Spec files: `specs/003-unified-code-graph/` (plan.md, tasks.md, spec.md, data-model.md, research.md, contracts/)
- Configuration: `src/models/config.rs` — `CodeGraphConfig` struct with `EmbeddingConfig` nested
- Error codes: `src/errors/codes.rs` — 7xxx range
- Error types: `src/errors/mod.rs` — `CodeGraphError` enum
- Existing model stubs: `src/models/{code_file,function,class,interface,code_edge}.rs`
- Service stubs: `src/services/{parsing,code_graph}.rs`
